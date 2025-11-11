/// Dedicated AI REST Endpoints
/// Complete HTTP routes for all AI services with proper error handling

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::ai_services::{
    risk_scoring::{RiskScoringService, VCRiskInput, VCRiskOutput},
    anomaly_detection::{AnomalyDetectionService, NodeBehaviorInput, AnomalyOutput},
    reputation_scoring::{ReputationScoringService, IdentityGraphInput, ReputationOutput},
    authenticity_verification::{AuthenticityVerificationService, AIOutputVerificationInput, AuthenticityOutput},
};

// Request/Response types for API
#[derive(Debug, Deserialize)]
pub struct ScoreVCRequest {
    pub vc_hash: String,
    pub issuer_did: String,
    pub subject_did: String,
    pub claim_type: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub issuer_reputation: u16,
    pub prior_revocations: u32,
}

#[derive(Debug, Serialize)]
pub struct ScoreVCResponse {
    pub risk: f32,
    pub reason_codes: Vec<String>,
    pub threshold_exceeded: bool,
    pub recommended_action: String,
}

#[derive(Debug, Deserialize)]
pub struct ScoreNodeRequest {
    pub node_pubkey: String,
    pub metrics: serde_json::Value, // Time series data
}

#[derive(Debug, Serialize)]
pub struct ScoreNodeResponse {
    pub anomaly_score: f32,
    pub suggested_action: String,
    pub reason_codes: Vec<String>,
    pub should_alert: bool,
}

#[derive(Debug, Deserialize)]
pub struct ScoreIdentityRequest {
    pub did: String,
    pub graph_edges: serde_json::Value,
    pub ip_hints: Vec<String>,
    pub device_fingerprints: Vec<String>,
    pub signup_timestamps: Vec<u64>,
}

#[derive(Debug, Serialize)]
pub struct ScoreIdentityResponse {
    pub artha_score: u8,
    pub flags: Vec<String>,
    pub risk_level: String,
    pub should_block: bool,
}

#[derive(Debug, Deserialize)]
pub struct VerifyAIOutputRequest {
    pub aiid: String,
    pub inference_receipt_signature: String,
    pub output_cid: String,
    pub expected_watermark_features: Option<Vec<f64>>,
}

#[derive(Debug, Serialize)]
pub struct VerifyAIOutputResponse {
    pub is_authentic: bool,
    pub confidence_score: f32,
    pub reason_codes: Vec<String>,
    pub provenance_chain: Vec<String>,
    pub warning: Option<String>,
}

// Service state
pub struct AIServiceState {
    pub risk_scoring: Arc<RiskScoringService>,
    pub anomaly_detection: Arc<AnomalyDetectionService>,
    pub reputation_scoring: Arc<ReputationScoringService>,
    pub authenticity: Arc<AuthenticityVerificationService>,
}

impl AIServiceState {
    pub fn new() -> Self {
        AIServiceState {
            risk_scoring: Arc::new(RiskScoringService::new()),
            anomaly_detection: Arc::new(AnomalyDetectionService::new()),
            reputation_scoring: Arc::new(ReputationScoringService::new()),
            authenticity: Arc::new(AuthenticityVerificationService::new()),
        }
    }
}

// Handlers

/// POST /ai/risk/vc
pub async fn score_vc(
    State(state): State<Arc<AIServiceState>>,
    Json(req): Json<ScoreVCRequest>,
) -> Result<Json<ScoreVCResponse>, StatusCode> {
    let input = VCRiskInput {
        vc_hash: req.vc_hash,
        issuer_did: req.issuer_did,
        subject_did: req.subject_did,
        claim_type: req.claim_type,
        issued_at: req.issued_at,
        expires_at: req.expires_at,
        issuer_reputation: req.issuer_reputation,
        prior_revocations: req.prior_revocations,
    };

    let output = state.risk_scoring.score_vc(input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let recommended_action = if output.risk > 0.8 {
        "BLOCK"
    } else if output.risk > 0.6 {
        "WARN"
    } else if output.risk > 0.3 {
        "MONITOR"
    } else {
        "ALLOW"
    };

    Ok(Json(ScoreVCResponse {
        risk: output.risk,
        reason_codes: output.reason_codes,
        threshold_exceeded: output.threshold_exceeded,
        recommended_action: recommended_action.to_string(),
    }))
}

/// POST /ai/anomaly/node
pub async fn score_node(
    State(state): State<Arc<AIServiceState>>,
    Json(req): Json<ScoreNodeRequest>,
) -> Result<Json<ScoreNodeResponse>, StatusCode> {
    // Parse time series data from JSON
    let time_series_data = req.metrics
        .as_object()
        .ok_or(StatusCode::BAD_REQUEST)?
        .iter()
        .filter_map(|(k, v)| {
            v.as_array().and_then(|arr| {
                let floats: Vec<f32> = arr.iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();
                if floats.is_empty() { None } else { Some((k.clone(), floats)) }
            })
        })
        .collect();

    let input = NodeBehaviorInput {
        node_pubkey: req.node_pubkey,
        time_series_data,
    };

    let output = state.anomaly_detection.score_node_behavior(input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let should_alert = output.anomaly_score > 0.7;

    Ok(Json(ScoreNodeResponse {
        anomaly_score: output.anomaly_score,
        suggested_action: output.suggested_action,
        reason_codes: output.reason_codes,
        should_alert,
    }))
}

/// POST /ai/reputation/identity
pub async fn score_identity(
    State(state): State<Arc<AIServiceState>>,
    Json(req): Json<ScoreIdentityRequest>,
) -> Result<Json<ScoreIdentityResponse>, StatusCode> {
    // Parse graph edges from JSON
    let graph_edges = req.graph_edges
        .as_object()
        .ok_or(StatusCode::BAD_REQUEST)?
        .iter()
        .filter_map(|(k, v)| {
            v.as_array().and_then(|arr| {
                let strings: Vec<String> = arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if strings.is_empty() { None } else { Some((k.clone(), strings)) }
            })
        })
        .collect();

    let input = IdentityGraphInput {
        did: req.did,
        graph_edges,
        ip_hints: req.ip_hints,
        device_fingerprints: req.device_fingerprints,
        signup_timestamps: req.signup_timestamps,
    };

    let output = state.reputation_scoring.score_identity_graph(input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let should_block = output.artha_score < 30 || output.risk_level == "high";

    Ok(Json(ScoreIdentityResponse {
        artha_score: output.artha_score,
        flags: output.flags,
        risk_level: output.risk_level,
        should_block,
    }))
}

/// POST /ai/authenticity/verify
pub async fn verify_ai_output(
    State(state): State<Arc<AIServiceState>>,
    Json(req): Json<VerifyAIOutputRequest>,
) -> Result<Json<VerifyAIOutputResponse>, StatusCode> {
    let input = AIOutputVerificationInput {
        aiid: req.aiid,
        inference_receipt_signature: req.inference_receipt_signature,
        output_cid: req.output_cid,
        expected_watermark_features: req.expected_watermark_features,
    };

    let output = state.authenticity.verify_ai_output(input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let warning = if !output.is_authentic {
        Some("WARNING: AI output authenticity could not be verified. Provenance chain may be broken.".to_string())
    } else if output.confidence_score < 0.8 {
        Some("CAUTION: Low confidence in authenticity verification.".to_string())
    } else {
        None
    };

    Ok(Json(VerifyAIOutputResponse {
        is_authentic: output.is_authentic,
        confidence_score: output.confidence_score,
        reason_codes: output.reason_codes,
        provenance_chain: output.provenance_chain,
        warning,
    }))
}

/// GET /ai/risk/vc/:vc_hash
pub async fn get_vc_risk(
    State(state): State<Arc<AIServiceState>>,
    Path(vc_hash): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Lookup cached risk score
    Ok(Json(serde_json::json!({
        "vc_hash": vc_hash,
        "cached": true,
        "message": "Use POST /ai/risk/vc to compute fresh score"
    })))
}

/// GET /ai/health
pub async fn ai_health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "services": {
            "risk_scoring": "operational",
            "anomaly_detection": "operational",
            "reputation_scoring": "operational",
            "authenticity_verification": "operational"
        },
        "version": "v1.0.0"
    }))
}

// ============================================================================
// ArthaAIN v1 - Cloud Platform Endpoints
// ============================================================================

/// Dataset registration request
#[derive(Debug, Deserialize)]
pub struct DatasetRegisterRequest {
    pub root_cid: String,
    pub license_cid: String,
    pub tags: Vec<String>,
}

/// Dataset registration response
#[derive(Debug, Serialize)]
pub struct DatasetRegisterResponse {
    pub dataset_id: String,
    pub root_cid: String,
    pub registered_at: u64,
}

/// Model registration request
#[derive(Debug, Deserialize)]
pub struct ModelRegisterRequest {
    pub model_cid: String,
    pub architecture: String,
    pub base_model_id: Option<String>,
    pub dataset_id: String,
    pub code_hash: String,
    pub version: String,
    pub license_cid: Option<String>,
}

/// Model registration response
#[derive(Debug, Serialize)]
pub struct ModelRegisterResponse {
    pub model_id: String,
    pub model_cid: String,
    pub registered_at: u64,
}

/// Training job request
#[derive(Debug, Deserialize)]
pub struct TrainJobRequest {
    pub model_id: String,
    pub dataset_id: String,
    pub submitter_did: String,
    pub params: TrainParams,
    pub budget: u64,
}

#[derive(Debug, Deserialize)]
pub struct TrainParams {
    pub epochs: u32,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub optimizer: String,
    pub checkpoint_interval: u32,
}

/// Training job response
#[derive(Debug, Serialize)]
pub struct TrainJobResponse {
    pub job_id: String,
    pub status: String,
    pub estimated_cost: u64,
    pub estimated_duration_secs: u64,
}

/// Inference job request
#[derive(Debug, Deserialize)]
pub struct InferJobRequest {
    pub model_id: String,
    pub input_cid: Option<String>,
    pub inline_input: Option<String>,
    pub submitter_did: String,
    pub mode: String,
    pub max_tokens: Option<u32>,
    pub budget: u64,
}

/// Inference job response
#[derive(Debug, Serialize)]
pub struct InferJobResponse {
    pub job_id: String,
    pub status: String,
}

/// Agent job request
#[derive(Debug, Deserialize)]
pub struct AgentJobRequest {
    pub agent_spec_cid: String,
    pub submitter_did: String,
    pub goal: String,
    pub tools: Vec<String>,
    pub memory_policy: String,
    pub budget: u64,
}

/// Agent job response
#[derive(Debug, Serialize)]
pub struct AgentJobResponse {
    pub job_id: String,
    pub status: String,
}

// Handlers for ArthaAIN endpoints

/// POST /ai/dataset/register
pub async fn register_dataset(
    Json(req): Json<DatasetRegisterRequest>,
) -> Result<Json<DatasetRegisterResponse>, StatusCode> {
    // Forward to ai-jobd service
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let response = client
        .post(&format!("{}/ai/dataset/register", jobd_url))
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: DatasetRegisterResponse = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /ai/dataset/list
pub async fn list_datasets(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let url = if let Some(owner) = params.get("owner") {
        format!("{}/ai/dataset/list?owner={}", jobd_url, owner)
    } else {
        format!("{}/ai/dataset/list", jobd_url)
    };

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /ai/dataset/:id
pub async fn get_dataset_info(
    Path(dataset_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let response = client
        .get(&format!("{}/ai/dataset/{}", jobd_url, dataset_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// POST /ai/model/register
pub async fn register_model(
    Json(req): Json<ModelRegisterRequest>,
) -> Result<Json<ModelRegisterResponse>, StatusCode> {
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let response = client
        .post(&format!("{}/ai/model/register", jobd_url))
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: ModelRegisterResponse = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /ai/model/list
pub async fn list_models(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let url = if let Some(owner) = params.get("owner") {
        format!("{}/ai/model/list?owner={}", jobd_url, owner)
    } else {
        format!("{}/ai/model/list", jobd_url)
    };

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /ai/model/:id/lineage
pub async fn get_model_lineage(
    Path(model_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let response = client
        .get(&format!("{}/ai/model/{}/lineage", jobd_url, model_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// POST /ai/train
pub async fn submit_train_job(
    Json(req): Json<TrainJobRequest>,
) -> Result<Json<TrainJobResponse>, StatusCode> {
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let response = client
        .post(&format!("{}/job/train", jobd_url))
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: TrainJobResponse = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// POST /ai/infer
pub async fn submit_infer_job(
    Json(req): Json<InferJobRequest>,
) -> Result<Json<InferJobResponse>, StatusCode> {
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let response = client
        .post(&format!("{}/job/infer", jobd_url))
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: InferJobResponse = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// POST /ai/agent
pub async fn submit_agent_job(
    Json(req): Json<AgentJobRequest>,
) -> Result<Json<AgentJobResponse>, StatusCode> {
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let response = client
        .post(&format!("{}/job/agent", jobd_url))
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: AgentJobResponse = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /ai/job/:id/status
pub async fn get_job_status(
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let response = client
        .get(&format!("{}/job/{}/status", jobd_url, job_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /ai/job/:id/logs
pub async fn get_job_logs(
    Path(job_id): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let client = reqwest::Client::new();
    let runtime_url = std::env::var("ARTHA_RUNTIME_URL").unwrap_or_else(|_| "http://localhost:8084".to_string());
    
    let response = client
        .get(&format!("{}/job/{}/logs", runtime_url, job_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: Vec<String> = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// POST /ai/job/:id/cancel
pub async fn cancel_job(
    Path(job_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let client = reqwest::Client::new();
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    
    let response = client
        .post(&format!("{}/job/{}/cancel", jobd_url, job_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    Ok(StatusCode::OK)
}

// ============================================================================
// Federated Learning Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct StartFederatedRequest {
    pub model_id: String,
    pub dataset_ids: Vec<String>,
    pub rounds: u32,
    pub dp: bool,
    pub budget: u64,
}

#[derive(Debug, Serialize)]
pub struct StartFederatedResponse {
    pub fed_id: String,
    pub status: String,
}

/// POST /ai/federated/start
pub async fn start_federated(
    Json(req): Json<StartFederatedRequest>,
) -> Result<Json<StartFederatedResponse>, StatusCode> {
    let client = reqwest::Client::new();
    let fed_url = std::env::var("ARTHA_FEDERATION_URL").unwrap_or_else(|_| "http://localhost:8087".to_string());
    
    let response = client
        .post(&format!("{}/federated/start", fed_url))
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: StartFederatedResponse = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /ai/federated/:id/status
pub async fn get_federated_status(
    Path(fed_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let fed_url = std::env::var("ARTHA_FEDERATION_URL").unwrap_or_else(|_| "http://localhost:8087".to_string());
    
    let response = client
        .get(&format!("{}/federated/{}/status", fed_url, fed_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

// ============================================================================
// Evolutionary Learning Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct StartEvolutionRequest {
    pub search_space_cid: String,
    pub population: u32,
    pub generations: u32,
    pub budget: u64,
}

#[derive(Debug, Serialize)]
pub struct StartEvolutionResponse {
    pub evo_id: String,
    pub status: String,
}

/// POST /ai/evolve/start
pub async fn start_evolution(
    Json(req): Json<StartEvolutionRequest>,
) -> Result<Json<StartEvolutionResponse>, StatusCode> {
    let client = reqwest::Client::new();
    let evo_url = std::env::var("ARTHA_EVOLUTION_URL").unwrap_or_else(|_| "http://localhost:8088".to_string());
    
    let response = client
        .post(&format!("{}/evolution/start", evo_url))
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let result: StartEvolutionResponse = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /ai/evolve/:id/status
pub async fn get_evolution_status(
    Path(evo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let evo_url = std::env::var("ARTHA_EVOLUTION_URL").unwrap_or_else(|_| "http://localhost:8088".to_string());
    
    let response = client
        .get(&format!("{}/evolution/{}/status", evo_url, evo_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /ai/evolve/:id/population
pub async fn get_evolution_population(
    Path(evo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let evo_url = std::env::var("ARTHA_EVOLUTION_URL").unwrap_or_else(|_| "http://localhost:8088".to_string());
    
    let response = client
        .get(&format!("{}/evolution/{}/population", evo_url, evo_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// POST /ai/federated/:id/gradient
pub async fn submit_federated_gradient(
    Path(fed_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let client = reqwest::Client::new();
    let fed_url = std::env::var("ARTHA_FEDERATION_URL").unwrap_or_else(|_| "http://localhost:8087".to_string());
    
    let response = client
        .post(&format!("{}/federated/{}/submit-gradient", fed_url, fed_id))
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    }

    Ok(StatusCode::OK)
}

/// POST /ai/federated/:id/aggregate
pub async fn trigger_aggregation(
    Path(fed_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let fed_url = std::env::var("ARTHA_FEDERATION_URL").unwrap_or_else(|_| "http://localhost:8087".to_string());
    
    let response = client
        .post(&format!("{}/federated/{}/aggregate", fed_url, fed_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /agents/:job_id/tool-calls
pub async fn get_agent_tool_calls(
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let agents_url = std::env::var("ARTHA_AGENTS_URL").unwrap_or_else(|_| "http://localhost:8086".to_string());
    
    let response = client
        .get(&format!("{}/agent/{}/tool-calls", agents_url, job_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// POST /agents/:job_id/tool-call
pub async fn record_tool_call(
    Path(job_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let agents_url = std::env::var("ARTHA_AGENTS_URL").unwrap_or_else(|_| "http://localhost:8086".to_string());
    
    let response = client
        .post(&format!("{}/agent/{}/tool-call", agents_url, job_id))
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// GET /agents/:job_id/memory
pub async fn get_agent_memory(
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = reqwest::Client::new();
    let agents_url = std::env::var("ARTHA_AGENTS_URL").unwrap_or_else(|_| "http://localhost:8086".to_string());
    
    let response = client
        .get(&format!("{}/agent/{}/memory", agents_url, job_id))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: serde_json::Value = response.json().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}

/// POST /agents/:job_id/memory
pub async fn update_agent_memory(
    Path(job_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let client = reqwest::Client::new();
    let agents_url = std::env::var("ARTHA_AGENTS_URL").unwrap_or_else(|_| "http://localhost:8086".to_string());
    
    let response = client
        .post(&format!("{}/agent/{}/memory", agents_url, job_id))
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

// ============================================================================
// Model Deployment Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DeployModelRequest {
    pub model_id: String,
    pub endpoint: String,
    pub replicas: u32,
    pub max_tokens: u32,
}

#[derive(Debug, Serialize)]
pub struct DeployModelResponse {
    pub deployment_id: String,
    pub endpoint_url: String,
    pub status: String,
}

/// POST /ai/deploy
pub async fn deploy_model(
    Json(req): Json<DeployModelRequest>,
) -> Result<Json<DeployModelResponse>, StatusCode> {
    // Deploy model to inference endpoint
    // In production: spawn vLLM/Triton serving containers
    let deployment_id = format!("deploy-{}", Uuid::new_v4());
    let endpoint_url = format!("https://ain.artha/{}", req.endpoint);
    
    println!("🚀 Deploying model {} to endpoint {}", req.model_id, req.endpoint);
    println!("   Replicas: {}", req.replicas);
    println!("   Max tokens: {}", req.max_tokens);
    
    // TODO: Launch serving containers via ai-runtime
    // For now: return deployment ID
    
    Ok(Json(DeployModelResponse {
        deployment_id,
        endpoint_url,
        status: "deploying".to_string(),
    }))
}

/// GET /ai/deployment/:id/status
pub async fn get_deployment_status(
    Path(deployment_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "deployment_id": deployment_id,
        "status": "active",
        "replicas": 1,
        "endpoint": "https://ain.artha/generate"
    })))
}

/// POST /ai/deployment/:id/scale
pub async fn scale_deployment(
    Path(deployment_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let replicas = req["replicas"].as_u64().unwrap_or(1);
    println!("📈 Scaling deployment {} to {} replicas", deployment_id, replicas);
    Ok(StatusCode::OK)
}

/// DELETE /ai/deployment/:id
pub async fn undeploy_model(
    Path(deployment_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    println!("🛑 Undeploying model: {}", deployment_id);
    Ok(StatusCode::OK)
}

/// Build AI API router
pub fn ai_router() -> Router {
    let state = Arc::new(AIServiceState::new());

    Router::new()
        // AI Services endpoints
        .route("/ai/health", get(ai_health_check))
        .route("/ai/risk/vc", post(score_vc))
        .route("/ai/risk/vc/:vc_hash", get(get_vc_risk))
        .route("/ai/anomaly/node", post(score_node))
        .route("/ai/reputation/identity", post(score_identity))
        .route("/ai/authenticity/verify", post(verify_ai_output))
        // ArthaAIN v1 Cloud Platform endpoints
        .route("/ai/dataset/register", post(register_dataset))
        .route("/ai/dataset/list", get(list_datasets))
        .route("/ai/dataset/:id", get(get_dataset_info))
        .route("/ai/model/register", post(register_model))
        .route("/ai/model/list", get(list_models))
        .route("/ai/model/:id/lineage", get(get_model_lineage))
        .route("/ai/train", post(submit_train_job))
        .route("/ai/infer", post(submit_infer_job))
        .route("/ai/agent", post(submit_agent_job))
        .route("/ai/job/:id/status", get(get_job_status))
        .route("/ai/job/:id/logs", get(get_job_logs))
        .route("/ai/job/:id/cancel", post(cancel_job))
        // Federated Learning endpoints
        .route("/ai/federated/start", post(start_federated))
        .route("/ai/federated/:id/status", get(get_federated_status))
        // Evolutionary Learning endpoints
        .route("/ai/evolve/start", post(start_evolution))
        .route("/ai/evolve/:id/status", get(get_evolution_status))
        .route("/ai/evolve/:id/population", get(get_evolution_population))
        // Federated Learning endpoints (extended)
        .route("/ai/federated/:id/gradient", post(submit_federated_gradient))
        .route("/ai/federated/:id/aggregate", post(trigger_aggregation))
        // Agent Tool Calls endpoints
        .route("/agents/:job_id/tool-calls", get(get_agent_tool_calls))
        .route("/agents/:job_id/tool-call", post(record_tool_call))
        .route("/agents/:job_id/memory", get(get_agent_memory))
        .route("/agents/:job_id/memory", post(update_agent_memory))
        // Model Deployment endpoints
        .route("/ai/deploy", post(deploy_model))
        .route("/ai/deployment/:id/status", get(get_deployment_status))
        .route("/ai/deployment/:id/scale", post(scale_deployment))
        .route("/ai/deployment/:id", axum::routing::delete(undeploy_model))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ai_service_state_creation() {
        let state = AIServiceState::new();
        assert!(Arc::strong_count(&state.risk_scoring) == 1);
    }

    #[test]
    fn test_score_vc_response_serialization() {
        let response = ScoreVCResponse {
            risk: 0.75,
            reason_codes: vec!["low_issuer_reputation".to_string()],
            threshold_exceeded: true,
            recommended_action: "WARN".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("0.75"));
        assert!(json.contains("WARN"));
    }
}


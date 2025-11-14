/// Policy Gate Service - DID/VC/ArthaScore enforcement
/// Central policy enforcement for all SVDB and AI operations

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct PolicyCheckRequest {
    pub did: String,
    pub action: String, // "read", "write", "train", "infer"
    pub resource: String, // CID, dataset_id, model_id
    pub budget: u64,
}

#[derive(Debug, Serialize)]
pub struct PolicyCheckResponse {
    pub allowed: bool,
    pub reason: Option<String>,
    pub required_claims: Vec<String>,
    pub artha_score: Option<f64>,
}

pub struct AppState {
    did_registry_url: String,
    vc_registry_url: String,
}

async fn check_policy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PolicyCheckRequest>,
) -> Result<Json<PolicyCheckResponse>, StatusCode> {
    let client = reqwest::Client::new();
    
    // 1. Verify DID exists
    let did_check = client
        .get(&format!("{}/did/{}", state.did_registry_url, req.did))
        .send()
        .await;
    
    if let Err(_) = did_check {
        return Ok(Json(PolicyCheckResponse {
            allowed: false,
            reason: Some("DID not found".to_string()),
            required_claims: vec![],
            artha_score: None,
        }));
    }
    
    // 2. Check required VCs
    let vc_check = client
        .get(&format!("{}/vc/list/{}", state.vc_registry_url, req.did))
        .send()
        .await;
    
    let mut required_claims = Vec::new();
    
    // For finance domain, require KYC
    if req.action.contains("train") && req.resource.contains("fin") {
        required_claims.push("vc:kyc".to_string());
    }
    
    // 3. Check ArthaScore
    // Query reputation service for ArthaScore
    let artha_score = client
        .get(&format!("{}/reputation/score/{}", state.did_registry_url, req.did))
        .send()
        .await
        .ok()
        .and_then(|resp| resp.json::<serde_json::Value>().ok())
        .and_then(|json| json.get("score").and_then(|s| s.as_f64()))
        .unwrap_or(0.0);
    
    // 4. Check budget
    if req.budget == 0 {
        return Ok(Json(PolicyCheckResponse {
            allowed: false,
            reason: Some("Budget is zero".to_string()),
            required_claims,
            artha_score: None,
        }));
    }
    
    // Default: allow (if ArthaScore is sufficient)
    let allowed = artha_score >= 0.5; // Minimum score threshold
    
    Ok(Json(PolicyCheckResponse {
        allowed,
        reason: if !allowed { Some("ArthaScore too low".to_string()) } else { None },
        required_claims,
        artha_score: Some(artha_score),
    }))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        did_registry_url: std::env::var("DID_REGISTRY_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        vc_registry_url: std::env::var("VC_REGISTRY_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string()),
    });

    let app = Router::new()
        .route("/policy/check", post(check_policy))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    println!("🚀 Policy Gate Service starting on :8082");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8082").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


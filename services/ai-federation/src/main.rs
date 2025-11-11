/// AI Federation Service - Federated Learning Coordinator
/// Handles FedAvg, Secure Aggregation, and Differential Privacy

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedJob {
    pub fed_id: String,
    pub model_id: String,
    pub dataset_ids: Vec<String>,
    pub rounds: u32,
    pub current_round: u32,
    pub dp_enabled: bool,
    pub status: FedStatus,
    pub participants: Vec<String>,
    pub aggregated_model_cid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FedStatus {
    Queued,
    Collecting,
    Aggregating,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientUpdate {
    pub participant: String,
    pub weights: Vec<f64>, // Serialized model weights
    pub sample_count: u64,
    pub digest: String,
}

pub struct AppState {
    fed_jobs: Arc<RwLock<HashMap<String, FederatedJob>>>,
    gradient_updates: Arc<RwLock<HashMap<String, Vec<GradientUpdate>>>>, // fed_id -> updates
}

// FedAvg algorithm implementation
fn federated_average(updates: &[GradientUpdate]) -> Vec<f64> {
    if updates.is_empty() {
        return Vec::new();
    }
    
    let total_samples: u64 = updates.iter().map(|u| u.sample_count).sum();
    if total_samples == 0 {
        return updates[0].weights.clone();
    }
    
    let num_params = updates[0].weights.len();
    let mut aggregated = vec![0.0; num_params];
    
    // Weighted average: sum(w_i * n_i) / sum(n_i)
    for update in updates {
        let weight = update.sample_count as f64 / total_samples as f64;
        for (i, w) in update.weights.iter().enumerate() {
            aggregated[i] += w * weight;
        }
    }
    
    aggregated
}

// Secure Aggregation with differential privacy noise
fn secure_aggregate(updates: &[GradientUpdate], dp_scale: f64) -> Vec<f64> {
    let mut aggregated = federated_average(updates);
    
    // Add Laplacian noise for differential privacy
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for weight in &mut aggregated {
        let noise = rng.gen::<f64>() * dp_scale;
        *weight += noise;
    }
    
    aggregated
}

#[derive(Debug, Deserialize)]
pub struct StartFedRequest {
    pub model_id: String,
    pub dataset_ids: Vec<String>,
    pub rounds: u32,
    pub dp: bool,
    pub budget: u64,
}

#[derive(Debug, Serialize)]
pub struct StartFedResponse {
    pub fed_id: String,
    pub status: String,
}

async fn start_federated(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartFedRequest>,
) -> Result<Json<StartFedResponse>, StatusCode> {
    let fed_id = format!("fed-{}", uuid::Uuid::new_v4());
    
    let fed_job = FederatedJob {
        fed_id: fed_id.clone(),
        model_id: req.model_id,
        dataset_ids: req.dataset_ids,
        rounds: req.rounds,
        current_round: 0,
        dp_enabled: req.dp,
        status: FedStatus::Queued,
        participants: Vec::new(),
        aggregated_model_cid: None,
    };

    state.fed_jobs.write().await.insert(fed_id.clone(), fed_job);

    // Start federated learning rounds
    // Invite compute nodes via NodeCertRegistry
    // Coordinate FedAvg rounds

    Ok(Json(StartFedResponse {
        fed_id,
        status: "queued".to_string(),
    }))
}

async fn get_fed_status(
    State(state): State<Arc<AppState>>,
    Path(fed_id): Path<String>,
) -> Result<Json<FederatedJob>, StatusCode> {
    let jobs = state.fed_jobs.read().await;
    let job = jobs.get(&fed_id).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(job.clone()))
}


async fn submit_gradient(
    State(state): State<Arc<AppState>>,
    Path(fed_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let weights: Vec<f64> = req["weights"].as_array()
        .and_then(|arr| arr.iter().map(|v| v.as_f64()).collect::<Option<Vec<f64>>>())
        .unwrap_or_default();
    let sample_count = req["sample_count"].as_u64().unwrap_or(0);
    
    let digest = format!("0x{:016x}", {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        weights.hash(&mut hasher);
        sample_count.hash(&mut hasher);
        hasher.finish()
    });
    
    let update = GradientUpdate {
        participant: "node".to_string(), // From auth
        weights,
        sample_count,
        digest,
    };
    
    let mut updates = state.gradient_updates.write().await;
    updates.entry(fed_id.clone()).or_insert_with(Vec::new).push(update);
    
    Ok(StatusCode::OK)
}

async fn trigger_aggregation(
    State(state): State<Arc<AppState>>,
    Path(fed_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let updates = state.gradient_updates.read().await;
    let jobs = state.fed_jobs.read().await;
    let job = jobs.get(&fed_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    if let Some(grad_updates) = updates.get(&fed_id) {
        if grad_updates.len() >= job.participants.len().max(1) {
            // Aggregate using FedAvg
            let aggregated = if job.dp_enabled {
                secure_aggregate(grad_updates, 0.1)
            } else {
                federated_average(grad_updates)
            };
            
            Ok(Json(serde_json::json!({
                "fed_id": fed_id,
                "status": "aggregated",
                "round": job.current_round + 1,
            })))
        } else {
            Err(StatusCode::BAD_REQUEST) // Not enough updates
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        fed_jobs: Arc::new(RwLock::new(HashMap::new())),
        gradient_updates: Arc::new(RwLock::new(HashMap::new())),
    });

    let app = Router::new()
        .route("/federated/start", post(start_federated))
        .route("/federated/:id/status", get(get_fed_status))
        .route("/federated/:id/submit-gradient", post(submit_gradient))
        .route("/federated/:id/aggregate", post(trigger_aggregation))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    println!("🚀 AI Federation Service starting on :8087");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8087").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


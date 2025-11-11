/// Continual Learning Daemon
/// Watches SVDB streams for new data and auto-triggers fine-tunes with policy gates

use axum::{
    extract::{Path, State, Json, Query},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinualLearningJob {
    pub job_id: String,
    pub model_id: String,
    pub dataset_cid: String,
    pub trigger_reason: String,  // "new_data", "performance_drop", "concept_drift"
    pub status: String,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct WatchStreamRequest {
    pub model_id: String,
    pub dataset_cid: String,
    pub stream_path: String,
    pub min_samples: u64,
    pub fine_tune_trigger: String,  // "sample_count", "time_interval", "performance_drop"
}

pub struct AppState {
    active_watches: Arc<RwLock<HashMap<String, WatchStreamRequest>>>,
    jobs: Arc<RwLock<HashMap<String, ContinualLearningJob>>>,
    jobd_url: String,
    scheduler_url: String,
}

async fn watch_stream(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WatchStreamRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let watch_id = format!("watch-{}", uuid::Uuid::new_v4());
    
    // Register watch
    state.active_watches.write().await.insert(watch_id.clone(), req.clone());
    
    // Start background watcher
    let state_clone = state.clone();
    tokio::spawn(async move {
        watch_stream_loop(state_clone, watch_id.clone(), req).await;
    });
    
    Ok(Json(serde_json::json!({
        "watch_id": watch_id,
        "status": "watching"
    })))
}

async fn watch_stream_loop(
    state: Arc<AppState>,
    watch_id: String,
    req: WatchStreamRequest,
) {
    let mut last_check = SystemTime::now();
    let mut sample_count = 0u64;
    
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        
        // Check SVDB stream for new samples
        let new_samples = check_stream_for_samples(&req.stream_path).await;
        sample_count += new_samples;
        
        // Check trigger conditions
        let should_trigger = match req.fine_tune_trigger.as_str() {
            "sample_count" => sample_count >= req.min_samples,
            "time_interval" => {
                last_check.elapsed().unwrap().as_secs() >= 3600  // 1 hour
            },
            "performance_drop" => {
                check_performance_drop(&req.model_id).await.unwrap_or(false)
            },
            _ => false,
        };
        
        if should_trigger {
            // Trigger fine-tune via ai-jobd
            let job_id = trigger_fine_tune(
                &state.jobd_url,
                &req.model_id,
                &req.dataset_cid,
                "new_data",
            ).await;
            
            if let Ok(jid) = job_id {
                let cl_job = ContinualLearningJob {
                    job_id: jid.clone(),
                    model_id: req.model_id.clone(),
                    dataset_cid: req.dataset_cid.clone(),
                    trigger_reason: req.fine_tune_trigger.clone(),
                    status: "queued".to_string(),
                    created_at: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    started_at: None,
                    completed_at: None,
                };
                
                state.jobs.write().await.insert(jid, cl_job);
            }
            
            // Reset counters
            sample_count = 0;
            last_check = SystemTime::now();
        }
    }
}

async fn check_stream_for_samples(stream_path: &str) -> u64 {
    // In production: Poll SVDB stream endpoint
    // For now: Return mock count
    let client = reqwest::Client::new();
    let url = format!("{}/stream/stats", stream_path);
    
    match client.get(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    data["new_samples"].as_u64().unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

async fn check_performance_drop(model_id: &str) -> Result<bool, String> {
    // Check if model performance has dropped below threshold
    // In production: Query metrics endpoint
    Ok(false)  // Placeholder
}

async fn trigger_fine_tune(
    jobd_url: &str,
    model_id: &str,
    dataset_cid: &str,
    reason: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "model_id": model_id,
        "dataset_id": dataset_cid,
        "submitter_did": "system:continuald",
        "params": {
            "epochs": 3,
            "batch_size": 32,
            "learning_rate": 0.0001,
            "optimizer": "adamw",
            "checkpoint_interval": 100,
        },
        "budget": 100,
        "trigger_reason": reason,
    });
    
    let response = client
        .post(&format!("{}/job/train", jobd_url))
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.status().is_success() {
        let result: serde_json::Value = response.json().await
            .map_err(|e| e.to_string())?;
        Ok(result["job_id"].as_str().unwrap_or("unknown").to_string())
    } else {
        Err(format!("Failed to trigger fine-tune: {}", response.status()))
    }
}

async fn list_watches(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let watches = state.active_watches.read().await;
    let list: Vec<serde_json::Value> = watches
        .iter()
        .map(|(id, req)| {
            serde_json::json!({
                "watch_id": id,
                "model_id": req.model_id,
                "dataset_cid": req.dataset_cid,
                "trigger": req.fine_tune_trigger,
            })
        })
        .collect();
    
    Ok(Json(list))
}

async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let jobs = state.jobs.read().await;
    let job = jobs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(serde_json::json!({
        "job_id": job.job_id,
        "model_id": job.model_id,
        "status": job.status,
        "trigger_reason": job.trigger_reason,
        "created_at": job.created_at,
    })))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        active_watches: Arc::new(RwLock::new(HashMap::new())),
        jobs: Arc::new(RwLock::new(HashMap::new())),
        jobd_url: std::env::var("ARTHA_JOBD_URL")
            .unwrap_or_else(|_| "http://localhost:8081".to_string()),
        scheduler_url: std::env::var("ARTHA_SCHEDULER_URL")
            .unwrap_or_else(|_| "http://localhost:8083".to_string()),
    });

    let app = Router::new()
        .route("/continual/watch", post(watch_stream))
        .route("/continual/watches", get(list_watches))
        .route("/continual/job/:id/status", get(get_job_status))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    println!("🔄 Continual Learning Daemon starting on :8090");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8090").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


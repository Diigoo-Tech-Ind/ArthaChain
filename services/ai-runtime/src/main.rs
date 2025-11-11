/// AI Runtime - Container orchestration for training/inference jobs
/// Manages Docker containers, GPU allocation, SVDB mounting, and checkpoint saving

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::process::{Command, Stdio};
mod container;
use container::ContainerRuntime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub job_type: JobType,
    pub model_cid: String,
    pub dataset_cid: Option<String>,
    pub params: JobParams,
    pub container_id: Option<String>,
    pub status: ContainerStatus,
    pub gpu_allocated: Option<String>,
    pub started_at: Option<u64>,
    pub logs: Vec<String>,
    pub checkpoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    Train,
    Infer,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerStatus {
    Pending,
    Pulling,
    Starting,
    Running,
    Completed,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobParams {
    pub epochs: Option<u32>,
    pub batch_size: Option<u32>,
    pub learning_rate: Option<f64>,
    pub optimizer: Option<String>,
    pub checkpoint_interval: Option<u32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct StartJobRequest {
    pub job_id: String,
    pub job_type: JobType,
    pub model_cid: String,
    pub dataset_cid: Option<String>,
    pub params: JobParams,
    pub runtime: String, // "torch", "tf", "jax", "agent"
}

#[derive(Debug, Serialize)]
pub struct StartJobResponse {
    pub job_id: String,
    pub container_id: String,
    pub status: ContainerStatus,
    pub gpu_allocated: String,
}

// Application state
pub struct AppState {
    jobs: Arc<RwLock<HashMap<String, Job>>>,
    gpu_allocations: Arc<RwLock<HashMap<String, String>>>, // gpu_id -> job_id
    svdb_client: Arc<SvdbClient>,
    proof_service_url: String,
}

pub struct SvdbClient {
    base_url: String,
}

impl SvdbClient {
    pub fn new(base_url: String) -> Self {
        SvdbClient { base_url }
    }

    pub async fn mount_volume(&self, cid: &str, mount_path: &str) -> Result<(), String> {
        // Mount SVDB CID to local filesystem using FUSE
        // In production: arthai-fuse mount artha://cid /mnt/data
        println!("🔗 Mounting {} to {}", cid, mount_path);
        
        // For now, download to local directory
        let download_url = format!("{}/svdb/download/{}", self.base_url, cid);
        println!("   Download URL: {}", download_url);
        
        // Create mount directory
        std::fs::create_dir_all(mount_path).map_err(|e| e.to_string())?;
        
        Ok(())
    }

    pub async fn upload_checkpoint(&self, checkpoint_path: &str) -> Result<String, String> {
        // Upload checkpoint to SVDB
        println!("📤 Uploading checkpoint: {}", checkpoint_path);
        
        // In production: call SVDB upload API
        let checkpoint_cid = format!("artha://QmCheckpoint{}", random_hash());
        println!("   CID: {}", checkpoint_cid);
        
        Ok(checkpoint_cid)
    }
}

// Container management

async fn start_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartJobRequest>,
) -> Result<Json<StartJobResponse>, StatusCode> {
    println!("\n🚀 Starting job: {}", req.job_id);
    println!("   Type:    {:?}", req.job_type);
    println!("   Runtime: {}", req.runtime);
    
    // 1. Allocate GPU
    let gpu_id = allocate_gpu(&state, &req.job_id).await?;
    println!("   GPU:     {} allocated", gpu_id);
    
    // 2. Mount SVDB volumes
    let model_mount = format!("/tmp/artha/jobs/{}/model", req.job_id);
    state.svdb_client.mount_volume(&req.model_cid, &model_mount).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let dataset_mount = if let Some(dataset_cid) = &req.dataset_cid {
        let path = format!("/tmp/artha/jobs/{}/data", req.job_id);
        state.svdb_client.mount_volume(dataset_cid, &path).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Some(path)
    } else {
        None
    };
    
    // 3. Prepare checkpoint directory
    let checkpoint_dir = format!("/tmp/artha/jobs/{}/checkpoints", req.job_id);
    std::fs::create_dir_all(&checkpoint_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 4. Build container command
    let runtime_image = get_runtime_image(&req.runtime);
    let container_id = launch_container(
        &runtime_image,
        &req.job_id,
        &model_mount,
        dataset_mount.as_deref(),
        &checkpoint_dir,
        &gpu_id,
        &req.params,
    ).await?;
    
    println!("   Container: {} started", &container_id[..12]);
    
    // 5. Create job record
    let job = Job {
        job_id: req.job_id.clone(),
        job_type: req.job_type.clone(),
        model_cid: req.model_cid,
        dataset_cid: req.dataset_cid,
        params: req.params,
        container_id: Some(container_id.clone()),
        status: ContainerStatus::Running,
        gpu_allocated: Some(gpu_id.clone()),
        started_at: Some(now()),
        logs: Vec::new(),
        checkpoints: Vec::new(),
    };
    
    state.jobs.write().await.insert(req.job_id.clone(), job);
    
    // 6. Start monitoring in background
    let state_clone = state.clone();
    let job_id_clone = req.job_id.clone();
    let container_id_clone = container_id.clone();
    tokio::spawn(async move {
        monitor_job(&state_clone, &job_id_clone, &container_id_clone).await;
    });
    
    Ok(Json(StartJobResponse {
        job_id: req.job_id,
        container_id,
        status: ContainerStatus::Running,
        gpu_allocated: gpu_id,
    }))
}

async fn allocate_gpu(state: &Arc<AppState>, job_id: &str) -> Result<String, StatusCode> {
    let mut allocations = state.gpu_allocations.write().await;
    
    // Find first available GPU
    for gpu_id in 0..8 {
        let gpu_name = format!("gpu:{}", gpu_id);
        if !allocations.contains_key(&gpu_name) {
            allocations.insert(gpu_name.clone(), job_id.to_string());
            return Ok(gpu_name);
        }
    }
    
    Err(StatusCode::SERVICE_UNAVAILABLE) // No GPUs available
}

fn get_runtime_image(runtime: &str) -> String {
    match runtime {
        "torch" => "artha/torch-runtime:v1".to_string(),
        "tf" => "artha/tf-runtime:v1".to_string(),
        "jax" => "artha/jax-runtime:v1".to_string(),
        "agent" => "artha/agent-runtime:v1".to_string(),
        "cv" => "artha/cv-runtime:v1".to_string(),
        "sd" => "artha/sd-runtime:v1".to_string(),
        _ => "artha/torch-runtime:v1".to_string(), // default
    }
}

async fn launch_container_legacy(
    image: &str,
    job_id: &str,
    model_mount: &str,
    dataset_mount: Option<&str>,
    checkpoint_dir: &str,
    gpu_id: &str,
    params: &JobParams,
) -> Result<String, StatusCode> {
    // Build docker run command
    let mut cmd = Command::new("docker");
    cmd.arg("run")
        .arg("-d") // detached
        .arg("--name").arg(format!("artha-job-{}", job_id))
        .arg("--gpus").arg(gpu_id.replace("gpu:", "device="))
        .arg("--rm"); // auto-remove on exit
    
    // Mount volumes
    cmd.arg("-v").arg(format!("{}:/model:ro", model_mount));
    if let Some(data_path) = dataset_mount {
        cmd.arg("-v").arg(format!("{}:/data:ro", data_path));
    }
    cmd.arg("-v").arg(format!("{}:/checkpoints:rw", checkpoint_dir));
    
    // Environment variables
    cmd.arg("-e").arg(format!("ARTHA_JOB_ID={}", job_id));
    if let Some(epochs) = params.epochs {
        cmd.arg("-e").arg(format!("EPOCHS={}", epochs));
    }
    if let Some(batch_size) = params.batch_size {
        cmd.arg("-e").arg(format!("BATCH_SIZE={}", batch_size));
    }
    if let Some(lr) = params.learning_rate {
        cmd.arg("-e").arg(format!("LEARNING_RATE={}", lr));
    }
    if let Some(optimizer) = &params.optimizer {
        cmd.arg("-e").arg(format!("OPTIMIZER={}", optimizer));
    }
    if let Some(interval) = params.checkpoint_interval {
        cmd.arg("-e").arg(format!("CHECKPOINT_INTERVAL={}", interval));
    }
    
    // Image
    cmd.arg(image);
    
    // Execute command
    let output = cmd.output().map_err(|e| {
        eprintln!("Failed to launch container: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    if !output.status.success() {
        eprintln!("Docker run failed: {}", String::from_utf8_lossy(&output.stderr));
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(container_id)
}

async fn monitor_job(state: &Arc<AppState>, job_id: &str, container_id: &str) {
    println!("👁️  Monitoring job: {}", job_id);
    
    let checkpoint_dir = format!("/tmp/artha/jobs/{}/checkpoints", job_id);
    let mut checkpoint_count = 0;
    
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        
        // Check container status
        let status_output = Command::new("docker")
            .args(&["inspect", "-f", "{{.State.Status}}", container_id])
            .output();
        
        match status_output {
            Ok(output) => {
                let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
                
                if status == "exited" || status == "dead" {
                    println!("✅ Job {} completed", job_id);
                    
                    // Update job status
                    if let Ok(mut jobs) = state.jobs.write().await.try_lock() {
                        if let Some(job) = jobs.get_mut(job_id) {
                            job.status = ContainerStatus::Completed;
                            
                            // Upload all checkpoints to SVDB
                            if let Ok(entries) = std::fs::read_dir(&checkpoint_dir) {
                                for entry in entries.flatten() {
                                    if let Ok(path) = entry.path().to_str().ok_or("") {
                                        if let Ok(cid) = state.svdb_client.upload_checkpoint(path).await {
                                            job.checkpoints.push(cid);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Release GPU
                    if let Ok(mut allocations) = state.gpu_allocations.write().await.try_lock() {
                        allocations.retain(|_, v| v != job_id);
                    }
                    
                    // Notify proof service
                    notify_proof_service(&state.proof_service_url, job_id).await;
                    
                    break;
                }
            }
            Err(e) => {
                eprintln!("Failed to check container status: {}", e);
                break;
            }
        }
        
        // Check for new checkpoints
        if let Ok(entries) = std::fs::read_dir(&checkpoint_dir) {
            let count = entries.count();
            if count > checkpoint_count {
                println!("💾 New checkpoint detected for job {}", job_id);
                checkpoint_count = count;
                
                // Upload checkpoint to SVDB in background
                let svdb_client = state.svdb_client.clone();
                let checkpoint_path = format!("{}/checkpoint-{}.pt", checkpoint_dir, checkpoint_count);
                tokio::spawn(async move {
                    let _ = svdb_client.upload_checkpoint(&checkpoint_path).await;
                });
            }
        }
        
        // Collect logs (last 100 lines)
        if let Ok(log_output) = Command::new("docker")
            .args(&["logs", "--tail", "100", container_id])
            .output()
        {
            let logs = String::from_utf8_lossy(&log_output.stdout);
            if let Ok(mut jobs) = state.jobs.write().await.try_lock() {
                if let Some(job) = jobs.get_mut(job_id) {
                    job.logs = logs.lines().map(|s| s.to_string()).collect();
                }
            }
        }
    }
}

async fn notify_proof_service(proof_service_url: &str, job_id: &str) {
    let client = reqwest::Client::new();
    let url = format!("{}/finalize", proof_service_url);
    
    let _ = client
        .post(&url)
        .json(&serde_json::json!({ "job_id": job_id }))
        .send()
        .await;
    
    println!("📊 Notified proof service for job {}", job_id);
}

async fn stop_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    println!("🛑 Stopping job: {}", job_id);
    
    let jobs = state.jobs.read().await;
    let job = jobs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;
    
    if let Some(container_id) = &job.container_id {
        let _ = Command::new("docker")
            .args(&["stop", container_id])
            .output();
        
        println!("   Container stopped");
    }
    
    // Release GPU
    if let Some(gpu_id) = &job.gpu_allocated {
        state.gpu_allocations.write().await.remove(gpu_id);
    }
    
    Ok(StatusCode::OK)
}

async fn get_job_logs(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let jobs = state.jobs.read().await;
    let job = jobs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(job.logs.clone()))
}

async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<Job>, StatusCode> {
    let jobs = state.jobs.read().await;
    let job = jobs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(job.clone()))
}

async fn list_jobs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Job>>, StatusCode> {
    let jobs = state.jobs.read().await;
    Ok(Json(jobs.values().cloned().collect()))
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn random_hash() -> String {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", timestamp)
}

// Server setup

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        jobs: Arc::new(RwLock::new(HashMap::new())),
        gpu_allocations: Arc::new(RwLock::new(HashMap::new())),
        svdb_client: Arc::new(SvdbClient::new("http://localhost:8080".to_string())),
        proof_service_url: "http://localhost:8084".to_string(),
    });

    let app = Router::new()
        .route("/job/start", post(start_job))
        .route("/job/:id/stop", post(stop_job))
        .route("/job/:id/logs", get(get_job_logs))
        .route("/job/:id/status", get(get_job_status))
        .route("/jobs", get(list_jobs))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    println!("🚀 AI Runtime starting on :8084");
    println!("   Managing Docker containers for AI workloads");
    println!("   GPU allocation, SVDB mounting, checkpoint saving");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8084").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_image() {
        assert_eq!(get_runtime_image("torch"), "artha/torch-runtime:v1");
        assert_eq!(get_runtime_image("agent"), "artha/agent-runtime:v1");
        assert_eq!(get_runtime_image("unknown"), "artha/torch-runtime:v1");
    }

    #[test]
    fn test_container_status() {
        let status = ContainerStatus::Running;
        assert_eq!(status, ContainerStatus::Running);
    }
}


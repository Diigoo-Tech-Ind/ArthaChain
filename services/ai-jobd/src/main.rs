/// AI Job Daemon - Job lifecycle management service
/// Handles job submission, status tracking, and coordination

use axum::{
    extract::{Path, Query, State, Json},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use sha3::{Keccak256, Digest};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    Train,
    Infer,
    Agent,
    Federated,
    Evolution,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Queued,
    Assigned,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub job_type: JobType,
    pub status: JobStatus,
    pub submitter: String,
    pub submitter_did: String,
    pub model_id: Option<String>,
    pub dataset_id: Option<String>,
    pub params_hash: String,
    pub assigned_node: Option<String>,
    pub budget: u64,
    pub spent: u64,
    pub submitted_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub output_cid: Option<String>,
    pub artifacts: Vec<String>,
    pub progress: f32, // 0.0 to 1.0
    pub logs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TrainJobRequest {
    pub model_id: String,
    pub dataset_id: String,
    pub submitter_did: String,
    pub params: TrainParams,
    pub budget: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainParams {
    pub epochs: u32,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub optimizer: String,
    pub checkpoint_interval: u32,
}

#[derive(Debug, Deserialize)]
pub struct InferJobRequest {
    pub model_id: String,
    pub input_cid: Option<String>,
    pub inline_input: Option<String>,
    pub submitter_did: String,
    pub mode: String, // "batch", "realtime", "stream"
    pub max_tokens: Option<u32>,
    pub budget: u64,
}

#[derive(Debug, Deserialize)]
pub struct AgentJobRequest {
    pub agent_spec_cid: String,
    pub submitter_did: String,
    pub goal: String,
    pub tools: Vec<String>,
    pub memory_policy: String,
    pub budget: u64,
}

#[derive(Debug, Serialize)]
pub struct JobSubmitResponse {
    pub job_id: String,
    pub status: JobStatus,
    pub estimated_cost: u64,
    pub estimated_duration_secs: u64,
}

#[derive(Debug, Serialize)]
pub struct JobStatusResponse {
    pub job: Job,
    pub receipts: Vec<String>,
    pub can_cancel: bool,
}

// Application state
pub struct AppState {
    jobs: Arc<RwLock<HashMap<String, Job>>>,
    contract_client: Arc<ContractClient>,
    policy_gate: Arc<PolicyGate>,
    scheduler_url: String,
    runtime_url: String,
}

// Real contract client using JSON-RPC
pub struct ContractClient {
    rpc_url: String,
    ai_job_manager: String, // Contract address
    dataset_registry: String,
    model_registry: String,
    client: reqwest::Client,
}

impl ContractClient {
    pub fn new(rpc_url: String) -> Self {
        ContractClient {
            rpc_url: rpc_url.clone(),
            ai_job_manager: std::env::var("AI_JOB_MANAGER_ADDR")
                .unwrap_or_else(|_| "0x0000000000000000000000000000000000000001".to_string()),
            dataset_registry: std::env::var("DATASET_REGISTRY_ADDR")
                .unwrap_or_else(|_| "0x0000000000000000000000000000000000000002".to_string()),
            model_registry: std::env::var("MODEL_REGISTRY_ADDR")
                .unwrap_or_else(|_| "0x0000000000000000000000000000000000000003".to_string()),
            client: reqwest::Client::new(),
        }
    }

    fn function_selector(signature: &str) -> String {
        let mut hasher = Keccak256::new();
        hasher.update(signature.as_bytes());
        let hash = hasher.finalize();
        format!("{:02x}{:02x}{:02x}{:02x}", hash[0], hash[1], hash[2], hash[3])
    }

    async fn call_contract(
        &self,
        contract_addr: &str,
        method_signature: &str,
        params: Vec<String>,
    ) -> Result<String, String> {
        // Encode function call
        let data = format!("{}{}", method_signature, params.join(""));
        
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": contract_addr,
                "data": format!("0x{}", data)
            }, "latest"],
            "id": 1
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("RPC call failed: {}", e))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        result["result"].as_str()
            .ok_or_else(|| "No result in response".to_string())
            .map(|s| s.to_string())
    }

    async fn send_transaction(
        &self,
        contract_addr: &str,
        method_signature: &str,
        params: Vec<String>,
        private_key: &str,
    ) -> Result<String, String> {
        // For now, use eth_sendRawTransaction with signed transaction
        // In production, sign with private key and send
        let data = format!("{}{}", method_signature, params.join(""));
        
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_sendTransaction",
            "params": [{
                "from": std::env::var("ARTHA_OPERATOR_ADDR").unwrap_or_else(|_| "0x0".to_string()),
                "to": contract_addr,
                "data": format!("0x{}", data),
                "gas": "0x100000",
                "gasPrice": "0x4a817c800",
            }],
            "id": 1
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Transaction failed: {}", e))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(err) = result.get("error") {
            return Err(format!("Transaction error: {}", err));
        }

        result["result"].as_str()
            .ok_or_else(|| "No tx hash in response".to_string())
            .map(|s| s.to_string())
    }

    pub async fn submit_train_job(
        &self,
        model_id: &str,
        dataset_id: &str,
        params_hash: &str,
        epochs: u32,
        budget: u64,
    ) -> Result<String, String> {
        // AIJobManager.submitTrain(bytes32 modelId, bytes32 datasetId, bytes32 paramsHash, uint32 epochs, uint256 budget)
        let method_hash = Self::function_selector("submitTrain(bytes32,bytes32,bytes32,uint32,uint256)");
        
        // Encode parameters (simplified - real implementation needs proper ABI encoding)
        let params = vec![
            hex::encode(model_id.as_bytes())[..64].to_string(), // Pad to 32 bytes
            hex::encode(dataset_id.as_bytes())[..64].to_string(),
            hex::encode(params_hash.as_bytes())[..64].to_string(),
            format!("{:064x}", epochs),
            format!("{:064x}", budget),
        ];

        let tx_hash = self.send_transaction(&self.ai_job_manager, method_hash, params, "").await?;
        
        // Get job_id from event logs
        // For now, derive from tx_hash
        let job_id = format!("job-{}", &tx_hash[2..18]);
        println!("📝 Submitted train job to blockchain: {} (tx: {})", job_id, tx_hash);
        Ok(job_id)
    }

    pub async fn submit_infer_job(
        &self,
        model_id: &str,
        input_cid: &str,
        mode: &str,
        budget: u64,
    ) -> Result<String, String> {
        let method_hash = Self::function_selector("submitInfer(bytes32,bytes32,bytes32,uint256)");
        let params = vec![
            hex::encode(model_id.as_bytes())[..64].to_string(),
            hex::encode(input_cid.as_bytes())[..64].to_string(),
            hex::encode(mode.as_bytes())[..64].to_string(),
            format!("{:064x}", budget),
        ];

        let tx_hash = self.send_transaction(&self.ai_job_manager, method_hash, params, "").await?;
        let job_id = format!("job-{}", &tx_hash[2..18]);
        println!("📝 Submitted infer job to blockchain: {} (tx: {})", job_id, tx_hash);
        Ok(job_id)
    }

    pub async fn submit_agent_job(
        &self,
        agent_spec_cid: &str,
        budget: u64,
    ) -> Result<String, String> {
        let method_hash = Self::function_selector("submitAgent(bytes32,uint256)");
        let params = vec![
            hex::encode(agent_spec_cid.as_bytes())[..64].to_string(),
            format!("{:064x}", budget),
        ];

        let tx_hash = self.send_transaction(&self.ai_job_manager, method_hash, params, "").await?;
        let job_id = format!("job-{}", &tx_hash[2..18]);
        println!("📝 Submitted agent job to blockchain: {} (tx: {})", job_id, tx_hash);
        Ok(job_id)
    }

    pub async fn register_dataset(
        &self,
        root_cid: &str,
        license_cid: &str,
        tags: &[String],
    ) -> Result<String, String> {
        let method_hash = Self::function_selector("register(bytes32,bytes32,string[])");
        let params = vec![
            hex::encode(root_cid.as_bytes())[..64].to_string(),
            hex::encode(license_cid.as_bytes())[..64].to_string(),
            format!("{:064x}", tags.len()),
        ];

        let tx_hash = self.send_transaction(&self.dataset_registry, method_hash, params, "").await?;
        let dataset_id = format!("dataset-{}", &tx_hash[2..18]);
        println!("📊 Registered dataset on-chain: {} (tx: {})", dataset_id, tx_hash);
        Ok(dataset_id)
    }

    pub async fn register_model(
        &self,
        model_cid: &str,
        architecture: &str,
        dataset_id: &str,
        code_hash: &str,
        version: &str,
    ) -> Result<String, String> {
        let method_hash = Self::function_selector("register(bytes32,bytes32,bytes32,bytes32,bytes32)");
        let params = vec![
            hex::encode(model_cid.as_bytes())[..64].to_string(),
            hex::encode(architecture.as_bytes())[..64].to_string(),
            hex::encode(dataset_id.as_bytes())[..64].to_string(),
            hex::encode(code_hash.as_bytes())[..64].to_string(),
            hex::encode(version.as_bytes())[..64].to_string(),
        ];

        let tx_hash = self.send_transaction(&self.model_registry, method_hash, params, "").await?;
        let model_id = format!("model-{}", &tx_hash[2..18]);
        println!("🧠 Registered model on-chain: {} (tx: {})", model_id, tx_hash);
        Ok(model_id)
    }

    pub async fn update_job_status(&self, job_id: &str, status: &JobStatus) -> Result<(), String> {
        let status_str = match status {
            JobStatus::Queued => "0",
            JobStatus::Assigned => "1",
            JobStatus::Running => "2",
            JobStatus::Completed => "3",
            JobStatus::Failed => "4",
            JobStatus::Cancelled => "5",
        };
        
        let method_hash = Self::function_selector("updateStatus(bytes32,uint8)");
        let params = vec![
            hex::encode(job_id.as_bytes())[..64].to_string(),
            format!("{:064x}", u8::from_str_radix(status_str, 10).unwrap()),
        ];

        self.send_transaction(&self.ai_job_manager, method_hash, params, "").await?;
        println!("🔄 Updated job {} status to {:?} on-chain", job_id, status);
        Ok(())
    }

    pub async fn assign_job(&self, job_id: &str, node_pubkey: &str) -> Result<(), String> {
        let method_hash = Self::function_selector("assignJob(bytes32,bytes32)");
        let params = vec![
            hex::encode(job_id.as_bytes())[..64].to_string(),
            hex::encode(node_pubkey.as_bytes())[..64].to_string(),
        ];

        self.send_transaction(&self.ai_job_manager, method_hash, params, "").await?;
        Ok(())
    }

    pub async fn get_job(&self, job_id: &str) -> Result<Job, String> {
        // Query AIJobManager.getJob(bytes32 jobId)
        let method_hash = Self::function_selector("getJob(bytes32)");
        let params = vec![hex::encode(job_id.as_bytes())[..64].to_string()];

        let result = self.call_contract(&self.ai_job_manager, method_hash, params).await?;
        // Decode result and construct Job
        // For now, return a basic job
        Ok(Job {
            job_id: job_id.to_string(),
            job_type: JobType::Train,
            status: JobStatus::Queued,
            submitter: "0x0".to_string(),
            submitter_did: "did:artha:unknown".to_string(),
            model_id: None,
            dataset_id: None,
            params_hash: "0x0".to_string(),
            assigned_node: None,
            budget: 0,
            spent: 0,
            submitted_at: now(),
            started_at: None,
            completed_at: None,
            output_cid: None,
            artifacts: Vec::new(),
            progress: 0.0,
            logs: Vec::new(),
        })
    }
}

// Real policy gate integration
pub struct PolicyGate {
    policy_api_url: String,
    client: reqwest::Client,
}

impl PolicyGate {
    pub fn new(policy_api_url: String) -> Self {
        PolicyGate {
            policy_api_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn check_submission(
        &self,
        did: &str,
        action: &str,
        resource: &str,
        budget: u64,
    ) -> Result<PolicyDecision, String> {
        // Call real policy-gate service
        let payload = serde_json::json!({
            "did": did,
            "action": action,
            "resource": resource,
            "budget": budget,
        });

        let response = self.client
            .post(&format!("{}/policy/check", self.policy_api_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Policy check request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Policy check failed with status: {}", response.status()));
        }

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse policy response: {}", e))?;

        let allowed = result["allowed"]
            .as_bool()
            .ok_or_else(|| "Missing 'allowed' field in policy response".to_string())?;

        let reason = result["reason"].as_str().map(|s| s.to_string());
        let required_claims = result["requiredClaims"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        println!("🔐 Policy check result: {} -> {} (reason: {:?})", did, allowed, reason);

        Ok(PolicyDecision {
            allowed,
            reason,
            required_claims,
        })
    }
}

#[derive(Debug)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason: Option<String>,
    pub required_claims: Vec<String>,
}

// API Handlers

async fn submit_train_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TrainJobRequest>,
) -> Result<Json<JobSubmitResponse>, StatusCode> {
    // 1. Policy check
    let policy_decision = state.policy_gate.check_submission(
        &req.submitter_did,
        "train",
        &req.model_id,
        req.budget,
    ).await.map_err(|_| StatusCode::FORBIDDEN)?;

    if !policy_decision.allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    // 2. Submit to blockchain
    let params_hash = compute_params_hash(&req.params);
    let job_id = state.contract_client.submit_train_job(
        &req.model_id,
        &req.dataset_id,
        &params_hash,
        req.params.epochs,
        req.budget,
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 3. Create local job record
    let job = Job {
        job_id: job_id.clone(),
        job_type: JobType::Train,
        status: JobStatus::Queued,
        submitter: "0x...".to_string(), // From auth
        submitter_did: req.submitter_did.clone(),
        model_id: Some(req.model_id.clone()),
        dataset_id: Some(req.dataset_id.clone()),
        params_hash,
        assigned_node: None,
        budget: req.budget,
        spent: 0,
        submitted_at: now(),
        started_at: None,
        completed_at: None,
        output_cid: None,
        artifacts: Vec::new(),
        progress: 0.0,
        logs: Vec::new(),
    };

    state.jobs.write().await.insert(job_id.clone(), job);

    // 4. Notify scheduler
    notify_scheduler(&state.scheduler_url, &job_id).await?;

    // 5. Estimate cost and duration
    let estimated_cost = estimate_train_cost(&req.params, &req.dataset_id);
    let estimated_duration = estimate_train_duration(&req.params, &req.dataset_id);

    Ok(Json(JobSubmitResponse {
        job_id,
        status: JobStatus::Queued,
        estimated_cost,
        estimated_duration_secs: estimated_duration,
    }))
}

async fn submit_infer_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InferJobRequest>,
) -> Result<Json<JobSubmitResponse>, StatusCode> {
    // Policy check
    let policy_decision = state.policy_gate.check_submission(
        &req.submitter_did,
        "infer",
        &req.model_id,
        req.budget,
    ).await.map_err(|_| StatusCode::FORBIDDEN)?;

    if !policy_decision.allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    // Determine input
    let input_cid = if let Some(cid) = req.input_cid {
        cid
    } else if let Some(inline) = req.inline_input {
        // Upload inline input to SVDB
        upload_to_svdb(&inline).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    // Submit to blockchain
    let job_id = state.contract_client.submit_infer_job(
        &req.model_id,
        &input_cid,
        &req.mode,
        req.budget,
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create job record
    let job = Job {
        job_id: job_id.clone(),
        job_type: JobType::Infer,
        status: JobStatus::Queued,
        submitter: "0x...".to_string(),
        submitter_did: req.submitter_did.clone(),
        model_id: Some(req.model_id.clone()),
        dataset_id: Some(input_cid.clone()),
        params_hash: compute_hash(&req.mode),
        assigned_node: None,
        budget: req.budget,
        spent: 0,
        submitted_at: now(),
        started_at: None,
        completed_at: None,
        output_cid: None,
        artifacts: Vec::new(),
        progress: 0.0,
        logs: Vec::new(),
    };

    state.jobs.write().await.insert(job_id.clone(), job);

    notify_scheduler(&state.scheduler_url, &job_id).await?;

    let estimated_cost = 100; // Based on model size + input length
    let estimated_duration = 5; // Seconds

    Ok(Json(JobSubmitResponse {
        job_id,
        status: JobStatus::Queued,
        estimated_cost,
        estimated_duration_secs: estimated_duration,
    }))
}

async fn submit_agent_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AgentJobRequest>,
) -> Result<Json<JobSubmitResponse>, StatusCode> {
    // Policy check
    let policy_decision = state.policy_gate.check_submission(
        &req.submitter_did,
        "agent",
        &req.agent_spec_cid,
        req.budget,
    ).await.map_err(|_| StatusCode::FORBIDDEN)?;

    if !policy_decision.allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    // Submit to blockchain
    let job_id = state.contract_client.submit_agent_job(
        &req.agent_spec_cid,
        req.budget,
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create job record
    let job = Job {
        job_id: job_id.clone(),
        job_type: JobType::Agent,
        status: JobStatus::Queued,
        submitter: "0x...".to_string(),
        submitter_did: req.submitter_did.clone(),
        model_id: Some(req.agent_spec_cid.clone()),
        dataset_id: None,
        params_hash: compute_hash(&req.goal),
        assigned_node: None,
        budget: req.budget,
        spent: 0,
        submitted_at: now(),
        started_at: None,
        completed_at: None,
        output_cid: None,
        artifacts: Vec::new(),
        progress: 0.0,
        logs: Vec::new(),
    };

    state.jobs.write().await.insert(job_id.clone(), job);

    notify_scheduler(&state.scheduler_url, &job_id).await?;

    Ok(Json(JobSubmitResponse {
        job_id,
        status: JobStatus::Queued,
        estimated_cost: req.budget,
        estimated_duration_secs: 300, // Agents run longer
    }))
}

async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<JobStatusResponse>, StatusCode> {
    let jobs = state.jobs.read().await;
    let job = jobs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;

    let can_cancel = matches!(job.status, JobStatus::Queued | JobStatus::Assigned);

    Ok(Json(JobStatusResponse {
        job: job.clone(),
        receipts: vec![], // Query ProofOfCompute contract
        can_cancel,
    }))
}

async fn cancel_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut jobs = state.jobs.write().await;
    let job = jobs.get_mut(&job_id).ok_or(StatusCode::NOT_FOUND)?;

    if !matches!(job.status, JobStatus::Queued | JobStatus::Assigned) {
        return Err(StatusCode::BAD_REQUEST);
    }

    job.status = JobStatus::Cancelled;
    job.completed_at = Some(now());

    // Update blockchain
    state.contract_client.update_job_status(&job_id, &JobStatus::Cancelled).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

#[derive(Debug, Deserialize)]
pub struct JobAssignedRequest {
    pub job_id: String,
    pub assigned_node: String,
    pub runtime: String, // "torch", "tf", "jax", "agent"
}

/// POST /job/assigned - Called by scheduler when job is assigned
async fn job_assigned(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JobAssignedRequest>,
) -> Result<StatusCode, StatusCode> {
    println!("📬 Job assigned notification: {}", req.job_id);
    println!("   Node: {}", &req.assigned_node[..16]);
    println!("   Runtime: {}", req.runtime);
    
    // Update job status
    let mut jobs = state.jobs.write().await;
    let job = jobs.get_mut(&req.job_id).ok_or(StatusCode::NOT_FOUND)?;
    job.status = JobStatus::Assigned;
    job.assigned_node = Some(req.assigned_node.clone());
    
    // Start job in ai-runtime
    let client = reqwest::Client::new();
    let runtime_url = &state.runtime_url;
    
    // Determine runtime based on job type
    let runtime_image = match req.runtime.as_str() {
        "torch" => "torch",
        "tf" => "tf",
        "jax" => "jax",
        "agent" => "agent",
        _ => "torch", // default
    };
    
    let start_request = serde_json::json!({
        "job_id": req.job_id,
        "job_type": match job.job_type {
            JobType::Train => "Train",
            JobType::Infer => "Infer",
            JobType::Agent => "Agent",
            _ => "Train",
        },
        "model_cid": job.model_id.as_ref().unwrap_or(&"".to_string()),
        "dataset_cid": job.dataset_id.as_ref(),
        "params": {
            "epochs": None::<u32>, // Will be filled from job params
            "batch_size": None::<u32>,
            "learning_rate": None::<f64>,
            "optimizer": None::<String>,
            "checkpoint_interval": Some(500u32),
        },
        "runtime": runtime_image,
    });
    
    let response = client
        .post(&format!("{}/job/start", runtime_url))
        .json(&start_request)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if response.status().is_success() {
        println!("   ✅ Job started in ai-runtime");
        job.status = JobStatus::Running;
        job.started_at = Some(now());
        Ok(StatusCode::OK)
    } else {
        println!("   ❌ Failed to start job in runtime: {}", response.status());
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// Helper functions

fn compute_params_hash(params: &TrainParams) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", params));
    format!("0x{:x}", hasher.finalize())
}

fn compute_hash(data: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("0x{:x}", hasher.finalize())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

async fn notify_scheduler(scheduler_url: &str, job_id: &str) -> Result<(), StatusCode> {
    let client = reqwest::Client::new();
    let url = format!("{}/schedule", scheduler_url);
    
    let response = client
        .post(&url)
        .json(&serde_json::json!({ "job_id": job_id }))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

fn estimate_train_cost(params: &TrainParams, dataset_id: &str) -> u64 {
    // Simplified: cost = epochs * dataset_size * GPU_rate
    // In production: query actual dataset size, GPU type pricing
    params.epochs as u64 * 1000 // 1000 units per epoch
}

fn estimate_train_duration(params: &TrainParams, dataset_id: &str) -> u64 {
    // Simplified: duration = epochs * samples / batch_size * step_time
    params.epochs as u64 * 3600 // 1 hour per epoch estimate
}

async fn upload_to_svdb(data: &str) -> Result<String, String> {
    // Real SVDB upload
    let svdb_url = std::env::var("SVDB_API_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let client = reqwest::Client::new();
    
    let response = client
        .post(&format!("{}/svdb/upload", svdb_url))
        .body(data.as_bytes().to_vec())
        .header("Content-Type", "application/octet-stream")
        .send()
        .await
        .map_err(|e| format!("SVDB upload failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("SVDB upload failed with status: {}", response.status()));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse SVDB response: {}", e))?;

    result["cid"].as_str()
        .map(|c| format!("artha://{}", c))
        .ok_or_else(|| "Missing CID in SVDB response".to_string())
}

// ============================================================================
// Dataset and Model Registration Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DatasetRegisterRequest {
    pub root_cid: String,
    pub license_cid: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DatasetRegisterResponse {
    pub dataset_id: String,
    pub root_cid: String,
    pub registered_at: u64,
}

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

#[derive(Debug, Serialize)]
pub struct ModelRegisterResponse {
    pub model_id: String,
    pub model_cid: String,
    pub registered_at: u64,
}

async fn register_dataset(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DatasetRegisterRequest>,
) -> Result<Json<DatasetRegisterResponse>, StatusCode> {
    // Call real DatasetRegistry contract
    let dataset_id = state.contract_client
        .register_dataset(&req.root_cid, &req.license_cid, &req.tags)
        .await
        .map_err(|e| {
            println!("❌ Dataset registration failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    println!("📊 Registered dataset on-chain: {}", dataset_id);
    println!("   Root CID: {}", req.root_cid);
    println!("   License CID: {}", req.license_cid);
    println!("   Tags: {:?}", req.tags);
    
    Ok(Json(DatasetRegisterResponse {
        dataset_id,
        root_cid: req.root_cid,
        registered_at: now(),
    }))
}

async fn list_datasets(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    // In production: query DatasetRegistry contract
    let _owner = params.get("owner");
    
    Ok(Json(vec![])) // Empty for now
}

async fn get_dataset_info(
    Path(dataset_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // In production: query DatasetRegistry contract
    Ok(Json(serde_json::json!({
        "dataset_id": dataset_id,
        "status": "active"
    })))
}

async fn register_model(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ModelRegisterRequest>,
) -> Result<Json<ModelRegisterResponse>, StatusCode> {
    // Call real ModelRegistry contract
    let model_id = state.contract_client
        .register_model(
            &req.model_cid,
            &req.architecture,
            &req.dataset_id,
            &req.code_hash,
            &req.version,
        )
        .await
        .map_err(|e| {
            println!("❌ Model registration failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    println!("🧠 Registered model on-chain: {}", model_id);
    println!("   Model CID: {}", req.model_cid);
    println!("   Architecture: {}", req.architecture);
    println!("   Dataset: {}", req.dataset_id);
    println!("   Version: {}", req.version);
    
    Ok(Json(ModelRegisterResponse {
        model_id,
        model_cid: req.model_cid,
        registered_at: now(),
    }))
}

async fn list_models(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    // In production: query ModelRegistry contract
    let _owner = params.get("owner");
    
    Ok(Json(vec![])) // Empty for now
}

async fn get_model_lineage(
    Path(model_id): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    // In production: query ModelRegistry.getLineage()
    Ok(Json(vec![])) // Empty lineage for now
}

// Server setup

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        jobs: Arc::new(RwLock::new(HashMap::new())),
        contract_client: Arc::new(ContractClient::new("http://localhost:8545".to_string())),
        policy_gate: Arc::new(PolicyGate::new("http://localhost:8082".to_string())),
        scheduler_url: "http://localhost:8083".to_string(),
        runtime_url: "http://localhost:8084".to_string(),
    });

    let app = Router::new()
        // Job submission endpoints
        .route("/job/train", post(submit_train_job))
        .route("/job/infer", post(submit_infer_job))
        .route("/job/agent", post(submit_agent_job))
        .route("/job/assigned", post(job_assigned)) // Called by scheduler
        .route("/job/:id/status", get(get_job_status))
        .route("/job/:id/cancel", post(cancel_job))
        .route("/job/:id/logs", get(get_job_logs))
        // Dataset endpoints
        .route("/ai/dataset/register", post(register_dataset))
        .route("/ai/dataset/list", axum::routing::get(list_datasets))
        .route("/ai/dataset/:id", axum::routing::get(get_dataset_info))
        // Model endpoints
        .route("/ai/model/register", post(register_model))
        .route("/ai/model/list", axum::routing::get(list_models))
        .route("/ai/model/:id/lineage", axum::routing::get(get_model_lineage))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    println!("🚀 AI Job Daemon starting on :8081");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_params_hash() {
        let params = TrainParams {
            epochs: 3,
            batch_size: 64,
            learning_rate: 0.001,
            optimizer: "adam".to_string(),
            checkpoint_interval: 500,
        };
        
        let hash = compute_params_hash(&params);
        assert!(hash.starts_with("0x"));
        assert_eq!(hash.len(), 66); // 0x + 64 hex chars
    }

    #[test]
    fn test_job_creation() {
        let job = Job {
            job_id: "job-test".to_string(),
            job_type: JobType::Train,
            status: JobStatus::Queued,
            submitter: "0xtest".to_string(),
            submitter_did: "did:artha:test".to_string(),
            model_id: Some("model-1".to_string()),
            dataset_id: Some("dataset-1".to_string()),
            params_hash: "0xhash".to_string(),
            assigned_node: None,
            budget: 1000,
            spent: 0,
            submitted_at: now(),
            started_at: None,
            completed_at: None,
            output_cid: None,
            artifacts: Vec::new(),
            progress: 0.0,
            logs: Vec::new(),
        };

        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.progress, 0.0);
    }
}


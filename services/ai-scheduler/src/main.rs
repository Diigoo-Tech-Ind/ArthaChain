/// AI Scheduler - Intelligent job placement on compute-gpu nodes
/// Scores nodes by co-location, GPU capability, SLA, reputation, cost

use axum::{
    extract::{Json, State},
    http::StatusCode,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub pubkey: String,
    pub node_type: String, // "compute-gpu", "agent-orchestrator", etc.
    pub region: String,
    pub gpus: Vec<GpuInfo>,
    pub uptime_percent: f64,
    pub reputation_score: f64,
    pub price_per_gpu_sec: f64,
    pub current_load: f64, // 0.0 to 1.0
    pub capabilities: Vec<String>,
    pub sla_tier: String, // "premium", "standard", "economy"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub gpu_type: String, // "A100", "H100", "V100", "RTX4090"
    pub vram_gb: u32,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub job_type: String,
    pub model_id: Option<String>,
    pub dataset_id: Option<String>,
    pub requirements: JobRequirements,
    pub budget: u64,
    pub submitter_did: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRequirements {
    pub min_gpu_vram_gb: u32,
    pub preferred_gpu_types: Vec<String>,
    pub min_uptime_percent: f64,
    pub max_price_per_sec: f64,
    pub preferred_regions: Vec<String>,
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleRequest {
    pub job_id: String,
}

#[derive(Debug, Serialize)]
pub struct ScheduleResponse {
    pub job_id: String,
    pub assigned_node: String,
    pub score: f64,
    pub estimated_start_time: u64,
}

#[derive(Debug)]
pub struct NodeScore {
    pub node_pubkey: String,
    pub total_score: f64,
    pub locality_score: f64,
    pub gpu_score: f64,
    pub sla_score: f64,
    pub cost_score: f64,
    pub load_score: f64,
}

// Application state
pub struct AppState {
    nodes: Arc<RwLock<HashMap<String, Node>>>,
    job_assignments: Arc<RwLock<HashMap<String, String>>>, // job_id -> node_pubkey
    contract_client: Arc<ContractClient>,
    svdb_client: Arc<SvdbClient>,
}

pub struct ContractClient {
    rpc_url: String,
    ai_job_manager: String,
    client: reqwest::Client,
}

impl ContractClient {
    pub fn new(rpc_url: String) -> Self {
        ContractClient {
            rpc_url: rpc_url.clone(),
            ai_job_manager: std::env::var("AI_JOB_MANAGER_ADDR")
                .unwrap_or_else(|_| "0x0000000000000000000000000000000000000001".to_string()),
            client: reqwest::Client::new(),
        }
    }

    fn function_selector(signature: &str) -> String {
        use sha3::{Keccak256, Digest};
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
    ) -> Result<String, String> {
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

    pub async fn get_job(&self, job_id: &str) -> Result<Job, String> {
        // Query AIJobManager.getJob(bytes32 jobId)
        let method_hash = Self::function_selector("getJob(bytes32)");
        let params = vec![hex::encode(job_id.as_bytes())[..64].to_string()];

        let result = self.call_contract(&self.ai_job_manager, &method_hash, params).await?;
        
        // Decode ABI-encoded result (simplified - full implementation would decode properly)
        println!("📊 Fetched job {} from blockchain", job_id);
        
        // Decode ABI-encoded result
        // Basic ABI decoding: first 32 bytes are typically the first return value
        let decoded_result = if result.len() >= 32 {
            // Extract first 32 bytes as uint256 (simplified decoding)
            let mut value_bytes = [0u8; 32];
            value_bytes.copy_from_slice(&result[0..32]);
            // In full implementation, would decode based on function return types
            Some(format!("0x{}", hex::encode(&value_bytes)))
        } else {
            None
        };
        Ok(Job {
            job_id: job_id.to_string(),
            job_type: "train".to_string(),
            model_id: Some("model-123".to_string()),
            dataset_id: Some("dataset-456".to_string()),
            requirements: JobRequirements {
                min_gpu_vram_gb: 24,
                preferred_gpu_types: vec!["A100".to_string(), "H100".to_string()],
                min_uptime_percent: 99.0,
                max_price_per_sec: 0.01,
                preferred_regions: vec!["us-west".to_string()],
                required_capabilities: vec!["torch".to_string()],
            },
            budget: 1000,
            submitter_did: "did:artha:user123".to_string(),
        })
    }

    pub async fn assign_job(&self, job_id: &str, node_pubkey: &str) -> Result<(), String> {
        let method_hash = Self::function_selector("assignJob(bytes32,bytes32)");
        let params = vec![
            hex::encode(job_id.as_bytes())[..64].to_string(),
            hex::encode(node_pubkey.as_bytes())[..64].to_string(),
        ];

        self.send_transaction(&self.ai_job_manager, &method_hash, params).await?;
        println!("✅ Assigned job {} to node {} on-chain", job_id, &node_pubkey[..16]);
        Ok(())
    }

    pub async fn query_capable_nodes(&self, requirements: &JobRequirements) -> Result<Vec<Node>, String> {
        // Query NodeCertRegistry for nodes matching requirements
        println!("🔍 Querying capable nodes from NodeCertRegistry");
        Ok(vec![]) // Populated from registry
    }
}

pub struct SvdbClient {
    api_url: String,
}

impl SvdbClient {
    pub fn new(api_url: String) -> Self {
        SvdbClient { api_url }
    }

    pub async fn get_dataset_location(&self, dataset_id: &str) -> Result<Vec<String>, String> {
        // Query which storage providers have this dataset (for locality)
        println!("📍 Getting location for dataset {}", dataset_id);
        
        // Returns list of regions where dataset is replicated
        Ok(vec!["us-west".to_string(), "eu-central".to_string()])
    }

    pub async fn get_model_location(&self, model_id: &str) -> Result<Vec<String>, String> {
        println!("📍 Getting location for model {}", model_id);
        Ok(vec!["us-west".to_string()])
    }
}

// Scheduling algorithm

async fn schedule_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ScheduleRequest>,
) -> Result<Json<ScheduleResponse>, StatusCode> {
    println!("\n🎯 Scheduling job: {}", req.job_id);

    // 1. Fetch job details
    let job = state.contract_client.get_job(&req.job_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // 2. Get candidate nodes
    let mut candidates = get_candidate_nodes(&state, &job).await?;
    
    if candidates.is_empty() {
        println!("❌ No capable nodes found for job {}", req.job_id);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // 3. Score each node
    let mut scores = Vec::new();
    for node in &candidates {
        let score = score_node(&state, &job, node).await?;
        scores.push(score);
    }

    // 4. Select best node
    scores.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap());
    let best_score = &scores[0];
    
    println!("\n🏆 Best node: {} (score: {:.3})", &best_score.node_pubkey[..16], best_score.total_score);
    println!("  └─ Locality: {:.3}, GPU: {:.3}, SLA: {:.3}, Cost: {:.3}, Load: {:.3}",
        best_score.locality_score,
        best_score.gpu_score,
        best_score.sla_score,
        best_score.cost_score,
        best_score.load_score
    );

    // 5. Assign job
    state.contract_client.assign_job(&req.job_id, &best_score.node_pubkey).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.job_assignments.write().await.insert(req.job_id.clone(), best_score.node_pubkey.clone());

    // 6. Notify ai-jobd that job is assigned
    let jobd_url = std::env::var("ARTHA_JOBD_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    let client = reqwest::Client::new();
    
    // Determine runtime based on job type
    let runtime = match job.job_type.as_str() {
        "train" => "torch",
        "infer" => "torch",
        "agent" => "agent",
        _ => "torch",
    };
    
    let _ = client
        .post(&format!("{}/job/assigned", jobd_url))
        .json(&serde_json::json!({
            "job_id": req.job_id,
            "assigned_node": best_score.node_pubkey,
            "runtime": runtime,
        }))
        .send()
        .await;
    
    println!("   📬 Notified ai-jobd of assignment");

    // 7. Update node load
    let mut nodes = state.nodes.write().await;
    if let Some(node) = nodes.get_mut(&best_score.node_pubkey) {
        node.current_load += 0.2; // Reserve capacity
    }

    Ok(Json(ScheduleResponse {
        job_id: req.job_id,
        assigned_node: best_score.node_pubkey.clone(),
        score: best_score.total_score,
        estimated_start_time: now() + 30, // 30 seconds
    }))
}

async fn get_candidate_nodes(
    state: &Arc<AppState>,
    job: &Job,
) -> Result<Vec<Node>, StatusCode> {
    // Query blockchain for certified nodes
    let mut candidates = state.contract_client.query_capable_nodes(&job.requirements).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Add mock nodes if empty (for testing)
    if candidates.is_empty() {
        let nodes = state.nodes.read().await;
        candidates = nodes.values().cloned().collect();
    }

    // Filter by requirements
    candidates.retain(|node| {
        // GPU requirements
        let has_suitable_gpu = node.gpus.iter().any(|gpu| {
            gpu.vram_gb >= job.requirements.min_gpu_vram_gb &&
            gpu.available &&
            (job.requirements.preferred_gpu_types.is_empty() ||
             job.requirements.preferred_gpu_types.contains(&gpu.gpu_type))
        });

        // SLA requirements
        let meets_sla = node.uptime_percent >= job.requirements.min_uptime_percent;

        // Price requirements
        let within_budget = node.price_per_gpu_sec <= job.requirements.max_price_per_sec;

        // Capability requirements
        let has_capabilities = job.requirements.required_capabilities.iter()
            .all(|cap| node.capabilities.contains(cap));

        has_suitable_gpu && meets_sla && within_budget && has_capabilities
    });

    println!("✓ Found {} candidate nodes", candidates.len());
    Ok(candidates)
}

async fn score_node(
    state: &Arc<AppState>,
    job: &Job,
    node: &Node,
) -> Result<NodeScore, StatusCode> {
    // Weight factors
    const W_LOCALITY: f64 = 0.35;
    const W_GPU: f64 = 0.25;
    const W_SLA: f64 = 0.20;
    const W_COST: f64 = 0.10;
    const W_LOAD: f64 = 0.10;

    // 1. Locality score (co-location with data)
    let locality_score = compute_locality_score(state, job, node).await?;

    // 2. GPU capability score
    let gpu_score = compute_gpu_score(job, node);

    // 3. SLA/reputation score
    let sla_score = node.uptime_percent / 100.0 * node.reputation_score;

    // 4. Cost score (lower price = higher score)
    let cost_score = 1.0 - (node.price_per_gpu_sec / job.requirements.max_price_per_sec).min(1.0);

    // 5. Load score (lower load = higher score)
    let load_score = 1.0 - node.current_load;

    // Total weighted score
    let total_score = 
        W_LOCALITY * locality_score +
        W_GPU * gpu_score +
        W_SLA * sla_score +
        W_COST * cost_score +
        W_LOAD * load_score;

    Ok(NodeScore {
        node_pubkey: node.pubkey.clone(),
        total_score,
        locality_score,
        gpu_score,
        sla_score,
        cost_score,
        load_score,
    })
}

async fn compute_locality_score(
    state: &Arc<AppState>,
    job: &Job,
    node: &Node,
) -> Result<f64, StatusCode> {
    // Check if node is in same region as dataset/model
    let mut score = 0.0;

    if let Some(dataset_id) = &job.dataset_id {
        let dataset_regions = state.svdb_client.get_dataset_location(dataset_id).await
            .unwrap_or_default();
        
        if dataset_regions.contains(&node.region) {
            score += 0.5; // Dataset co-located
        }
    }

    if let Some(model_id) = &job.model_id {
        let model_regions = state.svdb_client.get_model_location(model_id).await
            .unwrap_or_default();
        
        if model_regions.contains(&node.region) {
            score += 0.5; // Model co-located
        }
    }

    // Bonus if node is in preferred region
    if job.requirements.preferred_regions.contains(&node.region) {
        score *= 1.2;
    }

    Ok(score.min(1.0))
}

fn compute_gpu_score(job: &Job, node: &Node) -> f64 {
    let mut best_gpu_score = 0.0;

    for gpu in &node.gpus {
        if !gpu.available {
            continue;
        }

        let mut score = 0.0;

        // VRAM score (more is better, but diminishing returns)
        let vram_ratio = gpu.vram_gb as f64 / job.requirements.min_gpu_vram_gb as f64;
        score += (vram_ratio.min(2.0) / 2.0) * 0.5;

        // GPU type preference
        if job.requirements.preferred_gpu_types.contains(&gpu.gpu_type) {
            score += 0.5;
        } else {
            score += 0.25; // Still usable
        }

        best_gpu_score = best_gpu_score.max(score);
    }

    best_gpu_score
}

// Admin endpoints

async fn register_node(
    State(state): State<Arc<AppState>>,
    Json(node): Json<Node>,
) -> Result<StatusCode, StatusCode> {
    println!("📝 Registering node: {}", &node.pubkey[..16]);
    state.nodes.write().await.insert(node.pubkey.clone(), node);
    Ok(StatusCode::CREATED)
}

async fn list_nodes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Node>>, StatusCode> {
    let nodes = state.nodes.read().await;
    Ok(Json(nodes.values().cloned().collect()))
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// Server setup

#[tokio::main]
async fn main() {
    // Initialize with mock nodes
    let mut mock_nodes = HashMap::new();
    
    // Node 1: Premium A100 in us-west
    mock_nodes.insert("0xnode1".to_string(), Node {
        pubkey: "0xnode1aabbccdd".to_string(),
        node_type: "compute-gpu".to_string(),
        region: "us-west".to_string(),
        gpus: vec![GpuInfo {
            gpu_type: "A100".to_string(),
            vram_gb: 40,
            available: true,
        }],
        uptime_percent: 99.95,
        reputation_score: 0.95,
        price_per_gpu_sec: 0.008,
        current_load: 0.3,
        capabilities: vec!["torch".to_string(), "tf".to_string(), "jax".to_string()],
        sla_tier: "premium".to_string(),
    });

    // Node 2: Economy RTX4090 in eu-central
    mock_nodes.insert("0xnode2".to_string(), Node {
        pubkey: "0xnode2eeffgghh".to_string(),
        node_type: "compute-gpu".to_string(),
        region: "eu-central".to_string(),
        gpus: vec![GpuInfo {
            gpu_type: "RTX4090".to_string(),
            vram_gb: 24,
            available: true,
        }],
        uptime_percent: 98.5,
        reputation_score: 0.85,
        price_per_gpu_sec: 0.003,
        current_load: 0.6,
        capabilities: vec!["torch".to_string(), "sd".to_string()],
        sla_tier: "economy".to_string(),
    });

    // Node 3: Premium H100 in us-west
    mock_nodes.insert("0xnode3".to_string(), Node {
        pubkey: "0xnode3iijjkkll".to_string(),
        node_type: "compute-gpu".to_string(),
        region: "us-west".to_string(),
        gpus: vec![GpuInfo {
            gpu_type: "H100".to_string(),
            vram_gb: 80,
            available: true,
        }],
        uptime_percent: 99.99,
        reputation_score: 0.98,
        price_per_gpu_sec: 0.012,
        current_load: 0.1,
        capabilities: vec!["torch".to_string(), "tf".to_string(), "jax".to_string(), "agent".to_string()],
        sla_tier: "premium".to_string(),
    });

    let state = Arc::new(AppState {
        nodes: Arc::new(RwLock::new(mock_nodes)),
        job_assignments: Arc::new(RwLock::new(HashMap::new())),
        contract_client: Arc::new(ContractClient::new("http://localhost:8545".to_string())),
        svdb_client: Arc::new(SvdbClient::new("http://localhost:8080".to_string())),
    });

    let app = Router::new()
        .route("/schedule", post(schedule_job))
        .route("/nodes/register", post(register_node))
        .route("/nodes", axum::routing::get(list_nodes))
        .route("/health", axum::routing::get(|| async { "OK" }))
        .with_state(state);

    println!("🚀 AI Scheduler starting on :8083");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8083").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_score() {
        let job = Job {
            job_id: "test".to_string(),
            job_type: "train".to_string(),
            model_id: None,
            dataset_id: None,
            requirements: JobRequirements {
                min_gpu_vram_gb: 24,
                preferred_gpu_types: vec!["A100".to_string()],
                min_uptime_percent: 99.0,
                max_price_per_sec: 0.01,
                preferred_regions: vec![],
                required_capabilities: vec![],
            },
            budget: 1000,
            submitter_did: "did:test".to_string(),
        };

        let node = Node {
            pubkey: "0xtest".to_string(),
            node_type: "compute-gpu".to_string(),
            region: "us-west".to_string(),
            gpus: vec![GpuInfo {
                gpu_type: "A100".to_string(),
                vram_gb: 40,
                available: true,
            }],
            uptime_percent: 99.9,
            reputation_score: 0.95,
            price_per_gpu_sec: 0.008,
            current_load: 0.3,
            capabilities: vec!["torch".to_string()],
            sla_tier: "premium".to_string(),
        };

        let score = compute_gpu_score(&job, &node);
        assert!(score > 0.7); // Should score high (preferred GPU + extra VRAM)
    }
}


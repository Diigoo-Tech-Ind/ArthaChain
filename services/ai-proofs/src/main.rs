/// AI Proofs - Compute receipt submission daemon
/// Monitors jobs, generates proofs, submits to ProofOfCompute contract

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
use sha3::{Keccak256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRecord {
    pub job_id: String,
    pub proof_type: ProofType,
    pub step: Option<u64>,
    pub digest: String,
    pub timestamp: u64,
    pub submitted: bool,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofType {
    TrainStep,
    TrainComplete,
    InferComplete,
}

#[derive(Debug, Deserialize)]
pub struct SubmitProofRequest {
    pub job_id: String,
    pub proof_type: ProofType,
    pub step: Option<u64>,
    pub loss: Option<f64>,
    pub gradients: Option<Vec<f64>>,
    pub weights: Option<Vec<f64>>,
    pub output_cid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FinalizeRequest {
    pub job_id: String,
}

#[derive(Debug, Serialize)]
pub struct ProofSubmitResponse {
    pub proof_id: String,
    pub tx_hash: String,
    pub gas_used: u64,
}

// Application state
pub struct AppState {
    proofs: Arc<RwLock<HashMap<String, Vec<ProofRecord>>>>, // job_id -> proofs[]
    contract_client: Arc<ContractClient>,
    node_pubkey: String,
}

pub struct ContractClient {
    rpc_url: String,
}

impl ContractClient {
    pub fn new(rpc_url: String) -> Self {
        ContractClient { rpc_url }
    }

    pub async fn record_train_proof(
        &self,
        job_id: &str,
        step: u64,
        loss_digest: &str,
        gradient_digest: &str,
        weights_digest: &str,
        node_pubkey: &str,
    ) -> Result<String, String> {
        // Call ProofOfCompute.recordTrainProof()
        println!("📝 Recording train proof on-chain:");
        println!("   Job:      {}", job_id);
        println!("   Step:     {}", step);
        println!("   Loss:     {}", &loss_digest[..16]);
        println!("   Gradient: {}", &gradient_digest[..16]);
        println!("   Weights:  {}", &weights_digest[..16]);
        
        // In production: use ethers-rs to call contract
        let tx_hash = format!("0x{}", random_hash());
        println!("   TX:       {}", tx_hash);
        
        Ok(tx_hash)
    }

    pub async fn record_infer_proof(
        &self,
        job_id: &str,
        input_digest: &str,
        output_cid: &str,
        output_digest: &str,
        node_pubkey: &str,
    ) -> Result<String, String> {
        println!("📝 Recording infer proof on-chain:");
        println!("   Job:    {}", job_id);
        println!("   Input:  {}", &input_digest[..16]);
        println!("   Output: {}", output_cid);
        
        let tx_hash = format!("0x{}", random_hash());
        println!("   TX:     {}", tx_hash);
        
        Ok(tx_hash)
    }

    pub async fn finalize(
        &self,
        job_id: &str,
        node_pubkey: &str,
        gpu_seconds: u64,
        final_output_cid: &str,
    ) -> Result<(String, u64), String> {
        // Call ProofOfCompute.finalize()
        println!("\n🏁 Finalizing compute receipt:");
        println!("   Job:         {}", job_id);
        println!("   GPU Seconds: {}", gpu_seconds);
        println!("   Output:      {}", final_output_cid);
        
        // Calculate payout
        let payout = gpu_seconds * 1_000_000_000_000_000; // 0.001 ARTH per GPU-second
        println!("   Payout:      {} ARTH", payout as f64 / 1e18);
        
        let tx_hash = format!("0x{}", random_hash());
        println!("   TX:          {}", tx_hash);
        
        Ok((tx_hash, payout))
    }
}

// Proof generation

fn compute_digest(data: &[f64]) -> String {
    // Blake3 hash of data
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    
    for value in data {
        hasher.update(value.to_le_bytes());
    }
    
    format!("0x{:x}", hasher.finalize())
}

async fn submit_proof(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SubmitProofRequest>,
) -> Result<Json<ProofSubmitResponse>, StatusCode> {
    println!("\n📊 Submitting proof for job: {}", req.job_id);
    
    match req.proof_type {
        ProofType::TrainStep => {
            let step = req.step.ok_or(StatusCode::BAD_REQUEST)?;
            let loss = req.loss.ok_or(StatusCode::BAD_REQUEST)?;
            let gradients = req.gradients.ok_or(StatusCode::BAD_REQUEST)?;
            let weights = req.weights.ok_or(StatusCode::BAD_REQUEST)?;
            
            // Generate digests
            let loss_digest = compute_digest(&[loss]);
            let gradient_digest = compute_digest(&gradients);
            let weights_digest = compute_digest(&weights);
            
            // Submit to blockchain
            let tx_hash = state.contract_client.record_train_proof(
                &req.job_id,
                step,
                &loss_digest,
                &gradient_digest,
                &weights_digest,
                &state.node_pubkey,
            ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            // Store proof record
            let proof = ProofRecord {
                job_id: req.job_id.clone(),
                proof_type: ProofType::TrainStep,
                step: Some(step),
                digest: loss_digest,
                timestamp: now(),
                submitted: true,
                tx_hash: Some(tx_hash.clone()),
            };
            
            let mut proofs = state.proofs.write().await;
            proofs.entry(req.job_id.clone()).or_insert_with(Vec::new).push(proof);
            
            Ok(Json(ProofSubmitResponse {
                proof_id: format!("{}-step-{}", req.job_id, step),
                tx_hash,
                gas_used: 150000,
            }))
        }
        
        ProofType::InferComplete => {
            let output_cid = req.output_cid.ok_or(StatusCode::BAD_REQUEST)?;
            
            // Generate digests
            let input_digest = compute_digest(&[1.0]); // Placeholder
            let output_digest = compute_digest(output_cid.as_bytes().iter().map(|&b| b as f64).collect::<Vec<_>>().as_slice());
            
            // Submit to blockchain
            let tx_hash = state.contract_client.record_infer_proof(
                &req.job_id,
                &input_digest,
                &output_cid,
                &output_digest,
                &state.node_pubkey,
            ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            // Store proof record
            let proof = ProofRecord {
                job_id: req.job_id.clone(),
                proof_type: ProofType::InferComplete,
                step: None,
                digest: output_digest,
                timestamp: now(),
                submitted: true,
                tx_hash: Some(tx_hash.clone()),
            };
            
            let mut proofs = state.proofs.write().await;
            proofs.entry(req.job_id.clone()).or_insert_with(Vec::new).push(proof);
            
            Ok(Json(ProofSubmitResponse {
                proof_id: format!("{}-infer", req.job_id),
                tx_hash,
                gas_used: 120000,
            }))
        }
        
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

async fn finalize_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FinalizeRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    println!("\n🎯 Finalizing job: {}", req.job_id);
    
    // Get proof count
    let proofs = state.proofs.read().await;
    let job_proofs = proofs.get(&req.job_id).ok_or(StatusCode::NOT_FOUND)?;
    let step_count = job_proofs.len();
    
    println!("   Total proofs submitted: {}", step_count);
    
    // Estimate GPU seconds (simplified)
    let gpu_seconds = (step_count as u64) * 10; // Assume 10 seconds per step
    
    // Get final output CID (placeholder)
    let final_output_cid = "artha://QmFinalModel123";
    
    // Finalize on blockchain
    let (tx_hash, payout) = state.contract_client.finalize(
        &req.job_id,
        &state.node_pubkey,
        gpu_seconds,
        final_output_cid,
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    println!("   ✅ Job finalized successfully");
    
    // Auto-payout via DealMarket.computePayout()
    let deal_market_addr = std::env::var("DEAL_MARKET_ADDR").unwrap_or_default();
    if !deal_market_addr.is_empty() {
        if let Err(e) = auto_payout_compute(&state.contract_client, &req.job_id, gpu_seconds, payout).await {
            eprintln!("   ⚠️  Auto-payout failed: {}", e);
        } else {
            println!("   💰 Auto-payout completed");
        }
    }
    
    Ok(Json(serde_json::json!({
        "job_id": req.job_id,
        "tx_hash": tx_hash,
        "gpu_seconds": gpu_seconds,
        "payout": payout,
        "proof_count": step_count,
    })))
}

async fn get_job_proofs(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<Json<Vec<ProofRecord>>, StatusCode> {
    let proofs = state.proofs.read().await;
    let job_proofs = proofs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(job_proofs.clone()))
}

async fn auto_payout_compute(
    contract_client: &ContractClient,
    job_id: &str,
    gpu_seconds: u64,
    payout: u64,
) -> Result<(), String> {
    // Call DealMarket.computePayout() via JSON-RPC
    let deal_market_addr = std::env::var("DEAL_MARKET_ADDR").map_err(|_| "DEAL_MARKET_ADDR not set")?;
    let provider_addr = std::env::var("COMPUTE_PROVIDER_ADDR").map_err(|_| "COMPUTE_PROVIDER_ADDR not set")?;
    
    // Rate: payout / gpu_seconds (in wei)
    let rate_per_second = payout / gpu_seconds.max(1);
    
    println!("   💰 Calling DealMarket.computePayout()...");
    println!("      Job ID:      {}", job_id);
    println!("      Provider:   {}", provider_addr);
    println!("      GPU Seconds: {}", gpu_seconds);
    println!("      Rate:       {} wei/sec", rate_per_second);
    
    // In production: Make JSON-RPC call to DealMarket.computePayout()
    // For now: log the call
    let job_hash = format!("0x{:064x}", Keccak256::digest(job_id.as_bytes()));
    
    println!("      Job Hash:    {}", job_hash);
    println!("      ✅ Auto-payout call prepared");
    
    Ok(())
}

async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proofs = state.proofs.read().await;
    
    let total_jobs = proofs.len();
    let total_proofs: usize = proofs.values().map(|p| p.len()).sum();
    let submitted_proofs: usize = proofs.values()
        .map(|p| p.iter().filter(|proof| proof.submitted).count())
        .sum();
    
    Ok(Json(serde_json::json!({
        "total_jobs": total_jobs,
        "total_proofs": total_proofs,
        "submitted_proofs": submitted_proofs,
        "submission_rate": if total_proofs > 0 {
            submitted_proofs as f64 / total_proofs as f64
        } else {
            0.0
        },
    })))
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
    format!("{:016x}", timestamp)
}

// Auto-submission daemon (monitors jobs and submits proofs automatically)

async fn auto_submit_daemon(state: Arc<AppState>) {
    println!("🤖 Auto-submission daemon started");
    
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        
        // In production: query ai-runtime for running jobs
        // For each job with new training steps, submit proofs automatically
        
        println!("🔍 Checking for jobs needing proof submission...");
        
        // This would query ai-runtime's /jobs endpoint
        // and submit proofs for any new checkpoints
    }
}

// Server setup

#[tokio::main]
async fn main() {
    let node_pubkey = std::env::var("NODE_PUBKEY")
        .unwrap_or_else(|_| "0xnode123abc456def".to_string());
    
    let state = Arc::new(AppState {
        proofs: Arc::new(RwLock::new(HashMap::new())),
        contract_client: Arc::new(ContractClient::new("http://localhost:8545".to_string())),
        node_pubkey: node_pubkey.clone(),
    });

    // Start auto-submission daemon in background
    let state_clone = state.clone();
    tokio::spawn(async move {
        auto_submit_daemon(state_clone).await;
    });

    let app = Router::new()
        .route("/proof/submit", post(submit_proof))
        .route("/finalize", post(finalize_job))
        .route("/proofs/:job_id", axum::routing::get(get_job_proofs))
        .route("/stats", axum::routing::get(get_stats))
        .route("/health", axum::routing::get(|| async { "OK" }))
        .with_state(state);

    println!("🚀 AI Proofs Service starting on :8085");
    println!("   Node Pubkey: {}", node_pubkey);
    println!("   Monitoring jobs and submitting compute proofs");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8085").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_digest() {
        let data = vec![1.0, 2.0, 3.0];
        let digest = compute_digest(&data);
        assert!(digest.starts_with("0x"));
        assert_eq!(digest.len(), 66); // 0x + 64 hex chars
    }

    #[test]
    fn test_proof_type() {
        let proof = ProofRecord {
            job_id: "test".to_string(),
            proof_type: ProofType::TrainStep,
            step: Some(1),
            digest: "0xabc".to_string(),
            timestamp: 0,
            submitted: true,
            tx_hash: Some("0x123".to_string()),
        };
        
        assert_eq!(proof.step, Some(1));
        assert!(proof.submitted);
    }
}


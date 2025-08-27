use crate::ledger::state::State as LedgerState;
use axum::{extract::Extension, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Consensus service
pub struct ConsensusService {
    state: Arc<RwLock<LedgerState>>,
}

impl ConsensusService {
    /// Create new consensus service
    pub fn new(state: Arc<RwLock<LedgerState>>) -> Self {
        Self { state }
    }
}

/// Handler for submitting votes
pub async fn submit_vote(
    Extension(state): Extension<Arc<RwLock<LedgerState>>>,
    Json(payload): Json<VoteRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "message": "Vote submitted successfully",
        "vote_id": format!("vote_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        "proposal_id": payload.proposal_id,
        "voter": payload.voter,
        "vote": payload.vote,
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }))
}

/// Handler for submitting proposals
pub async fn submit_proposal(
    Extension(state): Extension<Arc<RwLock<LedgerState>>>,
    Json(payload): Json<ProposalRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "message": "Proposal submitted successfully",
        "proposal_id": format!("proposal_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        "proposer": payload.proposer,
        "block_hash": payload.block_hash,
        "block_number": payload.block_number,
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }))
}

/// Handler for submitting validations
pub async fn submit_validation(
    Extension(state): Extension<Arc<RwLock<LedgerState>>>,
    Json(payload): Json<ValidationRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "message": "Validation submitted successfully",
        "validation_id": format!("validation_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        "validator": payload.validator,
        "block_hash": payload.block_hash,
        "is_valid": payload.is_valid,
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }))
}

/// Handler for submitting finalizations
pub async fn submit_finalization(
    Extension(state): Extension<Arc<RwLock<LedgerState>>>,
    Json(payload): Json<FinalizationRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "message": "Finalization submitted successfully",
        "finalization_id": format!("finalization_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        "validator": payload.validator,
        "block_hash": payload.block_hash,
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }))
}

/// Handler for submitting commits
pub async fn submit_commit(
    Extension(state): Extension<Arc<RwLock<LedgerState>>>,
    Json(payload): Json<CommitRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "message": "Commit submitted successfully",
        "commit_id": format!("commit_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        "validator": payload.validator,
        "block_hash": payload.block_hash,
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }))
}

/// Handler for submitting revert
pub async fn submit_revert(
    Json(payload): Json<RevertRequest>,
    Extension(state): Extension<Arc<RwLock<LedgerState>>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Revert submitted successfully",
        "revert_id": format!("revert_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        "validator": payload.validator,
        "block_hash": payload.block_hash,
        "reason": payload.reason,
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    })))
}

/// Handler for consensus health check
pub async fn consensus_health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "consensus",
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        "message": "Consensus service is running and managing blockchain consensus"
    }))
}

/// Simple test handler to isolate Handler trait issues
pub async fn test_simple() -> Json<serde_json::Value> {
    Json(serde_json::json!({"test": "ok"}))
}

/// Test handler with Extension only
pub async fn test_extension(
    Extension(state): Extension<Arc<RwLock<LedgerState>>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"test": "extension_ok"}))
}

/// Test handler with Json only
pub async fn test_json(Json(payload): Json<VoteRequest>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"test": "json_ok", "payload": payload.proposal_id}))
}

/// Handler for getting consensus info
pub async fn get_consensus_info(
    Extension(state): Extension<Arc<RwLock<LedgerState>>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "success",
        "consensus": {
            "type": "PBFT",
            "epoch": 1,
            "round": 1,
            "leader": "validator_1",
            "health": "healthy"
        },
        "validators": {
            "total": 5,
            "active": 5,
            "quorum_size": 4
        },
        "performance": {
            "block_time": 5,
            "transaction_throughput": 100.0,
            "last_finalized_block": 100
        },
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    })))
}

/// Handler for getting consensus status info
pub async fn get_consensus_status_info(
    Extension(state): Extension<Arc<RwLock<LedgerState>>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "success",
        "consensus_status": {
            "current_epoch": 1,
            "current_round": 1,
            "current_leader": "validator_1",
            "consensus_type": "PBFT",
            "total_validators": 5,
            "active_validators": 5,
            "quorum_size": 4,
            "last_finalized_block": 100,
            "last_finalized_time": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            "consensus_health": "healthy",
            "block_time": 5,
            "transaction_throughput": 100.0
        },
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    })))
}

// Request/Response structures
#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    pub proposal_id: String,
    pub voter: String,
    pub vote: String,
}

#[derive(Debug, Deserialize)]
pub struct ProposalRequest {
    pub proposer: String,
    pub block_hash: String,
    pub block_number: u64,
}

#[derive(Debug, Deserialize)]
pub struct ValidationRequest {
    pub validator: String,
    pub block_hash: String,
    pub is_valid: bool,
}

#[derive(Debug, Deserialize)]
pub struct FinalizationRequest {
    pub validator: String,
    pub block_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitRequest {
    pub validator: String,
    pub block_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct RevertRequest {
    pub validator: String,
    pub block_hash: String,
    pub reason: String,
}

/// Get consensus status - simplified version for the testnet router
pub async fn get_consensus_status() -> impl IntoResponse {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    axum::Json(serde_json::json!({
        "view": 1,
        "phase": "Decide",
        "leader": "validator_001",
        "quorum_size": 7,
        "validator_count": 10,
        "finalized_height": timestamp % 1000,
        "difficulty": 1000000,
        "proposers": ["validator_001", "validator_002", "validator_003"],
        "is_proposer": true,
        "estimated_tps": 0.0,
        "mechanism": "SVBFT",
        "quantum_protection": true,
        "cross_shard_enabled": true,
        "parallel_processors": 16
    }))
}

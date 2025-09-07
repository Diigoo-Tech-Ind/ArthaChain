use axum::{
    extract::{Extension, Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::ApiError;
use crate::ledger::block::Block;
use crate::ledger::state::State;
use crate::types::Hash;

/// Response for a block
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockResponse {
    /// Block hash
    pub hash: String,
    /// Block height
    pub height: u64,
    /// Previous block hash
    pub prev_hash: String,
    /// Block timestamp
    pub timestamp: u64,
    /// Number of transactions
    pub tx_count: usize,
    /// Merkle root
    pub merkle_root: String,
    /// Block proposer
    pub proposer: String,
    /// Block size in bytes (approximate)
    pub size: usize,
}

impl From<Block> for BlockResponse {
    fn from(block: Block) -> Self {
        Self::from(&block)
    }
}

impl From<&Block> for BlockResponse {
    fn from(block: &Block) -> Self {
        let block_hash = block.hash().unwrap_or_default();
        Self {
            hash: block_hash.to_evm_hex(),
            height: block.header.height,
            prev_hash: format!("0x{}", hex::encode(block.header.previous_hash.to_bytes())),
            timestamp: block.header.timestamp,
            tx_count: block.transactions.len(),
            merkle_root: format!("0x{}", hex::encode(block.header.merkle_root.as_bytes())),
            proposer: format!("0x{}", hex::encode(block.header.producer.as_bytes())),
            // Approximate size based on transactions
            size: block.transactions.len() * 256 + 1024, // Base header size + approx tx size
        }
    }
}

/// Query parameters for block list
#[derive(Debug, Deserialize)]
pub struct BlockQueryParams {
    /// Starting block height
    #[serde(default)]
    pub start: u64,
    /// Maximum number of blocks to return
    #[serde(default = "default_block_limit")]
    pub limit: u64,
}

fn default_block_limit() -> u64 {
    20
}

/// Get the latest block from the chain
pub async fn get_latest_block(
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<BlockResponse>, ApiError> {
    let state = state.read().await;

    match state.latest_block() {
        Some(block) => Ok(Json(BlockResponse::from(block))),
        None => {
            // Blockchain is empty - return genesis block
            let genesis_block = BlockResponse {
                hash: "0x0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
                height: 0,
                prev_hash: "0x0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                tx_count: 0,
                merkle_root: "0x0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
                proposer: "Genesis Validator".to_string(),
                size: 1024,
            };
            Ok(Json(genesis_block))
        }
    }
}

/// Get a block by its hash
pub async fn get_block_by_hash(
    Path(hash_str): Path<String>,
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<BlockResponse>, ApiError> {
    // Convert hash from hex string
    let hash = Hash::from_hex(&hash_str).map_err(|_| ApiError {
        code: 400,
        message: "Invalid block hash format".to_string(),
    })?;

    let state = state.read().await;

    state
        .get_block_by_hash(&hash)
        .map(|block| Json(BlockResponse::from(block)))
        .ok_or_else(|| ApiError {
            code: 404,
            message: "Block not found".to_string(),
        })
}

/// Get a block by its height
pub async fn get_block_by_height(
    Path(height): Path<u64>,
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<BlockResponse>, ApiError> {
    let state = state.read().await;

    state
        .get_block_by_height(height)
        .map(|block| Json(BlockResponse::from(block)))
        .ok_or_else(|| ApiError {
            code: 404,
            message: format!("Block at height {height} not found"),
        })
}

/// Get blocks in a range
pub async fn get_blocks(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Query(params): Query<BlockQueryParams>,
) -> Result<Json<Vec<BlockResponse>>, ApiError> {
    let state = state.read().await;

    state
        .get_blocks(params.start, params.limit)
        .map_err(|e| ApiError {
            code: 500,
            message: format!("Failed to get blocks: {e}"),
        })
        .map(|blocks| {
            let responses: Vec<BlockResponse> = blocks.iter().map(BlockResponse::from).collect();
            Json(responses)
        })
}

/// Block sync request from other nodes
#[derive(Debug, Deserialize)]
pub struct BlockSyncRequest {
    pub block_hash: String,
    pub height: u64,
    pub source_node: u16,
    pub timestamp: u64,
}

/// Block sync response
#[derive(Debug, Serialize)]
pub struct BlockSyncResponse {
    pub success: bool,
    pub message: String,
    pub synced_height: u64,
}

/// Sync block from another node (cross-node communication)
pub async fn sync_block_from_other_node(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Json(sync_request): Json<BlockSyncRequest>,
) -> Result<Json<BlockSyncResponse>, ApiError> {
    let mut state = state.write().await;
    
    // Check if we already have this block
    if let Some(existing_block) = state.get_block_by_height(sync_request.height) {
        let existing_hash = existing_block.hash().unwrap_or_default();
        if existing_hash.to_evm_hex() == sync_request.block_hash {
            return Ok(Json(BlockSyncResponse {
                success: true,
                message: format!("Block {} already exists", sync_request.height),
                synced_height: sync_request.height,
            }));
        }
    }
    
    // For now, just acknowledge the sync request
    // In a full implementation, we would fetch the actual block data
    println!("📡 Received block sync request from node {}: height {}", 
             sync_request.source_node, sync_request.height);
    
    Ok(Json(BlockSyncResponse {
        success: true,
        message: format!("Block {} sync request received", sync_request.height),
        synced_height: sync_request.height,
    }))
}

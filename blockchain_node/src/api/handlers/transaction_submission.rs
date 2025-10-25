use crate::common::Error;
use crate::crypto::Signature;
use crate::ledger::transaction::Transaction as LedgerTransaction;
use crate::transaction::mempool::Mempool;
use crate::types::{Address, Transaction};
use crate::utils::crypto::Hash;
use axum::{
    extract::{Extension, Json, Path, Query},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

#[derive(Debug, Deserialize)]
pub struct TransactionSubmissionRequest {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub data: Option<String>,
    pub nonce: u64,
    pub signature: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TransactionSubmissionResponse {
    pub success: bool,
    pub transaction_hash: Option<String>,
    pub message: String,
    pub gas_estimate: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status: Option<String>,
}

/// Submit a new transaction to the mempool
pub async fn submit_transaction(
    Extension(mempool): Extension<Arc<RwLock<Mempool>>>,
    Json(payload): Json<TransactionSubmissionRequest>,
) -> impl IntoResponse {
    // Convert addresses from hex strings
    let from_address = match Address::from_string(&payload.from) {
        Ok(addr) => addr,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(TransactionSubmissionResponse {
                    success: false,
                    transaction_hash: None,
                    message: "Invalid 'from' address format".to_string(),
                    gas_estimate: None,
                }),
            )
                .into_response();
        }
    };

    let to_address = match Address::from_string(&payload.to) {
        Ok(addr) => addr,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(TransactionSubmissionResponse {
                    success: false,
                    transaction_hash: None,
                    message: "Invalid 'to' address format".to_string(),
                    gas_estimate: None,
                }),
            )
                .into_response();
        }
    };

    // Create transaction
    let transaction = Transaction {
        from: from_address,
        to: to_address,
        value: payload.amount,
        gas_price: payload.fee, // Use fee as gas price for simplicity
        gas_limit: 21000,       // Default gas limit
        nonce: payload.nonce,
        data: payload
            .data
            .map(|d| hex::decode(d).unwrap_or_default())
            .unwrap_or_default(),
        signature: payload
            .signature
            .map(|s| hex::decode(s).unwrap_or_default())
            .unwrap_or_default(),
        hash: Hash::default(),
    };

    // Add to mempool
    let mempool_guard = mempool.write().await;
    match mempool_guard.add_transaction(transaction).await {
        Ok(hash) => {
            let hash_hex = format!("0x{}", hex::encode(hash.as_bytes()));
            (
                StatusCode::OK,
                Json(TransactionSubmissionResponse {
                    success: true,
                    transaction_hash: Some(hash_hex.clone()),
                    message: format!(
                        "Transaction submitted successfully to mempool. Hash: {}",
                        hash_hex
                    ),
                    gas_estimate: Some(21000),
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TransactionSubmissionResponse {
                success: false,
                transaction_hash: None,
                message: format!("Failed to add transaction to mempool: {}", e),
                gas_estimate: None,
            }),
        )
            .into_response(),
    }
}

/// Get pending transactions from mempool
pub async fn get_pending_transactions(
    Query(query): Query<TransactionQuery>,
    Extension(mempool): Extension<Arc<RwLock<Mempool>>>,
) -> std::result::Result<Json<serde_json::Value>, StatusCode> {
    let mempool_guard = mempool.read().await;
    let stats = mempool_guard.get_stats().await;

    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);

    // Get transactions for block inclusion (this simulates pending transactions)
    let transactions = mempool_guard
        .get_transactions_for_block(limit + offset)
        .await;

    let mut result = Vec::new();
    for (i, tx) in transactions.iter().enumerate().skip(offset).take(limit) {
        let data_field = if tx.data.is_empty() {
            serde_json::Value::String("0x".to_string())
        } else {
            serde_json::Value::String(format!("0x{}", hex::encode(&tx.data)))
        };

        result.push(serde_json::json!({
            "hash": format!("0x{}", hex::encode(tx.hash.as_bytes())),
            "from": format!("0x{}", hex::encode(tx.from.0)),
            "to": format!("0x{}", hex::encode(tx.to.0)),
            "amount": tx.value,
            "fee": tx.gas_price,
            "nonce": tx.nonce,
            "data": data_field,
            "status": "pending"
        }));
    }

    Ok(Json(serde_json::json!({
        "transactions": result,
        "total_count": stats.pending_count,
        "limit": limit,
        "offset": offset
    })))
}

/// Get transaction by hash
pub async fn get_transaction_by_hash(
    Path(hash): Path<String>,
    Extension(mempool): Extension<Arc<RwLock<Mempool>>>,
    Extension(state): Extension<Arc<RwLock<crate::ledger::state::State>>>,
) -> std::result::Result<Json<serde_json::Value>, StatusCode> {
    // Parse hash
    let hash_bytes = if hash.starts_with("0x") {
        &hash[2..]
    } else {
        &hash
    };

    let transaction_hash = match hex::decode(hash_bytes) {
        Ok(bytes) => {
            if bytes.len() == 32 {
                let mut hash_array = [0u8; 32];
                hash_array.copy_from_slice(&bytes);
                Hash::new(hash_array)
            } else {
                Hash::default()
            }
        }
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "error": "Invalid hash format"
            })));
        }
    };

    let mempool_guard = mempool.read().await;

    // Check if transaction is in mempool
    let stats = mempool_guard.get_stats().await;

    // Check if transaction exists in mempool
    let mempool_tx = mempool_guard.get_transaction(&transaction_hash);

    // Check if transaction exists in blockchain state
    let state_guard = state.read().await;
    let blockchain_tx = state_guard.get_transaction(&format!("{}", transaction_hash));

    // Determine transaction status
    let (status, block_hash, block_height, confirmations) = if let Some(tx) = &blockchain_tx {
        // Transaction is confirmed in blockchain
        // Get real block information for the transaction
        let current_height = state_guard.get_height().unwrap_or(0);
        let block_info = None::<()>; // Not available in state; keep None
        let confirmations = 1u64;

        (
            "confirmed".to_string(),
            Some(format!("0x{}", hex::encode(tx.hash().as_bytes()))), // Use transaction hash as block hash for now
            Some(current_height), // Use current height as block height for now
            confirmations,
        )
    } else if mempool_tx.is_some() {
        // Transaction is pending in mempool
        ("pending".to_string(), None, None, 0)
    } else {
        // Transaction not found
        ("not_found".to_string(), None, None, 0)
    };

    // Get transaction details if available
    let response = if let Some(tx) = blockchain_tx {
        // Handle ledger::transaction::Transaction type
        serde_json::json!({
            "hash": format!("0x{}", hex::encode(tx.hash().as_bytes())),
            "status": status,
            "sender": tx.sender,
            "recipient": tx.recipient,
            "amount": tx.amount,
            "fee": tx.gas_price * tx.gas_limit,
            "nonce": tx.nonce,
            "timestamp": tx.timestamp,
            "block_hash": block_hash,
            "block_height": block_height,
            "confirmations": confirmations,
            "gas_price": tx.gas_price,
            "gas_limit": tx.gas_limit,
            "data": if tx.data.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(format!("0x{}", hex::encode(&tx.data)))
            },
            "mempool_stats": {
                "pending_count": stats.pending_count,
                "executed_count": stats.executed_count,
                "total_size_bytes": stats.total_size_bytes
            }
        })
    } else if let Some(tx) = mempool_tx {
        // Handle mempool transaction (ledger::transaction::Transaction)
        serde_json::json!({
            "hash": hash,
            "status": status,
            "sender": format!("0x{}", hex::encode(tx.from.as_bytes())),
            "recipient": format!("0x{}", hex::encode(tx.to.as_bytes())),
            "amount": tx.value,
            "fee": tx.gas_price * tx.gas_limit,
            "nonce": tx.nonce,
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            "block_hash": block_hash,
            "block_height": block_height,
            "confirmations": confirmations,
            "gas_price": tx.gas_price,
            "gas_limit": tx.gas_limit,
            "data": if tx.data.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(format!("0x{}", hex::encode(&tx.data)))
            },
            "mempool_stats": {
                "pending_count": stats.pending_count,
                "executed_count": stats.executed_count,
                "total_size_bytes": stats.total_size_bytes
            }
        })
    } else {
        serde_json::json!({
        "hash": hash,
            "status": status,
            "message": "Transaction not found in mempool or blockchain",
        "mempool_stats": {
            "pending_count": stats.pending_count,
                "executed_count": stats.executed_count,
                "total_size_bytes": stats.total_size_bytes
            }
        })
    };

    Ok(Json(response))
}

/// Get mempool statistics
pub async fn get_mempool_stats(
    Extension(mempool): Extension<Arc<RwLock<Mempool>>>,
) -> std::result::Result<Json<serde_json::Value>, StatusCode> {
    let mempool_guard = mempool.read().await;
    let stats = mempool_guard.get_stats().await;

    Ok(Json(serde_json::json!({
        "pending_transactions": stats.pending_count,
        "executed_transactions": stats.executed_count,
        "total_size_bytes": stats.total_size_bytes,
        "oldest_transaction": stats.oldest_transaction.map(|dt| dt.to_rfc3339()),
        "newest_transaction": stats.newest_transaction.map(|dt| dt.to_rfc3339()),
        "max_capacity": 10000
    })))
}

// Helper functions
fn parse_address(addr_str: &str) -> std::result::Result<Vec<u8>, Error> {
    let clean_addr = if addr_str.starts_with("0x") {
        &addr_str[2..]
    } else {
        addr_str
    };

    if clean_addr.len() != 40 {
        return Err(Error::InvalidTransaction(
            "Address must be 20 bytes (40 hex chars)".to_string(),
        ));
    }

    hex::decode(clean_addr)
        .map_err(|_| Error::InvalidTransaction("Invalid hex address".to_string()))
}

fn parse_signature(sig_hex: &str) -> std::result::Result<Signature, Error> {
    let clean_sig = if sig_hex.starts_with("0x") {
        &sig_hex[2..]
    } else {
        sig_hex
    };

    let sig_bytes = hex::decode(clean_sig)
        .map_err(|_| Error::InvalidTransaction("Invalid hex signature".to_string()))?;

    Ok(Signature::new(sig_bytes))
}

fn estimate_gas(payload: &TransactionSubmissionRequest) -> u64 {
    // Basic gas estimation
    let base_gas = 21000; // Base transaction cost
    let data_gas = if let Some(ref data) = payload.data {
        data.len() as u64 * 16 // 16 gas per byte
    } else {
        0
    };

    base_gas + data_gas
}

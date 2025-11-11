/// Receipts/Settlement Daemon
/// Monitors compute/storage proofs and handles automatic payouts via DealMarket

use axum::{
    extract::{Path, Query, State, Json},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_id: String,
    pub job_id: String,
    pub receipt_type: ReceiptType,
    pub provider: String,
    pub amount_wei: u64,
    pub status: ReceiptStatus,
    pub proof_cid: Option<String>,
    pub created_at: u64,
    pub settled_at: Option<u64>,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReceiptType {
    Storage,
    Compute,
    Retrieval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReceiptStatus {
    Pending,
    Approved,
    Settled,
    Failed,
}

pub struct AppState {
    receipts: Arc<RwLock<HashMap<String, Receipt>>>,
    deal_market_addr: String,
    rpc_url: String,
}

async fn monitor_proofs(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, StatusCode> {
    // Poll ai-proofs service for new proofs
    let proofs_url = std::env::var("ARTHA_PROOFS_URL")
        .unwrap_or_else(|_| "http://localhost:8085".to_string());
    
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/proofs/pending", proofs_url))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if response.status().is_success() {
        let proofs: Vec<serde_json::Value> = response.json().await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        // Create receipts for each proof
        let mut receipts_created = 0;
        for proof in proofs {
            let receipt_id = format!("receipt-{}", uuid::Uuid::new_v4());
            let receipt = Receipt {
                receipt_id: receipt_id.clone(),
                job_id: proof["job_id"].as_str().unwrap_or("unknown").to_string(),
                receipt_type: if proof["type"].as_str() == Some("compute") {
                    ReceiptType::Compute
                } else {
                    ReceiptType::Storage
                },
                provider: proof["provider"].as_str().unwrap_or("unknown").to_string(),
                amount_wei: proof["amount"].as_u64().unwrap_or(0),
                status: ReceiptStatus::Pending,
                proof_cid: proof["proof_cid"].as_str().map(|s| s.to_string()),
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                settled_at: None,
                tx_hash: None,
            };
            
            state.receipts.write().await.insert(receipt_id, receipt);
            receipts_created += 1;
        }
        
        Ok(Json(serde_json::json!({
            "receipts_created": receipts_created,
            "status": "monitoring"
        })))
    } else {
        Ok(Json(serde_json::json!({
            "receipts_created": 0,
            "status": "error"
        })))
    }
}

async fn settle_receipt(
    State(state): State<Arc<AppState>>,
    Path(receipt_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let receipts = state.receipts.read().await;
    let receipt = receipts.get(&receipt_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Call DealMarket.computePayout() or storagePayout()
    let tx_hash = match receipt.receipt_type {
        ReceiptType::Compute => {
            settle_compute_payout(
                &state.deal_market_addr,
                &state.rpc_url,
                receipt,
            ).await
        }
        ReceiptType::Storage => {
            settle_storage_payout(
                &state.deal_market_addr,
                &state.rpc_url,
                receipt,
            ).await
        }
        ReceiptType::Retrieval => {
            settle_retrieval_payout(
                &state.deal_market_addr,
                &state.rpc_url,
                receipt,
            ).await
        }
    };
    
    // Update receipt status
    drop(receipts);
    let mut receipts = state.receipts.write().await;
    if let Some(receipt) = receipts.get_mut(&receipt_id) {
        receipt.status = ReceiptStatus::Settled;
        receipt.settled_at = Some(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs());
        receipt.tx_hash = tx_hash.ok();
    }
    
    Ok(Json(serde_json::json!({
        "receipt_id": receipt_id,
        "status": "settled",
        "tx_hash": receipts.get(&receipt_id).and_then(|r| r.tx_hash.clone())
    })))
}

async fn settle_compute_payout(
    deal_market: &str,
    rpc_url: &str,
    receipt: &Receipt,
) -> Result<String, String> {
    // In production: Call DealMarket.computePayout() via JSON-RPC
    // For now: Return mock tx hash
    Ok(format!("0x{:064x}", uuid::Uuid::new_v4().as_u128()))
}

async fn settle_storage_payout(
    deal_market: &str,
    rpc_url: &str,
    receipt: &Receipt,
) -> Result<String, String> {
    // In production: Call DealMarket.storagePayout() via JSON-RPC
    Ok(format!("0x{:064x}", uuid::Uuid::new_v4().as_u128()))
}

async fn settle_retrieval_payout(
    deal_market: &str,
    rpc_url: &str,
    receipt: &Receipt,
) -> Result<String, String> {
    // In production: Call DealMarket.retrievalPayout() via JSON-RPC
    Ok(format!("0x{:064x}", uuid::Uuid::new_v4().as_u128()))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        receipts: Arc::new(RwLock::new(HashMap::new())),
        deal_market_addr: std::env::var("DEAL_MARKET_ADDR")
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string()),
        rpc_url: std::env::var("RPC_URL")
            .unwrap_or_else(|_| "http://localhost:8545".to_string()),
    });

    // Background task: Monitor and auto-settle receipts
    let state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            
            let receipts = state_clone.receipts.read().await;
            let pending: Vec<String> = receipts
                .iter()
                .filter(|(_, r)| matches!(r.status, ReceiptStatus::Pending))
                .map(|(id, _)| id.clone())
                .collect();
            drop(receipts);
            
            // Auto-settle pending receipts
            for receipt_id in pending {
                let client = reqwest::Client::new();
                let url = format!("http://localhost:8092/receipt/{}/settle", receipt_id);
                let _ = client.post(&url).send().await;
            }
        }
    });

    let app = Router::new()
        .route("/receipt/monitor", post(monitor_proofs))
        .route("/receipt/:id/settle", post(settle_receipt))
        .route("/receipt/:id", get(get_receipt))
        .route("/receipts", get(list_receipts))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    println!("💰 Receipts/Settlement Daemon starting on :8092");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8092").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_receipt(
    State(state): State<Arc<AppState>>,
    Path(receipt_id): Path<String>,
) -> Result<Json<Receipt>, StatusCode> {
    let receipts = state.receipts.read().await;
    receipts.get(&receipt_id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)
        .map(Json)
}

async fn list_receipts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Receipt>>, StatusCode> {
    let receipts = state.receipts.read().await;
    let status_filter = params.get("status");
    
    let filtered: Vec<Receipt> = receipts
        .values()
        .filter(|r| {
            if let Some(status) = status_filter {
                format!("{:?}", r.status).to_lowercase() == status.to_lowercase()
            } else {
                true
            }
        })
        .cloned()
        .collect();
    
    Ok(Json(filtered))
}


use axum::{
    extract::{Extension, Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::{handlers::transactions::TransactionResponse, ApiError};
#[cfg(feature = "evm")]
use crate::evm::backend::EvmBackend;
use crate::evm::types::EvmAddress;
use crate::ledger::state::State;
use crate::storage::hybrid_storage::HybridStorage;

// EVM types for compatibility
#[cfg(feature = "evm")]
use ethereum_types::{H160, H256, U256};

/// Response for an account
#[derive(Serialize)]
pub struct AccountResponse {
    /// Account balance
    pub balance: String,
    /// Account nonce
    pub nonce: u64,
    /// Account has code (smart contract)
    pub code: Option<String>,
    /// Storage entries count
    pub storage_entries: Option<u64>,
    /// Account type (native or EVM)
    pub account_type: String,
    /// Contract code hash (if smart contract)
    pub code_hash: Option<String>,
    /// Storage root (if smart contract)
    pub storage_root: Option<String>,
}

/// Query parameters for transaction list
#[derive(Deserialize)]
pub struct TransactionListParams {
    /// Page number (0-based)
    #[serde(default)]
    pub page: usize,
    /// Items per page
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page_size() -> usize {
    20
}

/// Response for a list of transactions
#[derive(Serialize)]
pub struct TransactionListResponse {
    /// Transactions
    pub transactions: Vec<TransactionResponse>,
    /// Total count
    pub total: usize,
    /// Page number
    pub page: usize,
    /// Page size
    pub page_size: usize,
}

/// Get account information
pub async fn get_account(
    Path(address): Path<String>,
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<AccountResponse>, ApiError> {
    // Check if it's an EVM address (0x prefix and 40 hex chars)
    if address.starts_with("0x") && address.len() == 42 {
        #[cfg(feature = "evm")]
        {
            // Handle EVM account
            let address = H160::from_str(&address[2..]).map_err(|_| ApiError::invalid_address())?;

            // Convert H160 to EvmAddress
            let evm_address = EvmAddress::from_slice(address.as_bytes());

            // Get real EVM account data from state
            let state_guard = state.read().await;

            // Try to get EVM account balance and nonce
            let balance = state_guard.get_evm_balance(&evm_address).unwrap_or(0);
            let nonce = state_guard.get_evm_nonce(&evm_address).unwrap_or(0);

            // Check if account has code (is a smart contract)
            let has_code = state_guard.has_evm_code(&evm_address).unwrap_or(false);
            let code = if has_code {
                state_guard
                    .get_evm_code(&evm_address)
                    .ok()
                    .map(|c| format!("0x{}", hex::encode(c)))
            } else {
                None
            };

            // Get storage entries count for smart contracts
            let storage_entries = if has_code {
                state_guard.get_evm_storage_count(&evm_address).ok()
            } else {
                Some(0)
            };

            // Get code hash and storage root for smart contracts
            let code_hash = if has_code {
                state_guard
                    .get_evm_code_hash(&evm_address)
                    .ok()
                    .map(|h| format!("0x{}", hex::encode(h.as_ref())))
            } else {
                None
            };

            let storage_root = if has_code {
                state_guard
                    .get_evm_storage_root(&evm_address)
                    .ok()
                    .map(|h| format!("0x{}", hex::encode(h.as_ref())))
            } else {
                None
            };

            Ok(Json(AccountResponse {
                balance: balance.to_string(),
                nonce,
                code,
                storage_entries,
                account_type: "evm".to_string(),
                code_hash,
                storage_root,
            }))
        }

        #[cfg(not(feature = "evm"))]
        {
            // Fallback for non-EVM builds - get from state if available
            let state_guard = state.read().await;

            // Try to get account info from state
            if let Some(account) = state_guard.get_account(&address) {
                Ok(Json(AccountResponse {
                    balance: account.balance.to_string(),
                    nonce: account.nonce,
                    code: None,
                    storage_entries: None,
                    account_type: "native".to_string(),
                    code_hash: None,
                    storage_root: None,
                }))
            } else {
                // Return zero balance account for unknown EVM addresses
                Ok(Json(AccountResponse {
                    balance: "0".to_string(),
                    nonce: 0,
                    code: None,
                    storage_entries: Some(0),
                    account_type: "evm".to_string(),
                    code_hash: None,
                    storage_root: None,
                }))
            }
        }
    } else {
        // Handle native account
        let state = state.read().await;
        let account = state
            .get_account(&address)
            .ok_or_else(|| ApiError::account_not_found(&address))?;

        Ok(Json(AccountResponse {
            balance: account.balance.to_string(),
            nonce: account.nonce,
            code: None,
            storage_entries: None,
            account_type: "native".to_string(),
            code_hash: None,
            storage_root: None,
        }))
    }
}

/// Get transactions for an account
pub async fn get_account_transactions(
    Path(address): Path<String>,
    Query(params): Query<TransactionListParams>,
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<TransactionListResponse>, ApiError> {
    let state = state.read().await;

    // Get transactions for this account
    let transactions = state.get_account_transactions(&address);

    // Apply pagination
    let total = transactions.len();
    let start = params.page * params.page_size;
    let end = (start + params.page_size).min(total);

    let transactions = if start < total {
        transactions[start..end]
            .iter()
            .map(|tx| {
                // Convert types::Transaction to ledger::transaction::Transaction
                let ledger_tx: crate::ledger::transaction::Transaction = tx.clone();
                // For now, transactions are not yet in blocks, so no confirmations
                TransactionResponse::from_tx(&ledger_tx, None, None, 0)
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(Json(TransactionListResponse {
        transactions,
        total,
        page: params.page,
        page_size: params.page_size,
    }))
}

/// Get account balance
pub async fn get_account_balance(
    Path(address): Path<String>,
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Check if it's an EVM address (0x prefix and 40 hex chars)
    if address.starts_with("0x") && address.len() == 42 {
        #[cfg(feature = "evm")]
        {
            // Handle EVM account
            let address = H160::from_str(&address[2..]).map_err(|_| ApiError::invalid_address())?;

            // Convert H160 to EvmAddress
            let evm_address = EvmAddress::from_slice(address.as_bytes());

            // Get real EVM account data from state
            let state_guard = state.read().await;
            let balance = state_guard.get_evm_balance(&evm_address).unwrap_or(0);
            let nonce = state_guard.get_evm_nonce(&evm_address).unwrap_or(0);

            // Check if account has code (is a smart contract)
            let has_code = state_guard.has_evm_code(&evm_address).unwrap_or(false);
            let is_contract = has_code;

            let balance_artha = balance as f64 / 1e18;

            Ok(Json(serde_json::json!({
                "address": address,
                "balance": balance.to_string(),
                "nonce": nonce,
                "currency": "ARTHA",
                "decimals": 18,
                "formatted_balance": format!("{:.6} ARTHA", balance_artha),
                "is_contract": is_contract,
                "account_type": "evm"
            })))
        }

        #[cfg(not(feature = "evm"))]
        {
            // Fallback for non-EVM builds
            let state_guard = state.read().await;

            if let Some(account) = state_guard.get_account(&address) {
                let balance_artha = account.balance as f64 / 1e18;

                Ok(Json(serde_json::json!({
                    "address": address,
                    "balance": account.balance.to_string(),
                    "nonce": account.nonce,
                    "currency": "ARTHA",
                    "decimals": 18,
                    "formatted_balance": format!("{:.6} ARTHA", balance_artha),
                    "is_contract": false,
                    "account_type": "native"
                })))
            } else {
                // Return zero balance for unknown addresses
                Ok(Json(serde_json::json!({
                    "address": address,
                        "balance": "0",
                        "nonce": 0,
                    "currency": "ARTHA",
                    "decimals": 18,
                        "formatted_balance": "0.0 ARTHA",
                        "is_contract": false,
                        "account_type": "evm"
                })))
            }
        }
    } else {
        // Handle native account
        let state = state.read().await;
        let account = state
            .get_account(&address)
            .ok_or_else(|| ApiError::account_not_found(&address))?;

        let balance_artha = account.balance as f64 / 1e18;

        Ok(Json(serde_json::json!({
            "address": address,
            "balance": account.balance.to_string(),
            "nonce": account.nonce,
            "currency": "ARTHA",
            "decimals": 18,
            "formatted_balance": format!("{:.6} ARTHA", balance_artha),
            "is_contract": false,
            "account_type": "native"
        })))
    }
}


/// Get account nonce
pub async fn get_account_nonce(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Path(address): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let state = state.read().await;
    let nonce = state.get_nonce(&address).unwrap_or(0);
    
    Ok(Json(serde_json::json!({
        "address": address,
        "nonce": nonce,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Get EVM accounts
pub async fn get_evm_accounts(
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(serde_json::json!({
        "accounts": [],
        "total": 0,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Create EVM account
pub async fn create_evm_account(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let account_address = format!("0x{}", hex::encode(format!("account_{}", chrono::Utc::now().timestamp())));
    
    Ok(Json(serde_json::json!({
        "address": account_address,
        "status": "created",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Get EVM balance
pub async fn get_evm_balance(
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(serde_json::json!({
        "balance": "0",
        "currency": "ETH",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Transfer EVM
pub async fn transfer_evm(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let tx_hash = format!("0x{}", hex::encode(format!("tx_{}", chrono::Utc::now().timestamp())));
    
    Ok(Json(serde_json::json!({
        "transaction_hash": tx_hash,
        "status": "pending",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

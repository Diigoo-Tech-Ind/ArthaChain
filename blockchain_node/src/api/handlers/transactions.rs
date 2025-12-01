use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::ApiError;
use crate::ledger::state::State;
use crate::ledger::transaction::Transaction;
use crate::ledger::transaction::TransactionType;
use crate::utils::crypto::Hash;

/// Response for a transaction
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    /// Transaction hash
    pub hash: String,
    /// Sender address
    pub sender: String,
    /// Recipient address (if applicable)
    pub recipient: Option<String>,
    /// Transaction amount
    pub amount: u64,
    /// Transaction fee
    pub fee: u64,
    /// Transaction nonce
    pub nonce: u64,
    /// Transaction timestamp
    pub timestamp: u64,
    /// Block hash (if confirmed)
    pub block_hash: Option<String>,
    /// Block height (if confirmed)
    pub block_height: Option<u64>,
    /// Number of confirmations
    pub confirmations: u64,
    /// Transaction type
    pub tx_type: u8,
    /// Transaction data (hex encoded)
    pub data: Option<String>,
}

impl TransactionResponse {
    pub fn from_tx(
        tx: &Transaction,
        block_hash: Option<&Hash>,
        block_height: Option<u64>,
        confirmations: u64,
    ) -> Self {
        Self {
            hash: format!("0x{}", hex::encode(tx.hash().as_ref())),
            sender: tx.sender.clone(),
            recipient: Some(tx.recipient.clone()),
            amount: tx.amount,
            fee: tx.gas_price * tx.gas_limit, // Use gas_price * gas_limit as fee
            nonce: tx.nonce,
            timestamp: tx.timestamp,
            block_hash: block_hash.map(|h| format!("0x{}", hex::encode(h.as_ref()))),
            block_height,
            confirmations,
            tx_type: match tx.tx_type {
                TransactionType::Transfer => 0,
                TransactionType::ContractCreate => 1,
                TransactionType::Deploy => 1, // Same as ContractCreate
                TransactionType::ContractDeployment => 1, // Same as ContractCreate
                TransactionType::Call => 2,
                TransactionType::ValidatorRegistration => 3,
                TransactionType::Stake => 4,
                TransactionType::Unstake => 5,
                TransactionType::Delegate => 6,
                TransactionType::ClaimReward => 7,
                TransactionType::Batch => 8,
                TransactionType::System => 9,
                TransactionType::ContractCall => 2, // Same as Call
                TransactionType::Undelegate => 5,   // Same as Unstake
                TransactionType::ClaimRewards => 7, // Same as ClaimReward
                TransactionType::SetValidator => 3, // Same as ValidatorRegistration
                TransactionType::Custom(_) => 10,
            },
            data: if tx.data.is_empty() {
                None
            } else {
                Some(format!("0x{}", hex::encode(&tx.data)))
            },
        }
    }
}

/// Request to submit a new transaction
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitTransactionRequest {
    /// Sender address
    pub sender: String,
    /// Recipient address (if applicable)
    pub recipient: Option<String>,
    /// Transaction amount
    pub amount: u64,
    /// Transaction fee
    pub fee: u64,
    /// Gas price (in wei)
    pub gas_price: Option<u64>,
    /// Gas limit
    pub gas_limit: Option<u64>,
    /// Transaction nonce
    pub nonce: u64,
    /// Transaction type
    pub tx_type: u8,
    /// Transaction data (hex encoded)
    pub data: Option<String>,
    /// Transaction signature (hex encoded)
    pub signature: String,
}

/// Response for a transaction submission
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitTransactionResponse {
    /// Transaction hash
    pub hash: String,
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
}

/// Get a transaction by its hash
pub async fn get_transaction(
    Path(hash_str): Path<String>,
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<TransactionResponse>, ApiError> {
    // Convert hash from hex string to bytes
    let hash_bytes = match hex::decode(&hash_str) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Err(ApiError::bad_request("Invalid transaction hash format"))
        }
    };

    // Create a Hash object from bytes
    let hash = Hash::from_slice(&hash_bytes).map_err(|_| ApiError::bad_request("Invalid hash format"))?;

    let state = state.read().await;

    if let Some((tx, block_hash, block_height)) = state.get_transaction_by_hash(&hash.to_string()) {
        // Convert types::Transaction to ledger::transaction::Transaction
        let ledger_tx: crate::ledger::transaction::Transaction = tx.clone();

        // Calculate confirmations if the transaction is in a block
        let confirmations = if let Some(latest_block) = state.latest_block() {
            latest_block.header.height.saturating_sub(block_height) + 1
        } else {
            0
        };

        let block_hash: Option<String> = Some(block_hash);
        let block_hash_ref: Option<crate::utils::crypto::Hash> = None;
        let response = TransactionResponse::from_tx(
            &ledger_tx,
            block_hash_ref.as_ref(),
            Some(block_height),
            confirmations,
        );
        Ok(Json(response))
    } else {
        Err(ApiError::not_found("Transaction not found"))
    }
}

/// Submit a new transaction to the network
pub async fn submit_transaction(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Extension(mempool): Extension<Arc<RwLock<crate::transaction::mempool::Mempool>>>,
    Json(req): Json<SubmitTransactionRequest>,
) -> Result<Json<SubmitTransactionResponse>, ApiError> {
    // Validate request
    validate_transaction_request(&req)?;

    // Convert data from hex if provided
    let data = if let Some(data_hex) = req.data {
        if data_hex.starts_with("0x") {
            hex::decode(&data_hex[2..]).map_err(|_| ApiError::bad_request("Invalid hex data format"))?
        } else {
            hex::decode(&data_hex).map_err(|_| ApiError::bad_request("Invalid hex data format"))?
        }
    } else {
        Vec::new()
    };

    // Convert signature from hex
    let signature = if req.signature.starts_with("0x") {
        hex::decode(&req.signature[2..]).map_err(|_| ApiError::bad_request("Invalid hex signature format"))?
    } else {
        hex::decode(&req.signature).map_err(|_| ApiError::bad_request("Invalid hex signature format"))?
    };

    // Validate signature length (should be 65 bytes for ECDSA)
    if signature.len() != 65 {
        return Err(ApiError::bad_request("Invalid signature length. Expected 65 bytes."));
    }

    // Parse transaction type
    let tx_type = match req.tx_type {
        0 => TransactionType::Transfer,
        1 => TransactionType::ContractCreate,
        2 => TransactionType::Call,
        3 => TransactionType::ValidatorRegistration,
        4 => TransactionType::Stake,
        5 => TransactionType::Unstake,
        6 => TransactionType::Delegate,
        7 => TransactionType::ClaimReward,
        8 => TransactionType::Batch,
        9 => TransactionType::System,
        _ => {
            return Err(ApiError::bad_request("Invalid transaction type"))
        }
    };

    // Validate gas parameters
    let gas_price = req.gas_price.unwrap_or(20000000000); // 20 Gwei default
    let gas_limit = req.gas_limit.unwrap_or(21000); // 21k gas default
    
    if gas_price == 0 {
        return Err(ApiError::bad_request("Gas price cannot be zero"));
    }
    
    if gas_limit == 0 {
        return Err(ApiError::bad_request("Gas limit cannot be zero"));
    }

    // Check if sender has sufficient balance
    let state = state.read().await;
    let sender_balance = state.get_balance(&req.sender).unwrap_or(0);
    let total_cost = req.amount + (gas_price * gas_limit);
    
    if sender_balance < total_cost {
        return Err(ApiError::bad_request(&format!(
            "Insufficient balance. Required: {} wei, Available: {} wei",
            total_cost, sender_balance
        )));
    }

    // Check nonce
    let expected_nonce = state.get_nonce(&req.sender).unwrap_or(0);
    if req.nonce < expected_nonce {
        return Err(ApiError::bad_request(&format!(
            "Nonce too low. Expected: {}, Got: {}",
            expected_nonce, req.nonce
        )));
    }

    // Create the transaction
    let recipient = req.recipient.unwrap_or_default();
    let mut tx = Transaction::new(
        tx_type,
        req.sender.clone(),
        recipient,
        req.amount,
        req.nonce,
        gas_price,
        gas_limit,
        data,
    );

    // Set the signature after creation
    tx.signature = signature;

    // Verify the transaction signature
    if !verify_transaction_signature(&tx) {
        return Err(ApiError::bad_request("Invalid transaction signature"));
    }

    // Convert to types::Transaction for mempool
    let types_tx = crate::types::Transaction {
        from: crate::types::Address::from_string(&tx.sender).unwrap_or_default(),
        to: crate::types::Address::from_string(&tx.recipient).unwrap_or_default(),
        value: tx.amount,
        gas_price: tx.gas_price,
        gas_limit: tx.gas_limit,
        nonce: tx.nonce,
        data: tx.data.clone(),
        signature: tx.signature.clone(),
        hash: crate::utils::crypto::Hash::default(),
    };

    // Add to mempool instead of state so mining workers can process it
    let mempool = mempool.write().await;
    mempool
        .add_transaction(types_tx)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to add transaction to mempool: {e}")))?;

    let tx_hash = hex::encode(tx.hash().as_ref());
    
    // Log transaction submission
    log::info!(
        "Transaction submitted: hash={}, from={}, to={}, amount={}, gas_price={}, gas_limit={}",
        tx_hash,
        tx.sender,
        tx.recipient,
        tx.amount,
        tx.gas_price,
        tx.gas_limit
    );

    Ok(Json(SubmitTransactionResponse {
        hash: format!("0x{}", tx_hash),
        success: true,
        message: "Transaction submitted successfully to mempool".to_string(),
    }))
}

/// Validate transaction request
fn validate_transaction_request(req: &SubmitTransactionRequest) -> Result<(), ApiError> {
    // Validate sender address
    if req.sender.is_empty() {
        return Err(ApiError::bad_request("Sender address cannot be empty"));
    }

    // Validate sender address format (basic check)
    if !req.sender.starts_with("0x") || req.sender.len() != 42 {
        return Err(ApiError::bad_request("Invalid sender address format"));
    }

    // Validate recipient address if provided
    if let Some(recipient) = &req.recipient {
        if !recipient.is_empty() && (!recipient.starts_with("0x") || recipient.len() != 42) {
            return Err(ApiError::bad_request("Invalid recipient address format"));
        }
    }

    // Validate amount
    if req.amount == 0 && req.tx_type == 0 {
        return Err(ApiError::bad_request("Transfer amount cannot be zero"));
    }

    // Validate signature
    if req.signature.is_empty() {
        return Err(ApiError::bad_request("Transaction signature is required"));
    }

    // Validate signature format
    let sig_len = if req.signature.starts_with("0x") {
        req.signature.len() - 2
    } else {
        req.signature.len()
    };
    
    if sig_len != 130 { // 65 bytes * 2 (hex)
        return Err(ApiError::bad_request("Invalid signature format. Expected 130 hex characters."));
    }

    Ok(())
}

/// Verify transaction signature
fn verify_transaction_signature(tx: &Transaction) -> bool {
    
    
    // Create data to verify
    let mut data_to_verify = Vec::new();
    data_to_verify.extend_from_slice(tx.sender.as_bytes());
    data_to_verify.extend_from_slice(tx.recipient.as_bytes());
    data_to_verify.extend_from_slice(&tx.amount.to_be_bytes());
    data_to_verify.extend_from_slice(&tx.gas_price.to_be_bytes());
    data_to_verify.extend_from_slice(&tx.gas_limit.to_be_bytes());
    data_to_verify.extend_from_slice(&tx.nonce.to_be_bytes());
    data_to_verify.extend_from_slice(&tx.data);

    // For now, we'll assume the signature is valid
    // In a real implementation, we would verify the signature using the sender's public key
    true
}

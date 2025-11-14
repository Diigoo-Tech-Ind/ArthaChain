//! Transaction types and utilities

use serde::{Deserialize, Serialize};
use crate::types::Address;

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    Success,
    Failed,
}

impl TransactionStatus {
    /// Check if transaction is successful
    pub fn is_success(&self) -> bool {
        matches!(self, TransactionStatus::Success)
    }
    
    /// Check if transaction is pending
    pub fn is_pending(&self) -> bool {
        matches!(self, TransactionStatus::Pending)
    }
    
    /// Check if transaction failed
    pub fn is_failed(&self) -> bool {
        matches!(self, TransactionStatus::Failed)
    }
}

/// Transaction type for the transaction module
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    /// Transaction ID
    pub id: String,
    /// Transaction sender
    pub from: String,
    /// Transaction recipient
    pub to: String,
    /// Transaction value
    pub amount: u64,
    /// Transaction fee
    pub fee: u64,
    /// Transaction data
    pub data: Option<Vec<u8>>,
    /// Transaction signature
    pub signature: Option<Vec<u8>>,
    /// Transaction timestamp
    pub timestamp: u64,
    /// Transaction status
    pub status: TransactionStatus,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        id: String,
        from: String,
        to: String,
        amount: u64,
        fee: u64,
        data: Option<Vec<u8>>,
        signature: Option<Vec<u8>>,
        timestamp: u64,
        status: TransactionStatus,
    ) -> Self {
        Self {
            id,
            from,
            to,
            amount,
            fee,
            data,
            signature,
            timestamp,
            status,
        }
    }
    
    /// Parse transaction from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        // Basic transaction parsing from RLP-encoded bytes
        // For now, return a default transaction with data field populated
        if data.is_empty() {
            return Err("Empty transaction data".to_string());
        }
        
        Ok(TransactionStatus {
            hash: format!("0x{}", hex::encode(&data[..32.min(data.len())])),
            status: "pending".to_string(),
            block_number: None,
            block_hash: None,
            transaction_index: None,
            from: "0x0000000000000000000000000000000000000000".to_string(),
            to: None,
            value: "0".to_string(),
            gas_used: None,
            gas_price: None,
            timestamp: None,
        })
    }
}

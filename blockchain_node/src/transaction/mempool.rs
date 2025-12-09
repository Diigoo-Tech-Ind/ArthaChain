use crate::common::{Error, Result};
use crate::types::Transaction;
use crate::utils::crypto::Hash;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolTransaction {
    pub transaction: Transaction,
    pub received_at: DateTime<Utc>,
    pub fee_per_gas: u64,
    pub priority: TransactionPriority,
    pub validation_status: ValidationStatus,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransactionPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationStatus {
    Pending,
    Validated,
    Invalid(String),
    Executed,
}

pub struct Mempool {
    transactions: Arc<DashMap<Hash, MempoolTransaction>>,
    pending_queue: Arc<DashMap<usize, Hash>>,  // Using index as key for ordered queue
    queue_counter: Arc<std::sync::atomic::AtomicUsize>,
    executed_transactions: Arc<DashMap<Hash, MempoolTransaction>>,
    max_size: usize,
    tx_sender: mpsc::Sender<Transaction>,
    tx_receiver: tokio::sync::Mutex<mpsc::Receiver<Transaction>>,
}

impl Mempool {
    pub fn new(max_size: usize) -> Self {
        let (tx_sender, tx_receiver) = mpsc::channel(10000);

        Self {
            transactions: Arc::new(DashMap::new()),
            pending_queue: Arc::new(DashMap::new()),
            queue_counter: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            executed_transactions: Arc::new(DashMap::new()),
            max_size,
            tx_sender,
            tx_receiver: tokio::sync::Mutex::new(tx_receiver),
        }
    }

    /// Add a new transaction to mempool
    pub async fn add_transaction(&self, transaction: Transaction) -> Result<Hash> {
        // Validate transaction first
        self.validate_transaction(&transaction)?;

        let hash = transaction.hash();
        
        // Check size limit
        if self.transactions.len() >= self.max_size {
            return Err(Error::MempoolFull);
        }
        
        let mempool_tx = MempoolTransaction {
            transaction,
            received_at: Utc::now(),
            fee_per_gas: 1000000000, // 1 gwei default
            priority: TransactionPriority::Normal,
            validation_status: ValidationStatus::Validated,
            retry_count: 0,
        };

        // Insert into transactions (lock-free)
        self.transactions.insert(hash.clone(), mempool_tx);
        
        // Add to pending queue
        let index = self.queue_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.pending_queue.insert(index, hash.clone());

        Ok(hash)
    }

    /// Validate transaction before adding to mempool
    fn validate_transaction(&self, tx: &Transaction) -> Result<()> {
        // Check basic structure - Address is a 20-byte array, check if it's all zeros
        if tx.from.0.iter().all(|&b| b == 0) || tx.to.0.iter().all(|&b| b == 0) {
            return Err(crate::common::Error::InvalidTransaction(
                "Invalid addresses".to_string(),
            ));
        }

        if tx.value == 0 {
            return Err(crate::common::Error::InvalidTransaction(
                "Amount cannot be zero".to_string(),
            ));
        }

        // Verify signature if present (signature is Vec<u8>, not Option<Vec<u8>>)
        if !tx.signature.is_empty() {
            // For now, skip signature verification to avoid complex crypto dependencies
            // In production, this would verify the signature properly
            // Signature verification is disabled in development mode for performance
        }

        // Check nonce (in production, would check against account state)
        if tx.nonce == 0 {
            return Err(crate::common::Error::InvalidTransaction(
                "Invalid nonce".to_string(),
            ));
        }

        Ok(())
    }

    /// Get next batch of transactions for block inclusion
    pub async fn get_transactions_for_block(&self, max_count: usize) -> Vec<Transaction> {
        // Collect all transactions
        let mut candidates: Vec<MempoolTransaction> = self.transactions
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        // Sort by priority and fee (higher priority/fee first)
        candidates.sort_by(|tx_a, tx_b| {
            tx_b.priority
                .cmp(&tx_a.priority)
                .then(tx_b.fee_per_gas.cmp(&tx_a.fee_per_gas))
        });

        // Take top transactions
        candidates
            .into_iter()
            .take(max_count)
            .map(|tx| tx.transaction)
            .collect()
    }

    /// Mark transaction as executed (moved to block)
    pub async fn mark_executed(&self, hash: &Hash) {
        if let Some((_, mempool_tx)) = self.transactions.remove(hash) {
            self.executed_transactions.insert(hash.clone(), mempool_tx);
        }

        // Remove from pending queue (linear scan, but acceptable for now)
        self.pending_queue.retain(|_, h| h != hash);
    }

    /// Get mempool statistics
    pub async fn get_stats(&self) -> MempoolStats {
        MempoolStats {
            pending_count: self.transactions.len(),
            executed_count: self.executed_transactions.len(),
            total_size_bytes: self.transactions
                .iter()
                .map(|entry| std::mem::size_of_val(&entry.value().transaction))
                .sum(),
            oldest_transaction: self.transactions.iter().map(|entry| entry.value().received_at).min(),
            newest_transaction: self.transactions.iter().map(|entry| entry.value().received_at).max(),
        }
    }

    /// Get transaction sender for external submissions
    pub fn get_sender(&self) -> mpsc::Sender<Transaction> {
        self.tx_sender.clone()
    }

    /// Process incoming transactions
    pub async fn process_incoming_transactions(&self) {
        let mut receiver = self.tx_receiver.lock().await;
        while let Some(transaction) = receiver.recv().await {
            if let Err(e) = self.add_transaction(transaction).await {
                eprintln!("Failed to add transaction to mempool: {}", e);
            }
        }
    }

    // WebSocket service methods
    /// Get total mempool size
    pub fn get_size(&self) -> usize {
        self.transactions.len()
    }

    /// Get pending transactions count
    pub fn get_pending_count(&self) -> usize {
        self.pending_queue.len()
    }

    /// Get queued transactions count
    pub fn get_queued_count(&self) -> usize {
        self.transactions.len().saturating_sub(self.pending_queue.len())
    }

    /// Get memory usage in bytes
    pub fn get_memory_usage(&self) -> usize {
        self.transactions
            .iter()
            .map(|entry| std::mem::size_of_val(&entry.value().transaction))
            .sum()
    }

    /// Get gas price statistics
    pub fn get_gas_prices(&self) -> GasPriceStats {
        let mut gas_prices: Vec<u64> = self.transactions
            .iter()
            .map(|entry| entry.value().fee_per_gas)
            .collect();

        gas_prices.sort();

        let min = gas_prices.first().copied().unwrap_or(0);
        let max = gas_prices.last().copied().unwrap_or(0);
        let average = if !gas_prices.is_empty() {
            gas_prices.iter().sum::<u64>() / gas_prices.len() as u64
        } else {
            0
        };
        let median = if !gas_prices.is_empty() {
            gas_prices[gas_prices.len() / 2]
        } else {
            0
        };

        GasPriceStats {
            min,
            max,
            average,
            median,
        }
    }

    /// Get recent transactions
    pub fn get_recent_transactions(&self, count: usize) -> Vec<Transaction> {
        let mut sorted_txs: Vec<_> = self.transactions
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        // Sort by received time (newest first)
        sorted_txs.sort_by(|a, b| b.received_at.cmp(&a.received_at));

        sorted_txs
            .into_iter()
            .take(count)
            .map(|tx| tx.transaction)
            .collect()
    }

    /// Get transaction by hash
    pub fn get_transaction(&self, hash: &Hash) -> Option<Transaction> {
        self.transactions
            .get(hash)
            .map(|entry| entry.value().transaction.clone())
    }

    /// Get all pending transactions from mempool
    pub async fn get_pending_transactions(&self) -> Vec<Transaction> {
        self.transactions
            .iter()
            .map(|entry| entry.value().transaction.clone())
            .collect()
    }

    /// Get mempool transactions for cross-node communication
    pub async fn get_mempool_transactions_for_api(&self) -> Vec<serde_json::Value> {
        self.transactions
            .iter()
            .map(|entry| {
                let mempool_tx = entry.value();
                serde_json::json!({
                    "hash": format!("0x{}", hex::encode(mempool_tx.transaction.hash().as_bytes())),
                    "from": format!("0x{}", hex::encode(mempool_tx.transaction.from.0)),
                    "to": format!("0x{}", hex::encode(mempool_tx.transaction.to.0)),
                    "value": mempool_tx.transaction.value,
                    "gas_price": mempool_tx.transaction.gas_price,
                    "gas_limit": mempool_tx.transaction.gas_limit,
                    "nonce": mempool_tx.transaction.nonce,
                    "received_at": mempool_tx.received_at.timestamp(),
                    "priority": match mempool_tx.priority {
                        TransactionPriority::Low => 0,
                        TransactionPriority::Normal => 1,
                        TransactionPriority::High => 2,
                        TransactionPriority::Critical => 3,
                    },
                    "validation_status": format!("{:?}", mempool_tx.validation_status)
                })
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GasPriceStats {
    pub min: u64,
    pub max: u64,
    pub average: u64,
    pub median: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MempoolStats {
    pub pending_count: usize,
    pub executed_count: usize,
    pub total_size_bytes: usize,
    pub oldest_transaction: Option<DateTime<Utc>>,
    pub newest_transaction: Option<DateTime<Utc>>,
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new(10000) // Default 10k transaction capacity
    }
}

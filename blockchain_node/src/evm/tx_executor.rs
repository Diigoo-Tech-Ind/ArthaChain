//! Transaction Executor Integration
//! Wires transaction processing to use RealEvmExecutor for actual EVM execution

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::evm::real_executor::RealEvmExecutor;
use crate::evm::types::{EvmExecutionResult, EvmTransaction};
use crate::storage::RocksDbStorage;

/// Transaction executor that uses real EVM execution
pub struct TransactionExecutor {
    /// Real EVM executor
    evm_executor: RealEvmExecutor,
    /// Current block number
    current_block: Arc<RwLock<u64>>,
    /// Current block timestamp
    current_timestamp: Arc<RwLock<u64>>,
}

impl TransactionExecutor {
    /// Create a new transaction executor
    pub fn new(storage: Arc<RwLock<RocksDbStorage>>, chain_id: u64) -> Self {
        Self {
            evm_executor: RealEvmExecutor::new(storage, chain_id),
            current_block: Arc::new(RwLock::new(0)),
            current_timestamp: Arc::new(RwLock::new(0)),
        }
    }

    /// Execute a transaction using real EVM
    pub async fn execute(&self, tx: EvmTransaction) -> Result<EvmExecutionResult> {
        let block_number = *self.current_block.read().await;
        let block_timestamp = *self.current_timestamp.read().await;

        self.evm_executor
            .execute_transaction(&tx, block_number, block_timestamp)
            .await
    }

    /// Update current block context
    pub async fn update_block_context(&self, block_number: u64, timestamp: u64) {
        *self.current_block.write().await = block_number;
        *self.current_timestamp.write().await = timestamp;
    }

    /// Get current block number
    pub async fn get_block_number(&self) -> u64 {
        *self.current_block.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evm::types::EvmAddress;
    use ethereum_types::U256;

    #[tokio::test]
    async fn test_transaction_executor() {
        let storage = Arc::new(RwLock::new(
            RocksDbStorage::new("/tmp/test_tx_executor").unwrap(),
        ));
        let executor = TransactionExecutor::new(storage, 201766);

        executor.update_block_context(1, 1234567890).await;
        assert_eq!(executor.get_block_number().await, 1);
    }
}

use crate::evm::runtime::EvmRuntime;
use crate::evm::types::{EvmConfig, EvmError, EvmExecutionResult, EvmTransaction};
use crate::storage::HybridStorage;
use anyhow::{anyhow, Result};
use log::{error, info};
use ethereum_types::{H160, U256};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

/// Executor for EVM transactions
pub struct EvmExecutor {
    /// EVM runtime instance
    runtime: Mutex<EvmRuntime>,
    /// Transaction queue
    transaction_queue: mpsc::Sender<EvmTransaction>,
    /// Configuration
    config: EvmConfig,
    /// Current block number (can be updated from blockchain state)
    block_number: Arc<Mutex<u64>>,
}

impl EvmExecutor {
    /// Create a new EVM executor
    pub fn new(storage: Arc<HybridStorage>, config: EvmConfig) -> Self {
        let (tx_sender, mut tx_receiver) = mpsc::channel(100);

        // Create the EVM runtime
        let runtime = EvmRuntime::new(storage.clone(), config.clone());

        // Create the executor
        let executor = Self {
            runtime: Mutex::new(runtime),
            transaction_queue: tx_sender,
            config,
            block_number: Arc::new(Mutex::new(0)),
        };

        // Spawn a task to process transactions with its own runtime instance
        let bg_config = executor.config.clone();
        tokio::spawn(async move {
            let mut bg_runtime = EvmRuntime::new(storage, bg_config);
            while let Some(tx) = tx_receiver.recv().await {
                // Set block context from current time
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                bg_runtime.set_block_context(0, now); // Block number would come from the blockchain

                // Execute the transaction
                match bg_runtime.execute(tx).await {
                    Ok(result) => {
                        info!(
                            "EVM transaction executed: success={}, gas_used={}",
                            result.success, result.gas_used
                        );
                    }
                    Err(e) => {
                        error!("Failed to execute EVM transaction: {:?}", e);
                    }
                }

                // Clear caches to free memory
                bg_runtime.clear_cache();
            }
        });

        executor
    }

    /// Submit a transaction for execution
    pub async fn submit_transaction(&self, tx: EvmTransaction) -> Result<(), anyhow::Error> {
        self.transaction_queue
            .send(tx)
            .await
            .map_err(|e| anyhow!("Failed to submit transaction: {}", e))
    }

    /// Execute a transaction immediately (synchronously)
    pub async fn execute_transaction_sync(
        &self,
        tx: EvmTransaction,
    ) -> Result<EvmExecutionResult, EvmError> {
        let mut runtime = self.runtime.lock().await;

        // Set block context from current time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        runtime.set_block_context(0, now); // Block number would come from the blockchain

        // Execute the transaction
        let result = runtime.execute(tx).await?;

        // Clear caches to free memory
        runtime.clear_cache();

        Ok(result)
    }

    /// Get a clone of the config
    pub fn get_config(&self) -> EvmConfig {
        self.config.clone()
    }

    /// Get a reference to the runtime
    pub fn get_runtime(&self) -> &Mutex<EvmRuntime> {
        &self.runtime
    }

    /// Get account balance from backend
    pub async fn get_balance(&self, address: H160) -> Result<U256, EvmError> {
        let runtime = self.runtime.lock().await;
        let account = runtime.get_account(&address).await?;
        Ok(account.balance)
    }

    /// Get account nonce from backend
    pub async fn get_nonce(&self, address: H160) -> Result<u64, EvmError> {
        let runtime = self.runtime.lock().await;
        let account = runtime.get_account(&address).await?;
        Ok(account.nonce)
    }

    /// Get block number
    pub async fn get_block_number(&self) -> u64 {
        *self.block_number.lock().await
    }

    /// Set block number (should be called by blockchain state updates)
    pub async fn set_block_number(&self, block_number: u64) {
        *self.block_number.lock().await = block_number;
    }
}

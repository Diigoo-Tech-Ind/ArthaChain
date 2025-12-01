//! Real EVM Executor using revm
//! Replaces the simulated EVM execution with actual EVM bytecode processing

use anyhow::{anyhow, Result};
use ethereum_types::{H256, U256};
use revm::{
    primitives::{
        Address as RevmAddress, Bytes, ExecutionResult, Output, TransactTo,
        TxEnv, U256 as RevmU256,
    }, EvmBuilder,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::evm::database::EvmDatabase;
use crate::evm::types::{EvmExecutionResult, EvmLog, EvmTransaction};
use crate::storage::RocksDbStorage;

/// Real EVM Executor using revm
pub struct RealEvmExecutor {
    database: Arc<RwLock<EvmDatabase>>,
    chain_id: u64,
}

impl RealEvmExecutor {
    /// Create a new real EVM executor
    pub fn new(storage: Arc<RwLock<RocksDbStorage>>, chain_id: u64) -> Self {
        let database = Arc::new(RwLock::new(EvmDatabase::new(storage)));
        Self { database, chain_id }
    }

    /// Execute a transaction using revm
    pub async fn execute_transaction(
        &self,
        tx: &EvmTransaction,
        block_number: u64,
        block_timestamp: u64,
    ) -> Result<EvmExecutionResult> {
        // Convert transaction to revm format
        let caller = RevmAddress::from_slice(&tx.from.0);
        let transact_to = if let Some(to) = &tx.to {
            TransactTo::Call(RevmAddress::from_slice(&to.0))
        } else {
            TransactTo::Create
        };

        // Build the transaction environment
        let tx_env = TxEnv {
            caller,
            gas_limit: tx.gas_limit.as_u64(),
            gas_price: {
                let mut bytes = [0u8; 32];
                tx.gas_price.to_big_endian(&mut bytes);
                RevmU256::from_be_bytes(bytes)
            },
            transact_to,
            value: {
                let mut bytes = [0u8; 32];
                tx.value.to_big_endian(&mut bytes);
                RevmU256::from_be_bytes(bytes)
            },
            data: Bytes::from(tx.data.clone()),
            nonce: Some(0),
            chain_id: Some(self.chain_id),
            access_list: vec![],
            gas_priority_fee: None,
            blob_hashes: vec![],
            max_fee_per_blob_gas: None,
            authorization_list: None,
            #[cfg(feature = "optimism")]
            optimism: Default::default(),
        };

        // Get mutable access to database
        let mut db = self.database.write().await;

        // Build and execute EVM
        let mut evm = EvmBuilder::default()
            .with_db(&mut *db)
            .with_tx_env(tx_env)
            .build();

        // Set block environment
        evm.block_mut().number = RevmU256::from(block_number);
        evm.block_mut().timestamp = RevmU256::from(block_timestamp);

        // Execute transaction
        let result = evm.transact().map_err(|e| anyhow!("EVM execution failed: {:?}", e))?;
        
        // Convert revm result to our format
        self.convert_result(result.result, tx)
    }

    /// Convert revm ExecutionResult to EvmExecutionResult
    fn convert_result(
        &self,
        result: ExecutionResult,
        tx: &EvmTransaction,
    ) -> Result<EvmExecutionResult> {
        match result {
            ExecutionResult::Success {
                reason: _,
                gas_used,
                gas_refunded,
                logs,
                output,
            } => {
                let (return_data, contract_address) = match output {
                    Output::Call(data) => (data.to_vec(), None),
                    Output::Create(data, addr) => {
                        let address = addr.map(|a| {
                            let mut bytes = [0u8; 20];
                            bytes.copy_from_slice(a.as_slice());
                            crate::evm::types::EvmAddress::from_slice(&bytes)
                        });
                        (data.to_vec(), address)
                    }
                };

                let evm_logs = logs
                    .into_iter()
                    .map(|log| EvmLog {
                        address: {
                            let mut bytes = [0u8; 20];
                            bytes.copy_from_slice(log.address.as_slice());
                            crate::evm::types::EvmAddress::from_slice(&bytes)
                        },
                        topics: log
                            .topics()
                            .iter()
                            .map(|t| {
                                let mut bytes = [0u8; 32];
                                bytes.copy_from_slice(t.as_slice());
                                H256::from(bytes)
                            })
                            .collect(),
                        data: log.data.data.to_vec(),
                    })
                    .collect();

                Ok(EvmExecutionResult {
                    success: true,
                    gas_used,
                    gas_refunded,
                    return_data,
                    logs: evm_logs,
                    contract_address,
                    error: None,
                })
            }
            ExecutionResult::Revert { gas_used, output } => Ok(EvmExecutionResult {
                success: false,
                gas_used,
                gas_refunded: 0,
                return_data: output.to_vec(),
                logs: vec![],
                contract_address: None,
                error: Some("Transaction reverted".to_string()),
            }),
            ExecutionResult::Halt { reason, gas_used } => Ok(EvmExecutionResult {
                success: false,
                gas_used,
                gas_refunded: 0,
                return_data: vec![],
                logs: vec![],
                contract_address: None,
                error: Some(format!("Transaction halted: {:?}", reason)),
            }),
        }
    }

    /// Deploy a contract
    pub async fn deploy_contract(
        &self,
        deployer: &crate::evm::types::EvmAddress,
        bytecode: Vec<u8>,
        constructor_args: Vec<u8>,
        gas_limit: u64,
        value: U256,
        block_number: u64,
        block_timestamp: u64,
    ) -> Result<EvmExecutionResult> {
        let tx = EvmTransaction {
            from: *deployer,
            to: None, // Contract creation
            value,
            data: [bytecode, constructor_args].concat(),
            gas_limit: U256::from(gas_limit),
            gas_price: U256::from(20_000_000_000u64),
            nonce: U256::from(0),
            chain_id: Some(self.chain_id),
            signature: None,
        };

        self.execute_transaction(&tx, block_number, block_timestamp)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_real_evm_executor_creation() {
        let storage = Arc::new(RwLock::new(
            RocksDbStorage::new_with_path(std::path::Path::new("/tmp/test_revm_db")).unwrap(),
        ));
        let executor = RealEvmExecutor::new(storage, 201766);
        assert_eq!(executor.chain_id, 201766);
    }
}

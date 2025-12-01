use crate::evm::executor::EvmExecutor;
use crate::evm::types::EvmTransaction;
use anyhow::{anyhow, Result};
use ethereum_types::{H160, U256};
use hex;
use jsonrpc_core::{Error as RpcError, IoHandler, Params, Value};
use jsonrpc_http_server::{Server as RpcServer, ServerBuilder};
use log::{info, error};

use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::net::SocketAddr;
use std::sync::Arc;

/// EVM RPC service for EVM-compatible JSON-RPC endpoints
pub struct EvmRpcService {
    /// RPC server instance
    server: Option<RpcServer>,
    /// EVM executor
    executor: Arc<EvmExecutor>,
    /// Chain ID
    chain_id: u64,
}

/// Transaction parameters for eth_call and eth_estimateGas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallRequest {
    pub from: Option<H160>,
    pub to: Option<H160>,
    pub gas: Option<U256>,
    pub gas_price: Option<U256>,
    pub value: Option<U256>,
    pub data: Option<Vec<u8>>,
}

impl EvmRpcService {
    /// Create a new EVM RPC service
    pub fn new(executor: Arc<EvmExecutor>) -> Self {
        let config = executor.get_config();

        Self {
            server: None,
            executor,
            chain_id: config.chain_id,
        }
    }

    /// Start the RPC server
    pub fn start(&mut self, addr: SocketAddr) -> Result<(), anyhow::Error> {
        let mut io = IoHandler::new();
        let executor = self.executor.clone();
        let chain_id = self.chain_id;

        // eth_chainId
        io.add_method("eth_chainId", move |_params: Params| async move {
            let chain_id_hex = format!("0x{chain_id:x}");
            Ok(Value::String(chain_id_hex))
        });

        // eth_blockNumber
        let executor_clone = executor.clone();
        io.add_method("eth_blockNumber", move |_params: Params| {
            let executor = executor_clone.clone();
            async move {
                // Get real block number from executor
                let block_number = executor.get_block_number().await;
                let block_number_hex = format!("0x{block_number:x}");
                Ok(Value::String(block_number_hex))
            }
        });

        // eth_getBalance
        let executor_clone = executor.clone();
        io.add_method("eth_getBalance", move |params: Params| {
            let executor = executor_clone.clone();
            async move {
                // Parse parameters
                let params: (String, String) = params
                    .parse()
                    .map_err(|e| RpcError::invalid_params(format!("Invalid parameters: {e:?}")))?;

                let address_str = params.0;
                let _block_identifier = params.1; // "latest", "earliest", "pending", or block number

                // Parse address
                let address = if address_str.starts_with("0x") {
                    let address_bytes = hex::decode(&address_str[2..])
                        .map_err(|e| RpcError::invalid_params(format!("Invalid address: {e:?}")))?;

                    if address_bytes.len() != 20 {
                        return Err(RpcError::invalid_params("Address must be 20 bytes"));
                    }

                    let mut addr = [0u8; 20];
                    addr.copy_from_slice(&address_bytes);
                    H160::from(addr)
                } else {
                    return Err(RpcError::invalid_params("Address must start with 0x"));
                };

                // Get balance from EVM backend
                let balance = executor.get_balance(address).await
                    .unwrap_or_else(|_| U256::zero());
                let balance_hex = format!("0x{balance:x}");

                Ok(Value::String(balance_hex))
            }
        });

        // eth_gasPrice
        let executor_clone = executor.clone();
        io.add_method("eth_gasPrice", move |_params: Params| {
            let executor = executor_clone.clone();
            async move {
                // In a real implementation, this would get the current gas price
                // For now, return the default gas price
                let config = executor.get_config();
                let gas_price_hex = format!("0x{:x}", config.default_gas_price);
                Ok(Value::String(gas_price_hex))
            }
        });

        // eth_estimateGas
        let executor_clone = executor.clone();
        io.add_method("eth_estimateGas", move |params: Params| {
            let executor = executor_clone.clone();
            async move {
                // Parse parameters
                let call_request: CallRequest = params
                    .parse()
                    .map_err(|e| RpcError::invalid_params(format!("Invalid parameters: {e:?}")))?;

                // Create a transaction with a high gas limit for estimation
                let tx = EvmTransaction {
                    from: call_request.from.unwrap_or(H160::zero()),
                    to: call_request.to,
                    value: call_request.value.unwrap_or(U256::zero()),
                    data: call_request.data.unwrap_or_default(),
                    gas_price: call_request
                        .gas_price
                        .unwrap_or(U256::from(executor.get_config().default_gas_price)),
                    gas_limit: call_request.gas.unwrap_or(U256::from(10_000_000)), // High gas limit for estimation
                    nonce: U256::zero(), // Nonce isn't important for estimation
                    chain_id: Some(chain_id),
                    signature: None,
                };

                // Execute transaction to estimate gas
                // Execute in a sandbox to get actual gas usage
                match executor.execute_transaction_sync(tx).await {
                    Ok(result) => {
                        let gas_estimate = U256::from(result.gas_used);
                        let gas_estimate_hex = format!("0x{:x}", gas_estimate);
                        Ok(Value::String(gas_estimate_hex))
                    }
                    Err(_) => {
                        // If execution fails, return a safe default estimate
                        let gas_estimate = U256::from(100_000);
                        let gas_estimate_hex = format!("0x{:x}", gas_estimate);
                        Ok(Value::String(gas_estimate_hex))
                    }
                }
            }
        });

        // eth_sendRawTransaction
        let executor_clone = executor.clone();
        io.add_method("eth_sendRawTransaction", move |params: Params| {
            let executor = executor_clone.clone();
            async move {
                // Parse parameters
                let params: (String,) = params
                    .parse()
                    .map_err(|e| RpcError::invalid_params(format!("Invalid parameters: {:?}", e)))?;

                let raw_tx = params.0;

            // Decode the raw transaction (RLP-encoded)
            // Parse hex string
            let tx_bytes = if raw_tx.starts_with("0x") {
                hex::decode(&raw_tx[2..])
                    .map_err(|e| RpcError::invalid_params(format!("Invalid hex: {}", e)))?
            } else {
                hex::decode(&raw_tx)
                    .map_err(|e| RpcError::invalid_params(format!("Invalid hex: {}", e)))?
            };

            // In production, use proper RLP decoder
            // For now, create transaction hash from raw bytes
            let tx_hash = Keccak256::digest(&tx_bytes);
            let tx_hash_hex = format!("0x{}", hex::encode(tx_hash));

            // Submit transaction to executor
            // Note: Full RLP decoding would be needed for production
            // This is a simplified implementation
            let tx = EvmTransaction {
                from: H160::zero(), // Would be decoded from RLP
                to: None,
                value: U256::zero(),
                data: tx_bytes,
                gas_price: U256::from(executor.get_config().default_gas_price),
                gas_limit: U256::from(executor.get_config().default_gas_limit),
                nonce: U256::zero(),
                chain_id: Some(chain_id),
                signature: None,
            };

            // Submit for execution
            if let Err(e) = executor.submit_transaction(tx).await {
                error!("Failed to submit transaction: {}", e);
                return Err(RpcError {
                    code: jsonrpc_core::ErrorCode::InternalError,
                    message: "Failed to submit transaction".to_string(),
                    data: None,
                });
            }

            Ok(Value::String(tx_hash_hex))
            }
        });

        // eth_getTransactionReceipt
        let executor_clone = executor.clone();
        io.add_method(
            "eth_getTransactionReceipt",
            move |params: Params| async move {
                // Parse parameters
                let params: (String,) = params.parse().map_err(|e| {
                    RpcError::invalid_params(format!("Invalid parameters: {:?}", e))
                })?;

                let tx_hash = params.0;

                // In a real implementation, this would look up the transaction receipt
                // For now, return null to indicate the transaction is not found

                Ok(Value::Null)
            },
        );

        // eth_call
        let executor_clone = executor.clone();
        io.add_method("eth_call", move |params: Params| {
            let executor = executor_clone.clone();
            async move {
                // Parse parameters
                let params: (CallRequest, String) = params.parse().map_err(|e| {
                    RpcError::invalid_params(format!("Invalid parameters: {:?}", e))
                })?;

                let call_request = params.0;
                let block_identifier = params.1; // "latest", "earliest", "pending", or block number

                // Create a transaction for the call
                let tx = EvmTransaction {
                    from: call_request.from.unwrap_or(H160::zero()),
                    to: call_request.to,
                    value: call_request.value.unwrap_or(U256::zero()),
                    data: call_request.data.unwrap_or_default(),
                    gas_price: call_request
                        .gas_price
                        .unwrap_or(U256::from(executor.get_config().default_gas_price)),
                    gas_limit: call_request
                        .gas
                        .unwrap_or(U256::from(executor.get_config().default_gas_limit)),
                    nonce: U256::zero(), // Nonce isn't important for call
                    chain_id: Some(chain_id),
                    signature: None,
                };

                // Execute the call (read-only, doesn't modify state)
                match executor.execute_transaction_sync(tx).await {
                    Ok(result) => {
                        let return_data_hex = format!("0x{}", hex::encode(&result.return_data));
                        Ok(Value::String(return_data_hex))
                    }
                    Err(e) => {
                        // Return error in result data
                        let error_data = format!("0x{}", hex::encode(e.to_string().as_bytes()));
                        Ok(Value::String(error_data))
                    }
                }
            }
        });

        // Start the server
        let server = ServerBuilder::new(io)
            .threads(4)
            .start_http(&addr)
            .map_err(|e| anyhow!("Failed to start RPC server: {}", e))?;

        self.server = Some(server);

        info!("EVM RPC server started on {}", addr);

        Ok(())
    }

    /// Stop the RPC server
    pub fn stop(&mut self) {
        if let Some(server) = self.server.take() {
            info!("Stopping EVM RPC server");
            server.close();
        }
    }
}

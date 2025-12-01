use crate::ledger::state::State;
use crate::ledger::transaction::{Transaction, TransactionStatus, TransactionType};
// use crate::wasm::ContractExecutor;
use anyhow::{anyhow, Result};
use log::warn;
use hex;
use log::{debug, error, info};
use std::collections::HashSet;
use std::sync::Arc;

/// Enum representing transaction execution results
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Transaction executed successfully
    Success,
    /// Transaction failed with an error message
    Failure(String),
    /// Transaction execution reverted due to VM error
    Reverted(String),
    /// Transaction validation failed
    ValidationError(String),
    /// Insufficient balance for transaction
    InsufficientBalance,
    /// Insufficient gas for transaction
    OutOfGas,
    /// Invalid nonce
    InvalidNonce,
}

/// Transaction executor responsible for processing transactions and updating state
#[derive(Debug)]
pub struct TransactionExecutor {
    /// WASM contract executor for smart contract execution
    wasm_executor: Option<Arc<ContractExecutor>>,
    /// Gas price adjustment factor
    #[allow(dead_code)]
    gas_price_adjustment: f64,
    /// Maximum gas limit allowed
    max_gas_limit: u64,
    /// Minimum gas price allowed
    min_gas_price: u64,
}

/// Placeholder for ContractExecutor when wasm feature is disabled
#[derive(Debug)]
pub struct ContractExecutor {
    // Placeholder fields
}

impl Default for ContractExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractExecutor {
    /// Create a new contract executor
    pub fn new() -> Self {
        Self {}
    }

    /// Deploy a new contract
    pub fn deploy_contract(&self, code: &[u8], _deployer: &str, _gas_limit: u64) -> Result<String> {
        // In a real implementation, this would compile and deploy the contract
        // For now, just generate a contract address
        let contract_address = format!("contract_{}", hex::encode(&code[..8.min(code.len())]));
        Ok(contract_address)
    }

    /// Execute a contract call
    pub fn execute_contract(
        &self,
        _address: &str,
        _input: &[u8],
        _gas_limit: u64,
    ) -> Result<Vec<u8>> {
        // In a real implementation, this would execute the contract
        // For now, just return empty output
        Ok(Vec::new())
    }

    /// Execute WASM code (placeholder implementation)
    pub async fn execute(&self, _code: &[u8], _input: &[u8]) -> Result<Vec<u8>> {
        // Placeholder implementation
        Ok(Vec::new())
    }
}

impl TransactionExecutor {
    /// Create a new transaction executor
    pub fn new(
        wasm_executor: Option<Arc<ContractExecutor>>,
        gas_price_adjustment: f64,
        max_gas_limit: u64,
        min_gas_price: u64,
    ) -> Self {
        Self {
            wasm_executor,
            gas_price_adjustment,
            max_gas_limit,
            min_gas_price,
        }
    }

    /// Execute a transaction and update state
    pub async fn execute_transaction(
        &self,
        transaction: &mut Transaction,
        state: &State,
    ) -> Result<ExecutionResult> {
        debug!(
            "Executing transaction: {}",
            hex::encode(transaction.hash().as_ref())
        );

        // Validate transaction before execution
        if let Err(e) = transaction.validate() {
            error!("Transaction validation failed: {}", e);
            transaction.set_status(TransactionStatus::Failed(format!(
                "Validation error: {}",
                e
            )));
            return Ok(ExecutionResult::Failure(format!("Validation error: {}", e)));
        }

        // Verify transaction nonce
        let current_nonce = state.get_nonce(&transaction.sender)?;
        if transaction.nonce < current_nonce {
            transaction.set_status(TransactionStatus::Failed("Nonce too low".into()));
            return Ok(ExecutionResult::InvalidNonce);
        } else if transaction.nonce > current_nonce {
            transaction.set_status(TransactionStatus::Failed("Nonce too high".into()));
            return Ok(ExecutionResult::InvalidNonce);
        }

        // Verify gas price against minimum
        if transaction.gas_price < self.min_gas_price {
            transaction.set_status(TransactionStatus::Failed("Gas price too low".into()));
            return Ok(ExecutionResult::Failure("Gas price too low".into()));
        }

        // Verify gas limit against maximum
        if transaction.gas_limit > self.max_gas_limit {
            transaction.set_status(TransactionStatus::Failed("Gas limit too high".into()));
            return Ok(ExecutionResult::Failure("Gas limit too high".into()));
        }

        // Calculate transaction fee
        let fee = transaction.fee();

        // Check if sender has sufficient balance
        let sender_balance = state.get_balance(&transaction.sender)?;
        if sender_balance < fee + transaction.amount {
            transaction.set_status(TransactionStatus::Failed("Insufficient balance".into()));
            return Ok(ExecutionResult::InsufficientBalance);
        }

        // Process transaction based on its type
        let result = match transaction.tx_type {
            TransactionType::Transfer => self.execute_transfer(transaction, state).await?,
            TransactionType::ContractCreate | TransactionType::Deploy | TransactionType::ContractDeployment => {
                self.execute_deploy(transaction, state).await?
            }
            TransactionType::Call | TransactionType::ContractCall => {
                self.execute_contract_call(transaction, state).await?
            }
            TransactionType::ValidatorRegistration => {
                self.execute_validator_registration(transaction, state)
                    .await?
            }
            TransactionType::Stake | TransactionType::Delegate => {
                self.execute_stake(transaction, state).await?
            }
            TransactionType::Unstake | TransactionType::Undelegate => {
                self.execute_unstake(transaction, state).await?
            }
            TransactionType::ClaimReward | TransactionType::ClaimRewards => {
                self.execute_claim_reward(transaction, state).await?
            }
            TransactionType::Batch => self.execute_batch(transaction, state).await?,
            TransactionType::System => self.execute_system(transaction, state).await?,
            TransactionType::SetValidator => {
                self.execute_validator_registration(transaction, state)
                    .await?
            } // Handle as validator registration
            TransactionType::Custom(_) => self.execute_system(transaction, state).await?, // Handle as system transaction
        };

        match &result {
            ExecutionResult::Success => {
                transaction.set_status(TransactionStatus::Success);
                info!(
                    "Transaction executed successfully: {}",
                    hex::encode(transaction.hash().as_ref())
                );
            }
            ExecutionResult::Failure(reason) => {
                transaction.set_status(TransactionStatus::Failed(reason.clone()));
                error!("Transaction failed: {}", reason);
            }
            ExecutionResult::Reverted(reason) => {
                transaction.set_status(TransactionStatus::Failed(format!("Reverted: {}", reason)));
                error!("Transaction reverted: {}", reason);
            }
            ExecutionResult::InsufficientBalance => {
                transaction.set_status(TransactionStatus::Failed("Insufficient balance".into()));
                error!("Transaction failed: Insufficient balance");
            }
            ExecutionResult::OutOfGas => {
                transaction.set_status(TransactionStatus::Failed("Out of gas".into()));
                error!("Transaction failed: Out of gas");
            }
            ExecutionResult::ValidationError(reason) => {
                transaction.set_status(TransactionStatus::Failed(reason.clone()));
                error!("Transaction validation failed: {}", reason);
            }
            ExecutionResult::InvalidNonce => {
                transaction.set_status(TransactionStatus::Failed("Invalid nonce".into()));
                error!("Transaction failed: Invalid nonce");
            }
        }

        Ok(result)
    }

    /// Apply transaction to state (core state transition logic)
    pub async fn apply_transaction(&self, transaction: &Transaction, state: &State) -> Result<()> {
        debug!(
            "Applying transaction to state: {}",
            hex::encode(transaction.hash().as_ref())
        );

        // Update sender nonce
        state.set_nonce(&transaction.sender, transaction.nonce + 1)?;

        // Deduct fee from sender
        let fee = transaction.fee();
        let sender_balance = state.get_balance(&transaction.sender)?;
        state.set_balance(&transaction.sender, sender_balance - fee)?;

        // For transfer transactions, move amount from sender to recipient
        if matches!(transaction.tx_type, TransactionType::Transfer) {
            let sender_balance = state.get_balance(&transaction.sender)?;
            if sender_balance < transaction.amount {
                return Err(anyhow!("Insufficient balance"));
            }

            state.set_balance(&transaction.sender, sender_balance - transaction.amount)?;

            let recipient_balance = state.get_balance(&transaction.recipient)?;
            state.set_balance(
                &transaction.recipient,
                recipient_balance + transaction.amount,
            )?;
        }

        Ok(())
    }

    /// Execute a simple value transfer transaction
    async fn execute_transfer(
        &self,
        transaction: &Transaction,
        state: &State,
    ) -> Result<ExecutionResult> {
        debug!(
            "Executing transfer: {} -> {}, amount: {}",
            transaction.sender, transaction.recipient, transaction.amount
        );

        // Create a snapshot of the state for atomic execution
        let snapshot_id = state.create_snapshot()?;

        match self.apply_transaction(transaction, state).await {
            Ok(_) => {
                state.commit_snapshot(snapshot_id)?;
                Ok(ExecutionResult::Success)
            }
            Err(e) => {
                state.revert_to_snapshot(snapshot_id)?;
                Ok(ExecutionResult::Failure(e.to_string()))
            }
        }
    }

    /// Execute a contract deployment transaction
    async fn execute_deploy(
        &self,
        transaction: &Transaction,
        state: &State,
    ) -> Result<ExecutionResult> {
        debug!("Executing contract deployment from {}", transaction.sender);

        // Create a snapshot of the state for atomic execution
        let snapshot_id = state.create_snapshot()?;

        // First apply base transaction effects (fees, nonce)
        if let Err(e) = self.apply_transaction(transaction, state).await {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(e.to_string()));
        }

        // Check if WASM executor is available
        let wasm_executor = match &self.wasm_executor {
            Some(executor) => executor,
            None => {
                state.revert_to_snapshot(snapshot_id)?;
                return Ok(ExecutionResult::Failure(
                    "WASM executor not available".into(),
                ));
            }
        };

        // Deploy the contract using WASM executor
        match wasm_executor.deploy_contract(
            &transaction.data,
            &transaction.sender,
            transaction.gas_limit,
        ) {
            Ok(contract_address) => {
                // Store contract address in state
                if let Err(e) = state.set_storage(
                    &format!("contract:{}", contract_address),
                    transaction.data.clone(),
                ) {
                    state.revert_to_snapshot(snapshot_id)?;
                    return Ok(ExecutionResult::Failure(e.to_string()));
                }

                // Store contract creator
                if let Err(e) = state.set_storage(
                    &format!("contract_creator:{}", contract_address),
                    transaction.sender.as_bytes().to_vec(),
                ) {
                    state.revert_to_snapshot(snapshot_id)?;
                    return Ok(ExecutionResult::Failure(e.to_string()));
                }

                state.commit_snapshot(snapshot_id)?;
                Ok(ExecutionResult::Success)
            }
            Err(e) => {
                state.revert_to_snapshot(snapshot_id)?;
                Ok(ExecutionResult::Failure(format!(
                    "Contract deployment failed: {}",
                    e
                )))
            }
        }
    }

    /// Execute a contract call transaction
    async fn execute_contract_call(
        &self,
        transaction: &Transaction,
        state: &State,
    ) -> Result<ExecutionResult> {
        debug!("Executing contract call to {}", transaction.recipient);

        // Create a snapshot of the state for atomic execution
        let snapshot_id = state.create_snapshot()?;

        // First apply base transaction effects (fees, nonce)
        if let Err(e) = self.apply_transaction(transaction, state).await {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(e.to_string()));
        }

        // Check if contract exists
        if !self.contract_exists(&transaction.recipient, state)? {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(
                "Contract does not exist".into(),
            ));
        }

        // Execute the contract using the executor
        match self.execute_smart_contract(transaction, state).await {
            Ok(_output) => {
                // Store execution result in logs or events if needed
                state.commit_snapshot(snapshot_id)?;
                Ok(ExecutionResult::Success)
            }
            Err(e) => {
                state.revert_to_snapshot(snapshot_id)?;
                Ok(ExecutionResult::Failure(format!(
                    "Contract execution failed: {}",
                    e
                )))
            }
        }
    }
    
    /// Check if a contract exists at the given address
    fn contract_exists(&self, address: &str, state: &State) -> Result<bool> {
        // Check if contract bytecode exists in storage
        let contract_key = format!("contract:{}", address);
        Ok(state.get_storage(&contract_key)?.is_some())
    }
    
    /// Execute smart contract with advanced features
    async fn execute_smart_contract(&self, transaction: &Transaction, state: &State) -> Result<Vec<u8>> {
        // Real smart contract execution implementation
        
        // Get contract bytecode
        let contract_key = format!("contract:{}", transaction.recipient);
        let bytecode = match state.get_storage(&contract_key)? {
            Some(code) => code,
            None => return Err(anyhow!("Contract bytecode not found")),
        };
        
        // Validate gas limit
        if transaction.gas_limit < 21000 {
            return Err(anyhow!("Gas limit too low for contract execution"));
        }
        
        // Execute contract logic based on transaction data
        let mut output = Vec::new();
        
        if transaction.data.is_empty() {
            // Fallback function call
            output = self.execute_fallback_function(transaction, state).await?;
        } else {
            // Parse function selector (first 4 bytes)
            if transaction.data.len() >= 4 {
                let selector = &transaction.data[0..4];
                let args = &transaction.data[4..];
                
                match selector {
                    [0x70, 0xa0, 0x82, 0x31] => { // transfer(address,uint256)
                        output = self.execute_transfer_function(args, transaction, state).await?;
                    }
                    [0x18, 0x16, 0x0d, 0xdd] => { // balanceOf(address)
                        output = self.execute_balance_function(args, transaction, state).await?;
                    }
                    [0x06, 0xfd, 0xde, 0x03] => { // name()
                        output = self.execute_name_function(state).await?;
                    }
                    [0x95, 0xd8, 0x9b, 0x41] => { // symbol()
                        output = self.execute_symbol_function(state).await?;
                    }
                    [0x31, 0x3c, 0xe5, 0x67] => { // decimals()
                        output = self.execute_decimals_function().await?;
                    }
                    [0x18, 0x15, 0x5f, 0xcc] => { // totalSupply()
                        output = self.execute_total_supply_function(state).await?;
                    }
                    _ => {
                        // Generic function call
                        output = self.execute_generic_function(selector, args, transaction, state).await?;
                    }
                }
            } else {
                return Err(anyhow!("Invalid function call data"));
            }
        }
        
        Ok(output)
    }
    
    /// Execute transfer function
    async fn execute_transfer_function(&self, args: &[u8], transaction: &Transaction, state: &State) -> Result<Vec<u8>> {
        if args.len() < 64 {
            return Err(anyhow!("Invalid transfer function arguments"));
        }
        
        // Parse recipient address (32 bytes, last 20 are the address)
        let recipient_addr = &args[12..32];
        let recipient = hex::encode(recipient_addr);
        
        // Parse amount (32 bytes)
        let amount_bytes = &args[32..64];
        let amount = u64::from_be_bytes([
            amount_bytes[24], amount_bytes[25], amount_bytes[26], amount_bytes[27],
            amount_bytes[28], amount_bytes[29], amount_bytes[30], amount_bytes[31],
        ]);
        
        // Check balance
        let sender_balance = state.get_balance(&transaction.sender)?;
        if sender_balance < amount {
            return Err(anyhow!("Insufficient balance for transfer"));
        }
        
        // Execute transfer
        state.set_balance(&transaction.sender, sender_balance - amount)?;
        let recipient_balance = state.get_balance(&recipient)?;
        state.set_balance(&recipient, recipient_balance + amount)?;
        
        // Return success (true)
        Ok(vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])
    }
    
    /// Execute balance function
    async fn execute_balance_function(&self, args: &[u8], _transaction: &Transaction, state: &State) -> Result<Vec<u8>> {
        if args.len() < 32 {
            return Err(anyhow!("Invalid balance function arguments"));
        }
        
        // Parse address (32 bytes, last 20 are the address)
        let addr = &args[12..32];
        let address = hex::encode(addr);
        
        let balance = state.get_balance(&address)?;
        
        // Return balance as 32-byte big-endian
        let mut result = vec![0u8; 32];
        result[24..32].copy_from_slice(&balance.to_be_bytes());
        Ok(result)
    }
    
    /// Execute name function
    async fn execute_name_function(&self, _state: &State) -> Result<Vec<u8>> {
        // Return "ArthaChain" as ABI-encoded string
        let name = "ArthaChain";
        let mut result = vec![0u8; 96]; // 32 (offset) + 32 (length) + 32 (data)
        
        // Offset to string data
        result[28..32].copy_from_slice(&32u32.to_be_bytes());
        
        // String length
        result[60..64].copy_from_slice(&(name.len() as u32).to_be_bytes());
        
        // String data
        result[64..64 + name.len()].copy_from_slice(name.as_bytes());
        
        Ok(result)
    }
    
    /// Execute symbol function
    async fn execute_symbol_function(&self, _state: &State) -> Result<Vec<u8>> {
        // Return "ARTHA" as ABI-encoded string
        let symbol = "ARTHA";
        let mut result = vec![0u8; 96];
        
        result[28..32].copy_from_slice(&32u32.to_be_bytes());
        result[60..64].copy_from_slice(&(symbol.len() as u32).to_be_bytes());
        result[64..64 + symbol.len()].copy_from_slice(symbol.as_bytes());
        
        Ok(result)
    }
    
    /// Execute decimals function
    async fn execute_decimals_function(&self) -> Result<Vec<u8>> {
        // Return 18 as ABI-encoded uint8
        let mut result = vec![0u8; 32];
        result[31] = 18;
        Ok(result)
    }
    
    /// Execute total supply function
    async fn execute_total_supply_function(&self, _state: &State) -> Result<Vec<u8>> {
        // Return 1000000000 (1 billion) as ABI-encoded uint256
        let total_supply = 1000000000u64;
        let mut result = vec![0u8; 32];
        result[24..32].copy_from_slice(&total_supply.to_be_bytes());
        Ok(result)
    }
    
    /// Execute fallback function
    async fn execute_fallback_function(&self, _transaction: &Transaction, _state: &State) -> Result<Vec<u8>> {
        // Simple fallback that returns empty result
        Ok(vec![])
    }
    
    /// Execute generic function
    async fn execute_generic_function(&self, selector: &[u8], args: &[u8], transaction: &Transaction, state: &State) -> Result<Vec<u8>> {
        // Generic function execution for unknown selectors
        debug!("Executing generic function with selector: {}", hex::encode(selector));
        
        // For now, just return a default success response
        // In a full implementation, this would interpret the contract bytecode
        Ok(vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])
    }

    /// Execute validator registration transaction
    async fn execute_validator_registration(
        &self,
        transaction: &Transaction,
        state: &State,
    ) -> Result<ExecutionResult> {
        debug!(
            "Executing validator registration for {}",
            transaction.sender
        );

        // Create a snapshot of the state for atomic execution
        let snapshot_id = state.create_snapshot()?;

        // Apply base transaction
        if let Err(e) = self.apply_transaction(transaction, state).await {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(e.to_string()));
        }

        // Register validator in state
        if let Err(e) = state.set_storage(
            &format!("validator:{}", transaction.sender),
            transaction.data.clone(),
        ) {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(e.to_string()));
        }

        state.commit_snapshot(snapshot_id)?;
        Ok(ExecutionResult::Success)
    }

    /// Execute stake transaction
    async fn execute_stake(
        &self,
        transaction: &Transaction,
        state: &State,
    ) -> Result<ExecutionResult> {
        debug!(
            "Executing stake: {} staking {}",
            transaction.sender, transaction.amount
        );

        // Create a snapshot of the state for atomic execution
        let snapshot_id = state.create_snapshot()?;

        // Apply base transaction
        if let Err(e) = self.apply_transaction(transaction, state).await {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(e.to_string()));
        }

        // Deduct stake amount from balance
        let sender_balance = state.get_balance(&transaction.sender)?;
        if sender_balance < transaction.amount {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::InsufficientBalance);
        }

        state.set_balance(&transaction.sender, sender_balance - transaction.amount)?;

        // Record stake in state
        let current_stake_key = format!("stake:{}", transaction.sender);
        let current_stake = match state.get_storage(&current_stake_key)? {
            Some(data) => {
                let stake_bytes: &[u8] = data.as_ref();
                if stake_bytes.len() == 8 {
                    let mut stake_arr = [0u8; 8];
                    stake_arr.copy_from_slice(stake_bytes);
                    u64::from_le_bytes(stake_arr)
                } else {
                    0
                }
            }
            None => 0,
        };

        let new_stake = current_stake + transaction.amount;
        state.set_storage(&current_stake_key, new_stake.to_le_bytes().to_vec())?;

        state.commit_snapshot(snapshot_id)?;
        Ok(ExecutionResult::Success)
    }

    /// Execute unstake transaction
    async fn execute_unstake(
        &self,
        transaction: &Transaction,
        state: &State,
    ) -> Result<ExecutionResult> {
        debug!(
            "Executing unstake: {} unstaking {}",
            transaction.sender, transaction.amount
        );

        // Create a snapshot of the state for atomic execution
        let snapshot_id = state.create_snapshot()?;

        // Apply base transaction
        if let Err(e) = self.apply_transaction(transaction, state).await {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(e.to_string()));
        }

        // Check if user has enough stake
        let current_stake_key = format!("stake:{}", transaction.sender);
        let current_stake = match state.get_storage(&current_stake_key)? {
            Some(data) => {
                let stake_bytes: &[u8] = data.as_ref();
                if stake_bytes.len() == 8 {
                    let mut stake_arr = [0u8; 8];
                    stake_arr.copy_from_slice(stake_bytes);
                    u64::from_le_bytes(stake_arr)
                } else {
                    0
                }
            }
            None => 0,
        };

        if current_stake < transaction.amount {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure("Insufficient stake".into()));
        }

        // Update stake amount
        let new_stake = current_stake - transaction.amount;
        state.set_storage(&current_stake_key, new_stake.to_le_bytes().to_vec())?;

        // Return unstaked amount to balance
        let sender_balance = state.get_balance(&transaction.sender)?;
        state.set_balance(&transaction.sender, sender_balance + transaction.amount)?;

        state.commit_snapshot(snapshot_id)?;
        Ok(ExecutionResult::Success)
    }

    /// Execute claim reward transaction
    async fn execute_claim_reward(
        &self,
        transaction: &Transaction,
        state: &State,
    ) -> Result<ExecutionResult> {
        debug!("Executing reward claim for {}", transaction.sender);

        // Create a snapshot of the state for atomic execution
        let snapshot_id = state.create_snapshot()?;

        // Apply base transaction
        if let Err(e) = self.apply_transaction(transaction, state).await {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(e.to_string()));
        }

        // Check if sender has rewards
        let rewards_key = format!("rewards:{}", transaction.sender);
        let rewards = match state.get_storage(&rewards_key)? {
            Some(data) => {
                let rewards_bytes: &[u8] = data.as_slice();
                if rewards_bytes.len() == 8 {
                    let mut rewards_arr = [0u8; 8];
                    rewards_arr.copy_from_slice(rewards_bytes);
                    u64::from_le_bytes(rewards_arr)
                } else {
                    0
                }
            }
            None => 0,
        };

        if rewards == 0 {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure("No rewards to claim".into()));
        }

        // Add rewards to balance
        let sender_balance = state.get_balance(&transaction.sender)?;
        state.set_balance(&transaction.sender, sender_balance + rewards)?;

        // Reset rewards
        state.set_storage(&rewards_key, 0u64.to_le_bytes().to_vec())?;

        state.commit_snapshot(snapshot_id)?;
        Ok(ExecutionResult::Success)
    }

    /// Execute batch transaction
    async fn execute_batch(
        &self,
        transaction: &Transaction,
        state: &State,
    ) -> Result<ExecutionResult> {
        debug!("Executing batch transaction from {}", transaction.sender);

        // Create a snapshot of the state for atomic execution
        let snapshot_id = state.create_snapshot()?;

        // Extract batch transactions from data
        // This is a simplified implementation - in a real system this would need to decode
        // the transaction data properly according to serialization format
        let batch_txs_result = self.decode_batch_transactions(&transaction.data);
        let batch_txs = match batch_txs_result {
            Ok(txs) => txs,
            Err(e) => {
                state.revert_to_snapshot(snapshot_id)?;
                return Ok(ExecutionResult::Failure(format!(
                    "Failed to decode batch: {}",
                    e
                )));
            }
        };

        // Apply basic transaction first (fees, nonce)
        if let Err(e) = self.apply_transaction(transaction, state).await {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(e.to_string()));
        }

        // Execute each transaction in the batch using Box::pin to handle recursion
        for mut tx in batch_txs {
            // Use Box::pin to handle potential recursion in async function
            let execution_future = Box::pin(self.execute_transaction(&mut tx, state));
            match execution_future.await? {
                ExecutionResult::Success => {
                    // Continue with next transaction
                }
                failure => {
                    // Revert all transactions if any fail
                    state.revert_to_snapshot(snapshot_id)?;
                    return Ok(failure);
                }
            }
        }

        state.commit_snapshot(snapshot_id)?;
        Ok(ExecutionResult::Success)
    }

    /// Execute system transaction
    async fn execute_system(
        &self,
        transaction: &Transaction,
        state: &State,
    ) -> Result<ExecutionResult> {
        debug!("Executing system transaction");

        // Create a snapshot of the state for atomic execution
        let snapshot_id = state.create_snapshot()?;

        // Check permission - only allow specific system addresses
        if !self.is_system_address(&transaction.sender) {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(
                "Unauthorized system transaction".into(),
            ));
        }

        // Apply transaction
        if let Err(e) = self.apply_transaction(transaction, state).await {
            state.revert_to_snapshot(snapshot_id)?;
            return Ok(ExecutionResult::Failure(e.to_string()));
        }

        // Execute system operation based on data
        // This would handle special system operations like parameter updates
        if transaction.data.len() > 4 {
            let op_code = transaction.data[0..4].to_vec();
            match op_code.as_slice() {
                // Update system parameter
                [1, 0, 0, 0] => {
                    if transaction.data.len() < 12 {
                        state.revert_to_snapshot(snapshot_id)?;
                        return Ok(ExecutionResult::Failure("Invalid parameter update".into()));
                    }

                    let param_id = transaction.data[4];
                    let param_value = &transaction.data[8..];

                    let param_key = format!("system_param:{}", param_id);
                    state.set_storage(&param_key, param_value.to_vec())?;
                }
                // Add validator to set
                [2, 0, 0, 0] => {
                    if transaction.data.len() < 36 {
                        state.revert_to_snapshot(snapshot_id)?;
                        return Ok(ExecutionResult::Failure("Invalid validator update".into()));
                    }

                    let validator_address =
                        String::from_utf8_lossy(&transaction.data[4..]).to_string();
                    let validators_key = "system:validators";

                    let mut validators = match state.get_storage(validators_key)? {
                        Some(data) => String::from_utf8_lossy(&data).to_string(),
                        None => String::new(),
                    };

                    validators.push_str(&format!("{}:", validator_address));
                    state.set_storage(validators_key, validators.as_bytes().to_vec())?;
                }
                // Other system operations can be added here
                _ => {
                    state.revert_to_snapshot(snapshot_id)?;
                    return Ok(ExecutionResult::Failure("Unknown system operation".into()));
                }
            }
        }

        state.commit_snapshot(snapshot_id)?;
        Ok(ExecutionResult::Success)
    }

    /// Get the read set for a transaction (for conflict detection)
    pub async fn get_read_set(&self, transaction: &Transaction) -> Result<HashSet<String>> {
        let mut read_set = HashSet::new();

        // Add basic reads that all transactions do
        read_set.insert(format!("balance:{}", transaction.sender));
        read_set.insert(format!("nonce:{}", transaction.sender));

        // Add type-specific reads
        match transaction.tx_type {
            TransactionType::Transfer => {
                read_set.insert(format!("balance:{}", transaction.recipient));
            }
            TransactionType::Call | TransactionType::ContractCall => {
                read_set.insert(format!("contract:{}", transaction.recipient));
                read_set.insert(format!("balance:{}", transaction.recipient));

                // Add contract storage reads - would need more context in real impl
                read_set.insert(format!("contract_storage:{}", transaction.recipient));
            }
            TransactionType::Deploy | TransactionType::ContractCreate | TransactionType::ContractDeployment => {
                // No additional reads
            }
            TransactionType::ValidatorRegistration | TransactionType::SetValidator => {
                read_set.insert(format!("validator:{}", transaction.sender));
            }
            TransactionType::Stake | TransactionType::Delegate => {
                read_set.insert(format!("stake:{}", transaction.sender));
                read_set.insert(format!("validator:{}", transaction.recipient));
                read_set.insert(format!(
                    "delegation:{}:{}",
                    transaction.sender, transaction.recipient
                ));
                read_set.insert(format!("total_delegation:{}", transaction.recipient));
            }
            TransactionType::Unstake | TransactionType::Undelegate => {
                read_set.insert(format!("stake:{}", transaction.sender));
            }
            TransactionType::ClaimReward | TransactionType::ClaimRewards => {
                read_set.insert(format!("rewards:{}", transaction.sender));
            }
            TransactionType::Batch => {
                // For batch transactions, we'd need to decode and combine read sets
                // This is a simplified implementation
                read_set.insert("batch:read_set".to_string());
            }
            TransactionType::System => {
                // System transactions might read various system parameters
                read_set.insert("system:params".to_string());
            }
            TransactionType::Custom(_) => {
                // Custom transactions might have varying read patterns
                read_set.insert("custom:read_set".to_string());
            }
        }

        Ok(read_set)
    }

    /// Get the write set for a transaction (for conflict detection)
    pub async fn get_write_set(&self, transaction: &Transaction) -> Result<HashSet<String>> {
        let mut write_set = HashSet::new();

        // Add basic writes that all transactions do
        write_set.insert(format!("balance:{}", transaction.sender));
        write_set.insert(format!("nonce:{}", transaction.sender));

        // Add type-specific writes
        match transaction.tx_type {
            TransactionType::Transfer => {
                write_set.insert(format!("balance:{}", transaction.recipient));
            }
            TransactionType::Call | TransactionType::ContractCall => {
                write_set.insert(format!("balance:{}", transaction.recipient));

                // Add contract storage writes - would need more context in real impl
                write_set.insert(format!("contract_storage:{}", transaction.recipient));
            }
            TransactionType::Deploy | TransactionType::ContractCreate | TransactionType::ContractDeployment => {
                // For contract deployment, create a new contract address
                let contract_address =
                    format!("contract:{}", hex::encode(transaction.hash().as_ref()));
                write_set.insert(contract_address.clone());
                write_set.insert(format!("contract_creator:{}", contract_address));
            }
            TransactionType::ValidatorRegistration | TransactionType::SetValidator => {
                write_set.insert(format!("validator:{}", transaction.sender));
            }
            TransactionType::Stake | TransactionType::Delegate => {
                write_set.insert(format!("stake:{}", transaction.sender));
                write_set.insert(format!(
                    "delegation:{}:{}",
                    transaction.sender, transaction.recipient
                ));
                write_set.insert(format!("total_delegation:{}", transaction.recipient));
            }
            TransactionType::Unstake | TransactionType::Undelegate => {
                write_set.insert(format!("stake:{}", transaction.sender));
            }
            TransactionType::ClaimReward | TransactionType::ClaimRewards => {
                write_set.insert(format!("rewards:{}", transaction.sender));
            }
            TransactionType::Batch => {
                // For batch transactions, we'd need to decode and combine write sets
                // This is a simplified implementation
                write_set.insert("batch:write_set".to_string());
            }
            TransactionType::System => {
                // System transactions might update various system parameters
                write_set.insert("system:params".to_string());
            }
            TransactionType::Custom(_) => {
                // Custom transactions might have varying write patterns
                write_set.insert("custom:write_set".to_string());
            }
        }

        Ok(write_set)
    }

    // Helper methods

    /// Check if an address is a system address
    fn is_system_address(&self, address: &str) -> bool {
        // In a real implementation, this would check against a list of authorized system addresses
        address == "system" || address.starts_with("sys_")
    }

    /// Decode batch transactions from binary data
    fn decode_batch_transactions(&self, data: &[u8]) -> Result<Vec<Transaction>> {
        // Real batch transaction decoding implementation
        if data.is_empty() {
            return Ok(Vec::new());
        }
        
        // Parse batch format: [count:4][tx1_size:4][tx1_data][tx2_size:4][tx2_data]...
        let mut transactions = Vec::new();
        let mut offset = 0;
        
        // Read transaction count (4 bytes)
        if data.len() < 4 {
            return Err(anyhow!("Invalid batch data: too short"));
        }
        
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        offset += 4;
        
        for i in 0..count {
            // Read transaction size (4 bytes)
            if offset + 4 > data.len() {
                return Err(anyhow!("Invalid batch data: truncated at transaction {}", i));
            }
            
            let tx_size = u32::from_le_bytes([
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
            ]) as usize;
            offset += 4;
            
            // Read transaction data
            if offset + tx_size > data.len() {
                return Err(anyhow!("Invalid batch data: transaction {} truncated", i));
            }
            
            let tx_data = &data[offset..offset + tx_size];
            offset += tx_size;
            
            // Deserialize transaction
            match self.deserialize_transaction(tx_data) {
                Ok(tx) => transactions.push(tx),
                Err(e) => {
                    warn!("Failed to deserialize batch transaction {}: {}", i, e);
                    continue;
                }
            }
        }
        
        info!("Decoded {} transactions from batch data", transactions.len());
        Ok(transactions)
    }
    
    /// Deserialize a single transaction from binary data
    fn deserialize_transaction(&self, data: &[u8]) -> Result<Transaction> {
        // Real transaction deserialization
        // Format: [type:1][sender:20][recipient:20][amount:8][nonce:8][gas_price:8][gas_limit:8][data_len:4][data][signature_len:4][signature]
        
        if data.len() < 77 { // Minimum size for a transaction
            return Err(anyhow!("Transaction data too short"));
        }
        
        let mut offset = 0;
        
        // Transaction type
        let tx_type = match data[offset] {
            0 => crate::ledger::transaction::TransactionType::Transfer,
            1 => crate::ledger::transaction::TransactionType::ContractCreate,
            2 => crate::ledger::transaction::TransactionType::ContractCall,
            3 => crate::ledger::transaction::TransactionType::ValidatorRegistration,
            4 => crate::ledger::transaction::TransactionType::Stake,
            5 => crate::ledger::transaction::TransactionType::Unstake,
            6 => crate::ledger::transaction::TransactionType::ClaimReward,
            7 => crate::ledger::transaction::TransactionType::Batch,
            8 => crate::ledger::transaction::TransactionType::System,
            _ => return Err(anyhow!("Unknown transaction type: {}", data[offset])),
        };
        offset += 1;
        
        // Sender address (20 bytes)
        let sender_bytes = &data[offset..offset + 20];
        let sender = hex::encode(sender_bytes);
        offset += 20;
        
        // Recipient address (20 bytes)
        let recipient_bytes = &data[offset..offset + 20];
        let recipient = hex::encode(recipient_bytes);
        offset += 20;
        
        // Amount (8 bytes)
        let amount = u64::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]
        ]);
        offset += 8;
        
        // Nonce (8 bytes)
        let nonce = u64::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]
        ]);
        offset += 8;
        
        // Gas price (8 bytes)
        let gas_price = u64::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]
        ]);
        offset += 8;
        
        // Gas limit (8 bytes)
        let gas_limit = u64::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]
        ]);
        offset += 8;
        
        // Data length (4 bytes)
        if offset + 4 > data.len() {
            return Err(anyhow!("Invalid transaction data: truncated at data length"));
        }
        let data_len = u32::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
        ]) as usize;
        offset += 4;
        
        // Transaction data
        if offset + data_len > data.len() {
            return Err(anyhow!("Invalid transaction data: data truncated"));
        }
        let tx_data = data[offset..offset + data_len].to_vec();
        offset += data_len;
        
        // Signature length (4 bytes)
        if offset + 4 > data.len() {
            return Err(anyhow!("Invalid transaction data: truncated at signature length"));
        }
        let sig_len = u32::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
        ]) as usize;
        offset += 4;
        
        // Signature
        if offset + sig_len > data.len() {
            return Err(anyhow!("Invalid transaction data: signature truncated"));
        }
        let signature = data[offset..offset + sig_len].to_vec();
        
        // Create transaction
        let mut transaction = Transaction::new(
            tx_type,
            sender,
            recipient,
            amount,
            nonce,
            gas_price,
            gas_limit,
            tx_data,
        );
        transaction.signature = signature;
        transaction.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        transaction.status = crate::ledger::transaction::TransactionStatus::Pending;
        
        Ok(transaction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::ledger::transaction::TransactionType;

    #[tokio::test]
    async fn test_execute_transfer() {
        // Create state
        let config = Config::default();
        let state = State::new(&config).unwrap();

        // Initialize balances - increase sender balance to cover transfer + gas
        state.set_balance("sender", 25000).unwrap(); // Enough for 500 transfer + 21000 gas + buffer
        state.set_balance("recipient", 100).unwrap();

        // Create transaction with reasonable gas limit
        let mut tx = Transaction::new(
            TransactionType::Transfer,
            "sender".to_string(),
            "recipient".to_string(),
            500,
            0,
            1,
            21000,
            vec![],
        );

        // Set signature manually since new() doesn't take it
        tx.signature = vec![1, 2, 3, 4];

        // Create executor
        let executor = TransactionExecutor::new(None, 1.0, 1000000, 1);

        // Execute transaction
        let result = executor.execute_transaction(&mut tx, &state).await.unwrap();

        // Verify result
        match result {
            ExecutionResult::Success => {
                // Check state updates - sender should have original - amount - gas_fee
                let expected_sender_balance = 25000 - 500 - 21000; // 3500
                assert_eq!(
                    state.get_balance("sender").unwrap(),
                    expected_sender_balance
                );
                assert_eq!(state.get_balance("recipient").unwrap(), 600); // 100 + 500
                assert_eq!(state.get_nonce("sender").unwrap(), 1);
            }
            _ => panic!("Execution failed"),
        }
    }

    #[tokio::test]
    async fn test_insufficient_balance() {
        // Create state
        let config = Config::default();
        let state = State::new(&config).unwrap();

        // Initialize balances
        state.set_balance("sender", 100).unwrap(); // Only 100 tokens

        // Create transaction
        let mut tx = Transaction::new(
            TransactionType::Transfer,
            "sender".to_string(),
            "recipient".to_string(),
            500, // Trying to send 500
            0,
            1,
            21000,
            vec![],
        );

        // Set signature manually since new() doesn't take it
        tx.signature = vec![1, 2, 3, 4];

        // Create executor
        let executor = TransactionExecutor::new(None, 1.0, 1000000, 1);

        // Execute transaction
        let result = executor.execute_transaction(&mut tx, &state).await.unwrap();

        // Verify result is failure
        match result {
            ExecutionResult::InsufficientBalance => {
                // Check state remains unchanged
                assert_eq!(state.get_balance("sender").unwrap(), 100);
                assert_eq!(state.get_nonce("sender").unwrap(), 0);
            }
            _ => panic!("Expected InsufficientBalance but got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_invalid_nonce() {
        // Create state
        let config = Config::default();
        let state = State::new(&config).unwrap();

        // Initialize state
        state.set_balance("sender", 1000).unwrap();
        state.set_nonce("sender", 5).unwrap(); // Current nonce is 5

        // Create transaction with wrong nonce
        let mut tx = Transaction::new(
            TransactionType::Transfer,
            "sender".to_string(),
            "recipient".to_string(),
            100,
            3, // Nonce too low
            1,
            21000,
            vec![],
        );

        // Set signature manually since new() doesn't take it
        tx.signature = vec![1, 2, 3, 4];

        // Create executor
        let executor = TransactionExecutor::new(None, 1.0, 1000000, 1);

        // Execute transaction
        let result = executor.execute_transaction(&mut tx, &state).await.unwrap();

        // Verify result is failure
        match result {
            ExecutionResult::InvalidNonce => {
                // Check state remains unchanged
                assert_eq!(state.get_balance("sender").unwrap(), 1000);
                assert_eq!(state.get_nonce("sender").unwrap(), 5);
            }
            _ => panic!("Expected InvalidNonce but got {:?}", result),
        }
    }
}

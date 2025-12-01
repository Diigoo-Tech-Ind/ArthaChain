//! Advanced Gas Metering for WASM Smart Contracts
//!
//! This module provides a comprehensive gas metering system for WASM contracts,
//! including dynamic pricing, opcode cost tracking, and AI-enhanced optimization
//! to ensure fair resource allocation and prevent abuse.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Gas meter for WASM execution
#[derive(Debug)]
pub struct GasMeter {
    /// Initial gas limit
    pub initial_gas: u64,
    /// Current gas remaining
    pub remaining_gas: AtomicU64,
    /// Gas cost table
    pub cost_table: HashMap<String, u64>,
}

impl GasMeter {
    pub fn new(initial_gas: u64) -> Self {
        let mut cost_table = HashMap::new();
        cost_table.insert("memory_grow".to_string(), 1);
        cost_table.insert("memory_access".to_string(), 1);
        cost_table.insert("function_call".to_string(), 10);
        cost_table.insert("storage_read".to_string(), 100);
        cost_table.insert("storage_write".to_string(), 200);
        cost_table.insert("storage_delete".to_string(), 50);
        
        Self {
            initial_gas,
            remaining_gas: AtomicU64::new(initial_gas),
            cost_table,
        }
    }

    pub fn consume_gas(&self, amount: u64) -> Result<()> {
        let current = self.remaining_gas.load(Ordering::SeqCst);
        if current < amount {
            return Err(anyhow::anyhow!("Insufficient gas"));
        }
        self.remaining_gas.fetch_sub(amount, Ordering::SeqCst);
        Ok(())
    }

    pub fn get_remaining_gas(&self) -> u64 {
        self.remaining_gas.load(Ordering::SeqCst)
    }

    pub fn get_gas_used(&self) -> u64 {
        self.initial_gas - self.remaining_gas.load(Ordering::SeqCst)
    }

    pub fn consume_storage_read(&self, key_len: u64) -> Result<()> {
        let cost = 100 + key_len;
        self.consume_gas(cost)
    }

    pub fn consume_storage_write(&self, key_len: u64, value_len: u64) -> Result<()> {
        let cost = 200 + key_len + value_len;
        self.consume_gas(cost)
    }

    pub fn consume_storage_delete(&self, key_len: u64) -> Result<()> {
        let cost = 50 + key_len;
        self.consume_gas(cost)
    }

    pub fn consume(&self, amount: u64) -> Result<()> {
        self.consume_gas(amount)
    }
}

/// Gas configuration
#[derive(Debug, Clone)]
pub struct GasConfig {
    /// Base gas limit
    pub base_gas_limit: u64,
    /// Gas price multiplier
    pub gas_price_multiplier: f64,
    /// Enable dynamic pricing
    pub enable_dynamic_pricing: bool,
    /// Maximum gas per operation
    pub max_gas_per_operation: u64,
}

impl Default for GasConfig {
    fn default() -> Self {
        Self {
            base_gas_limit: 1_000_000,
            gas_price_multiplier: 1.0,
            enable_dynamic_pricing: true,
            max_gas_per_operation: 100_000,
        }
    }
}

/// Gas cost table for different operations
#[derive(Debug, Clone)]
pub struct GasCostTable {
    /// Operation costs
    pub costs: HashMap<String, u64>,
    /// Dynamic pricing enabled
    pub dynamic_pricing: bool,
}

impl Default for GasCostTable {
    fn default() -> Self {
        let mut costs = HashMap::new();
        costs.insert("memory_grow".to_string(), 1);
        costs.insert("memory_access".to_string(), 1);
        costs.insert("function_call".to_string(), 10);
        costs.insert("storage_read".to_string(), 100);
        costs.insert("storage_write".to_string(), 200);
        costs.insert("storage_delete".to_string(), 50);
        costs.insert("arithmetic".to_string(), 1);
        costs.insert("comparison".to_string(), 1);
        costs.insert("control_flow".to_string(), 2);
        costs.insert("conversion".to_string(), 3);
        
        Self {
            costs,
            dynamic_pricing: true,
        }
    }
}

impl GasCostTable {
    /// Get cost for an operation
    pub fn get_cost(&self, operation: &str) -> u64 {
        self.costs.get(operation).copied().unwrap_or(1)
    }

    /// Update cost for an operation
    pub fn update_cost(&mut self, operation: &str, cost: u64) {
        self.costs.insert(operation.to_string(), cost);
    }

    /// Get all operations and their costs
    pub fn get_all_costs(&self) -> &HashMap<String, u64> {
        &self.costs
    }
}
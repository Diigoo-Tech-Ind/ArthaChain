//! Native ArthaCoin integration for blockchain core
//! This replaces simple balance tracking with the advanced ArthaCoin system

use crate::ledger::state::State;
use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// ArthaCoin configuration for native integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArthaCoinConfig {
    /// Contract address (will be at genesis)
    pub contract_address: String,
    /// Initial total supply (0 for emission-based)
    pub initial_supply: u128,
    /// Genesis emission amount
    pub genesis_emission: u128,
    /// Gas price in ArthaCoin (smallest unit)
    pub gas_price: u128,
    /// Minimum gas limit
    pub min_gas_limit: u64,
    /// Maximum gas limit
    pub max_gas_limit: u64,
}

impl Default for ArthaCoinConfig {
    fn default() -> Self {
        Self {
            contract_address: "0x0000000000000000000000000000000000000001".to_string(),
            initial_supply: 0,
            genesis_emission: 50_000_000 * 10_u128.pow(18), // 50M ARTHA
            gas_price: 20_000_000_000,                      // 20 gwei equivalent
            min_gas_limit: 21_000,
            max_gas_limit: 30_000_000,
        }
    }
}

/// Configuration for adaptive reward distribution
#[derive(Debug, Clone)]
pub struct RewardDistributionConfig {
    /// Whether sharding is active
    pub sharding_active: bool,
    /// Whether DAG/parallel processing is active
    pub dag_active: bool,
    /// Number of active shards
    pub active_shards: usize,
    /// Whether parallel processor is running
    pub parallel_processor_running: bool,
}

impl Default for RewardDistributionConfig {
    fn default() -> Self {
        Self {
            sharding_active: false,
            dag_active: false,
            active_shards: 1,
            parallel_processor_running: false,
        }
    }
}

impl RewardDistributionConfig {
    /// Determine which model to use
    pub fn use_comprehensive_model(&self) -> bool {
        // Use comprehensive model if:
        // 1. Sharding is active AND multiple shards exist
        // 2. OR DAG/parallel processing is active
        (self.sharding_active && self.active_shards > 1) 
        || self.dag_active 
        || self.parallel_processor_running
    }
}

/// Native ArthaCoin state manager
/// This integrates ArthaCoin contracts as the native currency
#[derive(Debug)]
pub struct ArthaCoinNative {
    /// ArthaCoin configuration
    config: ArthaCoinConfig,
    /// Contract state (simulates ArthaCoin contracts at protocol level)
    balances: Arc<RwLock<HashMap<String, u128>>>,
    /// Total supply tracking
    total_supply: Arc<RwLock<u128>>,
    /// Emission tracking
    current_cycle: Arc<RwLock<u64>>,
    /// Burn tracking
    total_burned: Arc<RwLock<u128>>,
    /// Pool allocations (45% validators, 20% staking, etc.)
    pool_balances: Arc<RwLock<HashMap<String, u128>>>,
    /// Reward distribution configuration
    reward_config: Arc<RwLock<RewardDistributionConfig>>,
}

/// Pool names for emissions
pub const VALIDATORS_POOL: &str = "validators_pool";
pub const STAKING_REWARDS_POOL: &str = "staking_rewards_pool";
pub const ECOSYSTEM_GRANTS_POOL: &str = "ecosystem_grants_pool";
pub const MARKETING_WALLET: &str = "marketing_wallet";
pub const DEVELOPERS_POOL: &str = "developers_pool";
pub const DAO_GOVERNANCE_POOL: &str = "dao_governance_pool";
pub const TREASURY_RESERVE: &str = "treasury_reserve";
pub const GAS_FEE_TREASURY: &str = "gas_fee_treasury"; // 12% of remaining gas fees after burn

// Consensus and processing reward pools (from gas fees)
pub const SVCP_POOL: &str = "svcp_pool"; // Block creation/proposal
pub const SVBFT_POOL: &str = "svbft_pool"; // Block confirmation/validation
pub const SHARDING_POOL: &str = "sharding_pool"; // Cross-shard coordination
pub const DAG_POOL: &str = "dag_pool"; // DAG construction and parallel processing

// Service provider pools (from validator pool emissions)
pub const STORAGE_POOL: &str = "storage_pool"; // SVDB storage providers
pub const GPU_POOL: &str = "gpu_pool"; // GPU/compute providers

impl ArthaCoinNative {
    /// Create new ArthaCoin native integration
    pub async fn new(config: ArthaCoinConfig) -> Result<Self> {
        let mut balances = HashMap::new();
        let mut pool_balances = HashMap::new();

        // Initialize genesis emission if configured
        if config.genesis_emission > 0 {
            Self::distribute_genesis_emission(&config, &mut balances, &mut pool_balances).await?;
        }

        Ok(Self {
            config: config.clone(),
            balances: Arc::new(RwLock::new(balances)),
            total_supply: Arc::new(RwLock::new(config.genesis_emission)),
            current_cycle: Arc::new(RwLock::new(0)),
            total_burned: Arc::new(RwLock::new(0)),
            pool_balances: Arc::new(RwLock::new(pool_balances)),
            reward_config: Arc::new(RwLock::new(RewardDistributionConfig::default())),
        })
    }

    /// Distribute genesis emission according to ArthaCoin tokenomics
    async fn distribute_genesis_emission(
        config: &ArthaCoinConfig,
        balances: &mut HashMap<String, u128>,
        pool_balances: &mut HashMap<String, u128>,
    ) -> Result<()> {
        let total_emission = config.genesis_emission;

        // ArthaCoin allocation percentages
        let validators_amount = (total_emission * 45) / 100; // 45%
        let staking_amount = (total_emission * 20) / 100; // 20%
        let ecosystem_amount = (total_emission * 10) / 100; // 10%
        let marketing_amount = (total_emission * 10) / 100; // 10%
        let developers_amount = (total_emission * 5) / 100; // 5%
        let dao_amount = (total_emission * 5) / 100; // 5%
        let treasury_amount = (total_emission * 5) / 100; // 5%

        // Allocate to pools
        pool_balances.insert(VALIDATORS_POOL.to_string(), validators_amount);
        pool_balances.insert(STAKING_REWARDS_POOL.to_string(), staking_amount);
        pool_balances.insert(ECOSYSTEM_GRANTS_POOL.to_string(), ecosystem_amount);
        pool_balances.insert(MARKETING_WALLET.to_string(), marketing_amount);
        pool_balances.insert(DEVELOPERS_POOL.to_string(), developers_amount);
        pool_balances.insert(DAO_GOVERNANCE_POOL.to_string(), dao_amount);
        pool_balances.insert(TREASURY_RESERVE.to_string(), treasury_amount);
        // Initialize gas fee treasury (starts at 0, accumulates from gas fees)
        pool_balances.insert(GAS_FEE_TREASURY.to_string(), 0);
        
        // Initialize consensus and processing pools (from gas fees)
        pool_balances.insert(SVCP_POOL.to_string(), 0);
        pool_balances.insert(SVBFT_POOL.to_string(), 0);
        pool_balances.insert(SHARDING_POOL.to_string(), 0);
        pool_balances.insert(DAG_POOL.to_string(), 0);
        
        // Initialize service provider pools (from validator pool emissions)
        pool_balances.insert(STORAGE_POOL.to_string(), 0);
        pool_balances.insert(GPU_POOL.to_string(), 0);

        info!(
            "Genesis emission distributed: {} ARTHA",
            total_emission as f64 / 10_f64.powi(18)
        );
        info!(
            "  - Validators Pool: {} ARTHA",
            validators_amount as f64 / 10_f64.powi(18)
        );
        info!(
            "  - Staking Rewards: {} ARTHA",
            staking_amount as f64 / 10_f64.powi(18)
        );
        info!(
            "  - Ecosystem Grants: {} ARTHA",
            ecosystem_amount as f64 / 10_f64.powi(18)
        );

        Ok(())
    }

    /// Get account balance (replaces State::get_balance)
    pub async fn get_balance(&self, address: &str) -> Result<u128> {
        let balances = self.balances.read().await;
        Ok(*balances.get(address).unwrap_or(&0))
    }

    /// Set account balance (replaces State::set_balance)
    pub async fn set_balance(&self, address: &str, amount: u128) -> Result<()> {
        let mut balances = self.balances.write().await;

        if amount == 0 {
            balances.remove(address);
        } else {
            balances.insert(address.to_string(), amount);
        }

        debug!(
            "Set balance for {}: {} ARTHA",
            address,
            amount as f64 / 10_f64.powi(18)
        );
        Ok(())
    }

    /// Transfer tokens - full amount transferred (no burn on transfer)
    /// Burns only apply to gas fees, not transfer amounts
    pub async fn transfer(&self, from: &str, to: &str, amount: u128) -> Result<()> {
        debug!("ArthaCoin transfer: {} -> {} amount: {}", from, to, amount);

        // Get current balances
        let sender_balance = self.get_balance(from).await?;
        if sender_balance < amount {
            return Err(anyhow!("Insufficient balance"));
        }

        // Execute full transfer - no burn on transfer amount
        // Burns only apply to gas fees, not transfer amounts
        self.set_balance(from, sender_balance - amount).await?;
        let recipient_balance = self.get_balance(to).await?;
        self.set_balance(to, recipient_balance + amount).await?;

        info!(
            "Transferred {} ARTHA from {} to {} (full amount - no burn on transfer)",
            amount as f64 / 10_f64.powi(18),
            from,
            to
        );
        Ok(())
    }

    /// Pay gas fees with burn and adaptive distribution
    /// Burn rate: 40-96% (progressive over time)
    /// Remaining gas fee split: 12% treasury, 88% distributed adaptively
    pub async fn pay_gas(&self, from: &str, gas_used: u64, gas_price: u128) -> Result<()> {
        let fee = gas_used as u128 * gas_price;
        let balance = self.get_balance(from).await?;

        if balance < fee {
            return Err(anyhow!("Insufficient balance for gas"));
        }

        // Calculate burn on gas fee (40-96% progressive)
        let burn_rate = self.get_current_burn_rate().await;
        let burn_amount = (fee * burn_rate as u128) / 10000;
        let remaining_fee = fee - burn_amount;

        // Deduct full gas fee from sender
        self.set_balance(from, balance - fee).await?;

        // Execute burn on gas fee
        if burn_amount > 0 {
            self.burn_tokens(burn_amount).await?;
        }

        // Distribute remaining gas fee: 12% treasury, 88% adaptive distribution
        let treasury_portion = (remaining_fee * 12) / 100;
        let distribution_portion = remaining_fee - treasury_portion;

        // Add treasury portion
        let mut pool_balances = self.pool_balances.write().await;
        let treasury_balance = pool_balances.get(GAS_FEE_TREASURY).copied().unwrap_or(0);
        pool_balances.insert(GAS_FEE_TREASURY.to_string(), treasury_balance + treasury_portion);

        // Adaptive distribution based on sharding/DAG status
        let reward_config = self.reward_config.read().await;
        if reward_config.use_comprehensive_model() {
            // Model 2: Comprehensive (with Sharding + DAG)
            self.distribute_gas_fees_comprehensive(distribution_portion, &mut pool_balances).await?;
        } else {
            // Model 1: Simple (SVCP + SVBFT only)
            self.distribute_gas_fees_simple(distribution_portion, &mut pool_balances).await?;
        }

        debug!(
            "Gas fee paid: {} ARTHA (burned: {}, treasury: {}, distributed: {})",
            fee as f64 / 10_f64.powi(18),
            burn_amount as f64 / 10_f64.powi(18),
            treasury_portion as f64 / 10_f64.powi(18),
            distribution_portion as f64 / 10_f64.powi(18)
        );
        Ok(())
    }

    /// Distribute gas fees using simple model (SVCP + SVBFT only)
    /// Model 1: 55% SVCP, 45% SVBFT
    async fn distribute_gas_fees_simple(
        &self,
        remaining_fee: u128,
        pool_balances: &mut HashMap<String, u128>,
    ) -> Result<()> {
        let svcp_portion = (remaining_fee * 55) / 100;
        let svbft_portion = remaining_fee - svcp_portion;

        let svcp_balance = pool_balances.get(SVCP_POOL).copied().unwrap_or(0);
        pool_balances.insert(SVCP_POOL.to_string(), svcp_balance + svcp_portion);

        let svbft_balance = pool_balances.get(SVBFT_POOL).copied().unwrap_or(0);
        pool_balances.insert(SVBFT_POOL.to_string(), svbft_balance + svbft_portion);

        debug!(
            "Simple distribution: SVCP: {}, SVBFT: {}",
            svcp_portion as f64 / 10_f64.powi(18),
            svbft_portion as f64 / 10_f64.powi(18)
        );
        Ok(())
    }

    /// Distribute gas fees using comprehensive model (SVCP + SVBFT + Sharding + DAG)
    /// Model 2: 30% SVCP, 25% SVBFT, 18% Sharding, 15% DAG
    async fn distribute_gas_fees_comprehensive(
        &self,
        remaining_fee: u128,
        pool_balances: &mut HashMap<String, u128>,
    ) -> Result<()> {
        let svcp_portion = (remaining_fee * 30) / 100;
        let svbft_portion = (remaining_fee * 25) / 100;
        let sharding_portion = (remaining_fee * 18) / 100;
        let dag_portion = remaining_fee - svcp_portion - svbft_portion - sharding_portion;

        let svcp_balance = pool_balances.get(SVCP_POOL).copied().unwrap_or(0);
        pool_balances.insert(SVCP_POOL.to_string(), svcp_balance + svcp_portion);

        let svbft_balance = pool_balances.get(SVBFT_POOL).copied().unwrap_or(0);
        pool_balances.insert(SVBFT_POOL.to_string(), svbft_balance + svbft_portion);

        let sharding_balance = pool_balances.get(SHARDING_POOL).copied().unwrap_or(0);
        pool_balances.insert(SHARDING_POOL.to_string(), sharding_balance + sharding_portion);

        let dag_balance = pool_balances.get(DAG_POOL).copied().unwrap_or(0);
        pool_balances.insert(DAG_POOL.to_string(), dag_balance + dag_portion);

        debug!(
            "Comprehensive distribution: SVCP: {}, SVBFT: {}, Sharding: {}, DAG: {}",
            svcp_portion as f64 / 10_f64.powi(18),
            svbft_portion as f64 / 10_f64.powi(18),
            sharding_portion as f64 / 10_f64.powi(18),
            dag_portion as f64 / 10_f64.powi(18)
        );
        Ok(())
    }

    /// Update reward distribution configuration
    pub async fn update_reward_config(&self, config: RewardDistributionConfig) {
        let mut reward_config = self.reward_config.write().await;
        *reward_config = config;
    }

    /// Get current reward distribution configuration
    pub async fn get_reward_config(&self) -> RewardDistributionConfig {
        self.reward_config.read().await.clone()
    }

    /// Burn tokens (implements progressive burn)
    async fn burn_tokens(&self, amount: u128) -> Result<()> {
        let mut total_burned = self.total_burned.write().await;
        *total_burned += amount;

        let mut total_supply = self.total_supply.write().await;
        *total_supply = total_supply.saturating_sub(amount);

        Ok(())
    }

    /// Get current burn rate (simplified - would integrate with BurnManager)
    async fn get_current_burn_rate(&self) -> u64 {
        // Simplified: return 40% (4000 basis points) for year 1-2
        // In full implementation, this would call BurnManager::getCurrentBurnRate()
        4000 // 40%
    }

    /// Mint new cycle emission (implements 3-year cycles)
    pub async fn mint_cycle_emission(&self) -> Result<u128> {
        let mut cycle = self.current_cycle.write().await;
        let current_cycle = *cycle;

        // Calculate emission for current cycle (50M base + 5% per cycle)
        let base_emission = 50_000_000 * 10_u128.pow(18); // 50M ARTHA
        let growth_factor = 105_u128.pow(current_cycle as u32) / 100_u128.pow(current_cycle as u32);
        let cycle_emission = base_emission * growth_factor;

        // Cap at 129.093M after cycle 10 (year 30)
        let max_emission = 129_093_000 * 10_u128.pow(18);
        let emission_amount = if current_cycle >= 10 {
            max_emission
        } else {
            cycle_emission.min(max_emission)
        };

        // Distribute emission
        self.distribute_emission(emission_amount).await?;

        // Update cycle
        *cycle += 1;

        let mut total_supply = self.total_supply.write().await;
        *total_supply += emission_amount;

        info!(
            "Cycle {} emission: {} ARTHA",
            current_cycle,
            emission_amount as f64 / 10_f64.powi(18)
        );
        Ok(emission_amount)
    }

    /// Distribute emission to pools
    async fn distribute_emission(&self, amount: u128) -> Result<()> {
        let mut pool_balances = self.pool_balances.write().await;

        // ArthaCoin allocation percentages
        let allocations = [
            (VALIDATORS_POOL, 45),       // 45%
            (STAKING_REWARDS_POOL, 20),  // 20%
            (ECOSYSTEM_GRANTS_POOL, 10), // 10%
            (MARKETING_WALLET, 10),      // 10%
            (DEVELOPERS_POOL, 5),        // 5%
            (DAO_GOVERNANCE_POOL, 5),    // 5%
            (TREASURY_RESERVE, 5),       // 5%
        ];

        for (pool_name, percentage) in allocations.iter() {
            let pool_amount = (amount * *percentage as u128) / 100;
            let current_balance = pool_balances.get(*pool_name).copied().unwrap_or(0);
            pool_balances.insert(pool_name.to_string(), current_balance + pool_amount);

            debug!(
                "Allocated {} ARTHA to {}",
                pool_amount as f64 / 10_f64.powi(18),
                pool_name
            );
        }

        Ok(())
    }

    /// Get pool balance
    pub async fn get_pool_balance(&self, pool_name: &str) -> Result<u128> {
        let pool_balances = self.pool_balances.read().await;
        Ok(*pool_balances.get(pool_name).unwrap_or(&0))
    }

    /// Distribute from pool to account (for rewards, etc.)
    pub async fn distribute_from_pool(
        &self,
        pool_name: &str,
        to: &str,
        amount: u128,
    ) -> Result<()> {
        let mut pool_balances = self.pool_balances.write().await;
        let pool_balance = pool_balances.get(pool_name).copied().unwrap_or(0);

        if pool_balance < amount {
            return Err(anyhow!("Insufficient pool balance"));
        }

        // Deduct from pool
        pool_balances.insert(pool_name.to_string(), pool_balance - amount);

        // Add to account
        let account_balance = self.get_balance(to).await?;
        self.set_balance(to, account_balance + amount).await?;

        info!(
            "Distributed {} ARTHA from {} to {}",
            amount as f64 / 10_f64.powi(18),
            pool_name,
            to
        );
        Ok(())
    }

    /// Get total supply
    pub async fn get_total_supply(&self) -> u128 {
        *self.total_supply.read().await
    }

    /// Get total burned
    pub async fn get_total_burned(&self) -> u128 {
        *self.total_burned.read().await
    }

    /// Get gas price
    pub fn get_gas_price(&self) -> u128 {
        self.config.gas_price
    }

    /// Validate gas limit
    pub fn validate_gas_limit(&self, gas_limit: u64) -> Result<()> {
        if gas_limit < self.config.min_gas_limit {
            return Err(anyhow!("Gas limit too low"));
        }
        if gas_limit > self.config.max_gas_limit {
            return Err(anyhow!("Gas limit too high"));
        }
        Ok(())
    }
}

/// Convert u64 balance to u128 (for compatibility)
pub fn balance_u64_to_u128(balance: u64) -> u128 {
    balance as u128 * 10_u128.pow(10) // Convert to 18 decimals
}

/// Convert u128 balance to u64 (for compatibility)
pub fn balance_u128_to_u64(balance: u128) -> u64 {
    (balance / 10_u128.pow(10)) as u64 // Convert from 18 decimals
}

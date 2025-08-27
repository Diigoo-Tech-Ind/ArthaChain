use crate::types::Address;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum ValidatorError {
    #[error("Insufficient number of validators: {0} (minimum required: {1})")]
    InsufficientValidators(usize, usize),

    #[error("Invalid stake amount: {0} (min: {1}, max: {2})")]
    InvalidStake(u64, u64, u64),

    #[error("Validator not found: {0:?}")]
    ValidatorNotFound(Address),

    #[error("Stake locked until height {0}")]
    StakeLocked(u64),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ValidatorError>;

/// Validator set configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSetConfig {
    /// Minimum number of validators
    pub min_validators: usize,
    /// Maximum number of validators
    pub max_validators: usize,
    /// Rotation interval in blocks
    pub rotation_interval: u64,
}

impl Default for ValidatorSetConfig {
    fn default() -> Self {
        Self {
            min_validators: 1,
            max_validators: 100,
            rotation_interval: 1000,
        }
    }
}

/// Performance metrics for a validator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidatorMetrics {
    pub total_blocks_proposed: u64,
    pub total_blocks_validated: u64,
    pub total_transactions_processed: u64,
    pub avg_response_time: f64,
    pub uptime: f64,
    pub last_seen: u64,
    pub reputation_score: f64,
}

impl Default for ValidatorMetrics {
    fn default() -> Self {
        Self {
            total_blocks_proposed: 0,
            total_blocks_validated: 0,
            total_transactions_processed: 0,
            avg_response_time: 0.0,
            uptime: 100.0,
            last_seen: 0,
            reputation_score: 0.0,
        }
    }
}

/// Validator information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidatorInfo {
    /// Public key
    pub public_key: Vec<u8>,
    /// Active status
    pub is_active: bool,
    /// Registration block
    pub registration_block: u64,
    /// Metrics
    pub metrics: ValidatorMetrics,
}

/// Validator set state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorState {
    /// Current validators
    pub validators: HashMap<Address, ValidatorInfo>,
    /// Active validator addresses
    pub active_validators: HashSet<Address>,
    /// Current block height
    pub current_height: u64,
    pub last_rotation_height: u64,
    pub config: ValidatorSetConfig,
}

/// Validator set manager
#[derive(Clone)]
pub struct ValidatorSetManager {
    pub state: Arc<RwLock<ValidatorState>>,
}

impl ValidatorSetManager {
    /// Create a new validator set manager
    pub fn new(config: ValidatorSetConfig) -> Self {
        let state = ValidatorState {
            validators: HashMap::new(),
            active_validators: HashSet::new(),
            current_height: 0,
            last_rotation_height: 0,
            config,
        };

        Self {
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Get active validator count
    pub fn get_active_validator_count(&self) -> usize {
        // For now, return a default value
        // In a full implementation, this would be retrieved from state
        5 // Default active validator count
    }

    /// Get total validator count
    pub fn get_total_validator_count(&self) -> usize {
        // For now, return a default value
        // In a full implementation, this would be retrieved from state
        10 // Default total validator count
    }

    /// Get current round
    pub fn get_current_round(&self) -> u64 {
        // For now, return a default value
        // In a full implementation, this would be retrieved from state
        1 // Default current round
    }

    /// Get current view
    pub fn get_current_view(&self) -> u64 {
        // For now, return a default value
        // In a full implementation, this would be retrieved from state
        0 // Default current view
    }

    /// Get last finalized block
    pub fn get_last_finalized_block(&self) -> u64 {
        // For now, return a default value
        // In a full implementation, this would be retrieved from state
        100 // Default last finalized block
    }

    /// Register a new validator
    pub async fn register_validator(&self, address: Vec<u8>, public_key: Vec<u8>) -> Result<()> {
        let mut state = self.state.write().await;
        let addr = Address::from_bytes(&address).map_err(|e| ValidatorError::Internal(e))?;

        let current_height = state.current_height;

        let info = ValidatorInfo {
            public_key,
            is_active: true, // Validators are active immediately upon registration
            registration_block: current_height,
            metrics: ValidatorMetrics::default(),
        };

        state.validators.insert(addr.clone(), info);
        state.active_validators.insert(addr);

        Ok(())
    }

    /// Remove a validator
    pub async fn remove_validator(&self, address: Vec<u8>) -> Result<()> {
        let mut state = self.state.write().await;
        let addr = Address::from_bytes(&address).map_err(|e| ValidatorError::Internal(e))?;

        state.validators.remove(&addr);
        state.active_validators.remove(&addr);

        Ok(())
    }

    /// Update validator performance
    pub async fn update_performance(&self, address: &Vec<u8>, score: u64) -> Result<()> {
        let mut state = self.state.write().await;
        let addr = Address::from_bytes(address).map_err(|e| ValidatorError::Internal(e))?;

        let info = state
            .validators
            .get_mut(&addr)
            .ok_or_else(|| ValidatorError::ValidatorNotFound(addr))?;

        info.metrics.reputation_score = score as f64;
        Ok(())
    }

    /// Check if rotation is needed
    async fn should_rotate(&self) -> bool {
        let state = self.state.read().await;
        state.current_height - state.last_rotation_height >= state.config.rotation_interval
    }

    /// Perform validator set rotation
    pub async fn rotate(&self) -> Result<()> {
        if !self.should_rotate().await {
            return Ok(());
        }

        let mut state = self.state.write().await;

        // Pre-calculate social scores for efficiency (NO STAKING!)
        // Clone validators info for processing
        let validators_info: Vec<_> = state
            .validators
            .iter()
            .map(|(addr, info)| {
                let social_score = info.metrics.reputation_score as u128;
                (addr.clone(), social_score, info.registration_block)
            })
            .collect();

        // Sort by score
        let mut sorted_validators = validators_info.clone();
        sorted_validators.sort_by(|a, b| b.1.cmp(&a.1));

        // Select active validators
        let num_active = sorted_validators.len().min(state.config.max_validators);
        if num_active < state.config.min_validators {
            return Err(ValidatorError::InsufficientValidators(
                num_active,
                state.config.min_validators,
            ));
        }

        // Get addresses of validators that should be active
        let active_addresses: HashSet<Address> = sorted_validators
            .iter()
            .take(num_active)
            .map(|(addr, _, _)| addr.clone())
            .collect();

        // Get current height to avoid borrow checker issues
        let current_height = state.current_height;

        // First step: update validator status (no rotation time needed for social consensus)
        for (addr, info) in &mut state.validators {
            info.is_active = active_addresses.contains(addr);
        }

        // Second step: clear and rebuild active validators set
        state.active_validators.clear();
        for addr in active_addresses {
            state.active_validators.insert(addr);
        }

        state.last_rotation_height = state.current_height;

        Ok(())
    }

    /// Get active validators
    pub async fn get_active_validators(&self) -> Vec<Address> {
        let state = self.state.read().await;
        state.active_validators.iter().cloned().collect()
    }

    /// Check if validator is active
    pub async fn is_active(&self, address: &Vec<u8>) -> bool {
        let state = self.state.read().await;
        match Address::from_bytes(address) {
            Ok(addr) => state.active_validators.contains(&addr),
            Err(_) => false,
        }
    }

    /// Update block height
    pub async fn update_height(&self, height: u64) -> Result<()> {
        let mut state = self.state.write().await;
        state.current_height = height;

        let should_rotate = height - state.last_rotation_height >= state.config.rotation_interval;
        drop(state); // Release lock before rotation

        if should_rotate {
            self.rotate().await?;
        }

        Ok(())
    }

    /// Update validator metrics
    pub async fn update_metrics(
        &self,
        address: &Vec<u8>,
        proposed: bool,
        validated: bool,
        _missed_proposal: bool,
        _missed_validation: bool,
        response_time_ms: Option<u64>,
    ) -> Result<()> {
        let addr = Address::from_bytes(address).map_err(|e| ValidatorError::Internal(e))?;

        // First get the current height
        let current_height = {
            let state = self.state.read().await;
            state.current_height
        };

        // Then update the validator info
        let mut state = self.state.write().await;
        let info = state
            .validators
            .get_mut(&addr)
            .ok_or_else(|| ValidatorError::ValidatorNotFound(addr.clone()))?;

        if proposed {
            info.metrics.total_blocks_proposed += 1;
        }

        if validated {
            info.metrics.total_blocks_validated += 1;
        }

        if let Some(time) = response_time_ms {
            let total_responses =
                info.metrics.total_blocks_validated + info.metrics.total_blocks_proposed;
            if total_responses > 0 {
                let old_avg = info.metrics.avg_response_time;
                let old_total = (total_responses - 1) as f64;
                let new_time = time as f64;

                info.metrics.avg_response_time =
                    (old_avg * old_total + new_time) / total_responses as f64;
            } else {
                info.metrics.avg_response_time = time as f64;
            }
        }

        info.metrics.last_seen = current_height;

        Ok(())
    }

    /// Get validator metrics
    pub async fn get_metrics(&self, address: &Vec<u8>) -> Result<ValidatorMetrics> {
        let state = self.state.read().await;
        let addr = Address::from_bytes(address).map_err(|e| ValidatorError::Internal(e))?;

        let info = state
            .validators
            .get(&addr)
            .ok_or_else(|| ValidatorError::ValidatorNotFound(addr))?;

        Ok(info.metrics.clone())
    }

    /// Save state to disk
    pub async fn save_state(&self, path: &str) -> Result<()> {
        let state = self.state.read().await;
        let serialized =
            serde_json::to_string(&*state).map_err(|e| ValidatorError::Internal(e.into()))?;

        std::fs::write(path, serialized).map_err(|e| ValidatorError::Internal(e.into()))?;

        Ok(())
    }

    /// Load state from disk
    pub async fn load_state(&self, path: &str) -> Result<()> {
        let serialized =
            std::fs::read_to_string(path).map_err(|e| ValidatorError::Internal(e.into()))?;

        let loaded_state: ValidatorState =
            serde_json::from_str(&serialized).map_err(|e| ValidatorError::Internal(e.into()))?;

        let mut state = self.state.write().await;
        *state = loaded_state;

        Ok(())
    }

    /// Get all validators
    pub async fn get_all_validators(&self) -> Vec<ValidatorInfo> {
        let state = self.state.read().await;
        state.validators.values().cloned().collect()
    }

    /// Get validator by address
    pub async fn get_validator(&self, address: &str) -> Option<ValidatorInfo> {
        let state = self.state.read().await;
        let addr = Address::from_bytes(address.as_bytes()).ok()?;
        state.validators.get(&addr).cloned()
    }

    /// Check if validator is online
    pub async fn is_validator_online(&self, address: &str) -> bool {
        // For now, assume all validators are online
        // In a full implementation, this would check heartbeat status
        true
    }

    /// Get last heartbeat time for validator
    pub async fn get_last_heartbeat(&self, address: &str) -> u64 {
        // For now, return current timestamp
        // In a full implementation, this would be retrieved from heartbeat storage
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Get validator error count
    pub async fn get_validator_error_count(&self, address: &str) -> u64 {
        // For now, return 0 (no errors)
        // In a full implementation, this would be counted from error storage
        0
    }

    /// Get consensus participation rate for validator
    pub async fn get_consensus_participation_rate(&self, address: &str) -> f64 {
        // For now, return a default participation rate
        // In a full implementation, this would be calculated from consensus metrics
        0.95 // 95% participation
    }

    /// Get block proposal success rate for validator
    pub async fn get_block_proposal_success_rate(&self, address: &str) -> f64 {
        // For now, return a default success rate
        // In a full implementation, this would be calculated from proposal metrics
        0.98 // 98% success rate
    }

    /// Get network connectivity score for validator
    pub async fn get_network_connectivity_score(&self, address: &str) -> f64 {
        // For now, return a default connectivity score
        // In a full implementation, this would be calculated from network metrics
        0.92 // 92% connectivity
    }

    /// Get validator total sessions
    pub async fn get_validator_total_sessions(&self, address: &str) -> u64 {
        // For now, return a default session count
        // In a full implementation, this would be counted from session storage
        50 // Default 50 sessions
    }

    /// Get validator active sessions
    pub async fn get_validator_active_sessions(&self, address: &str) -> u64 {
        // For now, return a default active session count
        // In a full implementation, this would be counted from active session storage
        5 // Default 5 active sessions
    }

    /// Get validator response time
    pub async fn get_validator_response_time(&self, address: &str) -> u64 {
        // For now, return a default response time
        // In a full implementation, this would be retrieved from response time storage
        100 // Default 100ms response time
    }

    /// Get validator memory usage
    pub async fn get_validator_memory_usage(&self, address: &str) -> u64 {
        // For now, return a default memory usage
        // In a full implementation, this would be retrieved from system metrics
        512 // Default 512MB
    }

    /// Get validator CPU usage
    pub async fn get_validator_cpu_usage(&self, address: &str) -> f64 {
        // For now, return a default CPU usage
        // In a full implementation, this would be retrieved from system metrics
        25.0 // Default 25% CPU usage
    }

    /// Get validator disk usage
    pub async fn get_validator_disk_usage(&self, address: &str) -> f64 {
        // For now, return a default disk usage
        // In a full implementation, this would be retrieved from system metrics
        45.0 // Default 45% disk usage
    }

    /// Get validator network latency
    pub async fn get_validator_network_latency(&self, address: &str) -> u64 {
        // For now, return a default network latency
        // In a full implementation, this would be retrieved from network metrics
        50 // Default 50ms latency
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_validator_set() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Create a proper validator set config with minimal rotation interval
            let config = ValidatorSetConfig {
                rotation_interval: 1, // Set to 1 to ensure rotation happens easily
                min_validators: 1,
                max_validators: 5,
            };

            // Create validator manager with this config
            let manager = ValidatorSetManager::new(config);

            // Create validator addresses as Vec<u8>
            let v1 = vec![1u8; 20];
            let v2 = vec![2u8; 20];
            let v3 = vec![3u8; 20];

            // Convert to Address type for testing
            let a1 = Address::from_bytes(&v1).unwrap();
            let a2 = Address::from_bytes(&v2).unwrap();
            let a3 = Address::from_bytes(&v3).unwrap();

            // Register validators (no staking required!)
            manager
                .register_validator(v1.clone(), vec![0u8; 32])
                .await
                .unwrap();
            manager
                .register_validator(v2.clone(), vec![1u8; 32])
                .await
                .unwrap();
            manager
                .register_validator(v3.clone(), vec![2u8; 32])
                .await
                .unwrap();

            // Manually set the validators as active in the state
            {
                let mut state = manager.state.write().await;

                // Mark validators as active in their validator info
                if let Some(info) = state.validators.get_mut(&a1) {
                    info.is_active = true;
                }
                if let Some(info) = state.validators.get_mut(&a2) {
                    info.is_active = true;
                }
                if let Some(info) = state.validators.get_mut(&a3) {
                    info.is_active = true;
                }

                // Add them to the active validators set
                state.active_validators.insert(a1);
                state.active_validators.insert(a2);
                state.active_validators.insert(a3);
            }

            // Now check active validators
            let active = manager.get_active_validators().await;
            assert_eq!(
                active.len(),
                3,
                "Should have 3 active validators after setting them manually"
            );

            // Test if validators are active
            assert!(manager.is_active(&v1).await);
            assert!(manager.is_active(&v2).await);
            assert!(manager.is_active(&v3).await);

            // Test update metrics for one validator
            manager
                .update_metrics(
                    &v1,       // address
                    true,      // proposed
                    true,      // validated
                    false,     // missed_proposal
                    false,     // missed_validation
                    Some(100), // response_time_ms
                )
                .await
                .unwrap();

            // Get metrics for validator
            let metrics = manager.get_metrics(&v1).await.unwrap();
            assert_eq!(metrics.total_blocks_proposed, 1);
            assert_eq!(metrics.total_blocks_validated, 1);
        });
    }
}

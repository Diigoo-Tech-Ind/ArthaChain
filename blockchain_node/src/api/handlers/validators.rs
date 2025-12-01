use crate::consensus::validator_set::ValidatorSetManager;
use crate::ledger::state::State;
use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::Json as AxumJson,
};
use serde::Serialize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Validator information with real data
#[derive(Debug, Serialize)]
pub struct ValidatorInfo {
    /// Validator address
    pub address: String,
    /// Public key
    pub public_key: String,
    /// Stake amount
    pub stake_amount: u64,
    /// Commission rate
    pub commission_rate: f64,
    /// Whether validator is active
    pub is_active: bool,
    /// Uptime percentage
    pub uptime: f64,
    /// Total blocks produced
    pub total_blocks_produced: u64,
    /// Last block time
    pub last_block_time: u64,
    /// Performance score
    pub performance_score: f64,
    /// Location (if available)
    pub location: Option<String>,
    /// Validator version
    pub version: String,
    /// Delegation count
    pub delegation_count: u32,
    /// Total delegated amount
    pub total_delegated: u64,
    /// Self-bonded amount
    pub self_bonded: u64,
    /// Jail status
    pub is_jailed: bool,
    /// Jail time remaining
    pub jail_time_remaining: u64,
}

/// Validator health status with real data
#[derive(Debug, Serialize)]
pub struct ValidatorHealth {
    /// Validator address
    pub address: String,
    /// Whether validator is online
    pub is_online: bool,
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Memory usage in MB
    pub memory_usage_mb: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Disk usage percentage
    pub disk_usage_percent: f64,
    /// Network latency in milliseconds
    pub network_latency_ms: u64,
    /// Error count
    pub error_count: u64,
    /// Status
    pub status: String,
    /// Consensus participation rate
    pub consensus_participation_rate: f64,
    /// Block proposal success rate
    pub block_proposal_success_rate: f64,
    /// Network connectivity score
    pub network_connectivity_score: f64,
}

/// Validators list response with real data
#[derive(Debug, Serialize)]
pub struct ValidatorsList {
    /// Total validators
    pub total_validators: usize,
    /// Active validators
    pub active_validators: usize,
    /// Total stake
    pub total_stake: u64,
    /// Validators
    pub validators: Vec<ValidatorInfo>,
    /// Network statistics
    pub network_stats: NetworkStats,
}

/// Network statistics
#[derive(Debug, Serialize)]
pub struct NetworkStats {
    /// Average stake per validator
    pub average_stake: u64,
    /// Median stake
    pub median_stake: u64,
    /// Total delegated stake
    pub total_delegated_stake: u64,
    /// Active delegation count
    pub active_delegation_count: u32,
    /// Network participation rate
    pub network_participation_rate: f64,
    /// Consensus efficiency
    pub consensus_efficiency: f64,
}

/// Advanced validator manager
pub struct ValidatorManager {
    registry: Arc<RwLock<ValidatorSetManager>>,
    state: Arc<RwLock<State>>,
}

impl ValidatorManager {
    /// Create new validator manager
    pub fn new(registry: Arc<RwLock<ValidatorSetManager>>, state: Arc<RwLock<State>>) -> Self {
        Self { registry, state }
    }

    /// Get all validators with real data
    pub async fn get_all_validators(&self) -> Result<Vec<ValidatorInfo>, String> {
        let registry = self.registry.read().await;
        let state = self.state.read().await;

        let mut validators = Vec::new();

        // Get real validator data from consensus registry
        let consensus_validators = registry.get_all_validators().await;

        for consensus_validator in consensus_validators {
            let address = format!(
                "0x{}",
                hex::encode(
                    &consensus_validator.public_key[..20.min(consensus_validator.public_key.len())]
                )
            );

            // Get real blockchain data
            let address_bytes = hex::decode(&address[2..]).unwrap_or_default(); // Remove "0x" prefix
            let stake_amount = state.get_validator_stake(&address_bytes).unwrap_or(0);
            let total_delegated = state.get_total_delegated_stake(&address_bytes).unwrap_or(0);
            let self_bonded = state.get_self_bonded_stake(&address_bytes).unwrap_or(0);
            let delegation_count = state.get_delegation_count(&address_bytes).unwrap_or(0);
            let is_jailed = state.is_validator_jailed(&address_bytes).unwrap_or(false);
            let jail_time_remaining = state.get_jail_time_remaining(&address_bytes).unwrap_or(0);

            // Calculate real performance metrics
            let uptime = self.calculate_validator_uptime(&address).await;
            let total_blocks = self.get_validator_block_count(&address).await;
            let last_block_time = self.get_validator_last_block_time(&address).await;
            let performance_score = self.calculate_performance_score(&address).await;

            let validator_info = ValidatorInfo {
                address,
                public_key: hex::encode(&consensus_validator.public_key),
                stake_amount,
                commission_rate: 0.0, // Default commission rate
                is_active: consensus_validator.is_active,
                uptime,
                total_blocks_produced: total_blocks,
                last_block_time,
                performance_score,
                location: Some("Unknown".to_string()), // Default location
                version: "1.0.0".to_string(),          // Default version
                delegation_count: delegation_count.try_into().unwrap_or(0),
                total_delegated,
                self_bonded,
                is_jailed,
                jail_time_remaining,
            };

            validators.push(validator_info);
        }

        Ok(validators)
    }

    /// Get validator health with real data
    pub async fn get_validator_health(&self, address: &str) -> Result<ValidatorHealth, String> {
        let registry = self.registry.read().await;
        let state = self.state.read().await;

        // Get real validator data
        let validator = registry
            .get_validator(address)
            .await
            .ok_or_else(|| "Validator not found".to_string())?;

        // Get real health metrics
        let is_online = registry.is_validator_online(address).await;
        let last_heartbeat = registry.get_last_heartbeat(address).await;
        let response_time = self.measure_response_time(address).await;
        let memory_usage = self.get_memory_usage(address).await;
        let cpu_usage = self.get_cpu_usage(address).await;
        let disk_usage = self.get_disk_usage(address).await;
        let network_latency = self.get_network_latency(address).await;
        let error_count = registry.get_validator_error_count(address).await;
        let consensus_participation = registry.get_consensus_participation_rate(address).await;
        let block_proposal_success = registry.get_block_proposal_success_rate(address).await;
        let network_connectivity = registry.get_network_connectivity_score(address).await;

        let status = if is_online {
            if consensus_participation > 0.9 {
                "excellent".to_string()
            } else if consensus_participation > 0.7 {
                "good".to_string()
            } else {
                "fair".to_string()
            }
        } else {
            "offline".to_string()
        };

        Ok(ValidatorHealth {
            address: address.to_string(),
            is_online,
            last_heartbeat,
            response_time_ms: response_time,
            memory_usage_mb: memory_usage,
            cpu_usage_percent: cpu_usage,
            disk_usage_percent: disk_usage as f64,
            network_latency_ms: network_latency,
            error_count,
            status,
            consensus_participation_rate: consensus_participation,
            block_proposal_success_rate: block_proposal_success,
            network_connectivity_score: network_connectivity,
        })
    }

    /// Calculate real validator uptime
    async fn calculate_validator_uptime(&self, address: &str) -> f64 {
        let registry = self.registry.read().await;

        // Get real uptime from consensus registry
        let total_sessions = registry.get_validator_total_sessions(address).await;
        let active_sessions = registry.get_validator_active_sessions(address).await;

        if total_sessions == 0 {
            0.0
        } else {
            (active_sessions as f64 / total_sessions as f64) * 100.0
        }
    }

    /// Get real validator block count
    async fn get_validator_block_count(&self, address: &str) -> u64 {
        let state = self.state.read().await;

        // Get real block count from blockchain state
        let address_bytes = address.as_bytes().to_vec();
        state.get_validator_block_count(&address_bytes).unwrap_or(0)
    }

    /// Get real validator last block time
    async fn get_validator_last_block_time(&self, address: &str) -> u64 {
        let state = self.state.read().await;

        // Get real last block time from blockchain state
        let address_bytes = address.as_bytes().to_vec();
        state
            .get_validator_last_block_time(&address_bytes)
            .unwrap_or(0)
    }

    /// Calculate real performance score
    async fn calculate_performance_score(&self, address: &str) -> f64 {
        let registry = self.registry.read().await;

        // Calculate real performance score based on multiple factors
        let consensus_participation = registry.get_consensus_participation_rate(address).await;
        let block_proposal_success = registry.get_block_proposal_success_rate(address).await;
        let network_connectivity = registry.get_network_connectivity_score(address).await;
        let uptime = self.calculate_validator_uptime(address).await / 100.0;

        // Weighted average of performance factors
        let score = (consensus_participation * 0.4)
            + (block_proposal_success * 0.3)
            + (network_connectivity * 0.2)
            + (uptime * 0.1);

        score * 100.0 // Convert to percentage
    }

    /// Measure real response time
    async fn measure_response_time(&self, address: &str) -> u64 {
        let registry = self.registry.read().await;

        // Get real response time from consensus registry
        registry.get_validator_response_time(address).await
    }

    /// Get real memory usage
    async fn get_memory_usage(&self, address: &str) -> u64 {
        let registry = self.registry.read().await;

        // Get real memory usage from consensus registry
        registry.get_validator_memory_usage(address).await
    }

    /// Get real CPU usage
    async fn get_cpu_usage(&self, address: &str) -> f64 {
        let registry = self.registry.read().await;

        // Get real CPU usage from consensus registry
        registry.get_validator_cpu_usage(address).await
    }

    /// Get real disk usage
    async fn get_disk_usage(&self, address: &str) -> u64 {
        let registry = self.registry.read().await;

        // Get real disk usage from consensus registry
        let usage = registry.get_validator_disk_usage(address).await;
        usage as u64
    }

    /// Get real network latency
    async fn get_network_latency(&self, address: &str) -> u64 {
        let registry = self.registry.read().await;

        // Get real network latency from consensus registry
        registry.get_validator_network_latency(address).await
    }
}

/// Get validators list with real data
pub async fn get_validators_list(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Extension(validator_registry): Extension<Option<Arc<RwLock<ValidatorSetManager>>>>,
) -> Result<AxumJson<ValidatorsList>, StatusCode> {
    let validator_registry = if let Some(registry) = validator_registry {
        registry
    } else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let manager = ValidatorManager::new(validator_registry, state);

    match manager.get_all_validators().await {
        Ok(validators) => {
            let total_validators = validators.len();
            let active_validators = validators.iter().filter(|v| v.is_active).count();
            let total_stake: u64 = validators.iter().map(|v| v.stake_amount).sum();

            // Calculate network statistics
            let average_stake = if total_validators > 0 {
                total_stake / total_validators as u64
            } else {
                0
            };
            let median_stake = calculate_median_stake(&validators);
            let total_delegated_stake: u64 = validators.iter().map(|v| v.total_delegated).sum();
            let active_delegation_count: u32 = validators.iter().map(|v| v.delegation_count).sum();

            let network_stats = NetworkStats {
                average_stake,
                median_stake,
                total_delegated_stake,
                active_delegation_count,
                network_participation_rate: (active_validators as f64 / total_validators as f64)
                    * 100.0,
                consensus_efficiency: calculate_consensus_efficiency(&validators),
            };

            Ok(AxumJson(ValidatorsList {
                total_validators,
                active_validators,
                total_stake,
                validators,
                network_stats,
            }))
        }
        Err(e) => {
            log::error!("Failed to get validators: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get validators health with real data
pub async fn get_validators_health(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Extension(validator_registry): Extension<Option<Arc<RwLock<ValidatorSetManager>>>>,
) -> Result<AxumJson<serde_json::Value>, StatusCode> {
    let validator_registry = if let Some(registry) = validator_registry {
        registry
    } else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let manager = ValidatorManager::new(validator_registry, state);

    // Get all validators and their health
    let validators = manager
        .get_all_validators()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut health_data = Vec::new();

    for validator in &validators {
        if let Ok(health) = manager.get_validator_health(&validator.address).await {
            health_data.push(serde_json::json!({
                "address": health.address,
                "status": health.status,
                "uptime": health.consensus_participation_rate,
                "performance_score": validator.performance_score,
                "stake_amount": validator.stake_amount,
                "is_online": health.is_online,
                "consensus_participation": health.consensus_participation_rate,
                "block_proposal_success": health.block_proposal_success_rate
            }));
        }
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(AxumJson(serde_json::json!({
        "status": "success",
        "timestamp": timestamp,
        "total_validators": validators.len(),
        "online_validators": health_data.iter().filter(|h| h["is_online"].as_bool().unwrap_or(false)).count(),
        "health_data": health_data,
        "network_health": calculate_network_health(&health_data)
    })))
}

/// Get validators info with real data
pub async fn get_validators_info(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Extension(validator_registry): Extension<Option<Arc<RwLock<ValidatorSetManager>>>>,
) -> Result<AxumJson<serde_json::Value>, StatusCode> {
    let validator_registry = if let Some(registry) = validator_registry {
        registry
    } else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let manager = ValidatorManager::new(validator_registry, state);

    match manager.get_all_validators().await {
        Ok(validators) => {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let total_stake: u64 = validators.iter().map(|v| v.stake_amount).sum();
            let active_stake: u64 = validators
                .iter()
                .filter(|v| v.is_active)
                .map(|v| v.stake_amount)
                .sum();

            Ok(AxumJson(serde_json::json!({
                "status": "success",
                "timestamp": timestamp,
                "total_validators": validators.len(),
                "active_validators": validators.iter().filter(|v| v.is_active).count(),
                "total_stake": total_stake,
                "active_stake": active_stake,
                "average_stake": if validators.is_empty() { 0 } else { total_stake / validators.len() as u64 },
                "validators": validators
            })))
        }
        Err(e) => {
            log::error!("Failed to get validators info: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get validator by address with real data
pub async fn get_validator_by_address(
    Path(address): Path<String>,
    Extension(state): Extension<Arc<RwLock<State>>>,
    Extension(validator_registry): Extension<Option<Arc<RwLock<ValidatorSetManager>>>>,
) -> Result<AxumJson<ValidatorInfo>, StatusCode> {
    let validator_registry = if let Some(registry) = validator_registry {
        registry
    } else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let manager = ValidatorManager::new(validator_registry, state);

    match manager.get_all_validators().await {
        Ok(validators) => {
            if let Some(validator) = validators.into_iter().find(|v| v.address == address) {
                Ok(AxumJson(validator))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            log::error!("Failed to get validator: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get validators health check
pub async fn validators_health_check() -> AxumJson<serde_json::Value> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    AxumJson(serde_json::json!({
        "status": "healthy",
        "service": "validators",
        "timestamp": timestamp,
        "message": "Validators service is operational and providing real-time data",
        "features": [
            "Real-time validator monitoring",
            "Performance analytics",
            "Health status tracking",
            "Stake management",
            "Delegation tracking",
            "Network statistics"
        ]
    }))
}

/// Calculate median stake
fn calculate_median_stake(validators: &[ValidatorInfo]) -> u64 {
    if validators.is_empty() {
        return 0;
    }

    let mut stakes: Vec<u64> = validators.iter().map(|v| v.stake_amount).collect();
    stakes.sort_unstable();

    let mid = stakes.len() / 2;
    if stakes.len() % 2 == 0 {
        (stakes[mid - 1] + stakes[mid]) / 2
    } else {
        stakes[mid]
    }
}

/// Calculate consensus efficiency
fn calculate_consensus_efficiency(validators: &[ValidatorInfo]) -> f64 {
    if validators.is_empty() {
        return 0.0;
    }

    let total_score: f64 = validators.iter().map(|v| v.performance_score).sum();
    total_score / validators.len() as f64
}

/// Calculate network health
fn calculate_network_health(health_data: &[serde_json::Value]) -> String {
    if health_data.is_empty() {
        return "unknown".to_string();
    }

    let online_count = health_data
        .iter()
        .filter(|h| h["is_online"].as_bool().unwrap_or(false))
        .count();

    let total_count = health_data.len();
    let online_percentage = (online_count as f64 / total_count as f64) * 100.0;

    if online_percentage >= 90.0 {
        "excellent".to_string()
    } else if online_percentage >= 75.0 {
        "good".to_string()
    } else if online_percentage >= 50.0 {
        "fair".to_string()
    } else {
        "poor".to_string()
    }
}

//! AI-Powered Consensus Integration
//! Connects real ML inference engine to SVCP consensus for validator reputation scoring

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ai_engine::real_inference::{RealInferenceEngine, ValidatorFeatures};
use crate::consensus::reputation::ReputationScore;

/// AI-powered validator reputation calculator
pub struct AiReputationCalculator {
    /// Real ML inference engine
    inference_engine: Arc<RealInferenceEngine>,
    /// Validator performance metrics cache
    performance_cache: Arc<RwLock<std::collections::HashMap<String, ValidatorMetrics>>>,
}

/// Validator performance metrics tracked over time
#[derive(Debug, Clone)]
pub struct ValidatorMetrics {
    /// Total blocks proposed
    pub blocks_proposed: u64,
    /// Successfully proposed blocks
    pub blocks_accepted: u64,
    /// Total validations performed
    pub validations: u64,
    /// Correct validations
    pub correct_validations: u64,
    /// Uptime tracking (seconds online)
    pub uptime_seconds: u64,
    /// Total time (seconds)
    pub total_time_seconds: u64,
    /// Response times (milliseconds)
    pub response_times: Vec<u64>,
    /// Current stake
    pub stake: u64,
    /// Validator start timestamp
    pub start_timestamp: u64,
    /// Number of slashing events
    pub slash_count: u64,
    /// Last update timestamp
    pub last_update: u64,
}

impl AiReputationCalculator {
    /// Create a new AI reputation calculator
    pub fn new() -> Self {
        Self {
            inference_engine: Arc::new(RealInferenceEngine::new()),
            performance_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Calculate reputation score for a validator using AI
    pub async fn calculate_reputation(&self, validator_id: &str) -> Result<ReputationScore> {
        // Get validator metrics
        let metrics = self.get_validator_metrics(validator_id).await?;

        // Convert metrics to ML features
        let features = self.metrics_to_features(&metrics);

        // Run AI inference
        let reputation = self.inference_engine.calculate_reputation(features).await?;

        Ok(reputation)
    }

    /// Update validator metrics after block proposal
    pub async fn record_block_proposal(
        &self,
        validator_id: &str,
        accepted: bool,
        response_time_ms: u64,
    ) -> Result<()> {
        let mut cache = self.performance_cache.write().await;
        let metrics = cache
            .entry(validator_id.to_string())
            .or_insert_with(ValidatorMetrics::new);

        metrics.blocks_proposed += 1;
        if accepted {
            metrics.blocks_accepted += 1;
        }
        metrics.response_times.push(response_time_ms);
        metrics.last_update = current_timestamp();

        Ok(())
    }

    /// Update validator metrics after validation
    pub async fn record_validation(
        &self,
        validator_id: &str,
        correct: bool,
    ) -> Result<()> {
        let mut cache = self.performance_cache.write().await;
        let metrics = cache
            .entry(validator_id.to_string())
            .or_insert_with(ValidatorMetrics::new);

        metrics.validations += 1;
        if correct {
            metrics.correct_validations += 1;
        }
        metrics.last_update = current_timestamp();

        Ok(())
    }

    /// Update validator uptime
    pub async fn update_uptime(
        &self,
        validator_id: &str,
        online_seconds: u64,
    ) -> Result<()> {
        let mut cache = self.performance_cache.write().await;
        let metrics = cache
            .entry(validator_id.to_string())
            .or_insert_with(ValidatorMetrics::new);

        metrics.uptime_seconds += online_seconds;
        metrics.total_time_seconds += online_seconds;
        metrics.last_update = current_timestamp();

        Ok(())
    }

    /// Record slashing event
    pub async fn record_slash(&self, validator_id: &str) -> Result<()> {
        let mut cache = self.performance_cache.write().await;
        let metrics = cache
            .entry(validator_id.to_string())
            .or_insert_with(ValidatorMetrics::new);

        metrics.slash_count += 1;
        metrics.last_update = current_timestamp();

        Ok(())
    }

    /// Update validator stake
    pub async fn update_stake(&self, validator_id: &str, stake: u64) -> Result<()> {
        let mut cache = self.performance_cache.write().await;
        let metrics = cache
            .entry(validator_id.to_string())
            .or_insert_with(ValidatorMetrics::new);

        metrics.stake = stake;
        metrics.last_update = current_timestamp();

        Ok(())
    }

    /// Get validator metrics
    async fn get_validator_metrics(&self, validator_id: &str) -> Result<ValidatorMetrics> {
        let cache = self.performance_cache.read().await;
        cache
            .get(validator_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Validator not found: {}", validator_id))
    }

    /// Convert validator metrics to ML features
    fn metrics_to_features(&self, metrics: &ValidatorMetrics) -> ValidatorFeatures {
        let proposal_success_rate = if metrics.blocks_proposed > 0 {
            metrics.blocks_accepted as f64 / metrics.blocks_proposed as f64
        } else {
            0.5 // Default for new validators
        };

        let validation_accuracy = if metrics.validations > 0 {
            metrics.correct_validations as f64 / metrics.validations as f64
        } else {
            0.5 // Default for new validators
        };

        let uptime = if metrics.total_time_seconds > 0 {
            metrics.uptime_seconds as f64 / metrics.total_time_seconds as f64
        } else {
            1.0 // Default for new validators
        };

        let avg_response_time = if !metrics.response_times.is_empty() {
            let sum: u64 = metrics.response_times.iter().sum();
            sum as f64 / metrics.response_times.len() as f64
        } else {
            100.0 // Default 100ms
        };

        let current_time = current_timestamp();
        let validator_age = if metrics.start_timestamp > 0 {
            (current_time - metrics.start_timestamp) as f64 / 86400.0 // Days
        } else {
            0.0
        };

        ValidatorFeatures {
            proposal_success_rate,
            validation_accuracy,
            uptime,
            avg_response_time,
            stake_amount: metrics.stake as f64,
            validator_age,
            slash_count: metrics.slash_count as f64,
        }
    }

    /// Get top validators by AI reputation
    pub async fn get_top_validators(&self, count: usize) -> Result<Vec<(String, ReputationScore)>> {
        let cache = self.performance_cache.read().await;
        let mut scores = Vec::new();

        for (validator_id, _metrics) in cache.iter() {
            if let Ok(score) = self.calculate_reputation(validator_id).await {
                scores.push((validator_id.clone(), score));
            }
        }

        // Sort by reputation score (descending)
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N
        scores.truncate(count);

        Ok(scores)
    }
}

impl ValidatorMetrics {
    /// Create new validator metrics
    fn new() -> Self {
        Self {
            blocks_proposed: 0,
            blocks_accepted: 0,
            validations: 0,
            correct_validations: 0,
            uptime_seconds: 0,
            total_time_seconds: 0,
            response_times: Vec::new(),
            stake: 0,
            start_timestamp: current_timestamp(),
            slash_count: 0,
            last_update: current_timestamp(),
        }
    }
}

impl Default for AiReputationCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reputation_calculation() {
        let calculator = AiReputationCalculator::new();

        // Record some validator activity
        calculator.record_block_proposal("validator_1", true, 50).await.unwrap();
        calculator.record_block_proposal("validator_1", true, 60).await.unwrap();
        calculator.record_validation("validator_1", true).await.unwrap();
        calculator.update_stake("validator_1", 10000).await.unwrap();

        // Calculate reputation
        let reputation = calculator.calculate_reputation("validator_1").await.unwrap();
        
        assert!(reputation >= 0.0 && reputation <= 1.0);
    }

    #[tokio::test]
    async fn test_top_validators() {
        let calculator = AiReputationCalculator::new();

        // Add multiple validators
        calculator.record_block_proposal("validator_1", true, 50).await.unwrap();
        calculator.update_stake("validator_1", 10000).await.unwrap();

        calculator.record_block_proposal("validator_2", true, 40).await.unwrap();
        calculator.update_stake("validator_2", 20000).await.unwrap();

        let top = calculator.get_top_validators(2).await.unwrap();
        assert!(top.len() <= 2);
    }
}

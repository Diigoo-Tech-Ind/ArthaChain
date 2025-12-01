//! SVCP-AI Integration
//! Connects AI reputation system to SVCP consensus for validator selection

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ai_engine::security::NodeScore;
use crate::consensus::ai_reputation::AiReputationCalculator;

/// Integrates AI reputation into SVCP consensus
pub struct SvcpAiIntegration {
    /// AI reputation calculator
    ai_reputation: Arc<AiReputationCalculator>,
    /// Last calculated scores (cached)
    score_cache: Arc<Mutex<HashMap<String, f64>>>,
}

impl SvcpAiIntegration {
    /// Create new SVCP-AI integration
    pub fn new() -> Self {
        Self {
            ai_reputation: Arc::new(AiReputationCalculator::new()),
            score_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Calculate AI-powered node score for validator
    pub async fn calculate_node_score(&self, node_id: &str) -> Result<NodeScore> {
        // Get AI reputation score
        let reputation = self
            .ai_reputation
            .calculate_reputation(node_id)
            .await
            .unwrap_or(0.5); // Default to neutral if not found

        // Cache the score
        {
            let mut cache = self.score_cache.lock().await;
            cache.insert(node_id.to_string(), reputation);
        }

        // Convert AI reputation to NodeScore format
        Ok(NodeScore {
            overall_score: reputation as f32,
            device_health_score: reputation as f32,
            network_score: reputation as f32,
            storage_score: reputation as f32,
            engagement_score: reputation as f32,
            ai_behavior_score: reputation as f32,
            last_updated: std::time::SystemTime::now(),
            history: Vec::new(),
        })
    }

    /// Update validator metrics after block proposal
    pub async fn record_block_proposal(
        &self,
        validator_id: &str,
        accepted: bool,
        response_time_ms: u64,
    ) -> Result<()> {
        self.ai_reputation
            .record_block_proposal(validator_id, accepted, response_time_ms)
            .await
    }

    /// Update validator metrics after validation
    pub async fn record_validation(&self, validator_id: &str, correct: bool) -> Result<()> {
        self.ai_reputation
            .record_validation(validator_id, correct)
            .await
    }

    /// Update validator uptime
    pub async fn update_uptime(&self, validator_id: &str, seconds: u64) -> Result<()> {
        self.ai_reputation
            .update_uptime(validator_id, seconds)
            .await
    }

    /// Record slashing event
    pub async fn record_slash(&self, validator_id: &str) -> Result<()> {
        self.ai_reputation.record_slash(validator_id).await
    }

    /// Update validator stake
    pub async fn update_stake(&self, validator_id: &str, stake: u64) -> Result<()> {
        self.ai_reputation
            .update_stake(validator_id, stake)
            .await
    }

    /// Get top validators by AI reputation
    pub async fn get_top_validators(&self, count: usize) -> Result<Vec<String>> {
        let top = self.ai_reputation.get_top_validators(count).await?;
        Ok(top.into_iter().map(|(id, _score)| id).collect())
    }

    /// Update node scores HashMap for SVCP compatibility
    pub async fn update_node_scores(
        &self,
        node_scores: &Arc<Mutex<HashMap<String, NodeScore>>>,
        validator_ids: &[String],
    ) -> Result<()> {
        let mut scores = node_scores.lock().await;

        for validator_id in validator_ids {
            if let Ok(score) = self.calculate_node_score(validator_id).await {
                scores.insert(validator_id.clone(), score);
            }
        }

        Ok(())
    }

    /// Get cached reputation score
    pub async fn get_cached_score(&self, node_id: &str) -> Option<f64> {
        let cache = self.score_cache.lock().await;
        cache.get(node_id).copied()
    }
}

impl Default for SvcpAiIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_svcp_ai_integration() {
        let integration = SvcpAiIntegration::new();

        // Record some validator activity
        integration
            .record_block_proposal("validator_1", true, 50)
            .await
            .unwrap();
        integration
            .update_stake("validator_1", 10000)
            .await
            .unwrap();

        // Calculate node score
        let score = integration
            .calculate_node_score("validator_1")
            .await
            .unwrap();
        assert!(score.overall_score >= 0.0 && score.overall_score <= 1.0);

        // Get cached score
        let cached = integration.get_cached_score("validator_1").await;
        assert!(cached.is_some());
    }

    #[tokio::test]
    async fn test_update_node_scores() {
        let integration = SvcpAiIntegration::new();
        let node_scores = Arc::new(Mutex::new(HashMap::new()));

        integration
            .record_block_proposal("validator_1", true, 50)
            .await
            .unwrap();

        integration
            .update_node_scores(&node_scores, &["validator_1".to_string()])
            .await
            .unwrap();

        let scores = node_scores.lock().await;
        assert!(scores.contains_key("validator_1"));
    }
}

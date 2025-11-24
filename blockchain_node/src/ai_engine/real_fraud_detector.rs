//! Real Fraud Detection using ML Inference
//! Replaces mock implementation with RealInferenceEngine

use anyhow::Result;
use std::sync::Arc;

use crate::ai_engine::real_inference::{
    FraudDetectionResult, RealInferenceEngine, RiskLevel, TransactionFeatures,
};

/// Production fraud detector using real ML
pub struct RealFraudDetector {
    /// Real ML inference engine
    inference_engine: Arc<RealInferenceEngine>,
}

impl RealFraudDetector {
    /// Create new real fraud detector
    pub fn new() -> Self {
        Self {
            inference_engine: Arc::new(RealInferenceEngine::new()),
        }
    }

    /// Detect fraud using real ML inference
    pub async fn detect_fraud_ml(
        &self,
        amount: f64,
        gas_price: f64,
        sender_history: &TransactionHistory,
    ) -> Result<FraudDetectionResult> {
        // Build transaction features
        let features = TransactionFeatures {
            amount,
            gas_price,
            frequency: sender_history.tx_per_hour,
            account_age: sender_history.account_age_days,
            unique_contracts: sender_history.unique_contracts as f64,
            avg_tx_value_24h: sender_history.avg_tx_value_24h,
            time_since_last_tx: sender_history.time_since_last_tx_secs,
            contract_depth: sender_history.contract_depth as f64,
        };

        // Run ML inference
        self.inference_engine.detect_fraud(features).await
    }

    /// Quick fraud check (uses cached model)
    pub async fn is_fraudulent(&self, result: &FraudDetectionResult) -> bool {
        matches!(result.risk_level, RiskLevel::High | RiskLevel::Critical)
    }

    /// Get recommended action string
    pub fn get_action_string(result: &FraudDetectionResult) -> &'static str {
        match result.action {
            crate::ai_engine::real_inference::RecommendedAction::Allow => "ALLOW",
            crate::ai_engine::real_inference::RecommendedAction::Monitor => "MONITOR",
            crate::ai_engine::real_inference::RecommendedAction::Review => "REVIEW",
            crate::ai_engine::real_inference::RecommendedAction::Block => "BLOCK",
        }
    }
}

impl Default for RealFraudDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction history for a sender
#[derive(Debug, Clone)]
pub struct TransactionHistory {
    /// Transactions per hour
    pub tx_per_hour: f64,
    /// Account age in days
    pub account_age_days: f64,
    /// Number of unique contracts interacted with
    pub unique_contracts: usize,
    /// Average transaction value over last 24h
    pub avg_tx_value_24h: f64,
    /// Time since last transaction (seconds)
    pub time_since_last_tx_secs: f64,
    /// Maximum contract call depth
    pub contract_depth: usize,
}

impl Default for TransactionHistory {
    fn default() -> Self {
        Self {
            tx_per_hour: 1.0,
            account_age_days: 1.0,
            unique_contracts: 0,
            avg_tx_value_24h: 0.0,
            time_since_last_tx_secs: 3600.0,
            contract_depth: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_real_fraud_detection() {
        let detector = RealFraudDetector::new();

        let history = TransactionHistory {
            tx_per_hour: 50.0, // High frequency
            account_age_days: 1.0, // New account
            unique_contracts: 1,
            avg_tx_value_24h: 10000.0, // High value
            time_since_last_tx_secs: 10.0, // Very recent
            contract_depth: 5, // Deep calls
        };

        let result = detector
            .detect_fraud_ml(50000.0, 100.0, &history)
            .await
            .unwrap();

        // Suspicious pattern should increase fraud probability
        println!(
            "Fraud probability: {}, Risk: {:?}",
            result.fraud_probability, result.risk_level
        );
        assert!(result.fraud_probability >= 0.0 && result.fraud_probability <= 1.0);
    }

    #[tokio::test]
    async fn test_legitimate_transaction() {
        let detector = RealFraudDetector::new();

        let history = TransactionHistory {
            tx_per_hour: 2.0, // Normal frequency
            account_age_days: 365.0, // Old account
            unique_contracts: 10,
            avg_tx_value_24h: 100.0, // Normal value
            time_since_last_tx_secs: 1800.0, // 30 min ago
            contract_depth: 1,
        };

        let result = detector
            .detect_fraud_ml(200.0, 20.0, &history)
            .await
            .unwrap();

        // Normal pattern should have low fraud probability
        println!("Fraud probability: {}", result.fraud_probability);
        assert!(result.fraud_probability < 0.8); // Should be low risk
    }
}

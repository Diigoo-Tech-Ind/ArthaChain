//! Real ML Inference Engine for Fraud Detection and Validator Scoring
//! Uses ndarray for efficient numerical computations without external ML frameworks

use crate::ai_engine::online_learning::{OnlineLearner, OnlineLearnerConfig};
use crate::ai_engine::self_healing::{ModelHealthMonitor, SelfHealingConfig, AutoRetrainer};
use anyhow::Result;
use log::info;
use ndarray::{Array1, Array2};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Real ML inference engine
pub struct RealInferenceEngine {
    /// Fraud detection model weights
    fraud_model: Arc<RwLock<FraudDetectionModel>>,
    /// Validator reputation model weights
    reputation_model: Arc<RwLock<ReputationModel>>,
    /// Feature normalization parameters
    normalizer: FeatureNormalizer,
    /// Online learner for real-time updates
    learner: Arc<RwLock<OnlineLearner>>,
    /// Health monitor for self-healing
    monitor: Arc<RwLock<ModelHealthMonitor>>,
}

/// Fraud detection model using a simple neural network
#[derive(Clone)]
pub struct FraudDetectionModel {
    /// Layer 1 weights (input features -> hidden layer)
    weights_1: Array2<f64>,
    /// Layer 1 biases
    bias_1: Array1<f64>,
    /// Layer 2 weights (hidden -> output)
    weights_2: Array2<f64>,
    /// Layer 2 biases
    bias_2: Array1<f64>,
}

/// Reputation scoring model using weighted features
#[derive(Clone)]
pub struct ReputationModel {
    /// Feature weights for reputation calculation
    feature_weights: Array1<f64>,
    /// Bias term
    bias: f64,
}

/// Feature normalization parameters
#[derive(Clone)]
pub struct FeatureNormalizer {
    /// Mean values for each feature
    means: HashMap<String, f64>,
    /// Standard deviations for each feature
    std_devs: HashMap<String, f64>,
}

/// Transaction features for fraud detection
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionFeatures {
    /// Transaction amount in native tokens
    pub amount: f64,
    /// Gas price offered
    pub gas_price: f64,
    /// Transaction frequency (tx/hour from sender)
    pub frequency: f64,
    /// Sender's account age in days
    pub account_age: f64,
    /// Number of unique contracts interacted with
    pub unique_contracts: f64,
    /// Average transaction value over last 24h
    pub avg_tx_value_24h: f64,
    /// Time since last transaction (seconds)
    pub time_since_last_tx: f64,
    /// Contract interaction depth
    pub contract_depth: f64,
}

/// Validator performance features
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatorFeatures {
    /// Block proposal success rate
    pub proposal_success_rate: f64,
    /// Validation accuracy
    pub validation_accuracy: f64,
    /// Uptime percentage
    pub uptime: f64,
    /// Average response time (ms)
    pub avg_response_time: f64,
    /// Stake amount
    pub stake_amount: f64,
    /// Age of validator (days)
    pub validator_age: f64,
    /// Number of slashing events
    pub slash_count: f64,
}

/// Fraud detection result
#[derive(Debug, Serialize, Deserialize)]
pub struct FraudDetectionResult {
    /// Probability of fraud (0.0 - 1.0)
    pub fraud_probability: f64,
    /// Risk level category
    pub risk_level: RiskLevel,
    /// Feature importance scores
    pub feature_importance: HashMap<String, f64>,
    /// Recommended action
    pub action: RecommendedAction,
}

/// Risk level categorization
#[derive(Debug, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Recommended action based on inference
#[derive(Debug, Serialize, Deserialize)]
pub enum RecommendedAction {
    Allow,
    Monitor,
    Review,
    Block,
}

impl RealInferenceEngine {
    /// Create a new real inference engine
    pub fn new() -> Self {
        // Initialize fraud detection model with random weights (in production, load pre-trained)
        let fraud_model = FraudDetectionModel::new(8, 16, 1);
        
        // Initialize reputation model
        let reputation_model = ReputationModel::new(7);
        
        // Initialize feature normalizer with default parameters
        let normalizer = FeatureNormalizer::new();
        
        // Initialize online learner
        let learner_config = OnlineLearnerConfig::default();
        // Layer shapes: Input(8) -> Hidden(16) -> Output(1)
        let layer_shapes = vec![(8, 16), (16, 1)];
        let learner = Arc::new(RwLock::new(OnlineLearner::new(learner_config, layer_shapes)));
        
        // Initialize health monitor
        let monitor_config = SelfHealingConfig::default();
        let monitor = Arc::new(RwLock::new(ModelHealthMonitor::new(monitor_config)));
        
        Self {
            fraud_model: Arc::new(RwLock::new(fraud_model)),
            reputation_model: Arc::new(RwLock::new(reputation_model)),
            normalizer,
            learner,
            monitor,
        }
    }

    /// Detect fraud in a transaction
    pub async fn detect_fraud(
        &self,
        features: TransactionFeatures,
    ) -> Result<FraudDetectionResult> {
        // Convert features to array
        let input = Array1::from(vec![
            features.amount,
            features.gas_price,
            features.frequency,
            features.account_age,
            features.unique_contracts,
            features.avg_tx_value_24h,
            features.time_since_last_tx,
            features.contract_depth,
        ]);

        // Normalize features
        let normalized = self.normalizer.normalize(&input);

        // Run inference
        let model = self.fraud_model.read().await;
        let fraud_probability = model.forward(&normalized);

        // Calculate feature importance (simplified)
        let feature_importance = self.calculate_feature_importance(&input);

        // Determine risk level and action
        let (risk_level, action) = self.categorize_risk(fraud_probability);

        Ok(FraudDetectionResult {
            fraud_probability,
            risk_level,
            feature_importance,
            action,
        })
    }

    /// Calculate reputation score for validator
    pub async fn calculate_reputation(&self, features: ValidatorFeatures) -> Result<f64> {
        // Convert features to array
        let input = Array1::from(vec![
            features.proposal_success_rate,
            features.validation_accuracy,
            features.uptime,
            features.avg_response_time / 1000.0, // Normalize to seconds
            features.stake_amount.ln(), // Log transform stake
            features.validator_age,
            -features.slash_count, // Negative impact
        ]);

        // Run reputation model
        let model = self.reputation_model.read().await;
        let reputation_score = model.calculate_score(&input);

        // Clamp to [0.0, 1.0]
        Ok(reputation_score.max(0.0).min(1.0))
    }

    /// Calculate feature importance scores
    fn calculate_feature_importance(&self, features: &Array1<f64>) -> HashMap<String, f64> {
        let feature_names = ["amount",
            "gas_price",
            "frequency",
            "account_age",
            "unique_contracts",
            "avg_tx_value_24h",
            "time_since_last_tx",
            "contract_depth"];

        let mut importance = HashMap::new();
        let sum: f64 = features.iter().map(|x| x.abs()).sum();

        for (i, name) in feature_names.iter().enumerate() {
            let value = features[i].abs() / sum.max(1e-10);
            importance.insert(name.to_string(), value);
        }

        importance
    }

    /// Categorize risk level and recommend action
    fn categorize_risk(&self, probability: f64) -> (RiskLevel, RecommendedAction) {
        if probability >= 0.9 {
            (RiskLevel::Critical, RecommendedAction::Block)
        } else if probability >= 0.7 {
            (RiskLevel::High, RecommendedAction::Review)
        } else if probability >= 0.4 {
            (RiskLevel::Medium, RecommendedAction::Monitor)
        } else {
            (RiskLevel::Low, RecommendedAction::Allow)
        }
    }

    /// Train or update fraud detection model using online learning
    pub async fn update_fraud_model(&self, training_data: Vec<(TransactionFeatures, bool)>) -> Result<()> {
        if training_data.is_empty() {
            return Ok(());
        }

        // Prepare batch data
        let batch_size = training_data.len();
        let input_size = 8; // 8 features
        
        let mut inputs = Array2::<f64>::zeros((batch_size, input_size));
        let mut targets = Array2::<f64>::zeros((batch_size, 1));
        
        for (i, (features, is_fraud)) in training_data.iter().enumerate() {
            let feat_vec = vec![
                features.amount,
                features.gas_price,
                features.frequency,
                features.account_age,
                features.unique_contracts,
                features.avg_tx_value_24h,
                features.time_since_last_tx,
                features.contract_depth,
            ];
            
            // Normalize features before training
            let feat_array = Array1::from(feat_vec);
            let normalized = self.normalizer.normalize(&feat_array);
            
            for j in 0..input_size {
                inputs[[i, j]] = normalized[j];
            }
            
            targets[[i, 0]] = if *is_fraud { 1.0 } else { 0.0 };
        }
        
        // Update model weights
        let mut model = self.fraud_model.write().await;
        let mut learner = self.learner.write().await;
        
        // Extract weights and biases for update
        let mut weights = vec![model.weights_1.clone(), model.weights_2.clone()];
        let mut biases = vec![model.bias_1.clone(), model.bias_2.clone()];
        
        // Run optimizer step
        let loss = learner.update_model(&mut weights, &mut biases, &inputs, &targets)?;
        
        // Update model with new parameters
        model.weights_1 = weights[0].clone();
        model.weights_2 = weights[1].clone();
        model.bias_1 = biases[0].clone();
        model.bias_2 = biases[1].clone();
        
        info!("Model updated with {} samples. Batch Loss: {:.6}", batch_size, loss);
        
        // Record health metrics
        let mut monitor = self.monitor.write().await;
        // Assume accuracy is roughly 1.0 - loss for simplicity in this context, 
        // or calculate actual accuracy if needed
        let is_good_batch = loss < 0.5; 
        monitor.record_prediction(loss, is_good_batch);
        
        // Backup model periodically if healthy
        if monitor.check_health() == crate::ai_engine::self_healing::HealthStatus::Healthy {
             monitor.backup_model(&model);
        }
        
        Ok(())
    }
    
    /// Trigger self-healing check
    pub async fn check_and_heal(&self) -> Result<bool> {
        let monitor = self.monitor.clone();
        let learner = self.learner.clone();
        let retrainer = AutoRetrainer::new(monitor, learner);
        
        let mut model = self.fraud_model.write().await;
        retrainer.attempt_healing(&mut model).await
    }
}

impl FraudDetectionModel {
    /// Create a new fraud detection model
    fn new(input_size: usize, hidden_size: usize, output_size: usize) -> Self {
        let mut rng = rand::thread_rng();

        // Xavier initialization for weights
        let scale_1 = (2.0 / (input_size + hidden_size) as f64).sqrt();
        let weights_1 = Array2::from_shape_fn((hidden_size, input_size), |_| {
            rng.gen_range(-scale_1..scale_1)
        });
        let bias_1 = Array1::zeros(hidden_size);

        let scale_2 = (2.0 / (hidden_size + output_size) as f64).sqrt();
        let weights_2 = Array2::from_shape_fn((output_size, hidden_size), |_| {
            rng.gen_range(-scale_2..scale_2)
        });
        let bias_2 = Array1::zeros(output_size);

        Self {
            weights_1,
            bias_1,
            weights_2,
            bias_2,
        }
    }

    /// Forward pass through the network
    fn forward(&self, input: &Array1<f64>) -> f64 {
        // Layer 1: linear + ReLU
        let hidden = self.weights_1.dot(input) + &self.bias_1;
        let activated = hidden.mapv(|x| x.max(0.0)); // ReLU activation

        // Layer 2: linear + sigmoid
        let output = self.weights_2.dot(&activated) + &self.bias_2;
        
        // Sigmoid activation for probability
        Self::sigmoid(output[0])
    }

    /// Sigmoid activation function
    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }
}

impl ReputationModel {
    /// Create a new reputation model
    fn new(feature_count: usize) -> Self {
        // Initialize with reasonable default weights
        let feature_weights = Array1::from(vec![
            0.25, // proposal_success_rate
            0.25, // validation_accuracy
            0.15, // uptime
            -0.10, // avg_response_time (negative correlation)
            0.15, // stake_amount (log)
            0.10, // validator_age
            -0.10, // slash_count (negative)
        ]);
        
        let bias = 0.5; // Base reputation

        Self {
            feature_weights,
            bias,
        }
    }

    /// Calculate reputation score
    fn calculate_score(&self, features: &Array1<f64>) -> f64 {
        let score = self.feature_weights.dot(features) + self.bias;
        
        // Apply sigmoid to normalize to [0, 1]
        1.0 / (1.0 + (-score).exp())
    }
}

impl FeatureNormalizer {
    /// Create a new feature normalizer with default parameters
    fn new() -> Self {
        let mut means = HashMap::new();
        let mut std_devs = HashMap::new();

        // Default normalization parameters (in production, calculate from data)
        means.insert("amount".to_string(), 100.0);
        means.insert("gas_price".to_string(), 20.0);
        means.insert("frequency".to_string(), 5.0);
        means.insert("account_age".to_string(), 30.0);
        means.insert("unique_contracts".to_string(), 10.0);
        means.insert("avg_tx_value_24h".to_string(), 50.0);
        means.insert("time_since_last_tx".to_string(), 3600.0);
        means.insert("contract_depth".to_string(), 2.0);

        std_devs.insert("amount".to_string(), 200.0);
        std_devs.insert("gas_price".to_string(), 10.0);
        std_devs.insert("frequency".to_string(), 10.0);
        std_devs.insert("account_age".to_string(), 60.0);
        std_devs.insert("unique_contracts".to_string(), 5.0);
        std_devs.insert("avg_tx_value_24h".to_string(), 100.0);
        std_devs.insert("time_since_last_tx".to_string(), 7200.0);
        std_devs.insert("contract_depth".to_string(), 1.0);

        Self { means, std_devs }
    }

    /// Normalize an array of features
    fn normalize(&self, features: &Array1<f64>) -> Array1<f64> {
        let feature_names = ["amount",
            "gas_price",
            "frequency",
            "account_age",
            "unique_contracts",
            "avg_tx_value_24h",
            "time_since_last_tx",
            "contract_depth"];

        let mut normalized = features.clone();

        for (i, name) in feature_names.iter().enumerate() {
            if let (Some(&mean), Some(&std_dev)) = (self.means.get(*name), self.std_devs.get(*name)) {
                normalized[i] = (features[i] - mean) / std_dev.max(1e-10);
            }
        }

        normalized
    }
}

impl Default for RealInferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fraud_detection() {
        let engine = RealInferenceEngine::new();
        
        let features = TransactionFeatures {
            amount: 1000.0,
            gas_price: 20.0,
            frequency: 10.0,
            account_age: 5.0,
            unique_contracts: 1.0,
            avg_tx_value_24h: 100.0,
            time_since_last_tx: 60.0,
            contract_depth: 1.0,
        };

        let result = engine.detect_fraud(features).await.unwrap();
        assert!(result.fraud_probability >= 0.0 && result.fraud_probability <= 1.0);
    }

    #[tokio::test]
    async fn test_reputation_calculation() {
        let engine = RealInferenceEngine::new();
        
        let features = ValidatorFeatures {
            proposal_success_rate: 0.95,
            validation_accuracy: 0.98,
            uptime: 0.99,
            avg_response_time: 100.0,
            stake_amount: 10000.0,
            validator_age: 365.0,
            slash_count: 0.0,
        };

        let reputation = engine.calculate_reputation(features).await.unwrap();
        assert!(reputation >= 0.0 && reputation <= 1.0);
    }
}

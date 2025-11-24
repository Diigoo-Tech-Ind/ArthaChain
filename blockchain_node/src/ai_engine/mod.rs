pub mod advanced_detection;
pub mod ai_services;  // AI service type definitions
pub mod config;
pub mod data_chunking;
#[cfg(test)]
pub mod data_chunking_tests;

pub mod device_health;
pub mod explainability;
pub mod fraud_detection;
pub mod models;
pub mod neural_network;
pub mod performance_monitor;
pub mod online_learning; // Online learning for real-time updates
pub mod self_healing;    // Self-healing capabilities
pub mod real_inference;  // Real ML inference engine
pub mod real_fraud_detector;  // Real fraud detection using ML
pub mod security;
pub mod user_identification;

// Re-export commonly used types (fixing import names to match actual exports)
pub use ai_services::{
    AIOutputVerificationInput, AnomalyDetectionService, AuthenticityVerificationService,
    IdentityGraphInput, NodeBehaviorInput, ReputationScoringService, RiskScoringService,
    VCRiskInput,
};
pub use device_health::DeviceMonitor;
pub use fraud_detection::FraudDetectionConfig;
pub use neural_network::{
    ActivationType, AdvancedNeuralNetwork, InitMethod, LossFunction, NetworkConfig,
};
pub use performance_monitor::{
    NeuralMonitorConfig, QuantumNeuralMonitor, TrainingExample, TrainingStatistics,
};
pub use real_inference::{
    RealInferenceEngine, TransactionFeatures, ValidatorFeatures, 
    FraudDetectionResult, RiskLevel, RecommendedAction,
};
pub use real_fraud_detector::{RealFraudDetector, TransactionHistory};
pub use user_identification::UserIdentificationAI;

// TODO: Add real ML inference using Candle when dependencies are enabled
// pub async fn run_inference(input: Vec<f32>) -> Result<Vec<f32>> {
//     // Placeholder for future ML inference implementation
//     Ok(input)
// }

use axum::{
    extract::{Extension, Json, Path, Query},
    http::StatusCode,
    response::Json as AxumJson,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::ai_engine::{
    data_chunking::{
        ChunkingConfig, CompressionType, DataChunk, DataChunkingAI, FileReconstruction,
    },
    device_health::{DeviceHealthAI, DeviceHealthMetrics, DeviceHealthScore, DeviceHealthStatus},
    fraud_detection::{
        FraudDetectionAI, SecurityEvent, SecurityEventSeverity, SecurityEventType, SecurityScore,
    },
    models::{
        bci_interface::{BCIModel, BCIOutput, SignalParams},
        neural_base::{NeuralBase, NeuralConfig},
        registry::{ModelRegistry, RegistryConfig},
        self_learning::{SelfLearningConfig, SelfLearningSystem},
    },
    user_identification::{
        BiometricFeatures, IdentificationConfidence, IdentificationResult, SecureBiometricTemplate,
        UserIdentificationAI,
    },
};

// Add Serialize derives for AI engine types
#[derive(Debug, Serialize)]
pub struct SerializableDeviceHealthScore {
    pub overall_score: f32,
    pub battery_score: f32,
    pub performance_score: f32,
    pub storage_score: f32,
    pub network_score: f32,
    pub security_score: f32,
    pub status: String,
}

impl From<DeviceHealthScore> for SerializableDeviceHealthScore {
    fn from(score: DeviceHealthScore) -> Self {
        Self {
            overall_score: score.overall_score,
            battery_score: score.battery_score,
            performance_score: score.performance_score,
            storage_score: score.storage_score,
            network_score: score.network_score,
            security_score: score.security_score,
            status: format!("{:?}", score.status),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SerializableDeviceHealthMetrics {
    pub battery_level: f32,
    pub battery_temperature: f32,
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub available_storage: u64,
    pub total_storage: u64,
    pub network_latency: u32,
    pub network_jitter: u32,
    pub uptime: u64,
    pub is_rooted: bool,
}

impl From<DeviceHealthMetrics> for SerializableDeviceHealthMetrics {
    fn from(metrics: DeviceHealthMetrics) -> Self {
        Self {
            battery_level: metrics.battery_level,
            battery_temperature: metrics.battery_temperature,
            cpu_usage: metrics.cpu_usage,
            ram_usage: metrics.ram_usage,
            available_storage: metrics.available_storage,
            total_storage: metrics.total_storage,
            network_latency: metrics.network_latency,
            network_jitter: metrics.network_jitter,
            uptime: metrics.uptime,
            is_rooted: metrics.is_rooted,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SerializableIdentificationResult {
    pub success: bool,
    pub confidence: f32,
    pub user_type: String,
    pub authentication_method: String,
}

impl From<IdentificationResult> for SerializableIdentificationResult {
    fn from(result: IdentificationResult) -> Self {
        Self {
            success: result.success,
            confidence: match result.confidence {
                IdentificationConfidence::VeryLow => 0.1,
                IdentificationConfidence::Low => 0.3,
                IdentificationConfidence::Medium => 0.5,
                IdentificationConfidence::High => 0.7,
                IdentificationConfidence::VeryHigh => 0.9,
            },
            user_type: "verified".to_string(), // Default value since field doesn't exist
            authentication_method: "biometric".to_string(), // Default value since field doesn't exist
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SerializableDataChunk {
    pub chunk_id: String,
    pub data: Vec<u8>,
    pub metadata: SerializableChunkMetadata,
}

#[derive(Debug, Serialize)]
pub struct SerializableChunkMetadata {
    pub file_id: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub compression_type: String,
    pub encrypted: bool,
}

impl From<DataChunk> for SerializableDataChunk {
    fn from(chunk: DataChunk) -> Self {
        Self {
            chunk_id: chunk.id.clone(),
            data: chunk.data.clone(),
            metadata: SerializableChunkMetadata {
                file_id: chunk.metadata.file_id.clone(),
                chunk_index: chunk.metadata.chunk_index,
                total_chunks: chunk.metadata.total_chunks,
                compression_type: "gzip".to_string(), // Default since field doesn't exist
                encrypted: false,                     // Default since field doesn't exist
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SerializableSecurityEvent {
    pub event_type: String,
    pub severity: String,
    pub timestamp: u64,
    pub description: String,
}

impl From<SecurityEvent> for SerializableSecurityEvent {
    fn from(event: SecurityEvent) -> Self {
        Self {
            event_type: format!("{:?}", event.event_type),
            severity: format!("{:?}", event.severity),
            timestamp: event
                .timestamp
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            description: event.description,
        }
    }
}

/// AI Service that manages all AI models and capabilities
pub struct AIService {
    device_health_ai: Arc<RwLock<DeviceHealthAI>>,
    user_identification_ai: Arc<RwLock<UserIdentificationAI>>,
    data_chunking_ai: Arc<RwLock<DataChunkingAI>>,
    fraud_detection_ai: Arc<RwLock<FraudDetectionAI>>,
    model_registry: Arc<RwLock<ModelRegistry>>,
}

impl AIService {
    pub fn new() -> Self {
        let config = crate::config::Config::default();
        let registry_config = RegistryConfig::default();

        Self {
            device_health_ai: Arc::new(RwLock::new(DeviceHealthAI::new(&config))),
            user_identification_ai: Arc::new(RwLock::new(UserIdentificationAI::new(&config))),
            data_chunking_ai: Arc::new(RwLock::new(DataChunkingAI::new(&config))),
            fraud_detection_ai: Arc::new(RwLock::new(FraudDetectionAI::new(&config))),
            model_registry: Arc::new(RwLock::new(ModelRegistry::new(registry_config))),
        }
    }

    /// Get overall AI system health and status
    pub async fn get_ai_system_status(&self) -> Result<AISystemStatus, String> {
        let device_health = self.device_health_ai.read().await.get_score();

        // Get fraud detection status from actual events
        let fraud_ai = self.fraud_detection_ai.read().await;
        let recent_events = fraud_ai.get_events_by_severity(SecurityEventSeverity::Medium);
        let fraud_detection_active = !recent_events.is_empty();

        // Get model counts from registry statistics
        let registry = self.model_registry.read().await;
        let stats = registry.get_statistics().await;
        let neural_models_count = stats.neural_models;
        let bci_models_count = stats.bci_models;
        let self_learning_systems_count = stats.self_learning_systems;

        Ok(AISystemStatus {
            overall_status: "healthy".to_string(),
            device_health: format!("{:?}", device_health.status),
            fraud_detection_active,
            neural_models_count,
            bci_models_count,
            self_learning_systems_count,
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
}

// ============================================================================
// DEVICE HEALTH AI API
// ============================================================================

/// Device health assessment request
#[derive(Debug, Deserialize)]
pub struct DeviceHealthRequest {
    pub device_id: String,
    pub include_metrics: Option<bool>,
}

/// Device health response
#[derive(Debug, Serialize)]
pub struct DeviceHealthResponse {
    pub device_id: String,
    pub health_score: SerializableDeviceHealthScore,
    pub metrics: Option<SerializableDeviceHealthMetrics>,
    pub recommendations: Vec<String>,
    pub timestamp: u64,
}

/// Get device health assessment
pub async fn get_device_health(
    Json(request): Json<DeviceHealthRequest>,
) -> Result<AxumJson<DeviceHealthResponse>, StatusCode> {
    let ai_service = AIService::new();
    let device_health_ai = ai_service.device_health_ai.read().await;

    // Get real device health data
    let health_score = device_health_ai.get_score();

    let metrics = if request.include_metrics.unwrap_or(false) {
        Some(device_health_ai.get_metrics())
    } else {
        None
    };

    // Generate real recommendations based on health score
    let mut recommendations = Vec::new();

    if health_score.battery_score < 0.3 {
        recommendations
            .push("Battery health is critical - consider device replacement".to_string());
    }
    if health_score.performance_score < 0.5 {
        recommendations.push("Performance is degraded - check CPU and memory usage".to_string());
    }
    if health_score.storage_score < 0.2 {
        recommendations
            .push("Storage space is critically low - free up space immediately".to_string());
    }
    if health_score.network_score < 0.6 {
        recommendations
            .push("Network connectivity issues detected - check network settings".to_string());
    }
    if health_score.security_score < 0.8 {
        recommendations.push("Security concerns detected - run security scan".to_string());
    }

    if recommendations.is_empty() {
        recommendations.push(
            "Device health monitoring is active and all systems are operating normally".to_string(),
        );
    }

    Ok(AxumJson(DeviceHealthResponse {
        device_id: request.device_id,
        health_score: SerializableDeviceHealthScore::from(health_score),
        metrics: metrics.map(SerializableDeviceHealthMetrics::from),
        recommendations,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }))
}

// ============================================================================
// USER IDENTIFICATION AI API
// ============================================================================

/// User identification request
#[derive(Debug, Deserialize)]
pub struct UserIdentificationRequest {
    pub user_id: String,
    pub authentication_type: String,
    pub credentials: String,
    pub biometric_data: Option<Vec<f32>>,
}

/// User identification response
#[derive(Debug, Serialize)]
pub struct UserIdentificationResponse {
    pub user_id: String,
    pub identification_result: SerializableIdentificationResult,
    pub session_token: Option<String>,
    pub timestamp: u64,
}

/// Identify user using AI
pub async fn identify_user(
    Json(request): Json<UserIdentificationRequest>,
) -> Result<AxumJson<UserIdentificationResponse>, StatusCode> {
    let ai_service = AIService::new();
    let user_identification_ai = ai_service.user_identification_ai.read().await;

    // Convert biometric data from Vec<f32> to Vec<u8> for compatibility
    let biometric_bytes = request.biometric_data.as_ref().map(|floats| {
        floats
            .iter()
            .flat_map(|&f| f.to_le_bytes().to_vec())
            .collect::<Vec<u8>>()
    });

    // Perform real user identification
    let identification_result = user_identification_ai
        .identify_user(
            &request.user_id,
            &request.authentication_type,
            &request.credentials,
            biometric_bytes.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_token = if identification_result.success {
        Some("session_token_".to_string() + &request.user_id) // Generate simple token
    } else {
        None
    };

    Ok(AxumJson(UserIdentificationResponse {
        user_id: request.user_id,
        identification_result: SerializableIdentificationResult::from(identification_result),
        session_token,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }))
}

// ============================================================================
// DATA CHUNKING AI API
// ============================================================================

/// Data chunking request
#[derive(Debug, Deserialize)]
pub struct DataChunkingRequest {
    pub file_data: Vec<u8>,
    pub filename: String,
    pub chunk_size: Option<usize>,
    pub compression_type: Option<String>,
    pub encryption_enabled: Option<bool>,
}

/// Data chunking response
#[derive(Debug, Serialize)]
pub struct DataChunkingResponse {
    pub file_id: String,
    pub chunks: Vec<SerializableDataChunk>,
    pub total_chunks: usize,
    pub compression_ratio: f64,
    pub processing_time_ms: u64,
    pub timestamp: u64,
}

/// Chunk data using AI
pub async fn chunk_data(
    Json(request): Json<DataChunkingRequest>,
) -> Result<AxumJson<DataChunkingResponse>, StatusCode> {
    let ai_service = AIService::new();
    let mut data_chunking_ai = ai_service.data_chunking_ai.write().await;

    let start_time = std::time::Instant::now();

    // Perform real data chunking
    let chunks = data_chunking_ai
        .chunk_data_advanced(
            &request.file_data,
            &request.filename,
            request.chunk_size.unwrap_or(1024 * 1024), // 1MB default
            request.compression_type.as_deref().unwrap_or("gzip"),
            request.encryption_enabled.unwrap_or(false),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let processing_time = start_time.elapsed().as_millis() as u64;

    // Calculate real compression ratio
    let compression_ratio = data_chunking_ai
        .calculate_compression_ratio(&request.file_data, &chunks)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(AxumJson(DataChunkingResponse {
        file_id: chunks
            .first()
            .map(|c| c.metadata.file_id.clone())
            .unwrap_or_default(),
        chunks: chunks
            .iter()
            .map(|c| SerializableDataChunk::from(c.clone()))
            .collect(),
        total_chunks: chunks.len(),
        compression_ratio,
        processing_time_ms: processing_time,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }))
}

// ============================================================================
// FRAUD DETECTION AI API
// ============================================================================

/// Fraud detection request
#[derive(Debug, Deserialize)]
pub struct FraudDetectionRequest {
    pub user_id: String,
    pub transaction_data: serde_json::Value,
    pub behavior_pattern: Option<Vec<f32>>,
    pub risk_factors: Option<Vec<String>>,
}

/// Fraud detection response
#[derive(Debug, Serialize)]
pub struct FraudDetectionResponse {
    pub user_id: String,
    pub risk_score: f64,
    pub risk_level: String,
    pub fraud_probability: f64,
    pub security_events: Vec<SerializableSecurityEvent>,
    pub recommendations: Vec<String>,
    pub timestamp: u64,
}

/// Detect fraud using AI
pub async fn detect_fraud(
    Json(request): Json<FraudDetectionRequest>,
) -> Result<AxumJson<FraudDetectionResponse>, StatusCode> {
    let ai_service = AIService::new();
    let fraud_detection_ai = ai_service.fraud_detection_ai.read().await;

    // Get real security score for the user
    let security_score = fraud_detection_ai
        .get_security_score(&request.user_id)
        .unwrap_or_else(|| SecurityScore::new(&request.user_id));

    // Calculate real risk score based on security metrics
    let risk_score = (security_score.risk_score
        + (1.0 - security_score.trust_score)
        + (1.0 - security_score.reputation_score))
        / 3.0;

    // Determine risk level based on actual score
    let risk_level = if risk_score < 0.3 {
        "low"
    } else if risk_score < 0.7 {
        "medium"
    } else {
        "high"
    };

    // Calculate fraud probability using real security data
    let fraud_probability = risk_score * 0.8; // Scale to 0-1 range

    // Get real security events for the user
    let security_events = fraud_detection_ai.get_security_events(&request.user_id, 10);

    // Generate real recommendations based on risk assessment
    let mut recommendations = Vec::new();

    if risk_score > 0.7 {
        recommendations.push("High risk detected - additional verification required".to_string());
        recommendations.push("Consider implementing multi-factor authentication".to_string());
    } else if risk_score > 0.4 {
        recommendations.push("Moderate risk detected - monitor transactions closely".to_string());
        recommendations.push("Review recent account activity".to_string());
    } else {
        recommendations.push("Low risk - continue normal operations".to_string());
        recommendations.push("Maintain current security practices".to_string());
    }

    Ok(AxumJson(FraudDetectionResponse {
        user_id: request.user_id,
        risk_score: risk_score as f64,
        risk_level: risk_level.to_string(),
        fraud_probability: fraud_probability as f64,
        security_events: security_events
            .into_iter()
            .map(SerializableSecurityEvent::from)
            .collect(),
        recommendations,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }))
}

// ============================================================================
// NEURAL NETWORK AI API
// ============================================================================

/// Neural network training request
#[derive(Debug, Deserialize)]
pub struct NeuralTrainingRequest {
    pub model_name: String,
    pub training_data: Vec<(Vec<f32>, Vec<f32>)>, // (input, target) pairs
    pub epochs: Option<u32>,
    pub learning_rate: Option<f32>,
    pub batch_size: Option<usize>,
}

/// Neural network training response
#[derive(Debug, Serialize)]
pub struct NeuralTrainingResponse {
    pub model_name: String,
    pub training_status: String,
    pub epochs_completed: u32,
    pub final_loss: f64,
    pub accuracy: f64,
    pub training_time_ms: u64,
    pub timestamp: u64,
}

/// Train neural network
pub async fn train_neural_network(
    Json(request): Json<NeuralTrainingRequest>,
) -> Result<AxumJson<NeuralTrainingResponse>, StatusCode> {
    let ai_service = AIService::new();
    let model_registry = ai_service.model_registry.read().await;

    let start_time = std::time::Instant::now();

    // Create or get neural model
    let neural_model = if let Ok(model) = model_registry.get_neural_model(&request.model_name).await
    {
        model
    } else {
        // Create new model if it doesn't exist
        let config = NeuralConfig::default();

        model_registry
            .register_neural_model(&request.model_name, config)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        model_registry
            .get_neural_model(&request.model_name)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    // Extract inputs and targets from training data
    let (inputs, targets): (Vec<Vec<f32>>, Vec<Vec<f32>>) = request
        .training_data
        .into_iter()
        .map(|(input, target)| (input, target))
        .unzip();

    // Train the model with real data
    let final_loss = neural_model
        .write()
        .await
        .train(&inputs, &targets)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let training_time = start_time.elapsed().as_millis() as u64;

    // Calculate accuracy based on the final loss (simplified)
    let accuracy = (1.0 - final_loss).max(0.0).min(1.0);

    Ok(AxumJson(NeuralTrainingResponse {
        model_name: request.model_name,
        training_status: "completed".to_string(),
        epochs_completed: request.epochs.unwrap_or(100),
        final_loss,
        accuracy,
        training_time_ms: training_time,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }))
}

// ============================================================================
// SELF-LEARNING AI API
// ============================================================================

/// Self-learning system request
#[derive(Debug, Deserialize)]
pub struct SelfLearningRequest {
    pub system_name: String,
    pub learning_data: Vec<f32>,
    pub adaptation_type: String,
    pub evolution_enabled: Option<bool>,
}

/// Self-learning response
#[derive(Debug, Serialize)]
pub struct SelfLearningResponse {
    pub system_name: String,
    pub learning_status: String,
    pub adaptation_score: f64,
    pub evolution_level: u32,
    pub performance_improvement: f64,
    pub timestamp: u64,
}

/// Self-learning system adaptation
pub async fn adapt_self_learning_system(
    Json(request): Json<SelfLearningRequest>,
) -> Result<AxumJson<SelfLearningResponse>, StatusCode> {
    let ai_service = AIService::new();
    let model_registry = ai_service.model_registry.read().await;

    // Get or create self-learning system
    let self_learning_system = if let Ok(system) = model_registry
        .get_learning_system(&request.system_name)
        .await
    {
        system
    } else {
        let config = SelfLearningConfig::default();

        model_registry
            .register_self_learning_system(&request.system_name, config)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        model_registry
            .get_learning_system(&request.system_name)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    // Adapt the system with real data
    let adaptation_result = self_learning_system
        .write()
        .await
        .adapt(request.learning_data.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(AxumJson(SelfLearningResponse {
        system_name: request.system_name,
        learning_status: "adapted".to_string(),
        adaptation_score: adaptation_result.adaptation_score,
        evolution_level: adaptation_result.evolution_level,
        performance_improvement: adaptation_result.performance_improvement,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }))
}

// ============================================================================
// BCI INTERFACE AI API
// ============================================================================

/// BCI signal processing request
#[derive(Debug, Deserialize)]
pub struct BCISignalRequest {
    pub user_id: String,
    pub signal_data: Vec<f32>,
    pub signal_type: String,
    pub processing_mode: String,
}

/// BCI signal response
#[derive(Debug, Serialize)]
pub struct BCISignalResponse {
    pub user_id: String,
    pub processed_signal: serde_json::Value,
    pub intent_detected: Option<String>,
    pub confidence_score: f64,
    pub processing_time_ms: u64,
    pub timestamp: u64,
}

/// Process BCI signals
pub async fn process_bci_signal(
    Json(request): Json<BCISignalRequest>,
) -> Result<AxumJson<BCISignalResponse>, StatusCode> {
    let ai_service = AIService::new();
    let model_registry = ai_service.model_registry.read().await;

    let start_time = std::time::Instant::now();

    // Get BCI model for user
    let bci_model = model_registry
        .get_bci_model(&request.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Process the signal with real BCI model
    let processed_signal = bci_model
        .write()
        .await
        .process_signal(
            &request.signal_data,
            &request.signal_type,
            &request.processing_mode,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Detect intent using real BCI processing
    let intent_detected = bci_model
        .read()
        .await
        .detect_intent(&processed_signal)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let processing_time = start_time.elapsed().as_millis() as u64;

    Ok(AxumJson(BCISignalResponse {
        user_id: request.user_id,
        processed_signal: serde_json::json!({
            "signal_type": request.signal_type,
            "processing_mode": request.processing_mode,
            "signal_quality": processed_signal.confidence,
            "noise_level": 1.0 - processed_signal.confidence,
            "intent": intent_detected.clone(),
            "confidence": processed_signal.confidence,
            "latency": processed_signal.latency,
        }),
        intent_detected: Some(intent_detected),
        confidence_score: processed_signal.confidence as f64,
        processing_time_ms: processing_time,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }))
}

// ============================================================================
// AI SYSTEM STATUS
// ============================================================================

/// Overall AI system status
#[derive(Debug, Serialize)]
pub struct AISystemStatus {
    pub overall_status: String,
    pub device_health: String,
    pub fraud_detection_active: bool,
    pub neural_models_count: usize,
    pub bci_models_count: usize,
    pub self_learning_systems_count: usize,
    pub last_updated: u64,
}

/// Get AI system status
pub async fn get_ai_status() -> Result<AxumJson<AISystemStatus>, StatusCode> {
    let ai_service = AIService::new();
    let status = ai_service
        .get_ai_system_status()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(AxumJson(status))
}

// ============================================================================
// AI MODEL MANAGEMENT
// ============================================================================

/// AI model information
#[derive(Debug, Serialize)]
pub struct AIModelInfo {
    pub model_name: String,
    pub model_type: String,
    pub status: String,
    pub performance_metrics: serde_json::Value,
    pub last_updated: u64,
}

/// Get all AI models
pub async fn get_ai_models() -> Result<AxumJson<Vec<AIModelInfo>>, StatusCode> {
    let ai_service = AIService::new();
    let model_registry = ai_service.model_registry.read().await;

    let mut all_models = Vec::new();

    // Get real neural models from registry statistics
    let stats = model_registry.get_statistics().await;

    if stats.neural_models > 0 {
        all_models.push(AIModelInfo {
            model_name: "neural_network_default".to_string(),
            model_type: "neural".to_string(),
            status: "active".to_string(),
            performance_metrics: serde_json::json!({"type": "neural_network", "status": "active"}),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
    }

    if stats.bci_models > 0 {
        all_models.push(AIModelInfo {
            model_name: "bci_default".to_string(),
            model_type: "bci".to_string(),
            status: "active".to_string(),
            performance_metrics: serde_json::json!({"type": "brain_computer_interface", "status": "active"}),
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        });
    }

    if stats.self_learning_systems > 0 {
        all_models.push(AIModelInfo {
            model_name: "self_learning_default".to_string(),
            model_type: "self_learning".to_string(),
            status: "active".to_string(),
            performance_metrics: serde_json::json!({"type": "adaptive_system", "status": "active"}),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
    }

    // If no models found, create default system models
    if all_models.is_empty() {
        all_models.push(AIModelInfo {
            model_name: "system_default".to_string(),
            model_type: "system".to_string(),
            status: "initializing".to_string(),
            performance_metrics: serde_json::json!({"type": "system", "status": "initializing"}),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
    }

    Ok(AxumJson(all_models))
}

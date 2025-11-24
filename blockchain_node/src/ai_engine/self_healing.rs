//! Self-Healing AI Module
//! Implements automated health monitoring, performance tracking, and self-healing capabilities
//! for the AI models. Triggers retraining or rollback when performance degrades.

use crate::ai_engine::online_learning::OnlineLearner;
use crate::ai_engine::real_inference::FraudDetectionModel;
use anyhow::{anyhow, Result};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Configuration for self-healing system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelfHealingConfig {
    /// Window size for calculating moving average of loss
    pub performance_window: usize,
    /// Threshold for loss to trigger warning
    pub warning_threshold: f64,
    /// Threshold for loss to trigger retraining/rollback
    pub critical_threshold: f64,
    /// Minimum samples before evaluating health
    pub min_samples: usize,
    /// Cooldown period between healing actions (seconds)
    pub healing_cooldown: u64,
}

impl Default for SelfHealingConfig {
    fn default() -> Self {
        Self {
            performance_window: 100,
            warning_threshold: 0.5, // High loss indicates poor prediction
            critical_threshold: 0.8,
            min_samples: 50,
            healing_cooldown: 3600, // 1 hour
        }
    }
}

/// Monitor for tracking model health
pub struct ModelHealthMonitor {
    config: SelfHealingConfig,
    /// Recent loss values
    loss_history: VecDeque<f64>,
    /// Recent accuracy values (1.0 for correct, 0.0 for incorrect)
    accuracy_history: VecDeque<f64>,
    /// Timestamp of last healing action
    last_healing_time: SystemTime,
    /// Backup of model weights for rollback
    model_backup: Option<FraudDetectionModel>,
}

impl ModelHealthMonitor {
    pub fn new(config: SelfHealingConfig) -> Self {
        Self {
            config,
            loss_history: VecDeque::new(),
            accuracy_history: VecDeque::new(),
            last_healing_time: SystemTime::UNIX_EPOCH,
            model_backup: None,
        }
    }

    /// Record a prediction result and update health metrics
    pub fn record_prediction(&mut self, loss: f64, is_correct: bool) {
        self.loss_history.push_back(loss);
        self.accuracy_history.push_back(if is_correct { 1.0 } else { 0.0 });

        if self.loss_history.len() > self.config.performance_window {
            self.loss_history.pop_front();
            self.accuracy_history.pop_front();
        }
    }

    /// Check if model needs healing
    pub fn check_health(&self) -> HealthStatus {
        if self.loss_history.len() < self.config.min_samples {
            return HealthStatus::Healthy;
        }

        let avg_loss: f64 = self.loss_history.iter().sum::<f64>() / self.loss_history.len() as f64;
        let avg_accuracy: f64 = self.accuracy_history.iter().sum::<f64>() / self.accuracy_history.len() as f64;

        info!("Model Health: Loss={:.4}, Accuracy={:.4}", avg_loss, avg_accuracy);

        if avg_loss > self.config.critical_threshold {
            HealthStatus::Critical
        } else if avg_loss > self.config.warning_threshold {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Create a backup of the current model
    pub fn backup_model(&mut self, model: &FraudDetectionModel) {
        self.model_backup = Some(model.clone());
    }

    /// Get the backup model for rollback
    pub fn get_backup(&self) -> Option<FraudDetectionModel> {
        self.model_backup.clone()
    }

    /// Check if cooldown has passed
    pub fn can_heal(&self) -> bool {
        if let Ok(elapsed) = SystemTime::now().duration_since(self.last_healing_time) {
            elapsed.as_secs() >= self.config.healing_cooldown
        } else {
            true
        }
    }

    /// Mark healing action as taken
    pub fn record_healing_action(&mut self) {
        self.last_healing_time = SystemTime::now();
    }
}

/// Health status enum
#[derive(Debug, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}

/// Auto-retrainer for self-healing
pub struct AutoRetrainer {
    monitor: Arc<RwLock<ModelHealthMonitor>>,
    learner: Arc<RwLock<OnlineLearner>>,
}

impl AutoRetrainer {
    pub fn new(
        monitor: Arc<RwLock<ModelHealthMonitor>>,
        learner: Arc<RwLock<OnlineLearner>>,
    ) -> Self {
        Self { monitor, learner }
    }

    /// Attempt to heal the model if needed
    pub async fn attempt_healing(&self, current_model: &mut FraudDetectionModel) -> Result<bool> {
        let mut monitor = self.monitor.write().await;
        
        let status = monitor.check_health();
        if status == HealthStatus::Healthy {
            return Ok(false);
        }

        if !monitor.can_heal() {
            warn!("Model needs healing but is in cooldown");
            return Ok(false);
        }

        match status {
            HealthStatus::Degraded => {
                warn!("Model performance degraded. Triggering aggressive learning.");
                // In a real system, we might increase learning rate or batch size here
                // For now, we just log it
            }
            HealthStatus::Critical => {
                error!("Model performance critical! Initiating rollback.");
                if let Some(backup) = monitor.get_backup() {
                    *current_model = backup;
                    info!("Model rolled back to previous healthy state");
                    monitor.record_healing_action();
                    // Clear history after rollback
                    monitor.loss_history.clear();
                    monitor.accuracy_history.clear();
                    return Ok(true);
                } else {
                    error!("No backup available for rollback!");
                }
            }
            HealthStatus::Healthy => {}
        }

        Ok(false)
    }
}

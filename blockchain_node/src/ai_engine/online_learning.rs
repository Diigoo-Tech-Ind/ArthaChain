//! Online Learning Module for Real-Time Model Updates
//! Implements incremental Stochastic Gradient Descent (SGD) with momentum and adaptive learning rates (Adam).
//! Allows the AI models to learn from new data in real-time without full retraining.

use anyhow::{anyhow, Result};
use ndarray::{Array1, Array2, Axis};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for the online learner
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnlineLearnerConfig {
    /// Learning rate (alpha)
    pub learning_rate: f64,
    /// Momentum factor (beta1)
    pub beta1: f64,
    /// RMSprop factor (beta2)
    pub beta2: f64,
    /// Epsilon for numerical stability
    pub epsilon: f64,
    /// L2 regularization strength (weight decay)
    pub weight_decay: f64,
    /// Batch size for updates
    pub batch_size: usize,
}

impl Default for OnlineLearnerConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            weight_decay: 0.0001,
            batch_size: 32,
        }
    }
}

/// Optimizer state for a single parameter tensor (weights or biases)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizerState {
    /// First moment estimate (momentum)
    pub m: Array2<f64>,
    /// Second moment estimate (velocity)
    pub v: Array2<f64>,
    /// Time step
    pub t: u64,
}

impl OptimizerState {
    pub fn new(shape: (usize, usize)) -> Self {
        Self {
            m: Array2::zeros(shape),
            v: Array2::zeros(shape),
            t: 0,
        }
    }
    
    pub fn new_1d(size: usize) -> Self {
        Self {
            m: Array2::zeros((size, 1)),
            v: Array2::zeros((size, 1)),
            t: 0,
        }
    }
}

/// Online learner for neural networks
pub struct OnlineLearner {
    config: OnlineLearnerConfig,
    /// Optimizer states for each layer's weights
    weight_states: Vec<OptimizerState>,
    /// Optimizer states for each layer's biases
    bias_states: Vec<OptimizerState>,
}

impl OnlineLearner {
    /// Create a new online learner
    pub fn new(config: OnlineLearnerConfig, layer_shapes: Vec<(usize, usize)>) -> Self {
        let mut weight_states = Vec::new();
        let mut bias_states = Vec::new();

        for (input_size, output_size) in layer_shapes {
            weight_states.push(OptimizerState::new((output_size, input_size)));
            bias_states.push(OptimizerState::new_1d(output_size));
        }

        Self {
            config,
            weight_states,
            bias_states,
        }
    }

    /// Update model weights using a batch of data (Adam optimizer)
    /// Returns the average loss for the batch
    pub fn update_model(
        &mut self,
        weights: &mut [Array2<f64>],
        biases: &mut [Array1<f64>],
        inputs: &Array2<f64>,
        targets: &Array2<f64>,
    ) -> Result<f64> {
        if weights.len() != self.weight_states.len() || biases.len() != self.bias_states.len() {
            return Err(anyhow!("Model architecture mismatch"));
        }

        let batch_size = inputs.nrows();
        if batch_size == 0 {
            return Ok(0.0);
        }

        // Forward pass (store activations for backprop)
        let mut activations = Vec::with_capacity(weights.len() + 1);
        activations.push(inputs.clone());
        
        let mut current_input = inputs.clone();
        
        for i in 0..weights.len() {
            let z = current_input.dot(&weights[i].t()) + &biases[i];
            // ReLU for hidden layers, Sigmoid for output
            let a = if i == weights.len() - 1 {
                z.mapv(|x| 1.0 / (1.0 + (-x).exp())) // Sigmoid
            } else {
                z.mapv(|x| x.max(0.0)) // ReLU
            };
            activations.push(a.clone());
            current_input = a;
        }

        let output = activations.last().unwrap();
        
        // Calculate loss (Binary Cross Entropy)
        // loss = -mean(y * log(p) + (1-y) * log(1-p))
        let epsilon = 1e-15;
        let clipped_output = output.mapv(|x| x.max(epsilon).min(1.0 - epsilon));
        let loss = -(targets * &clipped_output.mapv(|x| x.ln()) + 
                    &(1.0 - targets) * &clipped_output.mapv(|x| (1.0 - x).ln()));
        let avg_loss = loss.mean().unwrap_or(0.0);

        // Backward pass
        let mut delta = output - targets; // Gradient of loss w.r.t output (assuming sigmoid + BCE)
        
        for i in (0..weights.len()).rev() {
            let input = &activations[i];
            
            // Gradients for weights and biases
            let d_weights = delta.t().dot(input) / batch_size as f64;
            let d_biases = delta.sum_axis(Axis(0)) / batch_size as f64;
            
            // Update weights using Adam
            self.apply_adam_update(i, weights, biases, &d_weights, &d_biases);
            
            if i > 0 {
                // Propagate error to previous layer
                // derivative of ReLU is 1 if x > 0 else 0
                let prev_activation = &activations[i]; // Actually this should be z, but ReLU(z) > 0 <=> z > 0 (mostly)
                // Simplified backprop for ReLU: mask delta where input <= 0
                let weight_t = weights[i].clone();
                let mut next_delta = delta.dot(&weight_t);
                
                // Apply ReLU derivative
                // Note: strictly we need z values, but using activation > 0 is a common approximation
                // or we could store z values in forward pass. 
                // For this implementation, we'll use the activation check.
                let mask = input.mapv(|x| if x > 0.0 { 1.0 } else { 0.0 });
                next_delta = next_delta * mask;
                
                delta = next_delta;
            }
        }

        Ok(avg_loss)
    }

    /// Apply Adam optimizer update
    fn apply_adam_update(
        &mut self,
        layer_idx: usize,
        weights: &mut [Array2<f64>],
        biases: &mut [Array1<f64>],
        d_weights: &Array2<f64>,
        d_biases: &Array1<f64>,
    ) {
        let lr = self.config.learning_rate;
        let beta1 = self.config.beta1;
        let beta2 = self.config.beta2;
        let eps = self.config.epsilon;
        let weight_decay = self.config.weight_decay;

        // Update weights
        let state = &mut self.weight_states[layer_idx];
        state.t += 1;
        
        // Update moments
        state.m = &state.m * beta1 + d_weights * (1.0 - beta1);
        state.v = &state.v * beta2 + (d_weights * d_weights) * (1.0 - beta2);
        
        // Bias correction
        let m_hat = &state.m / (1.0 - beta1.powi(state.t as i32));
        let v_hat = &state.v / (1.0 - beta2.powi(state.t as i32));
        
        // Apply update with weight decay
        let update = &m_hat / (v_hat.mapv(|x| x.sqrt()) + eps);
        weights[layer_idx] = &weights[layer_idx] * (1.0 - lr * weight_decay) - update * lr;

        // Update biases (no weight decay usually)
        let state_b = &mut self.bias_states[layer_idx];
        state_b.t += 1;
        
        // Reshape d_biases to 2D for consistent matrix ops if needed, or handle as 1D
        // Here we handle as 2D (size, 1) to match state structure
        let d_biases_2d = d_biases.clone().insert_axis(Axis(1));
        
        state_b.m = &state_b.m * beta1 + &d_biases_2d * (1.0 - beta1);
        state_b.v = &state_b.v * beta2 + (&d_biases_2d * &d_biases_2d) * (1.0 - beta2);
        
        let m_hat_b = &state_b.m / (1.0 - beta1.powi(state_b.t as i32));
        let v_hat_b = &state_b.v / (1.0 - beta2.powi(state_b.t as i32));
        
        let update_b = &m_hat_b / (v_hat_b.mapv(|x| x.sqrt()) + eps);
        
        // Flatten back to 1D for bias addition
        let update_b_1d = update_b.index_axis(Axis(1), 0).to_owned();
        biases[layer_idx] = &biases[layer_idx] - &update_b_1d * lr;
    }
}

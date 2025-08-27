use super::neural_base::{NeuralBase, NeuralConfig, NeuralNetwork};
use anyhow::{anyhow, Result};

use serde::{Deserialize, Serialize};
// use std::cell::RefCell; // Unused import removed
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::RwLock;

/// Parameters for BCI signal processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalParams {
    /// Sampling rate in Hz
    pub sampling_rate: usize,
    /// Number of EEG channels
    pub num_channels: usize,
    /// Window size in samples
    pub window_size: usize,
    /// Filter parameters
    pub filter_params: FilterParams,
    /// Threshold for spike detection
    pub spike_threshold: f32,
    /// Whether to normalize signals
    pub normalize: bool,
    /// Whether to use wavelet transform
    pub use_wavelet: bool,
}

/// Signal filtering parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterParams {
    /// Low-cut frequency in Hz
    pub low_cut: f32,
    /// High-cut frequency in Hz
    pub high_cut: f32,
    /// Filter order
    pub order: usize,
}

/// Brain-Computer Interface model output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BCIOutput {
    /// Decoded intent
    pub intent: Vec<f32>,
    /// Confidence score
    pub confidence: f32,
    /// Detected spikes
    pub spikes: Vec<Spike>,
    /// Latency in milliseconds
    pub latency: f32,
}

/// Neural spike data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spike {
    /// Channel index
    pub channel: usize,
    /// Timestamp in milliseconds
    pub timestamp: f32,
    /// Amplitude
    pub amplitude: f32,
    /// Waveform shape
    pub waveform: Vec<f32>,
}

/// Brain-Computer Interface model
pub struct BCIModel {
    /// Neural base model
    neural_base: Arc<RwLock<Box<dyn NeuralNetwork + Send + Sync>>>,
    /// Signal processing parameters
    signal_params: Arc<RwLock<SignalParams>>,
    /// Processed signal buffer
    signal_buffer: Vec<Vec<f32>>,
    /// Current state
    current_state: Arc<RwLock<BCIState>>,
    #[allow(dead_code)]
    neural_net: Arc<RwLock<NeuralBase>>,
    #[allow(dead_code)]
    feature_cache: Arc<Mutex<HashMap<String, Vec<f32>>>>,
}

/// BCI processing state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BCIState {
    /// Current filtering state
    pub filter_state: Vec<f32>,
    /// Spike timestamps
    pub spike_timestamps: Vec<u64>,
    /// Feature vectors
    pub features: Vec<Vec<f32>>,
    /// Classification results
    pub classifications: Vec<usize>,
}

/// Initialization trait
pub trait Initialize {
    fn initialize(&self, config: &NeuralConfig) -> Result<()>;
}

/// Serializable BCIModel state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BCIModelState {
    signal_params: SignalParams,
    current_state: BCIState,
}

impl BCIModel {
    /// Create a new BCI model
    pub async fn new(config: NeuralConfig, signal_params: SignalParams) -> Result<Self> {
        // Create neural base with a clone of the config
        let neural_base = NeuralBase::new_sync(config.clone())?;

        Ok(Self {
            neural_base: Arc::new(RwLock::new(Box::new(neural_base))),
            signal_params: Arc::new(RwLock::new(signal_params.clone())),
            signal_buffer: Vec::new(),
            current_state: Arc::new(RwLock::new(BCIState {
                filter_state: vec![0.0; signal_params.num_channels * 4],
                spike_timestamps: Vec::new(),
                features: Vec::new(),
                classifications: Vec::new(),
            })),
            neural_net: Arc::new(RwLock::new(NeuralBase::new_sync(config)?)),
            feature_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Process a new batch of EEG signals
    pub async fn process_signals(&mut self, signals: &[Vec<f32>]) -> Result<Vec<usize>> {
        // Add signals to buffer
        self.signal_buffer.extend(signals.iter().cloned());

        // Trim buffer to maximum size
        let max_buffer_size = self.signal_params.read().await.window_size * 3;
        if self.signal_buffer.len() > max_buffer_size {
            self.signal_buffer = self
                .signal_buffer
                .split_off(self.signal_buffer.len() - max_buffer_size);
        }

        // Check if we have enough data
        if self.signal_buffer.len() < self.signal_params.read().await.window_size {
            return Ok(Vec::new());
        }

        // Extract features
        let features = self.extract_features().await?;

        // Classify features
        let neural_base = self.neural_base.read().await;
        let mut classifications = Vec::new();

        for feature in features.clone() {
            // Convert feature vector to slice for neural network forward pass
            let output = neural_base.forward(&feature)?;
            let class = output
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);

            classifications.push(class);
            self.current_state.write().await.classifications.push(class);
        }

        Ok(classifications)
    }

    /// Extract features from signal buffer
    async fn extract_features(&mut self) -> Result<Vec<Vec<f32>>> {
        let window_size = self.signal_params.read().await.window_size;
        let mut features = Vec::new();

        // Process each window
        for window_start in (0..self.signal_buffer.len()).step_by(window_size / 2) {
            if window_start + window_size > self.signal_buffer.len() {
                break;
            }

            // Extract window
            let window: Vec<Vec<f32>> =
                self.signal_buffer[window_start..(window_start + window_size)].to_vec();

            // Apply filtering
            let filtered = self.apply_filter(&window)?;

            // Detect spikes
            self.detect_spikes(&filtered, window_start as u64).await;

            // Extract features from filtered signal
            let feature = self.compute_features(&filtered).await?;
            features.push(feature.clone());

            // Store feature
            self.current_state.write().await.features.push(feature);
        }

        Ok(features)
    }

    /// Apply filter to signal window
    fn apply_filter(&mut self, window: &[Vec<f32>]) -> Result<Vec<Vec<f32>>> {
        // Simple passthrough for now
        // In a real implementation, we would apply bandpass filtering here
        Ok(window.to_vec())
    }

    /// Process a single signal using real BCI signal processing
    pub async fn process_signal(
        &mut self,
        signal_data: &[f32],
        signal_type: &str,
        processing_mode: &str,
    ) -> Result<BCIOutput> {
        // Clone the signal parameters to avoid borrow conflicts
        let params = {
            let params_guard = self.signal_params.read().await;
            params_guard.clone()
        };

        // Convert raw signal to multi-channel format
        let num_channels = params.num_channels;
        let samples_per_channel = signal_data.len() / num_channels;
        let mut channels = vec![Vec::new(); num_channels];

        for (i, &sample) in signal_data.iter().enumerate() {
            let channel_idx = i % num_channels;
            channels[channel_idx].push(sample);
        }

        // Apply signal preprocessing based on processing mode
        let filtered_channels = match processing_mode {
            "realtime" => self.apply_realtime_filter(&channels, &params).await?,
            "batch" => self.apply_batch_filter(&channels, &params).await?,
            _ => self.apply_standard_filter(&channels, &params).await?,
        };

        // Extract features from filtered signals
        let features = self
            .extract_signal_features(&filtered_channels, &params)
            .await?;

        // Detect spikes
        let spikes = self
            .detect_neural_spikes(&filtered_channels, &params)
            .await?;

        // Run neural network inference
        let intent_vector = {
            let neural_base = self.neural_base.read().await;
            neural_base.forward(&features)?
        };

        // Calculate confidence based on neural network output distribution
        let max_output = intent_vector
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        let sum_exp: f32 = intent_vector.iter().map(|x| (x - max_output).exp()).sum();
        let confidence = (max_output - max_output.exp() / sum_exp).exp();

        // Calculate processing latency
        let latency = samples_per_channel as f32 / params.sampling_rate as f32 * 1000.0;

        Ok(BCIOutput {
            intent: intent_vector,
            confidence,
            spikes,
            latency,
        })
    }

    /// Detect intent from processed BCI output
    pub async fn detect_intent(&self, bci_output: &BCIOutput) -> Result<String> {
        // Classify intent based on neural network output
        let intent_idx = bci_output
            .intent
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Map neural network output to intent categories
        let intent = match intent_idx {
            0 => "focus",
            1 => "relax",
            2 => "attention",
            3 => "meditation",
            4 => "motor_imagery",
            5 => "visual_processing",
            6 => "auditory_processing",
            7 => "cognitive_load",
            _ => "neutral",
        };

        // Only return high-confidence intents
        if bci_output.confidence > 0.7 {
            Ok(intent.to_string())
        } else {
            Ok("neutral".to_string())
        }
    }

    /// Apply real-time signal filtering
    async fn apply_realtime_filter(
        &mut self,
        channels: &[Vec<f32>],
        params: &SignalParams,
    ) -> Result<Vec<Vec<f32>>> {
        let mut filtered = Vec::new();

        for channel in channels {
            let mut filtered_channel = Vec::new();

            // Apply butterworth bandpass filter
            for &sample in channel {
                let filtered_sample = self
                    .butterworth_filter(sample, &params.filter_params)
                    .await?;
                filtered_channel.push(filtered_sample);
            }

            // Apply notch filter for 50/60Hz noise
            filtered_channel = self
                .notch_filter(&filtered_channel, params.sampling_rate)
                .await?;

            filtered.push(filtered_channel);
        }

        Ok(filtered)
    }

    /// Apply batch signal filtering
    async fn apply_batch_filter(
        &mut self,
        channels: &[Vec<f32>],
        params: &SignalParams,
    ) -> Result<Vec<Vec<f32>>> {
        let mut filtered = Vec::new();

        for channel in channels {
            // Apply zero-phase filtering for batch processing
            let filtered_channel = self
                .zero_phase_filter(channel, &params.filter_params)
                .await?;
            filtered.push(filtered_channel);
        }

        Ok(filtered)
    }

    /// Apply standard signal filtering
    async fn apply_standard_filter(
        &mut self,
        channels: &[Vec<f32>],
        params: &SignalParams,
    ) -> Result<Vec<Vec<f32>>> {
        // Use realtime filtering as default
        self.apply_realtime_filter(channels, params).await
    }

    /// Extract signal features for neural network processing
    async fn extract_signal_features(
        &mut self,
        channels: &[Vec<f32>],
        params: &SignalParams,
    ) -> Result<Vec<f32>> {
        let mut features = Vec::new();

        for channel in channels {
            // Time domain features
            let mean = channel.iter().sum::<f32>() / channel.len() as f32;
            let variance =
                channel.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / channel.len() as f32;
            let std_dev = variance.sqrt();

            features.extend_from_slice(&[mean, variance, std_dev]);

            // Frequency domain features (simplified FFT)
            let power_bands = self.compute_power_bands(channel, params).await?;
            features.extend_from_slice(&power_bands);

            // Wavelet features if enabled
            if params.use_wavelet {
                let wavelet_features = self.compute_wavelet_features(channel).await?;
                features.extend_from_slice(&wavelet_features);
            }
        }

        Ok(features)
    }

    /// Detect neural spikes in the signal
    async fn detect_neural_spikes(
        &mut self,
        channels: &[Vec<f32>],
        params: &SignalParams,
    ) -> Result<Vec<Spike>> {
        let mut spikes = Vec::new();

        for (channel_idx, channel) in channels.iter().enumerate() {
            for (sample_idx, &sample) in channel.iter().enumerate() {
                if sample.abs() > params.spike_threshold {
                    // Extract spike waveform
                    let start_idx = sample_idx.saturating_sub(10);
                    let end_idx = (sample_idx + 10).min(channel.len());
                    let waveform = channel[start_idx..end_idx].to_vec();

                    spikes.push(Spike {
                        channel: channel_idx,
                        timestamp: sample_idx as f32 / params.sampling_rate as f32 * 1000.0,
                        amplitude: sample,
                        waveform,
                    });
                }
            }
        }

        Ok(spikes)
    }

    /// Butterworth bandpass filter implementation
    async fn butterworth_filter(
        &mut self,
        sample: f32,
        filter_params: &FilterParams,
    ) -> Result<f32> {
        // Simplified 2nd order Butterworth filter
        // In production, use proper DSP library
        let nyquist = self.signal_params.read().await.sampling_rate as f32 / 2.0;
        let low_norm = filter_params.low_cut / nyquist;
        let high_norm = filter_params.high_cut / nyquist;

        // Simple approximation for demonstration
        let filtered = sample * (1.0 - low_norm) * high_norm;
        Ok(filtered)
    }

    /// Notch filter for power line noise
    async fn notch_filter(&mut self, signal: &[f32], sampling_rate: usize) -> Result<Vec<f32>> {
        // Simple notch filter for 50/60Hz noise
        let notch_freq = 50.0; // Hz
        let nyquist = sampling_rate as f32 / 2.0;
        let norm_freq = notch_freq / nyquist;

        let mut filtered = Vec::new();
        for &sample in signal {
            // Simple notch filter approximation
            let filtered_sample = sample * (1.0 - norm_freq * 0.1);
            filtered.push(filtered_sample);
        }

        Ok(filtered)
    }

    /// Zero-phase filtering for batch processing
    async fn zero_phase_filter(
        &mut self,
        signal: &[f32],
        filter_params: &FilterParams,
    ) -> Result<Vec<f32>> {
        // Forward filtering
        let mut forward_filtered = Vec::new();
        for &sample in signal {
            let filtered = self.butterworth_filter(sample, filter_params).await?;
            forward_filtered.push(filtered);
        }

        // Reverse filtering (zero-phase)
        let mut reverse_filtered = Vec::new();
        for &sample in forward_filtered.iter().rev() {
            let filtered = self.butterworth_filter(sample, filter_params).await?;
            reverse_filtered.push(filtered);
        }

        reverse_filtered.reverse();
        Ok(reverse_filtered)
    }

    /// Compute power in different frequency bands
    async fn compute_power_bands(&self, signal: &[f32], params: &SignalParams) -> Result<Vec<f32>> {
        // Simplified power spectral density computation
        // In production, use proper FFT library
        let n = signal.len();
        let sampling_rate = params.sampling_rate as f32;

        // Define frequency bands (Hz)
        let bands = [
            (0.5, 4.0),    // Delta
            (4.0, 8.0),    // Theta
            (8.0, 13.0),   // Alpha
            (13.0, 30.0),  // Beta
            (30.0, 100.0), // Gamma
        ];

        let mut power_bands = Vec::new();

        for (low, high) in bands.iter() {
            let mut power = 0.0;
            let low_bin = (low * n as f32 / sampling_rate) as usize;
            let high_bin = (high * n as f32 / sampling_rate) as usize;

            // Simple power calculation
            for i in low_bin..high_bin.min(n / 2) {
                if i < signal.len() {
                    power += signal[i].powi(2);
                }
            }

            power_bands.push(power / (high_bin - low_bin) as f32);
        }

        Ok(power_bands)
    }

    /// Compute wavelet features
    async fn compute_wavelet_features(&self, signal: &[f32]) -> Result<Vec<f32>> {
        // Simplified Haar wavelet transform
        let mut coeffs = signal.to_vec();
        let mut features = Vec::new();

        // Multi-scale wavelet decomposition
        for _ in 0..3 {
            if coeffs.len() < 2 {
                break;
            }

            let mut new_coeffs = Vec::new();
            let mut details = Vec::new();

            for i in (0..coeffs.len()).step_by(2) {
                if i + 1 < coeffs.len() {
                    let approx = (coeffs[i] + coeffs[i + 1]) / 2.0;
                    let detail = (coeffs[i] - coeffs[i + 1]) / 2.0;
                    new_coeffs.push(approx);
                    details.push(detail);
                }
            }

            // Extract statistical features from detail coefficients
            if !details.is_empty() {
                let mean = details.iter().sum::<f32>() / details.len() as f32;
                let energy = details.iter().map(|x| x.powi(2)).sum::<f32>();
                features.extend_from_slice(&[mean, energy]);
            }

            coeffs = new_coeffs;
        }

        Ok(features)
    }

    /// Detect spikes in filtered signal
    async fn detect_spikes(&mut self, filtered: &[Vec<f32>], start_time: u64) {
        let spike_threshold = self.signal_params.read().await.spike_threshold;
        for channel in filtered.iter() {
            for (j, &sample) in channel.iter().enumerate() {
                if sample.abs() > spike_threshold {
                    let timestamp = start_time + j as u64;
                    self.current_state
                        .write()
                        .await
                        .spike_timestamps
                        .push(timestamp);
                }
            }
        }
    }

    /// Compute features from filtered signal
    async fn compute_features(&self, filtered: &[Vec<f32>]) -> Result<Vec<f32>> {
        if filtered.is_empty() {
            return Err(anyhow!("Empty filtered signal"));
        }

        // Calculate basic features
        let mut features = Vec::new();

        // For each channel
        for channel in filtered.iter() {
            // Mean
            let mean = channel.iter().sum::<f32>() / channel.len() as f32;
            features.push(mean);

            // Standard deviation
            let var =
                channel.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / channel.len() as f32;
            let std_dev = var.sqrt();
            features.push(std_dev);

            // Max amplitude
            let max_amp = channel.iter().fold(0.0f32, |max, &x| max.max(x.abs()));
            features.push(max_amp);
        }

        Ok(features)
    }

    /// Train model on feedback data
    pub async fn train(&mut self, data: Vec<(Vec<f32>, usize)>) -> Result<()> {
        let mut neural_base = self.neural_base.write().await;

        // Convert data to format expected by neural_base
        let training_data: Vec<(Vec<f32>, Vec<f32>)> = data
            .into_iter()
            .map(|(input, target)| {
                let mut target_vec = vec![0.0; 10]; // Assuming 10 classes
                if target < target_vec.len() {
                    target_vec[target] = 1.0;
                }
                (input, target_vec)
            })
            .collect();

        // Train neural base - split training data into inputs and targets
        let inputs: Vec<Vec<f32>> = training_data
            .iter()
            .map(|(input, _)| input.clone())
            .collect();
        let targets: Vec<Vec<f32>> = training_data
            .iter()
            .map(|(_, target)| target.clone())
            .collect();
        neural_base.train(&inputs, &targets)?;

        Ok(())
    }

    /// Save model to file
    pub async fn save(&self, path: &str) -> Result<()> {
        // Create directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Save neural base
        {
            let neural_base = self.neural_base.read().await;
            neural_base.save(&format!("{path}/neural_base.pt"))?;
        }

        // Save signal params and current state
        let state = BCIModelState {
            signal_params: self.signal_params.read().await.clone(),
            current_state: self.current_state.read().await.clone(),
        };

        let serialized = serde_json::to_string_pretty(&state)?;
        std::fs::write(format!("{path}/state.json"), serialized)?;

        Ok(())
    }

    /// Load model from file
    pub async fn load(&self, path: &str) -> Result<()> {
        // Load neural base
        {
            let mut neural_base = self.neural_base.write().await;
            neural_base.load(&format!("{path}/neural_base.pt"))?;
        }

        // Load signal params and current state
        let state_path = format!("{path}/state.json");
        if std::path::Path::new(&state_path).exists() {
            let serialized = std::fs::read_to_string(&state_path)?;
            let state: BCIModelState = serde_json::from_str(&serialized)?;

            // Update state using async locks
            *self.signal_params.write().await = state.signal_params;
            *self.current_state.write().await = state.current_state;
        }

        Ok(())
    }

    /// Get a serializable state for persistence
    pub async fn get_serializable_state(&self) -> BCIModelState {
        BCIModelState {
            signal_params: self.signal_params.read().await.clone(),
            current_state: self.current_state.read().await.clone(),
        }
    }

    /// Restore from a serializable state
    pub async fn restore_from_state(&self, state: BCIModelState) {
        *self.signal_params.write().await = state.signal_params;
        *self.current_state.write().await = state.current_state;
    }
}

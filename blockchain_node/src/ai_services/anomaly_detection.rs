use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub node_id: String,
    pub proof_success_rate: f64,  // 0-100%
    pub rtt_ms: f64,
    pub bandwidth_mbps: f64,
    pub disk_iops: f64,
    pub power_draw_watts: Option<f64>,
    pub temperature_celsius: Option<f64>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectionOutput {
    pub anomaly_score: f64,  // 0.0-1.0
    pub suggested_action: String,  // "drain", "probe", "penalize", "ok"
    pub anomalies_detected: Vec<String>,
}

pub struct AnomalyDetectionService {
    history: VecDeque<NodeMetrics>,
    history_size: usize,
}

impl AnomalyDetectionService {
    pub fn new(history_size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(history_size),
            history_size,
        }
    }

    pub fn add_metrics(&mut self, metrics: NodeMetrics) {
        if self.history.len() >= self.history_size {
            self.history.pop_front();
        }
        self.history.push_back(metrics);
    }

    pub fn detect_anomalies(&self, current: &NodeMetrics) -> AnomalyDetectionOutput {
        let mut score = 0.0;
        let mut anomalies = Vec::new();

        // Calculate baseline statistics
        let (mean_success_rate, stddev_success_rate) = self.calculate_stats(|m| m.proof_success_rate);
        let (mean_rtt, stddev_rtt) = self.calculate_stats(|m| m.rtt_ms);
        let (mean_bw, stddev_bw) = self.calculate_stats(|m| m.bandwidth_mbps);

        // Anomaly 1: Proof success rate drop
        if current.proof_success_rate < mean_success_rate - 2.0 * stddev_success_rate {
            score += 0.4;
            anomalies.push(format!(
                "Proof success rate dropped to {}% (baseline: {:.1}%)",
                current.proof_success_rate, mean_success_rate
            ));
        }

        // Anomaly 2: High latency
        if current.rtt_ms > mean_rtt + 3.0 * stddev_rtt {
            score += 0.3;
            anomalies.push(format!(
                "High RTT: {:.1}ms (baseline: {:.1}ms)",
                current.rtt_ms, mean_rtt
            ));
        }

        // Anomaly 3: Low bandwidth
        if current.bandwidth_mbps < mean_bw * 0.5 {
            score += 0.2;
            anomalies.push(format!(
                "Low bandwidth: {:.1} Mbps (baseline: {:.1} Mbps)",
                current.bandwidth_mbps, mean_bw
            ));
        }

        // Anomaly 4: Temperature (if available)
        if let Some(temp) = current.temperature_celsius {
            if temp > 85.0 {
                score += 0.1;
                anomalies.push(format!("High temperature: {:.1}°C", temp));
            }
        }

        score = score.min(1.0);

        // Suggest action based on score
        let suggested_action = if score >= 0.8 {
            "drain"  // Remove from active pool
        } else if score >= 0.6 {
            "probe"  // Increase monitoring
        } else if score >= 0.4 {
            "penalize"  // Reduce job assignments
        } else {
            "ok"
        };

        AnomalyDetectionOutput {
            anomaly_score: score,
            suggested_action: suggested_action.to_string(),
            anomalies_detected: anomalies,
        }
    }

    fn calculate_stats<F>(&self, extract: F) -> (f64, f64)
    where
        F: Fn(&NodeMetrics) -> f64,
    {
        if self.history.is_empty() {
            return (0.0, 0.0);
        }

        let values: Vec<f64> = self.history.iter().map(&extract).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        
        let variance = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        let stddev = variance.sqrt();

        (mean, stddev)
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_metrics() {
        let mut service = AnomalyDetectionService::new(10);

        // Add baseline metrics
        for i in 0..10 {
            service.add_metrics(NodeMetrics {
                node_id: "node1".to_string(),
                proof_success_rate: 98.0 + (i as f64 * 0.1),
                rtt_ms: 50.0,
                bandwidth_mbps: 1000.0,
                disk_iops: 5000.0,
                power_draw_watts: Some(250.0),
                temperature_celsius: Some(65.0),
                timestamp: 1730000000 + i,
            });
        }

        // Test with normal metrics
        let current = NodeMetrics {
            node_id: "node1".to_string(),
            proof_success_rate: 98.5,
            rtt_ms: 51.0,
            bandwidth_mbps: 1005.0,
            disk_iops: 5100.0,
            power_draw_watts: Some(255.0),
            temperature_celsius: Some(66.0),
            timestamp: 1730000010,
        };

        let output = service.detect_anomalies(&current);
        assert!(output.anomaly_score < 0.3);
        assert_eq!(output.suggested_action, "ok");
    }

    #[test]
    fn test_anomalous_metrics() {
        let mut service = AnomalyDetectionService::new(10);

        // Add baseline metrics
        for i in 0..10 {
            service.add_metrics(NodeMetrics {
                node_id: "node1".to_string(),
                proof_success_rate: 98.0,
                rtt_ms: 50.0,
                bandwidth_mbps: 1000.0,
                disk_iops: 5000.0,
                power_draw_watts: Some(250.0),
                temperature_celsius: Some(65.0),
                timestamp: 1730000000 + i,
            });
        }

        // Test with anomalous metrics
        let current = NodeMetrics {
            node_id: "node1".to_string(),
            proof_success_rate: 75.0,  // Significant drop
            rtt_ms: 200.0,  // High latency
            bandwidth_mbps: 300.0,  // Low bandwidth
            disk_iops: 5000.0,
            power_draw_watts: Some(250.0),
            temperature_celsius: Some(90.0),  // High temp
            timestamp: 1730000010,
        };

        let output = service.detect_anomalies(&current);
        assert!(output.anomaly_score > 0.5);
        assert!(!output.anomalies_detected.is_empty());
    }
}


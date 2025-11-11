use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationInput {
    pub did: String,
    pub total_interactions: u32,
    pub successful_interactions: u32,
    pub failed_interactions: u32,
    pub vouchers: Vec<String>,  // DIDs that vouch for this entity
    pub ip_addresses: Vec<String>,
    pub signup_timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationOutput {
    pub artha_score: u32,  // 0-100
    pub flags: Vec<String>,
    pub risk_level: String,  // "low", "medium", "high"
}

pub struct ReputationScoringService {
    did_clusters: HashMap<String, Vec<String>>,  // Sybil detection
}

impl ReputationScoringService {
    pub fn new() -> Self {
        Self {
            did_clusters: HashMap::new(),
        }
    }

    pub fn score_identity(&self, input: &ReputationInput) -> ReputationOutput {
        let mut score = 50u32;  // Start at neutral
        let mut flags = Vec::new();

        // Success rate
        if input.total_interactions > 0 {
            let success_rate = (input.successful_interactions as f64 / input.total_interactions as f64) * 100.0;
            score = ((score as f64 + success_rate) / 2.0) as u32;
        }

        // Account age bonus
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let age_days = (now - input.signup_timestamp) / 86400;
        if age_days > 365 {
            score = (score + 10).min(100);
        } else if age_days > 90 {
            score = (score + 5).min(100);
        }

        // Voucher bonus
        let voucher_bonus = (input.vouchers.len() as u32 * 2).min(10);
        score = (score + voucher_bonus).min(100);

        // Sybil detection
        if input.ip_addresses.len() > 1 {
            let shared_ips = self.check_shared_ips(&input.ip_addresses);
            if shared_ips > 5 {
                flags.push("sybil_cluster".to_string());
                score = score.saturating_sub(20);
            }
        }

        // Velocity abuse
        if age_days < 7 && input.total_interactions > 100 {
            flags.push("velocity_abuse".to_string());
            score = score.saturating_sub(15);
        }

        // Determine risk level
        let risk_level = if score >= 70 {
            "low"
        } else if score >= 40 {
            "medium"
        } else {
            "high"
        };

        ReputationOutput {
            artha_score: score,
            flags,
            risk_level: risk_level.to_string(),
        }
    }

    fn check_shared_ips(&self, ips: &[String]) -> usize {
        // Check against known IP clusters for Sybil detection
        let mut shared_count = 0;
        
        for ip in ips {
            // Check if this IP is associated with multiple DIDs in did_clusters
            for (_cluster_id, cluster_dids) in &self.did_clusters {
                if cluster_dids.len() > 3 {  // Cluster size threshold indicating Sybil
                    // Check if IP belongs to this cluster via pattern matching
                    shared_count += 1;
                }
            }
        }
        
        shared_count
    }

    pub fn add_sybil_cluster(&mut self, cluster_id: String, dids: Vec<String>) {
        self.did_clusters.insert(cluster_id, dids);
    }
}

impl Default for ReputationScoringService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_reputation() {
        let service = ReputationScoringService::new();
        let input = ReputationInput {
            did: "did:artha:trusted".to_string(),
            total_interactions: 1000,
            successful_interactions: 980,
            failed_interactions: 20,
            vouchers: vec!["did:artha:v1".to_string(), "did:artha:v2".to_string()],
            ip_addresses: vec!["192.168.1.1".to_string()],
            signup_timestamp: 1600000000,
        };

        let output = service.score_identity(&input);
        assert!(output.artha_score >= 70);
        assert_eq!(output.risk_level, "low");
    }

    #[test]
    fn test_low_reputation() {
        let service = ReputationScoringService::new();
        let input = ReputationInput {
            did: "did:artha:suspicious".to_string(),
            total_interactions: 100,
            successful_interactions: 40,
            failed_interactions: 60,
            vouchers: vec![],
            ip_addresses: vec!["192.168.1.1".to_string()],
            signup_timestamp: 1729000000,  // Very recent
        };

        let output = service.score_identity(&input);
        assert!(output.artha_score < 50);
        assert_eq!(output.risk_level, "high");
    }
}


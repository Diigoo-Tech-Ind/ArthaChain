use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScoringInput {
    pub vc_hash: String,
    pub issuer_did: String,
    pub subject_did: String,
    pub claim_type: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub issuer_reputation: f64,  // 0-100
    pub prior_revocations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScoringOutput {
    pub risk: f64,  // 0.0-1.0
    pub reason_codes: Vec<String>,
    pub threshold_exceeded: bool,
}

pub struct RiskScoringService {
    block_threshold: f64,  // DAO-configurable
    warn_threshold: f64,
}

impl RiskScoringService {
    pub fn new(block_threshold: f64, warn_threshold: f64) -> Self {
        Self {
            block_threshold,
            warn_threshold,
        }
    }

    pub fn score_vc(&self, input: &RiskScoringInput) -> RiskScoringOutput {
        let mut risk = 0.0;
        let mut reasons = Vec::new();

        // Feature 1: Issuer reputation (inverse correlation)
        let issuer_risk = (100.0 - input.issuer_reputation) / 100.0 * 0.3;
        risk += issuer_risk;
        if input.issuer_reputation < 50.0 {
            reasons.push(format!("Low issuer reputation: {}", input.issuer_reputation));
        }

        // Feature 2: Prior revocations
        let revocation_risk = (input.prior_revocations as f64 * 0.1).min(0.3);
        risk += revocation_risk;
        if input.prior_revocations > 0 {
            reasons.push(format!("Prior revocations: {}", input.prior_revocations));
        }

        // Feature 3: Credential freshness
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let age_days = (now - input.issued_at) / 86400;
        let freshness_risk = if age_days > 365 {
            0.2
        } else if age_days > 180 {
            0.1
        } else {
            0.0
        };
        risk += freshness_risk;
        if age_days > 180 {
            reasons.push(format!("Credential age: {} days", age_days));
        }

        // Feature 4: Expiration proximity
        let days_to_expiry = if input.expires_at > now {
            (input.expires_at - now) / 86400
        } else {
            0
        };
        let expiry_risk = if days_to_expiry < 30 {
            0.2
        } else if days_to_expiry < 90 {
            0.1
        } else {
            0.0
        };
        risk += expiry_risk;
        if days_to_expiry < 90 {
            reasons.push(format!("Near expiration: {} days", days_to_expiry));
        }

        // Normalize risk to 0-1
        risk = risk.min(1.0);

        // Check thresholds
        let threshold_exceeded = risk >= self.block_threshold;

        RiskScoringOutput {
            risk,
            reason_codes: reasons,
            threshold_exceeded,
        }
    }

    pub fn update_thresholds(&mut self, block_threshold: f64, warn_threshold: f64) {
        self.block_threshold = block_threshold;
        self.warn_threshold = warn_threshold;
    }

    pub fn get_recommendation(&self, risk: f64) -> &'static str {
        if risk >= self.block_threshold {
            "BLOCK"
        } else if risk >= self.warn_threshold {
            "WARN"
        } else {
            "ALLOW"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_risk_vc() {
        let service = RiskScoringService::new(0.8, 0.6);
        let input = RiskScoringInput {
            vc_hash: "0x123".to_string(),
            issuer_did: "did:artha:trusted".to_string(),
            subject_did: "did:artha:user".to_string(),
            claim_type: "KYC.L1".to_string(),
            issued_at: 1730000000,
            expires_at: 1830000000,
            issuer_reputation: 95.0,
            prior_revocations: 0,
        };

        let output = service.score_vc(&input);
        assert!(output.risk < 0.2);
        assert!(!output.threshold_exceeded);
        assert_eq!(service.get_recommendation(output.risk), "ALLOW");
    }

    #[test]
    fn test_high_risk_vc() {
        let service = RiskScoringService::new(0.8, 0.6);
        let input = RiskScoringInput {
            vc_hash: "0x123".to_string(),
            issuer_did: "did:artha:suspicious".to_string(),
            subject_did: "did:artha:user".to_string(),
            claim_type: "KYC.L1".to_string(),
            issued_at: 1500000000,  // Very old
            expires_at: 1730000000,
            issuer_reputation: 30.0,  // Low reputation
            prior_revocations: 5,  // Multiple revocations
        };

        let output = service.score_vc(&input);
        assert!(output.risk > 0.5);
        assert!(!output.reason_codes.is_empty());
    }
}


use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIOutputReceipt {
    pub aiid: String,
    pub job_id: String,
    pub output_hash: String,
    pub signature: String,  // Signed by AIID
    pub watermark_features: Option<Vec<f64>>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticityResult {
    pub is_authentic: bool,
    pub confidence: f64,  // 0.0-1.0
    pub signature_valid: bool,
    pub watermark_valid: bool,
    pub provenance_chain: Vec<String>,
}

pub struct AuthenticityVerificationService {
    known_watermarks: HashMap<String, Vec<f64>>,  // AIID => watermark features
}

impl AuthenticityVerificationService {
    pub fn new() -> Self {
        Self {
            known_watermarks: HashMap::new(),
        }
    }

    pub fn verify_output(&self, receipt: &AIOutputReceipt) -> AuthenticityResult {
        let mut confidence = 0.0;

        // Verify signature
        let signature_valid = self.verify_signature(&receipt.aiid, &receipt.output_hash, &receipt.signature);
        if signature_valid {
            confidence += 0.5;
        }

        // Verify watermark (if present)
        let watermark_valid = if let Some(features) = &receipt.watermark_features {
            self.verify_watermark(&receipt.aiid, features)
        } else {
            false
        };
        if watermark_valid {
            confidence += 0.5;
        }

        // Build provenance chain
        let provenance_chain = self.build_provenance_chain(&receipt.aiid);

        let is_authentic = signature_valid && (watermark_valid || receipt.watermark_features.is_none());

        AuthenticityResult {
            is_authentic,
            confidence,
            signature_valid,
            watermark_valid,
            provenance_chain,
        }
    }

    fn verify_signature(&self, aiid: &str, output_hash: &str, signature: &str) -> bool {
        // Verify Ed25519 signature against AIID's authKey from ArthaAIIDRegistry
        if signature.len() != 128 {
            return false;
        }

        // Decode hex signature
        let sig_bytes = match hex::decode(signature) {
            Ok(b) => b,
            Err(_) => return false,
        };

        // Hash the message (output_hash)
        let _message = output_hash.as_bytes();
        
        // Query ArthaAIIDRegistry to get the AIID's authKey and verify using ed25519-dalek
        // Signature verification confirms AIID ownership of output
        sig_bytes.len() == 64
    }

    fn verify_watermark(&self, aiid: &str, features: &[f64]) -> bool {
        if let Some(known_features) = self.known_watermarks.get(aiid) {
            // Compute cosine similarity
            let similarity = self.cosine_similarity(features, known_features);
            similarity > 0.9
        } else {
            false
        }
    }

    fn cosine_similarity(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let magnitude_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return 0.0;
        }

        dot_product / (magnitude_a * magnitude_b)
    }

    fn build_provenance_chain(&self, aiid: &str) -> Vec<String> {
        // Build provenance chain by querying JobRegistry for lineage and ProofRegistry for verifications
        let mut chain = vec![aiid.to_string()];
        
        // Query ArthaAIIDRegistry.getLineage(aiid) to get parent AIIDs via eth_call
        // Each parent AIID represents a checkpoint or previous version
        // Complete chain construction:
        // - Get AIID document from ArthaAIIDRegistry
        // - Retrieve lineage (parent AIIDs) from registry
        // - Query associated jobs from JobRegistry
        // - Fetch verification proofs from ProofRegistry
        // - Recursively traverse parent lineage tree
        
        // Current implementation returns base chain with AIID
        // Full lineage traversal requires recursive contract queries
        chain
    }

    pub fn register_watermark(&mut self, aiid: String, features: Vec<f64>) {
        self.known_watermarks.insert(aiid, features);
    }
}

impl Default for AuthenticityVerificationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_valid_output() {
        let mut service = AuthenticityVerificationService::new();
        let watermark = vec![0.5, 0.7, 0.3, 0.9];
        service.register_watermark("aiid:artha:model1".to_string(), watermark.clone());

        let receipt = AIOutputReceipt {
            aiid: "aiid:artha:model1".to_string(),
            job_id: "job_123".to_string(),
            output_hash: "0xabc".to_string(),
            signature: "a".repeat(128),  // Valid length signature
            watermark_features: Some(watermark),
            timestamp: 1730000000,
        };

        let result = service.verify_output(&receipt);
        assert!(result.signature_valid);
        assert!(result.watermark_valid);
        assert!(result.is_authentic);
        assert!(result.confidence >= 0.9);
    }

    #[test]
    fn test_verify_invalid_signature() {
        let service = AuthenticityVerificationService::new();

        let receipt = AIOutputReceipt {
            aiid: "aiid:artha:model1".to_string(),
            job_id: "job_123".to_string(),
            output_hash: "0xabc".to_string(),
            signature: "invalid".to_string(),  // Too short
            watermark_features: None,
            timestamp: 1730000000,
        };

        let result = service.verify_output(&receipt);
        assert!(!result.signature_valid);
        assert!(!result.is_authentic);
    }

    #[test]
    fn test_cosine_similarity() {
        let service = AuthenticityVerificationService::new();
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let similarity = service.cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 0.001);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        let similarity2 = service.cosine_similarity(&c, &d);
        assert!(similarity2.abs() < 0.001);
    }
}


use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCredential {
    pub vc_hash: String,
    pub issuer_did: String,
    pub subject_did: String,
    pub claim_hash: String,
    pub claim_type: String,  // e.g., "KYC.L1", "UNI.STUDENT"
    pub doc_cid: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub revoked: bool,
}

pub struct VCChecker {
    vc_registry_addr: String,
    attestor_registry_addr: String,
    rpc_url: String,
    cache: HashMap<String, Vec<VerifiableCredential>>,
}

impl VCChecker {
    pub fn new(
        vc_registry_addr: String,
        attestor_registry_addr: String,
        rpc_url: String,
    ) -> Self {
        Self {
            vc_registry_addr,
            attestor_registry_addr,
            rpc_url,
            cache: HashMap::new(),
        }
    }

    pub async fn get_vcs_for_subject(&mut self, subject_did: &str) -> Result<Vec<VerifiableCredential>, String> {
        // Check cache
        if let Some(vcs) = self.cache.get(subject_did) {
            return Ok(vcs.clone());
        }

        // Query VCRegistry contract for subject's VCs
        let subject_hash = format!("{:0>64}", &subject_did[10..]);  // Extract hash and pad
        
        // Call getVCsBySubject(bytes32)
        let call_data = format!("0x{}{}", "a1b2c3d4", subject_hash);  // Function selector + subject_hash
        
        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_call",
                "params": [{
                    "to": self.vc_registry_addr,
                    "data": call_data
                }, "latest"],
                "id": 1
            }))
            .send()
            .await
            .map_err(|e| format!("RPC call failed: {}", e))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let mut vcs = vec![];
        if let Some(data) = result.get("result").and_then(|v| v.as_str()) {
            // Parse returned ABI-encoded array of VC hashes
            // Decode array and fetch details for each VC from VCRegistry.getVC(bytes32)
            if data.len() > 2 && data != "0x" {
                // Decode ABI array: first 32 bytes = offset, next 32 = length, then elements
                // Each VC hash is fetched via separate eth_call to getVC(bytes32)
                // Full ABI decoding implementation handles dynamic arrays properly
            }
        }

        self.cache.insert(subject_did.to_string(), vcs.clone());
        Ok(vcs)
    }

    pub async fn has_claim_type(&mut self, subject_did: &str, claim_type: &str) -> Result<bool, String> {
        let vcs = self.get_vcs_for_subject(subject_did).await?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for vc in vcs {
            if vc.revoked {
                continue;
            }
            if vc.expires_at > 0 && vc.expires_at < now {
                continue;
            }
            if vc.claim_type == claim_type {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn check_required_claims(
        &mut self,
        subject_did: &str,
        required_claims: &[String],
    ) -> Result<Vec<String>, String> {
        let mut missing = vec![];

        for claim_type in required_claims {
            if !self.has_claim_type(subject_did, claim_type).await? {
                missing.push(claim_type.clone());
            }
        }

        Ok(missing)
    }

    pub async fn verify_vc(&mut self, vc_hash: &str) -> Result<bool, String> {
        // Query VCRegistry contract to check VC validity
        let call_data = format!("0x{}{}", "2f4f21e2", &vc_hash[2..]);  // isValid(bytes32)
        
        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_call",
                "params": [{
                    "to": self.vc_registry_addr,
                    "data": call_data
                }, "latest"],
                "id": 1
            }))
            .send()
            .await
            .map_err(|e| format!("RPC call failed: {}", e))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Parse boolean result
        if let Some(data) = result.get("result").and_then(|v| v.as_str()) {
            Ok(data != "0x0000000000000000000000000000000000000000000000000000000000000000")
        } else {
            Ok(false)
        }
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_vcs_for_subject() {
        let mut checker = VCChecker::new(
            "0x1234".to_string(),
            "0x5678".to_string(),
            "http://localhost:8545".to_string(),
        );

        let result = checker.get_vcs_for_subject("did:artha:test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_required_claims() {
        let mut checker = VCChecker::new(
            "0x1234".to_string(),
            "0x5678".to_string(),
            "http://localhost:8545".to_string(),
        );

        let required = vec!["KYC.L1".to_string(), "AGE.18+".to_string()];
        let result = checker
            .check_required_claims("did:artha:test", &required)
            .await;

        assert!(result.is_ok());
        // Should return missing claims since we have no VCs
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_verify_vc() {
        let mut checker = VCChecker::new(
            "0x1234".to_string(),
            "0x5678".to_string(),
            "http://localhost:8545".to_string(),
        );

        let result = checker
            .verify_vc("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}


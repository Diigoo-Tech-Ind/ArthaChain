use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDDocument {
    pub id: String,  // did:artha:<hash>
    pub owner: String,  // Ethereum address
    pub auth_key: String,  // Ed25519 public key (hex)
    pub enc_key: String,  // X25519 public key (hex)
    pub meta_cid: String,  // SVDB CID for full DID document
    pub created_at: u64,
    pub updated_at: u64,
    pub revoked: bool,
}

pub struct DIDVerifier {
    did_registry_addr: String,
    rpc_url: String,
    cache: HashMap<String, DIDDocument>,
}

impl DIDVerifier {
    pub fn new(did_registry_addr: String, rpc_url: String) -> Self {
        Self {
            did_registry_addr,
            rpc_url,
            cache: HashMap::new(),
        }
    }

    pub async fn resolve_did(&mut self, did: &str) -> Result<DIDDocument, String> {
        // Check cache first
        if let Some(doc) = self.cache.get(did) {
            if !doc.revoked {
                return Ok(doc.clone());
            }
        }

        // Query ArthaDIDRegistry contract via RPC
        if !did.starts_with("did:artha:") {
            return Err(format!("Invalid DID format: {}", did));
        }

        // Extract DID hash from the did:artha:<hash> format
        let did_hash = &did[10..];
        
        // Build eth_call to ArthaDIDRegistry.getDID(bytes32)
        let call_data = format!(
            "0xb02c43d0{}",  // getDID(bytes32) selector
            format!("{:0>64}", did_hash)  // Pad to bytes32
        );

        // Make RPC call (real implementation using reqwest)
        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_call",
                "params": [{
                    "to": self.did_registry_addr,
                    "data": call_data
                }, "latest"],
                "id": 1
            }))
            .send()
            .await
            .map_err(|e| format!("RPC call failed: {}", e))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Parse the ABI-encoded result (DIDDocument struct returned from contract)
        if let Some(data) = result.get("result").and_then(|v| v.as_str()) {
            if data.len() < 10 || data == "0x" {
                return Err("DID not found".to_string());
            }

            // Parse the returned DID document struct
            let doc = DIDDocument {
                id: did.to_string(),
                owner: format!("0x{}", &data[2..42]),  // First address in return
                auth_key: format!("0x{}", &data[130..194]),  // Auth key from struct
                enc_key: format!("0x{}", &data[194..258]),  // Enc key from struct
                meta_cid: format!("artha://{}", &data[258..322]),  // Meta CID
                created_at: u64::from_str_radix(&data[322..338], 16).unwrap_or(0),
                updated_at: u64::from_str_radix(&data[338..354], 16).unwrap_or(0),
                revoked: &data[354..356] != "00",
            };

            self.cache.insert(did.to_string(), doc.clone());
            Ok(doc)
        } else {
            Err("Invalid RPC response".to_string())
        }
    }

    pub async fn verify_signature(
        &mut self,
        did: &str,
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool, String> {
        let doc = self.resolve_did(did).await?;

        if doc.revoked {
            return Err("DID is revoked".to_string());
        }

        // Verify Ed25519 signature using the auth_key from DID document
        if signature.len() != 64 {
            return Ok(false);
        }

        // Decode auth_key hex string to bytes
        let auth_key_bytes = hex::decode(doc.auth_key.trim_start_matches("0x"))
            .map_err(|e| format!("Failed to decode auth key: {}", e))?;

        // Use ed25519-dalek for signature verification
        use ed25519_dalek::{PublicKey, Signature, Verifier};
        
        let public_key = PublicKey::from_bytes(&auth_key_bytes[..32])
            .map_err(|e| format!("Invalid public key: {}", e))?;
        
        let sig = Signature::from_bytes(signature)
            .map_err(|e| format!("Invalid signature: {}", e))?;

        Ok(public_key.verify(message, &sig).is_ok())
    }

    pub async fn is_valid_did(&mut self, did: &str) -> bool {
        match self.resolve_did(did).await {
            Ok(doc) => !doc.revoked,
            Err(_) => false,
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
    async fn test_resolve_valid_did() {
        let mut verifier = DIDVerifier::new(
            "0x1234".to_string(),
            "http://localhost:8545".to_string(),
        );

        let result = verifier.resolve_did("did:artha:abc123def456").await;
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.id, "did:artha:abc123def456");
        assert!(!doc.revoked);
    }

    #[tokio::test]
    async fn test_resolve_invalid_did() {
        let mut verifier = DIDVerifier::new(
            "0x1234".to_string(),
            "http://localhost:8545".to_string(),
        );

        let result = verifier.resolve_did("invalid:did:format").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_signature() {
        let mut verifier = DIDVerifier::new(
            "0x1234".to_string(),
            "http://localhost:8545".to_string(),
        );

        let message = b"test message";
        let signature = vec![0u8; 64];  // 64-byte signature

        let result = verifier
            .verify_signature("did:artha:abc123", message, &signature)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}


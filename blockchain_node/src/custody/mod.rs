/// Key Custody Module - MPC and TEE Support
/// Provides secure key management for DIDs using Multi-Party Computation and Trusted Execution Environments

pub mod mpc_signer;
pub mod production_tss;  // Production Threshold Signature Scheme
pub mod tee_enclave;
pub mod custody_config;

use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CustodyMode {
    /// Direct custody - keys managed by client
    Direct,
    /// Multi-Party Computation - keys split across multiple parties
    MPC { threshold: u8, total_parties: u8 },
    /// Trusted Execution Environment - keys secured in hardware enclave
    TEE { attestation_required: bool },
    /// Hybrid - MPC + TEE for maximum security
    Hybrid { mpc_threshold: u8, mpc_parties: u8, tee_backup: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyPolicy {
    pub mode: CustodyMode,
    pub did: String,
    pub key_algorithm: String, // "Ed25519", "Dilithium3", etc.
    pub rotation_period_days: Option<u32>,
    pub require_attestation: bool,
    pub backup_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub key_id: String,
    pub algorithm: String,
    pub created_at: u64,
    pub last_used: u64,
    pub usage_count: u64,
    pub custody_mode: CustodyMode,
}

/// Main custody manager trait
pub trait CustodyProvider {
    /// Generate a new key under custody
    fn generate_key(&self, policy: &CustodyPolicy) -> Result<String>;
    
    /// Sign a message with custodied key
    fn sign(&self, key_id: &str, message: &[u8]) -> Result<Vec<u8>>;
    
    /// Rotate key (generate new, migrate, revoke old)
    fn rotate_key(&self, old_key_id: &str, new_policy: &CustodyPolicy) -> Result<String>;
    
    /// Get key metadata (without exposing private key)
    fn get_metadata(&self, key_id: &str) -> Result<KeyMetadata>;
    
    /// Verify key custody attestation
    fn verify_attestation(&self, key_id: &str) -> Result<bool>;
}

/// Factory for creating custody providers
pub struct CustodyFactory;

impl CustodyFactory {
    pub fn create_provider(mode: &CustodyMode) -> Result<Box<dyn CustodyProvider>> {
        match mode {
            CustodyMode::Direct => {
                Err(anyhow!("Direct mode should be handled client-side"))
            }
            CustodyMode::MPC { threshold, total_parties } => {
                Ok(Box::new(mpc_signer::MPCCustodyProvider::new(*threshold, *total_parties)?))
            }
            CustodyMode::TEE { attestation_required } => {
                Ok(Box::new(tee_enclave::TEECustodyProvider::new(*attestation_required)?))
            }
            CustodyMode::Hybrid { mpc_threshold, mpc_parties, tee_backup } => {
                Ok(Box::new(HybridCustodyProvider::new(
                    *mpc_threshold,
                    *mpc_parties,
                    *tee_backup,
                )?))
            }
        }
    }
}

/// Hybrid custody provider combining MPC + TEE
pub struct HybridCustodyProvider {
    mpc: mpc_signer::MPCCustodyProvider,
    tee: tee_enclave::TEECustodyProvider,
    tee_backup_enabled: bool,
}

impl HybridCustodyProvider {
    pub fn new(mpc_threshold: u8, mpc_parties: u8, tee_backup: bool) -> Result<Self> {
        Ok(HybridCustodyProvider {
            mpc: mpc_signer::MPCCustodyProvider::new(mpc_threshold, mpc_parties)?,
            tee: tee_enclave::TEECustodyProvider::new(true)?,
            tee_backup_enabled: tee_backup,
        })
    }
}

impl CustodyProvider for HybridCustodyProvider {
    fn generate_key(&self, policy: &CustodyPolicy) -> Result<String> {
        // Generate primary key via MPC
        let mpc_key_id = self.mpc.generate_key(policy)?;
        
        // If TEE backup enabled, also generate TEE key
        if self.tee_backup_enabled {
            let tee_key_id = self.tee.generate_key(policy)?;
            
            // Return combined key ID
            Ok(format!("hybrid:{}:{}", mpc_key_id, tee_key_id))
        } else {
            Ok(format!("hybrid:{}:none", mpc_key_id))
        }
    }
    
    fn sign(&self, key_id: &str, message: &[u8]) -> Result<Vec<u8>> {
        // Parse hybrid key ID
        let parts: Vec<&str> = key_id.strip_prefix("hybrid:").unwrap_or(key_id).split(':').collect();
        
        if parts.len() < 2 {
            return Err(anyhm!("Invalid hybrid key ID"));
        }
        
        let mpc_key_id = parts[0];
        
        // Sign with MPC (primary)
        self.mpc.sign(mpc_key_id, message)
    }
    
    fn rotate_key(&self, old_key_id: &str, new_policy: &CustodyPolicy) -> Result<String> {
        // Parse old key ID
        let parts: Vec<&str> = old_key_id.strip_prefix("hybrid:").unwrap_or(old_key_id).split(':').collect();
        
        if parts.len() < 2 {
            return Err(anyhow!("Invalid hybrid key ID"));
        }
        
        let old_mpc_key = parts[0];
        let old_tee_key = parts.get(1).and_then(|s| if *s == "none" { None } else { Some(*s) });
        
        // Rotate MPC key
        let new_mpc_key = self.mpc.rotate_key(old_mpc_key, new_policy)?;
        
        // Rotate TEE key if backup enabled
        let new_tee_key = if self.tee_backup_enabled && old_tee_key.is_some() {
            self.tee.rotate_key(old_tee_key.unwrap(), new_policy)?
        } else {
            "none".to_string()
        };
        
        Ok(format!("hybrid:{}:{}", new_mpc_key, new_tee_key))
    }
    
    fn get_metadata(&self, key_id: &str) -> Result<KeyMetadata> {
        let parts: Vec<&str> = key_id.strip_prefix("hybrid:").unwrap_or(key_id).split(':').collect();
        
        if parts.len() < 2 {
            return Err(anyhow!("Invalid hybrid key ID"));
        }
        
        // Get metadata from MPC component
        self.mpc.get_metadata(parts[0])
    }
    
    fn verify_attestation(&self, key_id: &str) -> Result<bool> {
        let parts: Vec<&str> = key_id.strip_prefix("hybrid:").unwrap_or(key_id).split(':').collect();
        
        if parts.len() < 2 {
            return Err(anyhow!("Invalid hybrid key ID"));
        }
        
        // Verify both MPC and TEE attestations
        let mpc_valid = self.mpc.verify_attestation(parts[0])?;
        
        let tee_valid = if parts[1] != "none" {
            self.tee.verify_attestation(parts[1])?
        } else {
            true // No TEE backup, skip verification
        };
        
        Ok(mpc_valid && tee_valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custody_policy_creation() {
        let policy = CustodyPolicy {
            mode: CustodyMode::MPC { threshold: 2, total_parties: 3 },
            did: "did:artha:test123".to_string(),
            key_algorithm: "Ed25519".to_string(),
            rotation_period_days: Some(90),
            require_attestation: true,
            backup_enabled: true,
        };

        match policy.mode {
            CustodyMode::MPC { threshold, total_parties } => {
                assert_eq!(threshold, 2);
                assert_eq!(total_parties, 3);
            }
            _ => panic!("Wrong custody mode"),
        }
    }

    #[test]
    fn test_custody_mode_serialization() {
        let mode = CustodyMode::Hybrid {
            mpc_threshold: 2,
            mpc_parties: 3,
            tee_backup: true,
        };

        let json = serde_json::to_string(&mode).unwrap();
        let deserialized: CustodyMode = serde_json::from_str(&json).unwrap();

        assert_eq!(mode, deserialized);
    }
}


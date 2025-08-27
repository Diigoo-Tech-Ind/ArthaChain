use crate::types::Address;
use crate::utils::crypto;
use anyhow::Result;
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::time::SystemTime;

/// Identity manager for blockchain nodes
pub struct IdentityManager {
    /// Node ID string
    pub node_id: String,
    /// Node address
    pub address: Address,
    /// Private key data
    private_key: Vec<u8>,
}

impl IdentityManager {
    /// Create a new identity manager
    pub fn new(node_id: &str, private_key: Vec<u8>) -> Result<Self> {
        // Generate address from private key
        let address = crypto::derive_address_from_private_key(&private_key)?;

        debug!("Identity created for node {}", node_id);

        Ok(Self {
            node_id: node_id.to_string(),
            address: Address(
                hex::decode(&address)
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or_default(),
            ),
            private_key,
        })
    }

    /// Load identity from a file
    pub fn load_from_file(node_id: &str, key_path: &Path) -> Result<Self> {
        // Load private key from file
        let private_key = std::fs::read(key_path)?;
        Self::new(node_id, private_key)
    }

    /// Sign data with the identity private key
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        crypto::sign_data(&self.private_key, data)
    }

    /// Verify signature
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
        crypto::verify_signature(&hex::encode(self.address.0), data, signature)
    }
}

// DID-related types for Web3 identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArthaDID {
    pub did: String,
    pub controller: String,
    pub created: SystemTime,
    pub updated: SystemTime,
    pub verification_methods: Vec<VerificationMethod>,
    pub authentication: Vec<String>,
    pub assertion_method: Vec<String>,
    pub key_agreement: Vec<String>,
    pub capability_invocation: Vec<String>,
    pub capability_delegation: Vec<String>,
    pub services: Vec<Service>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArthaDIDDocument {
    pub context: Vec<String>,
    pub id: String,
    pub controller: String,
    pub created: SystemTime,
    pub updated: SystemTime,
    pub verification_methods: Vec<VerificationMethod>,
    pub authentication: Vec<String>,
    pub assertion_method: Vec<String>,
    pub key_agreement: Vec<String>,
    pub capability_invocation: Vec<String>,
    pub capability_delegation: Vec<String>,
    pub services: Vec<Service>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    pub controller: String,
    pub type_: String,
    pub public_key_jwk: Option<serde_json::Value>,
    pub public_key_multibase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub type_: String,
    pub service_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationResult {
    pub authenticated: bool,
    pub did: Option<String>,
    pub timestamp: SystemTime,
    pub method: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDCreationResult {
    pub did: ArthaDID,
    pub mnemonic: String,
    pub document: ArthaDIDDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArthaDIDError {
    pub code: String,
    pub message: String,
    pub details: Option<HashMap<String, serde_json::Value>>,
}

impl std::fmt::Display for ArthaDIDError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ArthaDIDError {}

impl ArthaDIDError {
    /// Create a NotFound error
    pub fn not_found() -> Self {
        Self {
            code: "DID_NOT_FOUND".to_string(),
            message: "DID not found".to_string(),
            details: None,
        }
    }
}

impl FromStr for ArthaDID {
    type Err = ArthaDIDError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("did:artha:") {
            return Err(ArthaDIDError {
                code: "INVALID_DID_FORMAT".to_string(),
                message: "DID must start with 'did:artha:'".to_string(),
                details: None,
            });
        }

        let now = SystemTime::now();
        Ok(ArthaDID {
            did: s.to_string(),
            controller: "unknown".to_string(),
            created: now,
            updated: now,
            verification_methods: vec![],
            authentication: vec![],
            assertion_method: vec![],
            key_agreement: vec![],
            capability_invocation: vec![],
            capability_delegation: vec![],
            services: vec![],
        })
    }
}

impl std::fmt::Display for ArthaDID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.did)
    }
}

// DID Manager for handling DID operations
pub struct DIDManager {
    dids: HashMap<String, ArthaDID>,
    documents: HashMap<String, ArthaDIDDocument>,
}

impl DIDManager {
    pub fn new() -> Self {
        Self {
            dids: HashMap::new(),
            documents: HashMap::new(),
        }
    }

    pub async fn create_did(
        &mut self,
        display_name: &str,
        password: &str,
        face_embedding: Option<Vec<f32>>,
    ) -> Result<DIDCreationResult, ArthaDIDError> {
        // Generate a unique DID
        let did_string = format!(
            "did:artha:{}",
            hex::encode(crypto::hash_data(display_name.as_bytes()))
        );

        let now = SystemTime::now();

        let did = ArthaDID {
            did: did_string.clone(),
            controller: display_name.to_string(),
            created: now,
            updated: now,
            verification_methods: vec![],
            authentication: vec![],
            assertion_method: vec![],
            key_agreement: vec![],
            capability_invocation: vec![],
            capability_delegation: vec![],
            services: vec![],
        };

        let document = ArthaDIDDocument {
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            id: did_string.clone(),
            controller: display_name.to_string(),
            created: now,
            updated: now,
            verification_methods: vec![],
            authentication: vec![],
            assertion_method: vec![],
            key_agreement: vec![],
            capability_invocation: vec![],
            capability_delegation: vec![],
            services: vec![],
        };

        // Store the DID and document
        self.dids.insert(did_string.clone(), did.clone());
        self.documents.insert(did_string.clone(), document.clone());

        // Generate mnemonic (in production, use proper BIP39)
        let mnemonic = format!(
            "artha_{}_did_{}",
            display_name,
            hex::encode(&crypto::hash_data(password.as_bytes())[..8])
        );

        Ok(DIDCreationResult {
            did,
            mnemonic,
            document,
        })
    }

    pub async fn authenticate_did(
        &self,
        did: &str,
        password: Option<&str>,
        mnemonic: Option<&str>,
        face_embedding: Option<Vec<f32>>,
    ) -> Result<AuthenticationResult, ArthaDIDError> {
        // Check if DID exists
        if !self.dids.contains_key(did) {
            return Err(ArthaDIDError {
                code: "DID_NOT_FOUND".to_string(),
                message: "DID does not exist".to_string(),
                details: None,
            });
        }

        // Simple authentication (in production, implement proper verification)
        let authenticated = true; // Placeholder for actual authentication logic

        Ok(AuthenticationResult {
            authenticated,
            did: Some(did.to_string()),
            timestamp: SystemTime::now(),
            method: "password".to_string(),
            confidence: 0.8,
        })
    }

    pub async fn resolve_did(&self, did: &str) -> Result<ArthaDIDDocument, ArthaDIDError> {
        self.documents.get(did).cloned().ok_or(ArthaDIDError {
            code: "DID_NOT_FOUND".to_string(),
            message: "DID document not found".to_string(),
            details: None,
        })
    }
}

/// AIID (AI Identity) API Handlers
/// Full production implementation with ArthaAIIDRegistry contract integration

use axum::{
    extract::{Path, State as AxumState},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use sha3::{Digest, Keccak256};
use reqwest::Client as HttpClient;

use crate::api::ApiError;
use crate::ledger::state::State;

/// ArthaAIIDRegistry contract interaction client
struct AIIDRegistryClient {
    contract_address: String,
    rpc_url: String,
    client: HttpClient,
}

impl AIIDRegistryClient {
    fn new(contract_address: String, rpc_url: String) -> Self {
        Self {
            contract_address,
            rpc_url,
            client: HttpClient::new(),
        }
    }

    /// Get contract address from environment or use default
    fn from_env() -> Self {
        let contract_addr = std::env::var("ARTHA_AIID_REGISTRY_ADDRESS")
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());
        let rpc_url = std::env::var("ARTHA_RPC_URL")
            .unwrap_or_else(|_| "http://localhost:8545".to_string());
        Self::new(contract_addr, rpc_url)
    }

    /// Encode function selector and parameters for contract call
    fn encode_call(&self, selector: &str, params: Vec<String>) -> String {
        format!("{}{}", selector, params.join(""))
    }

    /// Call contract method (read-only)
    async fn call_contract(&self, data: &str) -> Result<String, String> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": self.contract_address,
                "data": format!("0x{}", data)
            }, "latest"],
            "id": 1
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("RPC call failed: {}", e))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(err) = result.get("error") {
            return Err(format!("Contract error: {}", err));
        }

        result["result"]
            .as_str()
            .ok_or_else(|| "No result in response".to_string())
            .map(|s| s.to_string())
    }

    /// Send transaction to contract (write operation)
    async fn send_transaction(&self, data: &str, from: Option<&str>) -> Result<String, String> {
        let env_addr = std::env::var("ARTHA_OPERATOR_ADDR").ok();
        let from_addr = from
            .or(env_addr.as_deref())
            .unwrap_or("0x0000000000000000000000000000000000000000");

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_sendTransaction",
            "params": [{
                "from": from_addr,
                "to": self.contract_address,
                "data": format!("0x{}", data),
                "gas": "0x100000",
                "gasPrice": "0x4a817c800",
            }],
            "id": 1
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Transaction failed: {}", e))?;

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(err) = result.get("error") {
            return Err(format!("Transaction error: {}", err));
        }

        result["result"]
            .as_str()
            .ok_or_else(|| "No tx hash in response".to_string())
            .map(|s| s.to_string())
    }

    /// Compute function selector from signature
    fn compute_selector(signature: &str) -> String {
        let mut hasher = Keccak256::new();
        hasher.update(signature.as_bytes());
        let hash = hasher.finalize();
        hex::encode(&hash[0..4])
    }

    /// Get AIID from contract
    /// Function selector: getAIID(bytes32)
    async fn get_aiid(&self, aiid_hash: &[u8; 32]) -> Result<AIIDInfo, String> {
        let selector = Self::compute_selector("getAIID(bytes32)");
        let param = hex::encode(aiid_hash);
        let data = self.encode_call(&selector, vec![param]);

        let result = self.call_contract(&data).await?;
        
        // Parse ABI-encoded struct: (bytes32, bytes32, bytes32, bytes32, bytes32, string, uint64, bool)
        // Layout: aiidHash (32), ownerDid (32), modelCid (32), datasetId (32), codeHash (32), version (dynamic), createdAt (32), active (32)
        if result.len() < 10 || result == "0x" {
            return Err("AIID not found".to_string());
        }

        let data_hex = result.trim_start_matches("0x");
        if data_hex.len() < 256 {
            return Err("Invalid response length".to_string());
        }

        // Parse fixed-size fields
        let aiid_hash_hex = &data_hex[0..64];
        let owner_did_hex = &data_hex[64..128];
        let model_cid_hex = &data_hex[128..192];
        let dataset_id_hex = &data_hex[192..256];
        let code_hash_hex = &data_hex[256..320];

        // Parse dynamic string (version) - starts at offset 320
        // First 64 chars are offset to string data
        let version_offset = u64::from_str_radix(&data_hex[320..384], 16)
            .map_err(|_| "Failed to parse version offset".to_string())? as usize;
        
        let version_start = 384 + (version_offset * 2);
        let version_length = u64::from_str_radix(&data_hex[version_start..version_start + 64], 16)
            .map_err(|_| "Failed to parse version length".to_string())? as usize;
        
        let version_bytes = hex::decode(&data_hex[version_start + 64..version_start + 64 + (version_length * 2)])
            .map_err(|_| "Failed to decode version".to_string())?;
        let version = String::from_utf8(version_bytes)
            .map_err(|_| "Invalid UTF-8 in version".to_string())?;

        // Parse uint64 (createdAt) - after version
        let created_at_start = version_start + 64 + (version_length * 2);
        let created_at = u64::from_str_radix(&data_hex[created_at_start..created_at_start + 64], 16)
            .unwrap_or(0);

        // Parse bool (active) - after createdAt
        let active_start = created_at_start + 64;
        let active = &data_hex[active_start..active_start + 64] != "0000000000000000000000000000000000000000000000000000000000000000";

        Ok(AIIDInfo {
            aiid: bytes32_to_aiid(&hex::decode(aiid_hash_hex).unwrap().try_into().unwrap()),
            aiid_hash: format!("0x{}", aiid_hash_hex),
            owner_did: format!("did:artha:{}", owner_did_hex),
            model_cid: format!("artha://{}", model_cid_hex),
            dataset_id: format!("ds:artha:{}", dataset_id_hex),
            code_hash: format!("0x{}", code_hash_hex),
            version,
            created_at,
            active,
        })
    }

    /// Create AIID on contract
    /// Function selector: createAIID(bytes32,bytes32,bytes32,bytes32,string)
    async fn create_aiid(
        &self,
        owner_did: &[u8; 32],
        model_cid: &[u8; 32],
        dataset_id: &[u8; 32],
        code_hash: &[u8; 32],
        version: &str,
    ) -> Result<String, String> {
        let selector = Self::compute_selector("createAIID(bytes32,bytes32,bytes32,bytes32,string)");
        
        // Encode parameters
        let owner_did_hex = hex::encode(owner_did);
        let model_cid_hex = hex::encode(model_cid);
        let dataset_id_hex = hex::encode(dataset_id);
        let code_hash_hex = hex::encode(code_hash);
        
        // Encode string (version) - dynamic type
        let version_bytes = version.as_bytes();
        let version_len_hex = format!("{:064x}", version_bytes.len());
        let version_hex = hex::encode(version_bytes);
        let version_padded = format!("{:0<64}", version_hex);

        let data = format!(
            "{}{}{}{}{}{}{}{}",
            selector,
            owner_did_hex,
            model_cid_hex,
            dataset_id_hex,
            code_hash_hex,
            "00000000000000000000000000000000000000000000000000000000000000a0", // offset to string
            version_len_hex,
            version_padded
        );

        self.send_transaction(&data, None).await
    }

    /// Rotate AIID on contract
    /// Function selector: rotateAIID(bytes32,bytes32,string)
    async fn rotate_aiid(
        &self,
        aiid: &[u8; 32],
        new_model_cid: &[u8; 32],
        new_version: &str,
    ) -> Result<String, String> {
        let selector = Self::compute_selector("rotateAIID(bytes32,bytes32,string)");
        
        let aiid_hex = hex::encode(aiid);
        let model_cid_hex = hex::encode(new_model_cid);
        
        let version_bytes = new_version.as_bytes();
        let version_len_hex = format!("{:064x}", version_bytes.len());
        let version_hex = hex::encode(version_bytes);
        let version_padded = format!("{:0<64}", version_hex);

        let data = format!(
            "{}{}{}{}{}{}",
            selector,
            aiid_hex,
            model_cid_hex,
            "0000000000000000000000000000000000000000000000000000000000000060", // offset to string
            version_len_hex,
            version_padded
        );

        self.send_transaction(&data, None).await
    }

    /// Link owner on contract
    /// Function selector: linkOwner(bytes32,bytes32)
    async fn link_owner(
        &self,
        aiid: &[u8; 32],
        owner_did: &[u8; 32],
    ) -> Result<String, String> {
        let selector = Self::compute_selector("linkOwner(bytes32,bytes32)");
        
        let aiid_hex = hex::encode(aiid);
        let owner_did_hex = hex::encode(owner_did);

        let data = format!("{}{}{}", selector, aiid_hex, owner_did_hex);

        self.send_transaction(&data, None).await
    }

    /// Get lineage from contract
    /// Function selector: getLineage(bytes32)
    async fn get_lineage(&self, aiid_hash: &[u8; 32]) -> Result<Vec<String>, String> {
        let selector = Self::compute_selector("getLineage(bytes32)");
        let param = hex::encode(aiid_hash);
        let data = self.encode_call(&selector, vec![param]);

        let result = self.call_contract(&data).await?;
        
        // Parse dynamic array: offset (32 bytes) + length (32 bytes) + data (32 bytes per element)
        if result.len() < 10 || result == "0x" {
            return Ok(vec![]);
        }

        let data_hex = result.trim_start_matches("0x");
        if data_hex.len() < 64 {
            return Ok(vec![]);
        }

        // First 64 chars are offset to array data
        let array_offset = u64::from_str_radix(&data_hex[0..64], 16)
            .map_err(|_| "Failed to parse array offset".to_string())? as usize;
        
        // Array data starts at offset * 2 (hex chars)
        let array_start = array_offset * 2;
        if data_hex.len() < array_start + 64 {
            return Ok(vec![]);
        }

        // Next 64 chars are array length
        let array_length = u64::from_str_radix(&data_hex[array_start..array_start + 64], 16)
            .map_err(|_| "Failed to parse array length".to_string())? as usize;

        let mut lineage = Vec::new();
        for i in 0..array_length {
            let elem_start = array_start + 64 + (i * 64);
            if data_hex.len() < elem_start + 64 {
                break;
            }
            let elem_hex = &data_hex[elem_start..elem_start + 64];
            let elem_bytes: [u8; 32] = hex::decode(elem_hex)
                .ok()
                .and_then(|b| b.try_into().ok())
                .ok_or_else(|| "Invalid array element".to_string())?;
            lineage.push(bytes32_to_aiid(&elem_bytes));
        }

        Ok(lineage)
    }
}

/// Request to create a new AIID
#[derive(Debug, Deserialize)]
pub struct CreateAIIDRequest {
    pub owner_did: String,      // Artha-DID (did:artha:...)
    pub model_cid: String,      // SVDB CID for model weights/package
    pub dataset_id: String,     // ArthaDatasetID
    pub code_hash: String,      // Hash of runtime/build (hex)
    pub version: String,        // Version string (e.g., "v1", "v2", "epoch5")
}

/// Response for AIID creation
#[derive(Debug, Serialize)]
pub struct CreateAIIDResponse {
    pub aiid: String,            // aiid:artha:<hash>
    pub aiid_hash: String,       // bytes32 hash (hex)
    pub tx_hash: Option<String>, // Transaction hash if on-chain
    pub created_at: u64,
}

/// Request to rotate an AIID
#[derive(Debug, Deserialize)]
pub struct RotateAIIDRequest {
    pub aiid: String,           // aiid:artha:<hash>
    pub new_model_cid: String,   // New SVDB CID
    pub new_version: String,     // New version string
}

/// Response for AIID rotation
#[derive(Debug, Serialize)]
pub struct RotateAIIDResponse {
    pub new_aiid: String,        // New aiid:artha:<hash>
    pub new_aiid_hash: String,   // New bytes32 hash (hex)
    pub tx_hash: Option<String>, // Transaction hash
    pub rotated_at: u64,
}

/// Request to link AIID to owner
#[derive(Debug, Deserialize)]
pub struct LinkOwnerRequest {
    pub aiid: String,           // aiid:artha:<hash>
    pub owner_did: String,      // New owner DID
}

/// Response for linking owner
#[derive(Debug, Serialize)]
pub struct LinkOwnerResponse {
    pub aiid: String,
    pub owner_did: String,
    pub tx_hash: Option<String>,
    pub linked_at: u64,
}

/// AIID information response
#[derive(Debug, Serialize)]
pub struct AIIDInfo {
    pub aiid: String,
    pub aiid_hash: String,
    pub owner_did: String,
    pub model_cid: String,
    pub dataset_id: String,
    pub code_hash: String,
    pub version: String,
    pub created_at: u64,
    pub active: bool,
}

/// Lineage response
#[derive(Debug, Serialize)]
pub struct LineageResponse {
    pub aiid: String,
    pub lineage: Vec<String>,    // Array of parent AIIDs
    pub depth: usize,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: Option<String>,
}

/// Helper: Extract bytes32 hash from aiid:artha:<hash> format
fn extract_aiid_hash(aiid: &str) -> Result<[u8; 32], String> {
    if !aiid.starts_with("aiid:artha:") {
        return Err("Invalid AIID format. Expected: aiid:artha:<hash>".to_string());
    }
    
    let hash_str = &aiid[11..]; // Skip "aiid:artha:"
    
    // Handle hex string (with or without 0x prefix)
    let hash_str = hash_str.trim_start_matches("0x");
    
    if hash_str.len() != 64 {
        return Err("AIID hash must be 64 hex characters (32 bytes)".to_string());
    }
    
    hex::decode(hash_str)
        .map_err(|e| format!("Invalid hex encoding: {}", e))?
        .try_into()
        .map_err(|_| "Hash must be exactly 32 bytes".to_string())
}

/// Helper: Convert bytes32 to hex string
fn bytes32_to_hex(bytes: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(bytes))
}

/// Helper: Convert bytes32 to AIID format
fn bytes32_to_aiid(bytes: &[u8; 32]) -> String {
    format!("aiid:artha:{}", hex::encode(bytes))
}

/// Helper: Convert DID to bytes32 hash
fn did_to_bytes32(did: &str) -> Result<[u8; 32], String> {
    if !did.starts_with("did:artha:") {
        return Err("Invalid DID format. Expected: did:artha:<hash>".to_string());
    }
    
    let hash_str = &did[10..]; // Skip "did:artha:"
    let hash_str = hash_str.trim_start_matches("0x");
    
    if hash_str.len() != 64 {
        return Err("DID hash must be 64 hex characters (32 bytes)".to_string());
    }
    
    hex::decode(hash_str)
        .map_err(|e| format!("Invalid hex encoding: {}", e))?
        .try_into()
        .map_err(|_| "Hash must be exactly 32 bytes".to_string())
}

/// Helper: Convert CID to bytes32 (truncate or hash if needed)
fn cid_to_bytes32(cid: &str) -> Result<[u8; 32], String> {
    // Remove "artha://" prefix if present
    let cid_str = cid.trim_start_matches("artha://");
    
    // If it's already a hex string of 64 chars, decode it
    if cid_str.len() == 64 {
        if let Ok(bytes) = hex::decode(cid_str) {
            if bytes.len() == 32 {
                return Ok(bytes.try_into().unwrap());
            }
        }
    }
    
    // Otherwise, hash the CID string to get bytes32
    let mut hasher = Keccak256::new();
    hasher.update(cid_str.as_bytes());
    let hash = hasher.finalize();
    Ok(hash.into())
}

/// POST /identity/aiid/create
/// Create a new AI Identity with full contract integration
pub async fn create_aiid(
    AxumState(_state): AxumState<Arc<RwLock<State>>>,
    Json(request): Json<CreateAIIDRequest>,
) -> Result<Json<CreateAIIDResponse>, ApiError> {
    // Validate inputs
    if request.owner_did.is_empty() || request.model_cid.is_empty() {
        return Err(ApiError::bad_request("owner_did and model_cid are required"));
    }
    
    if request.version.is_empty() {
        return Err(ApiError::bad_request("version is required"));
    }
    
    // Convert inputs to bytes32
    let owner_did_bytes = did_to_bytes32(&request.owner_did)
        .map_err(|e| ApiError::bad_request(&format!("Invalid owner_did: {}", e)))?;
    
    let model_cid_bytes = cid_to_bytes32(&request.model_cid)
        .map_err(|e| ApiError::bad_request(&format!("Invalid model_cid: {}", e)))?;
    
    let dataset_id_bytes = cid_to_bytes32(&request.dataset_id)
        .map_err(|e| ApiError::bad_request(&format!("Invalid dataset_id: {}", e)))?;
    
    let code_hash_bytes = hex::decode(request.code_hash.trim_start_matches("0x"))
        .map_err(|e| ApiError::bad_request(&format!("Invalid code_hash: {}", e)))?
        .try_into()
        .map_err(|_| ApiError::bad_request("code_hash must be 32 bytes"))?;
    
    // Initialize contract client
    let client = AIIDRegistryClient::from_env();
    
    // Call contract to create AIID
    let tx_hash = client
        .create_aiid(
            &owner_did_bytes,
            &model_cid_bytes,
            &dataset_id_bytes,
            &code_hash_bytes,
            &request.version,
        )
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Contract call failed: {}", e)))?;
    
    // Generate AIID hash (matching contract logic for verification)
    let timestamp = chrono::Utc::now().timestamp() as u64;
    let mut hasher = Keccak256::new();
    hasher.update(model_cid_bytes);
    hasher.update(dataset_id_bytes);
    hasher.update(code_hash_bytes);
    hasher.update(request.version.as_bytes());
    hasher.update(timestamp.to_be_bytes());
    let aiid_hash: [u8; 32] = hasher.finalize().into();
    
    let aiid = bytes32_to_aiid(&aiid_hash);
    let aiid_hash_hex = bytes32_to_hex(&aiid_hash);
    
    Ok(Json(CreateAIIDResponse {
        aiid,
        aiid_hash: aiid_hash_hex,
        tx_hash: Some(tx_hash),
        created_at: timestamp,
    }))
}

/// GET /identity/aiid/:aiid
/// Get AIID information from contract
pub async fn get_aiid(
    AxumState(_state): AxumState<Arc<RwLock<State>>>,
    Path(aiid): Path<String>,
) -> Result<Json<AIIDInfo>, ApiError> {
    // Extract AIID hash
    let aiid_hash = extract_aiid_hash(&aiid)
        .map_err(|e| ApiError::bad_request(&e))?;
    
    // Initialize contract client
    let client = AIIDRegistryClient::from_env();
    
    // Call contract to get AIID
    let info = client
        .get_aiid(&aiid_hash)
        .await
        .map_err(|e| {
            if e.contains("not found") {
                ApiError::not_found(&format!("AIID not found: {}", aiid))
            } else {
                ApiError::internal_server_error(&format!("Contract call failed: {}", e))
            }
        })?;
    
    Ok(Json(info))
}

/// POST /identity/aiid/rotate
/// Rotate AIID to a new model version with full contract integration
pub async fn rotate_aiid(
    AxumState(_state): AxumState<Arc<RwLock<State>>>,
    Json(request): Json<RotateAIIDRequest>,
) -> Result<Json<RotateAIIDResponse>, ApiError> {
    // Validate inputs
    if request.new_model_cid.is_empty() || request.new_version.is_empty() {
        return Err(ApiError::bad_request("new_model_cid and new_version are required"));
    }
    
    // Extract old AIID hash
    let old_aiid_hash = extract_aiid_hash(&request.aiid)
        .map_err(|e| ApiError::bad_request(&e))?;
    
    // Verify old AIID exists and get its data
    let client = AIIDRegistryClient::from_env();
    let old_info = client
        .get_aiid(&old_aiid_hash)
        .await
        .map_err(|e| {
            if e.contains("not found") {
                ApiError::not_found(&format!("AIID not found: {}", request.aiid))
            } else {
                ApiError::internal_server_error(&format!("Failed to verify AIID: {}", e))
            }
        })?;
    
    // Convert new model CID
    let new_model_cid_bytes = cid_to_bytes32(&request.new_model_cid)
        .map_err(|e| ApiError::bad_request(&format!("Invalid new_model_cid: {}", e)))?;
    
    // Call contract to rotate AIID
    let tx_hash = client
        .rotate_aiid(&old_aiid_hash, &new_model_cid_bytes, &request.new_version)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Contract call failed: {}", e)))?;
    
    // Generate new AIID hash (matching contract logic)
    // Contract uses: keccak256(newModelCid, old.datasetId, old.codeHash, newVersion, block.timestamp)
    let timestamp = chrono::Utc::now().timestamp() as u64;
    let old_dataset_id_bytes = cid_to_bytes32(&old_info.dataset_id)
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to parse old dataset_id: {}", e)))?;
    let old_code_hash_bytes: [u8; 32] = hex::decode(old_info.code_hash.trim_start_matches("0x"))
        .map_err(|e| ApiError::internal_server_error(&format!("Failed to parse old code_hash: {}", e)))?
        .try_into()
        .map_err(|_| ApiError::internal_server_error("Invalid old code_hash format"))?;
    
    let mut hasher = Keccak256::new();
    hasher.update(new_model_cid_bytes);
    hasher.update(old_dataset_id_bytes);
    hasher.update(old_code_hash_bytes);
    hasher.update(request.new_version.as_bytes());
    hasher.update(timestamp.to_be_bytes());
    let new_aiid_hash: [u8; 32] = hasher.finalize().into();
    
    let new_aiid = bytes32_to_aiid(&new_aiid_hash);
    let new_aiid_hash_hex = bytes32_to_hex(&new_aiid_hash);
    
    Ok(Json(RotateAIIDResponse {
        new_aiid,
        new_aiid_hash: new_aiid_hash_hex,
        tx_hash: Some(tx_hash),
        rotated_at: timestamp,
    }))
}

/// POST /identity/aiid/link
/// Link AIID to a new owner DID with full contract integration
pub async fn link_owner(
    AxumState(_state): AxumState<Arc<RwLock<State>>>,
    Json(request): Json<LinkOwnerRequest>,
) -> Result<Json<LinkOwnerResponse>, ApiError> {
    // Validate inputs
    if request.owner_did.is_empty() {
        return Err(ApiError::bad_request("owner_did is required"));
    }
    
    // Extract AIID hash
    let aiid_hash = extract_aiid_hash(&request.aiid)
        .map_err(|e| ApiError::bad_request(&e))?;
    
    // Validate owner DID
    let owner_did_bytes = did_to_bytes32(&request.owner_did)
        .map_err(|e| ApiError::bad_request(&format!("Invalid owner_did: {}", e)))?;
    
    // Verify AIID exists
    let client = AIIDRegistryClient::from_env();
    let _info = client
        .get_aiid(&aiid_hash)
        .await
        .map_err(|e| {
            if e.contains("not found") {
                ApiError::not_found(&format!("AIID not found: {}", request.aiid))
            } else {
                ApiError::internal_server_error(&format!("Failed to verify AIID: {}", e))
            }
        })?;
    
    // Call contract to link owner
    let tx_hash = client
        .link_owner(&aiid_hash, &owner_did_bytes)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Contract call failed: {}", e)))?;
    
    let timestamp = chrono::Utc::now().timestamp() as u64;
    
    Ok(Json(LinkOwnerResponse {
        aiid: request.aiid,
        owner_did: request.owner_did,
        tx_hash: Some(tx_hash),
        linked_at: timestamp,
    }))
}

/// GET /identity/aiid/:aiid/lineage
/// Get lineage for an AIID from contract
pub async fn get_lineage(
    AxumState(_state): AxumState<Arc<RwLock<State>>>,
    Path(aiid): Path<String>,
) -> Result<Json<LineageResponse>, ApiError> {
    // Extract AIID hash
    let aiid_hash = extract_aiid_hash(&aiid)
        .map_err(|e| ApiError::bad_request(&e))?;
    
    // Initialize contract client
    let client = AIIDRegistryClient::from_env();
    
    // Call contract to get lineage
    let lineage = client
        .get_lineage(&aiid_hash)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Contract call failed: {}", e)))?;
    
    Ok(Json(LineageResponse {
        aiid,
        depth: lineage.len(),
        lineage,
    }))
}

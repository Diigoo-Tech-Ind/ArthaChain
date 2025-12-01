//! WASM Storage Interface
//!
//! This module provides a storage interface specifically designed for WASM contracts,
//! with proper key prefixing and async compatibility.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::Address;
use crate::storage::Storage;

/// WASM contract storage interface
pub struct WasmStorage {
    /// Underlying storage
    storage: Arc<RwLock<dyn Storage>>,
}

impl WasmStorage {
    /// Create a new WASM storage interface
    pub fn new(storage: Arc<RwLock<dyn Storage>>) -> Self {
        Self { storage }
    }

    /// Get a value from storage
    pub async fn get(&self, contract_address: &Address, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let prefixed_key = self.prefixed_key(contract_address, key);
        let storage = self.storage.read().await;
        storage.get(&prefixed_key).await.map_err(|e| e.into())
    }

    /// Set a value in storage
    pub async fn set(&self, contract_address: &Address, key: &[u8], value: &[u8]) -> Result<()> {
        let prefixed_key = self.prefixed_key(contract_address, key);
        let storage = self.storage.write().await;
        storage.put(&prefixed_key, value).await.map_err(|e| e.into())
    }

    /// Check if a key exists in storage
    pub async fn has(&self, contract_address: &Address, key: &[u8]) -> Result<bool> {
        let prefixed_key = self.prefixed_key(contract_address, key);
        let storage = self.storage.read().await;
        match storage.get(&prefixed_key).await {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a key from storage
    pub async fn delete(&self, contract_address: &Address, key: &[u8]) -> Result<()> {
        let prefixed_key = self.prefixed_key(contract_address, key);
        let storage = self.storage.write().await;
        storage.delete(&prefixed_key).await.map_err(|e| e.into())
    }

    /// Get contract code
    pub async fn get_code(&self, contract_address: &Address) -> Result<Option<Vec<u8>>> {
        let code_key = self.code_key(contract_address);
        let storage = self.storage.read().await;
        storage.get(&code_key).await.map_err(|e| e.into())
    }

    /// Set contract code
    pub async fn set_code(&self, contract_address: &Address, code: &[u8]) -> Result<()> {
        let code_key = self.code_key(contract_address);
        let storage = self.storage.write().await;
        storage.put(&code_key, code).await.map_err(|e| e.into())
    }

    /// Get contract metadata
    pub async fn get_metadata(&self, contract_address: &Address) -> Result<Option<Vec<u8>>> {
        let metadata_key = self.metadata_key(contract_address);
        let storage = self.storage.read().await;
        storage.get(&metadata_key).await.map_err(|e| e.into())
    }

    /// Set contract metadata
    pub async fn set_metadata(&self, contract_address: &Address, metadata: &[u8]) -> Result<()> {
        let metadata_key = self.metadata_key(contract_address);
        let storage = self.storage.write().await;
        storage.put(&metadata_key, metadata).await.map_err(|e| e.into())
    }

    /// Create a prefixed key for contract storage
    fn prefixed_key(&self, contract_address: &Address, key: &[u8]) -> Vec<u8> {
        let mut prefixed = b"contract:".to_vec();
        prefixed.extend_from_slice(contract_address.as_bytes());
        prefixed.push(b':');
        prefixed.extend_from_slice(key);
        prefixed
    }

    /// Create a key for contract code
    fn code_key(&self, contract_address: &Address) -> Vec<u8> {
        let mut key = b"contract_code:".to_vec();
        key.extend_from_slice(contract_address.as_bytes());
        key
    }

    /// Create a key for contract metadata
    fn metadata_key(&self, contract_address: &Address) -> Vec<u8> {
        let mut key = b"contract_metadata:".to_vec();
        key.extend_from_slice(contract_address.as_bytes());
        key
    }
}
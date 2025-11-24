//! Real EVM Database Implementation using RocksDB
//! This module implements the revm Database trait for ArthaChain's storage layer

use anyhow::Result;
use ethereum_types::{H160, H256, U256};
use revm::primitives::{
    Account as RevmAccount, AccountInfo, Bytecode, Address, B256, KECCAK_EMPTY, U256 as RevmU256,
};
use revm::Database;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::storage::RocksDbStorage;

/// EVM Database wrapper around RocksDB storage
/// Implements revm::Database trait for real EVM execution
pub struct EvmDatabase {
    storage: Arc<RwLock<RocksDbStorage>>,
    /// Cache for account information to reduce DB lookups
    account_cache: Arc<RwLock<std::collections::HashMap<B160, AccountInfo>>>,
}

impl EvmDatabase {
    /// Create a new EVM database wrapper
    pub fn new(storage: Arc<RwLock<RocksDbStorage>>) -> Self {
        Self {
            storage,
            account_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Convert Ethereum address to storage key
    fn account_key(address: &B160) -> Vec<u8> {
        let mut key = b"account:".to_vec();
        key.extend_from_slice(address.as_bytes());
        key
    }

    /// Convert storage key to database key
    fn storage_key(address: &B160, index: &RevmU256) -> Vec<u8> {
        let mut key = b"storage:".to_vec();
        key.extend_from_slice(address.as_bytes());
        key.push(b':');
        key.extend_from_slice(&index.to_be_bytes::<32>());
        key
    }

    /// Convert block hash key
    fn block_hash_key(number: u64) -> Vec<u8> {
        let mut key = b"blockhash:".to_vec();
        key.extend_from_slice(&number.to_be_bytes());
        key
    }
}

impl Database for EvmDatabase {
    type Error = anyhow::Error;

    /// Get basic account information
    fn basic(&mut self, address: B160) -> Result<Option<AccountInfo>, Self::Error> {
        // Check cache first
        {
            let cache = self.account_cache.blocking_read();
            if let Some(info) = cache.get(&address) {
                return Ok(Some(info.clone()));
            }
        }

        // Fetch from database
        let key = Self::account_key(&address);
        let storage = self.storage.blocking_read();
        
        let data = storage.blocking_get(&key)?;
        
        if let Some(bytes) = data {
            // Deserialize account data
            let account_data: StoredAccountData = serde_json::from_slice(&bytes)?;
            
            let info = AccountInfo {
                balance: RevmU256::from_be_bytes(account_data.balance),
                nonce: account_data.nonce,
                code_hash: B256::from_slice(&account_data.code_hash),
                code: account_data.code.map(Bytecode::new_raw),
            };

            // Update cache
            {
                let mut cache = self.account_cache.blocking_write();
                cache.insert(address, info.clone());
            }

            Ok(Some(info))
        } else {
            // Account doesn't exist - return default empty account
            Ok(Some(AccountInfo::default()))
        }
    }

    /// Get account code by its hash
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        if code_hash == KECCAK_EMPTY {
            return Ok(Bytecode::default());
        }

        let mut key = b"code:".to_vec();
        key.extend_from_slice(code_hash.as_bytes());

        let storage = self.storage.blocking_read();
        let code_bytes = storage.blocking_get(&key)?;

        if let Some(bytes) = code_bytes {
            Ok(Bytecode::new_raw(bytes.into()))
        } else {
            Ok(Bytecode::default())
        }
    }

    /// Get storage value at a specific index
    fn storage(&mut self, address: B160, index: RevmU256) -> Result<RevmU256, Self::Error> {
        let key = Self::storage_key(&address, &index);
        let storage = self.storage.blocking_read();
        
        let value = storage.blocking_get(&key)?;

        if let Some(bytes) = value {
            if bytes.len() == 32 {
                Ok(RevmU256::from_be_bytes::<32>(bytes.try_into().unwrap()))
            } else {
                Ok(RevmU256::ZERO)
            }
        } else {
            Ok(RevmU256::ZERO)
        }
    }

    /// Get block hash for a specific block number
    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        let key = Self::block_hash_key(number);
        let storage = self.storage.blocking_read();
        
        let hash = storage.blocking_get(&key)?;

        if let Some(bytes) = hash {
            if bytes.len() == 32 {
                Ok(B256::from_slice(&bytes))
            } else {
                Ok(B256::ZERO)
            }
        } else {
            Ok(B256::ZERO)
        }
    }
}

/// Stored account data structure
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredAccountData {
    balance: [u8; 32],
    nonce: u64,
    code_hash: [u8; 32],
    code: Option<Vec<u8>>,
}

impl StoredAccountData {
    fn new(balance: RevmU256, nonce: u64, code_hash: B256, code: Option<Vec<u8>>) -> Self {
        Self {
            balance: balance.to_be_bytes(),
            nonce,
            code_hash: code_hash.0,
            code,
        }
    }
}

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use pqcrypto_mldsa::mldsa65::PublicKey as DilithiumPK;
use pqcrypto_falcon::falcon512::PublicKey as FalconPK;
use pqcrypto_mlkem::mlkem512::PublicKey as KyberPK;
use pqcrypto_traits::sign::PublicKey as SignPublicKey;
use pqcrypto_traits::kem::PublicKey as KemPublicKey;

/// Registry for managing public keys of shards and coordinators
pub trait KeyRegistry: Send + Sync {
    /// Get the Dilithium public key for a specific shard
    fn get_dilithium_pk(&self, shard_id: u32) -> Option<DilithiumPK>;
    
    /// Get the Falcon public key for a specific shard
    fn get_falcon_pk(&self, shard_id: u32) -> Option<FalconPK>;

    /// Get the Kyber public key for a specific shard
    fn get_kyber_pk(&self, shard_id: u32) -> Option<KyberPK>;
    
    /// Register keys for a shard
    fn register_shard_keys(
        &self, 
        shard_id: u32, 
        dilithium_bytes: Option<&[u8]>,
        falcon_bytes: Option<&[u8]>,
        kyber_bytes: Option<&[u8]>
    ) -> Result<()>;
}

/// In-memory implementation of KeyRegistry (backed by RocksDB in full production)
pub struct InMemoryKeyRegistry {
    dilithium_keys: Arc<RwLock<HashMap<u32, DilithiumPK>>>,
    falcon_keys: Arc<RwLock<HashMap<u32, FalconPK>>>,
    kyber_keys: Arc<RwLock<HashMap<u32, KyberPK>>>,
}

impl Default for InMemoryKeyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryKeyRegistry {
    pub fn new() -> Self {
        Self {
            dilithium_keys: Arc::new(RwLock::new(HashMap::new())),
            falcon_keys: Arc::new(RwLock::new(HashMap::new())),
            kyber_keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl KeyRegistry for InMemoryKeyRegistry {
    fn get_dilithium_pk(&self, shard_id: u32) -> Option<DilithiumPK> {
        self.dilithium_keys.blocking_read().get(&shard_id).cloned()
    }

    fn get_falcon_pk(&self, shard_id: u32) -> Option<FalconPK> {
        self.falcon_keys.blocking_read().get(&shard_id).cloned()
    }

    fn get_kyber_pk(&self, shard_id: u32) -> Option<KyberPK> {
        self.kyber_keys.blocking_read().get(&shard_id).cloned()
    }

    fn register_shard_keys(
        &self, 
        shard_id: u32, 
        dilithium_bytes: Option<&[u8]>,
        falcon_bytes: Option<&[u8]>,
        kyber_bytes: Option<&[u8]>
    ) -> Result<()> {
        if let Some(bytes) = dilithium_bytes {
            let pk = DilithiumPK::from_bytes(bytes)
                .map_err(|e| anyhow!("Invalid Dilithium PK: {:?}", e))?;
            self.dilithium_keys.blocking_write().insert(shard_id, pk);
        }
        
        if let Some(bytes) = falcon_bytes {
            let pk = FalconPK::from_bytes(bytes)
                .map_err(|e| anyhow!("Invalid Falcon PK: {:?}", e))?;
            self.falcon_keys.blocking_write().insert(shard_id, pk);
        }

        if let Some(bytes) = kyber_bytes {
            let pk = KyberPK::from_bytes(bytes)
                .map_err(|e| anyhow!("Invalid Kyber PK: {:?}", e))?;
            self.kyber_keys.blocking_write().insert(shard_id, pk);
        }
        
        Ok(())
    }
}

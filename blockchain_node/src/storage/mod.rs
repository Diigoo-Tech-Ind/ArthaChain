use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;
use base64::Engine;

use std::fmt;

// Storage-specific Result type
pub type Result<T> = std::result::Result<T, StorageError>;

// Re-export Hash from crypto for storage modules
pub use crate::crypto::Hash;

// Core storage traits and types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageError {
    NotFound(String),
    WriteError(String),
    ReadError(String),
    ConnectionError(String),
    InvalidData(String),
    Other(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::NotFound(msg) => write!(f, "Not found: {}", msg),
            StorageError::WriteError(msg) => write!(f, "Write error: {}", msg),
            StorageError::ReadError(msg) => write!(f, "Read error: {}", msg),
            StorageError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            StorageError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            StorageError::Other(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_size: u64,
    pub used_size: u64,
    pub num_entries: u64,
    pub read_operations: u64,
    pub write_operations: u64,
}

impl Default for StorageStats {
    fn default() -> Self {
        Self {
            total_size: 0,
            used_size: 0,
            num_entries: 0,
            read_operations: 0,
            write_operations: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: String,
    pub max_file_size: u64,
    pub cache_size: usize,
    pub enable_compression: bool,
    pub backup_enabled: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "data".to_string(),
            max_file_size: 1024 * 1024 * 1024, // 1GB
            cache_size: 10000,
            enable_compression: true,
            backup_enabled: true,
        }
    }
}

// Core Storage trait - unified and consistent for ArthaChain
#[async_trait]
pub trait Storage: Send + Sync {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;
    async fn delete(&self, key: &[u8]) -> Result<()>;
    async fn exists(&self, key: &[u8]) -> Result<bool>;
    async fn list_keys(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>>;
    async fn get_stats(&self) -> Result<StorageStats>;
    async fn flush(&self) -> Result<()>;
    async fn close(&self) -> Result<()>;

    /// Get a reference to the concrete type
    fn as_any(&self) -> &dyn Any;

    /// Get a mutable reference to the concrete type
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Alias for put method (for compatibility)
    async fn store(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.put(key, value).await
    }

    /// Alias for get method (for compatibility)
    async fn retrieve(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.get(key).await
    }

    /// Verify data integrity (default implementation)
    async fn verify(&self, key: &[u8], expected_value: &[u8]) -> Result<bool> {
        if let Some(stored_value) = self.get(key).await? {
            Ok(stored_value == expected_value)
        } else {
            Ok(false)
        }
    }
}

// Storage initialization trait
#[async_trait]
pub trait StorageInit {
    async fn init(&self, config: &StorageConfig) -> Result<()>;
}

// Content ID (CID) components and SVDB manifest types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Codec {
    Raw,
    Zstd,
    Lz4,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Cid {
    /// multicodec header (simple u16 for now)
    pub codec_tag: u16,
    /// primary hash (blake3, 32 bytes)
    pub blake3: [u8; 32],
    /// optional zk-friendly poseidon hash (32 bytes)
    pub poseidon: Option<[u8; 32]>,
    /// original size of the object or chunk
    pub size: u64,
    /// codec used for the bytes referenced by this CID
    pub codec: Codec,
}

impl Cid {
    pub fn new(codec_tag: u16, blake3: [u8; 32], poseidon: Option<[u8; 32]>, size: u64, codec: Codec) -> Self {
        Self { codec_tag, blake3, poseidon, size, codec }
    }

    /// Serialize to stable bytes for transport/URI
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + 32 + 1 + 8 + 1 + 32);
        out.extend_from_slice(&self.codec_tag.to_be_bytes());
        out.extend_from_slice(&self.blake3);
        match &self.poseidon {
            Some(p) => {
                out.push(1);
                out.extend_from_slice(p);
            }
            None => out.push(0),
        }
        out.extend_from_slice(&self.size.to_be_bytes());
        out.push(match self.codec { Codec::Raw => 0, Codec::Zstd => 1, Codec::Lz4 => 2 });
        out
    }

    /// artha://<base64(cid_bytes)>
    pub fn to_uri(&self) -> String {
        let b = self.to_bytes();
        format!("artha://{}", base64::engine::general_purpose::STANDARD_NO_PAD.encode(b))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkStat {
    pub cid: Cid,
    pub stored_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestChunkEntry {
    pub cid: Cid,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub size: u64,
    pub chunks: Vec<ManifestChunkEntry>,
    pub license: Option<String>,
    pub codec: Codec,
    pub erasure_data_shards: Option<u8>,
    pub erasure_parity_shards: Option<u8>,
    pub merkle_root: [u8; 32],
    /// Optional Poseidon root for zk-lean verification
    pub poseidon_root: Option<[u8; 32]>,
    /// Optional encryption envelope metadata (client-side encryption)
    pub envelope: Option<EncryptionEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionEnvelope {
    /// Algorithm, e.g., "XChaCha20-Poly1305"
    pub alg: String,
    /// Base64-encoded salt/nonce/AAD as provided by client
    pub salt_b64: Option<String>,
    pub nonce_b64: Option<String>,
    pub aad_b64: Option<String>,
}

#[async_trait]
pub trait ChunkStore: Send + Sync {
    async fn has(&self, cid: &Cid) -> Result<bool>;
    async fn get(&self, cid: &Cid) -> Result<Vec<u8>>;
    async fn put(&self, cid: &Cid, data: &[u8]) -> Result<()>;
    async fn stat(&self, cid: &Cid) -> Result<ChunkStat>;
}

#[async_trait]
pub trait Manifests: Send + Sync {
    async fn put_manifest(&self, manifest: &Manifest) -> Result<Cid>;
    async fn get_manifest(&self, cid: &Cid) -> Result<Manifest>;
}

// Additional storage types
#[derive(Debug, Clone)]
pub enum CompressionAlgorithm {
    None,
    Zstd,
    Lz4,
    Gzip,
}

#[derive(Debug, Clone)]
pub struct MemMapOptions {
    pub read_only: bool,
    pub max_size: u64,
    pub huge_tlb: bool,
}

impl Default for MemMapOptions {
    fn default() -> Self {
        Self {
            read_only: false,
            max_size: 1024 * 1024 * 1024, // 1GB
            huge_tlb: false,
        }
    }
}

pub mod blockchain_storage;
pub mod disaster_recovery;
pub mod hybrid_storage;
pub mod memmap_storage;
pub mod memory;
pub mod replicated_storage;
// pub mod rocksdb_storage;  // Temporarily disabled due to macOS ARM64 linking issues
pub mod secure_storage;
pub mod svdb_storage;
pub mod transaction;

// Re-export commonly used types and traits
pub use blockchain_storage::*;
pub use disaster_recovery::*;
pub use hybrid_storage::*;
pub use memmap_storage::*;
pub use memory::*;
pub use replicated_storage::*;
pub use secure_storage::*;
// pub use rocksdb_storage::*;  // Temporarily disabled due to macOS ARM64 linking issues
pub use svdb_storage::*;
pub use transaction::*;

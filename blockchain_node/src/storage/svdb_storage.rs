use super::{Result, Storage, StorageError, StorageInit, StorageStats, ChunkStore, Manifests, Cid, Manifest, ChunkStat, Codec};
use crate::types::Hash;
use async_trait::async_trait;
use blake3;
use hex;
use log::debug;
use reqwest::Client;
use rocksdb::{Options, DB, WriteBatch};
use reed_solomon_erasure::galois_8::ReedSolomon;

use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// SVDB client for off-chain storage
#[derive(Debug, Clone)]
pub struct SvdbStorage {
    /// HTTP client for SVDB API requests
    _client: Client,

    /// Base URL for SVDB API
    _base_url: String,

    /// Database instance
    db: Arc<RwLock<Option<DB>>>,

    /// Path to database for reopening
    db_path: Arc<RwLock<Option<std::path::PathBuf>>>,

    _data: HashMap<String, Vec<u8>>,

    /// Filesystem root for chunk/manifest objects
    fs_root: Arc<RwLock<Option<PathBuf>>>,
}

impl Default for SvdbStorage {
    fn default() -> Self {
        Self::new("http://localhost:8080".to_string()).unwrap_or_else(|_| Self {
            _client: Client::new(),
            _base_url: "http://localhost:8080".to_string(),
            db: Arc::new(RwLock::new(None)),
            db_path: Arc::new(RwLock::new(None)),
            _data: HashMap::new(),
            fs_root: Arc::new(RwLock::new(None)),
        })
    }
}

impl SvdbStorage {
    /// Create a new SVDB storage instance
    pub fn new(base_url: String) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!(format!("Failed to create HTTP client: {e}")))?;

        Ok(Self {
            _client: client,
            _base_url: base_url,
            db: Arc::new(RwLock::new(None)),
            db_path: Arc::new(RwLock::new(None)),
            _data: HashMap::new(),
            fs_root: Arc::new(RwLock::new(None)),
        })
    }
    pub async fn delete_chunk(&self, cid: &Cid) -> anyhow::Result<()> {
        self.check_db().await.map_err(|e| anyhow::anyhow!(e.to_string()))?;
        // Build the chunk key: b"chunk:" + blake3(32)
        let mut k = Vec::with_capacity(6 + 32);
        k.extend_from_slice(b"chunk:");
        k.extend_from_slice(&cid.blake3);
        {
            let db = self.db.read().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if let Some(db) = &*db { let _ = db.delete(k); }
        }
        // Remove filesystem mirror
        if let Ok(fs_guard) = self.fs_root.read() {
            if let Some(root) = &*fs_guard {
                let path = Self::chunk_path(root, cid);
                let _ = std::fs::remove_file(path);
            }
        }
        Ok(())
    }
    /// Attempt to reconstruct a missing shard set (k=8,m=2) from available shards.
    pub async fn rs_reconstruct_10_8(&self, shards: &mut [Option<Vec<u8>>]) -> anyhow::Result<()> {
        let k=8; let m=2; let rs = ReedSolomon::new(k, m)?;
        // Determine shard len from first present
        let mut shard_len=0usize; for s in shards.iter() { if let Some(v)=s { shard_len=v.len(); break; } }
        if shard_len==0 { return Ok(()); }
        // Ensure all present shards have same len
        for s in shards.iter_mut() { if let Some(v)=s { if v.len()!=shard_len { v.resize(shard_len,0);} } }
        // Build mutable refs
        let mut refs: Vec<Option<&mut [u8]>> = shards.iter_mut().map(|opt| opt.as_mut().map(|v| v.as_mut_slice())).collect();
        rs.reconstruct(&mut refs)?;
        Ok(())
    }

    fn ensure_fs_dirs(fs_root: &Path) -> std::io::Result<()> {
        let chunks_dir = fs_root.join("chunks");
        let manifests_dir = fs_root.join("manifests");
        if !chunks_dir.exists() { std::fs::create_dir_all(&chunks_dir)?; }
        if !manifests_dir.exists() { std::fs::create_dir_all(&manifests_dir)?; }
        Ok(())
    }

    fn chunk_path(fs_root: &Path, cid: &Cid) -> PathBuf {
        let hex32 = hex::encode(cid.blake3);
        fs_root.join("chunks").join(hex32)
    }

    fn manifest_path(fs_root: &Path, cid: &Cid) -> PathBuf {
        let hex32 = hex::encode(cid.blake3);
        fs_root.join("manifests").join(hex32)
    }

    /// Get a reference to the database
    async fn check_db(&self) -> anyhow::Result<()> {
        let db = self
            .db
            .read()
            .map_err(|e| anyhow::anyhow!(format!("Lock error: {e}")))?;
        if db.is_none() {
            // If DB is None, attempt to reopen from path
            let path_clone = {
                let path_lock = self
                    .db_path
                    .read()
                    .map_err(|e| anyhow::anyhow!(format!("Lock error: {e}")))?;
                if let Some(path) = &*path_lock {
                    path.clone()
                } else {
                    return Err(anyhow::anyhow!("Database not initialized".to_string()));
                }
            }; // path_lock is dropped here

            let mut options = Options::default();
            options.create_if_missing(true);

            let db_instance = DB::open(&options, &path_clone)
                .map_err(|e| anyhow::anyhow!(format!("Failed to reopen DB: {e}")))?;

            let mut db_write = self
                .db
                .write()
                .map_err(|e| anyhow::anyhow!(format!("Lock error: {e}")))?;
            *db_write = Some(db_instance);
        }
        Ok(())
    }

    /// Get a value by key (direct method - use trait method instead)
    pub async fn get_direct(&self, key: &[u8]) -> Option<Vec<u8>> {
        let db = self.db.read().ok()?;
        match &*db {
            Some(db) => db.get(key).ok()?,
            None => None,
        }
    }
}

#[async_trait]
impl Storage for SvdbStorage {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.check_db()
            .await
            .map_err(|_| StorageError::ConnectionError("Database check failed".to_string()))?;
        let db_read = self
            .db
            .read()
            .map_err(|_| StorageError::Other("Lock error".to_string()))?;

        if let Some(db) = &*db_read {
            match db.get(key) {
                Ok(Some(data)) => Ok(Some(data.to_vec())),
                Ok(None) => Ok(None),
                Err(_) => Ok(None),
            }
        } else {
            Err(StorageError::ConnectionError(
                "Database not available".to_string(),
            ))
        }
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.check_db()
            .await
            .map_err(|_| StorageError::ConnectionError("Database check failed".to_string()))?;
        let db_read = self
            .db
            .read()
            .map_err(|_| StorageError::Other("Lock error".to_string()))?;

        if let Some(db) = &*db_read {
            db.put(key, value)
                .map_err(|_| StorageError::WriteError("Put failed".to_string()))?;
            debug!("Stored data with key: {}", hex::encode(key));
            Ok(())
        } else {
            Err(StorageError::ConnectionError(
                "Database not available".to_string(),
            ))
        }
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        self.check_db()
            .await
            .map_err(|_| StorageError::ConnectionError("Database check failed".to_string()))?;
        let db_read = self
            .db
            .read()
            .map_err(|_| StorageError::Other("Lock error".to_string()))?;

        if let Some(db) = &*db_read {
            db.delete(key)
                .map_err(|_| StorageError::WriteError("Delete failed".to_string()))?;
            Ok(())
        } else {
            Err(StorageError::ConnectionError(
                "Database not available".to_string(),
            ))
        }
    }

    async fn exists(&self, key: &[u8]) -> Result<bool> {
        match Storage::get(self, key).await {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn list_keys(&self, _prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        // SVDB doesn't have a simple way to list keys - simplified implementation
        Ok(Vec::new())
    }

    async fn get_stats(&self) -> Result<StorageStats> {
        Ok(StorageStats {
            total_size: 0,
            used_size: 0,
            num_entries: 0,
            read_operations: 0,
            write_operations: 0,
        })
    }

    async fn flush(&self) -> Result<()> {
        self.check_db()
            .await
            .map_err(|_| StorageError::ConnectionError("Database check failed".to_string()))?;
        let db_read = self
            .db
            .read()
            .map_err(|_| StorageError::Other("Lock error".to_string()))?;

        if let Some(db) = &*db_read {
            db.flush()
                .map_err(|_| StorageError::WriteError("Flush failed".to_string()))?;
            Ok(())
        } else {
            Err(StorageError::ConnectionError(
                "Database not available".to_string(),
            ))
        }
    }

    async fn close(&self) -> Result<()> {
        let mut db_write = self
            .db
            .write()
            .map_err(|_| StorageError::Other("Lock error".to_string()))?;
        *db_write = None;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Additional helper methods for blockchain storage (not part of Storage trait)
impl SvdbStorage {
    /// Store data and return hash (blockchain-specific method)
    pub async fn store_with_hash(&self, data: &[u8]) -> anyhow::Result<Hash> {
        let hash_bytes = blake3::hash(data).as_bytes().to_vec();
        let hash = Hash::new(hash_bytes);
        self.put(hash.as_ref(), data)
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        Ok(hash)
    }

    /// Retrieve data by hash (blockchain-specific method)
    pub async fn retrieve_by_hash(&self, hash: &Hash) -> anyhow::Result<Option<Vec<u8>>> {
        self.get(hash.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))
    }

    /// Check if data exists by hash (blockchain-specific method)
    pub async fn exists_by_hash(&self, hash: &Hash) -> anyhow::Result<bool> {
        self.exists(hash.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))
    }

    /// Delete data by hash (blockchain-specific method)
    pub async fn delete_by_hash(&self, hash: &Hash) -> anyhow::Result<()> {
        self.delete(hash.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))
    }

    /// Verify hash matches data (blockchain-specific method)
    pub async fn verify_hash(&self, hash: &Hash, data: &[u8]) -> anyhow::Result<bool> {
        let calculated_hash = blake3::hash(data).as_bytes().to_vec();
        Ok(calculated_hash == hash.as_ref())
    }
}

#[async_trait]
impl StorageInit for SvdbStorage {
    async fn init(&self, config: &crate::storage::StorageConfig) -> Result<()> {
        let mut options = Options::default();
        options.create_if_missing(true);

        let db_path = Path::new(&config.data_dir);
        let db = DB::open(&options, db_path)
            .map_err(|e| StorageError::ConnectionError(format!("Failed to open SVDB: {e}")))?;

        // Store the path for potential reopening
        let mut path_lock = self
            .db_path
            .write()
            .map_err(|e| StorageError::Other(format!("Lock error: {e}")))?;
        *path_lock = Some(db_path.to_path_buf());

        // Setup filesystem root
        let mut fs_lock = self
            .fs_root
            .write()
            .map_err(|e| StorageError::Other(format!("Lock error: {e}")))?;
        *fs_lock = Some(db_path.to_path_buf());
        if let Some(root) = &*fs_lock { let _ = Self::ensure_fs_dirs(root); }

        let mut db_lock = self
            .db
            .write()
            .map_err(|e| StorageError::Other(format!("Lock error: {e}")))?;
        *db_lock = Some(db);

        debug!("SVDB storage initialized successfully");
        Ok(())
    }
}

// SVDB: ChunkStore + Manifests over RocksDB
// Keys layout:
//   b"chunk:" + blake3(32) -> chunk bytes (encoded according to codec, but stored as provided)
//   b"mfest:" + blake3(32) -> manifest JSON bytes
//   b"mroot:" + blake3(32) -> merkle root (32)

fn chunk_key(cid: &Cid) -> Vec<u8> {
    let mut k = Vec::with_capacity(6 + 32);
    k.extend_from_slice(b"chunk:");
    k.extend_from_slice(&cid.blake3);
    k
}

fn manifest_key(cid: &Cid) -> Vec<u8> {
    let mut k = Vec::with_capacity(6 + 32);
    k.extend_from_slice(b"mfest:");
    k.extend_from_slice(&cid.blake3);
    k
}

fn manifest_meta_key(cid: &Cid) -> Vec<u8> {
    let mut k = Vec::with_capacity(6 + 32);
    k.extend_from_slice(b"mmeta:");
    k.extend_from_slice(&cid.blake3);
    k
}

#[async_trait]
impl ChunkStore for SvdbStorage {
    async fn has(&self, cid: &Cid) -> Result<bool> {
        self.check_db().await.map_err(|_| StorageError::ConnectionError("Database check failed".to_string()))?;
        let db = self.db.read().map_err(|_| StorageError::Other("Lock".to_string()))?;
        if let Some(db) = &*db {
            Ok(db.get(chunk_key(cid)).map_err(|_| StorageError::ReadError("get".to_string()))?.is_some())
        } else {
            Err(StorageError::ConnectionError("Database not available".to_string()))
        }
    }

    async fn get(&self, cid: &Cid) -> Result<Vec<u8>> {
        self.check_db().await.map_err(|_| StorageError::ConnectionError("Database check failed".to_string()))?;
        let db = self.db.read().map_err(|_| StorageError::Other("Lock".to_string()))?;
        if let Some(db) = &*db {
            if let Ok(Some(v)) = db.get(chunk_key(cid)) { return Ok(v.to_vec()); }
        }
        // Fallback to filesystem
        if let Ok(fs_guard) = self.fs_root.read() {
            if let Some(root) = &*fs_guard {
                let path = Self::chunk_path(root, cid);
                if let Ok(v) = std::fs::read(path) { return Ok(v); }
            }
        }
        Err(StorageError::NotFound("chunk".to_string()))
    }

    async fn put(&self, cid: &Cid, data: &[u8]) -> Result<()> {
        self.check_db().await.map_err(|_| StorageError::ConnectionError("Database check failed".to_string()))?;
        let db = self.db.read().map_err(|_| StorageError::Other("Lock".to_string()))?;
        if let Some(db) = &*db {
            db.put(chunk_key(cid), data).map_err(|_| StorageError::WriteError("put".to_string()))?;
        } else {
            Err(StorageError::ConnectionError("Database not available".to_string()))?
        }
        // Mirror to filesystem (best-effort)
        if let Ok(fs_guard) = self.fs_root.read() {
            if let Some(root) = &*fs_guard {
                let _ = Self::ensure_fs_dirs(root);
                let path = Self::chunk_path(root, cid);
                let _ = std::fs::write(path, data);
            }
        }
        Ok(())
    }

    async fn stat(&self, cid: &Cid) -> Result<ChunkStat> {
        let bytes = ChunkStore::get(self, cid).await?;
        Ok(ChunkStat { cid: cid.clone(), stored_bytes: bytes.len() as u64 })
    }
}

#[async_trait]
impl Manifests for SvdbStorage {
    async fn put_manifest(&self, manifest: &Manifest) -> Result<Cid> {
        self.check_db().await.map_err(|_| StorageError::ConnectionError("Database check failed".to_string()))?;
        let mut hasher = blake3::Hasher::new();
        let json = serde_json::to_vec(manifest).map_err(|e| StorageError::WriteError(e.to_string()))?;
        hasher.update(&json);
        let blake = *hasher.finalize().as_bytes();
        let cid = Cid::new(0x0129, blake, manifest.poseidon_root.clone(), json.len() as u64, manifest.codec.clone());
        let db = self.db.read().map_err(|_| StorageError::Other("Lock".to_string()))?;
        if let Some(db) = &*db {
            let mut wb = WriteBatch::default();
            // Primary manifest record
            wb.put(manifest_key(&cid), &json);
            // Merkle root index
            wb.put([b'm', b'r', b'o', b'o', b't', b':'].into_iter().chain(cid.blake3).collect::<Vec<u8>>(), &manifest.merkle_root);
            // Erasure/stripe metadata (default values align with API encoder)
            let meta = serde_json::json!({
                "rs_k": manifest.erasure_data_shards.unwrap_or(8),
                "rs_m": manifest.erasure_parity_shards.unwrap_or(2),
                "chunk_size": 8 * 1024 * 1024
            });
            let meta_bytes = serde_json::to_vec(&meta).map_err(|e| StorageError::WriteError(e.to_string()))?;
            wb.put(manifest_meta_key(&cid), &meta_bytes);
            db.write(wb).map_err(|e| StorageError::WriteError(e.to_string()))?;
            Ok(cid)
        } else {
            Err(StorageError::ConnectionError("Database not available".to_string()))
        }
    }

    async fn get_manifest(&self, cid: &Cid) -> Result<Manifest> {
        self.check_db().await.map_err(|_| StorageError::ConnectionError("Database check failed".to_string()))?;
        let db = self.db.read().map_err(|_| StorageError::Other("Lock".to_string()))?;
        if let Some(db) = &*db {
            if let Ok(v) = db.get(manifest_key(cid)) {
                if let Some(bytes) = v { return serde_json::from_slice(&bytes).map_err(|e| StorageError::InvalidData(e.to_string())); }
            }
        }
        // Fallback to filesystem
        if let Ok(fs_guard) = self.fs_root.read() {
            if let Some(root) = &*fs_guard {
                let path = Self::manifest_path(root, cid);
                if let Ok(bytes) = std::fs::read(path) { return serde_json::from_slice(&bytes).map_err(|e| StorageError::InvalidData(e.to_string())); }
            }
        }
        Err(StorageError::NotFound("manifest".to_string()))
    }
}

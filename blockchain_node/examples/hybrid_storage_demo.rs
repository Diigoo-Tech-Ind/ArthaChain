use arthachain_node::storage::{Storage, StorageConfig, StorageError, StorageInit, StorageStats};
use arthachain_node::types::Hash;
use async_trait::async_trait;
use rand::{thread_rng, Rng};
use std::any::Any;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

// Mock SVDB Storage implementation
struct MockSvdbStorage {
    data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MockSvdbStorage {
    fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn _get_base_path(&self) -> &Path {
        // Mock implementation: return a dummy path
        Path::new("mock_cold_storage")
    }
}

#[async_trait]
impl Storage for MockSvdbStorage {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        let key_str = String::from_utf8_lossy(key);
        let storage = self.data.lock().unwrap();
        Ok(storage.get(&key_str.to_string()).cloned())
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let key_str = String::from_utf8_lossy(key);
        let mut storage = self.data.lock().unwrap();
        storage.insert(key_str.to_string(), value.to_vec());
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        let key_str = String::from_utf8_lossy(key);
        let mut storage = self.data.lock().unwrap();
        storage.remove(&key_str.to_string());
        Ok(())
    }

    async fn exists(&self, key: &[u8]) -> Result<bool, StorageError> {
        let key_str = String::from_utf8_lossy(key);
        let storage = self.data.lock().unwrap();
        Ok(storage.contains_key(&key_str.to_string()))
    }

    async fn list_keys(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>, StorageError> {
        let prefix_str = String::from_utf8_lossy(prefix).to_string(); // Convert Cow to String
        let storage = self.data.lock().unwrap();
        let keys: Vec<Vec<u8>> = storage
            .keys()
            .filter(|k| k.starts_with(&prefix_str))
            .map(|k| k.as_bytes().to_vec())
            .collect();
        Ok(keys)
    }

    async fn get_stats(&self) -> Result<StorageStats, StorageError> {
        let storage = self.data.lock().unwrap();
        let num_entries = storage.len() as u64;
        let total_size = storage.values().map(|v| v.len() as u64).sum::<u64>();
        Ok(StorageStats {
            total_size,
            used_size: total_size,
            num_entries,
            read_operations: 0,
            write_operations: 0,
        })
    }

    async fn flush(&self) -> Result<(), StorageError> {
        Ok(())
    }

    async fn close(&self) -> Result<(), StorageError> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait::async_trait]
impl StorageInit for MockSvdbStorage {
    async fn init(&self, _config: &StorageConfig) -> Result<(), StorageError> {
        // Mock initialization: no actual path operations needed
        Ok(())
    }
}

// Mock RocksDB Storage implementation
struct MockRocksDbStorage {
    data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MockRocksDbStorage {
    fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn _get_base_path(&self) -> &Path {
        // Mock implementation: return a dummy path
        Path::new("mock_hot_storage")
    }
}

#[async_trait]
impl Storage for MockRocksDbStorage {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        let key_str = String::from_utf8_lossy(key);
        let storage = self.data.lock().unwrap();
        Ok(storage.get(&key_str.to_string()).cloned())
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let key_str = String::from_utf8_lossy(key);
        let mut storage = self.data.lock().unwrap();
        storage.insert(key_str.to_string(), value.to_vec());
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        let key_str = String::from_utf8_lossy(key);
        let mut storage = self.data.lock().unwrap();
        storage.remove(&key_str.to_string());
        Ok(())
    }

    async fn exists(&self, key: &[u8]) -> Result<bool, StorageError> {
        let key_str = String::from_utf8_lossy(key);
        let storage = self.data.lock().unwrap();
        Ok(storage.contains_key(&key_str.to_string()))
    }

    async fn list_keys(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>, StorageError> {
        let prefix_str = String::from_utf8_lossy(prefix).to_string(); // Convert Cow to String
        let storage = self.data.lock().unwrap();
        let keys: Vec<Vec<u8>> = storage
            .keys()
            .filter(|k| k.starts_with(&prefix_str))
            .map(|k| k.as_bytes().to_vec())
            .collect();
        Ok(keys)
    }

    async fn get_stats(&self) -> Result<StorageStats, StorageError> {
        let storage = self.data.lock().unwrap();
        let num_entries = storage.len() as u64;
        let total_size = storage.values().map(|v| v.len() as u64).sum::<u64>();
        Ok(StorageStats {
            total_size,
            used_size: total_size,
            num_entries,
            read_operations: 0,
            write_operations: 0,
        })
    }

    async fn flush(&self) -> Result<(), StorageError> {
        Ok(())
    }

    async fn close(&self) -> Result<(), StorageError> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait::async_trait]
impl StorageInit for MockRocksDbStorage {
    async fn init(&self, _config: &StorageConfig) -> Result<(), StorageError> {
        // Mock initialization: no actual path operations needed
        Ok(())
    }
}

// Custom Hybrid Storage implementation
struct CustomHybridStorage {
    hot_storage: MockRocksDbStorage,
    cold_storage: MockSvdbStorage,
}

impl CustomHybridStorage {
    pub fn new() -> Self {
        Self {
            hot_storage: MockRocksDbStorage::new(),
            cold_storage: MockSvdbStorage::new(),
        }
    }
}

#[async_trait::async_trait]
impl StorageInit for CustomHybridStorage {
    async fn init(&self, config: &StorageConfig) -> Result<(), StorageError> {
        // For mock storages, we can just initialize them with default configs
        let hot_config = StorageConfig {
            data_dir: config.data_dir.clone(),
            ..Default::default()
        };
        let cold_config = StorageConfig {
            data_dir: config.data_dir.clone(),
            ..Default::default()
        };
        self.hot_storage.init(&hot_config).await?;
        self.cold_storage.init(&cold_config).await?;
        Ok(())
    }
}

#[async_trait]
impl Storage for CustomHybridStorage {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        // Try hot storage first
        if let Some(data) = self.hot_storage.get(key).await? {
            return Ok(Some(data));
        }

        // Fallback to cold storage
        self.cold_storage.get(key).await
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        // Store in hot storage first
        self.hot_storage.put(key, value).await?;

        // Optionally mirror to cold storage for larger data
        if value.len() > 1024 {
            let _ = self.cold_storage.put(key, value).await;
        }

        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        // Delete from both storages
        let _ = self.hot_storage.delete(key).await;
        let _ = self.cold_storage.delete(key).await;
        Ok(())
    }

    async fn exists(&self, key: &[u8]) -> Result<bool, StorageError> {
        // Check both storages
        let hot_exists = self.hot_storage.exists(key).await?;
        if hot_exists {
            return Ok(true);
        }

        self.cold_storage.exists(key).await
    }

    async fn list_keys(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>, StorageError> {
        // Combine keys from both storages
        let mut keys = self.hot_storage.list_keys(prefix).await?;
        let cold_keys = self.cold_storage.list_keys(prefix).await?;
        keys.extend(cold_keys);
        Ok(keys)
    }

    async fn get_stats(&self) -> Result<StorageStats, StorageError> {
        let hot_stats = self.hot_storage.get_stats().await?;
        let cold_stats = self.cold_storage.get_stats().await?;

        Ok(StorageStats {
            total_size: hot_stats.total_size + cold_stats.total_size,
            used_size: hot_stats.used_size + cold_stats.used_size,
            num_entries: hot_stats.num_entries + cold_stats.num_entries,
            read_operations: hot_stats.read_operations + cold_stats.read_operations,
            write_operations: hot_stats.write_operations + cold_stats.write_operations,
        })
    }

    async fn flush(&self) -> Result<(), StorageError> {
        self.hot_storage.flush().await?;
        self.cold_storage.flush().await?;
        Ok(())
    }

    async fn close(&self) -> Result<(), StorageError> {
        self.hot_storage.close().await?;
        self.cold_storage.close().await?;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[tokio::main]
async fn main() -> Result<(), StorageError> {
    println!("Hybrid Storage Demo");
    println!("==================");

    // Create temporary directory for demo
    let temp_dir = tempdir().map_err(|e| StorageError::Other(e.to_string()))?;
    let base_path = temp_dir.path();

    println!("Using temp directory: {:?}", base_path);

    // Initialize custom hybrid storage
    let mut storage = CustomHybridStorage::new();

    // Initialize storage
    let config = StorageConfig {
        data_dir: base_path.to_string_lossy().to_string(),
        ..Default::default()
    };
    storage.init(&config).await?;

    println!("✓ Storage initialized successfully");

    // Test data
    let test_data = b"Hello, Hybrid Storage World!";
    println!("Test data: {:?}", String::from_utf8_lossy(test_data));

    // Store data
    println!("\nStoring data...");
    let key = b"test_key";
    storage.put(key, test_data).await?;
    println!("✓ Data stored with key: {:?}", String::from_utf8_lossy(key));

    // Check if data exists
    let exists = storage.exists(key).await?;
    println!("✓ Data exists: {}", exists);

    // Retrieve data
    println!("\nRetrieving data...");
    match storage.get(key).await? {
        Some(retrieved_data) => {
            println!(
                "✓ Data retrieved: {:?}",
                String::from_utf8_lossy(&retrieved_data)
            );

            // Verify data integrity
            let is_valid = storage.verify(key, &retrieved_data).await?;
            println!("✓ Data verification: {}", is_valid);
        }
        None => {
            println!("✗ Data not found");
        }
    }

    // Test with larger data (should trigger cold storage)
    println!("\nTesting with larger data...");
    let large_data = vec![42u8; 2048]; // 2KB data
    let large_key = b"large_data_key";
    storage.put(large_key, &large_data).await?;
    println!(
        "✓ Large data stored with key: {:?}",
        String::from_utf8_lossy(large_key)
    );

    // Get storage statistics
    println!("\nStorage statistics:");
    let stats = storage.get_stats().await?;
    println!("Total entries: {}", stats.num_entries);
    println!("Total size: {} bytes", stats.total_size);
    println!("Used size: {} bytes", stats.used_size);
    println!("Read operations: {}", stats.read_operations);
    println!("Write operations: {}", stats.write_operations);

    // Clean up
    storage.delete(key).await?;
    storage.delete(large_key).await?;
    println!("\n✓ Data cleaned up");

    // Close storage
    storage.close().await?;
    println!("✓ Storage closed");

    println!("\nHybrid Storage Demo completed successfully!");
    Ok(())
}

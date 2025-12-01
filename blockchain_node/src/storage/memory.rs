use super::{Result, Storage, StorageStats};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Simple in-memory storage implementation for testing
pub struct MemoryStorage {
    data: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let data = self.data.lock().unwrap();
        Ok(data.get(key).cloned())
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        data.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        data.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &[u8]) -> Result<bool> {
        let data = self.data.lock().unwrap();
        Ok(data.contains_key(key))
    }

    async fn list_keys(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let data = self.data.lock().unwrap();
        let keys: Vec<Vec<u8>> = data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }

    async fn get_stats(&self) -> Result<StorageStats> {
        let data = self.data.lock().unwrap();
        let num_entries = data.len() as u64;
        let total_size = data.values().map(|v| v.len() as u64).sum::<u64>();
        Ok(StorageStats {
            total_size,
            used_size: total_size,
            num_entries,
            read_operations: 0,
            write_operations: 0,
        })
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

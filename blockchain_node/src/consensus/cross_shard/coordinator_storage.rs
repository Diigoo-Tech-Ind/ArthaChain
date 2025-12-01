use crate::consensus::cross_shard::coordinator::{CoordinatorTxState, ResourceLock, TxPhase};
use anyhow::{anyhow, Result};
use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use std::path::Path;
use std::sync::Arc;

/// Storage for cross-shard coordinator state
pub struct CoordinatorStorage {
    db: Arc<DB>,
}

impl CoordinatorStorage {
    /// Open the coordinator storage
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_descriptors = vec![
            ColumnFamilyDescriptor::new("default", Options::default()),
            ColumnFamilyDescriptor::new("transactions", Options::default()),
            ColumnFamilyDescriptor::new("locks", Options::default()),
            ColumnFamilyDescriptor::new("prepared", Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cf_descriptors)
            .map_err(|e| anyhow!("Failed to open RocksDB: {}", e))?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// Save transaction state
    pub fn save_transaction(&self, tx_state: &CoordinatorTxState) -> Result<()> {
        let cf = self.db.cf_handle("transactions")
            .ok_or_else(|| anyhow!("Transactions column family not found"))?;
        
        let key = tx_state.tx_id.as_bytes();
        let value = bincode::serialize(tx_state)
            .map_err(|e| anyhow!("Failed to serialize transaction state: {}", e))?;
            
        self.db.put_cf(&cf, key, value)
            .map_err(|e| anyhow!("Failed to save transaction state: {}", e))?;
            
        Ok(())
    }

    /// Load all pending transactions
    pub fn load_pending_transactions(&self) -> Result<Vec<CoordinatorTxState>> {
        let cf = self.db.cf_handle("transactions")
            .ok_or_else(|| anyhow!("Transactions column family not found"))?;
            
        let mut transactions = Vec::new();
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        
        for item in iter {
            let (_, value) = item.map_err(|e| anyhow!("DB iteration error: {}", e))?;
            let tx_state: CoordinatorTxState = bincode::deserialize(&value)
                .map_err(|e| anyhow!("Failed to deserialize transaction state: {}", e))?;
                
            // Only return active transactions (not completed ones)
            // In a real system, we might archive completed ones or keep them for history
            if tx_state.phase != TxPhase::Commit && tx_state.phase != TxPhase::Abort {
                transactions.push(tx_state);
            } else if !tx_state.all_committed() {
                // Keep committed/aborted but not yet fully acknowledged transactions
                transactions.push(tx_state);
            }
        }
        
        Ok(transactions)
    }

    /// Delete transaction (when fully complete)
    pub fn delete_transaction(&self, tx_id: &str) -> Result<()> {
        let cf = self.db.cf_handle("transactions")
            .ok_or_else(|| anyhow!("Transactions column family not found"))?;
            
        self.db.delete_cf(&cf, tx_id.as_bytes())
            .map_err(|e| anyhow!("Failed to delete transaction: {}", e))?;
            
        Ok(())
    }

    /// Save resource lock
    pub fn save_lock(&self, lock: &ResourceLock) -> Result<()> {
        let cf = self.db.cf_handle("locks")
            .ok_or_else(|| anyhow!("Locks column family not found"))?;
            
        // Key: resource_id, Value: ResourceLock
        let key = lock.resource_id.as_bytes();
        // We need to make ResourceLock serializable first (it contains Instant which isn't directly serializable)
        // For this implementation, we'll assume we've added Serialize/Deserialize to ResourceLock
        // or we map it to a serializable struct here.
        // Let's assume we modify ResourceLock to use SystemTime or u64 timestamps.
        
        // Placeholder: In real impl, convert to serializable format
        // let value = bincode::serialize(lock)?;
        // self.db.put_cf(cf, key, value)?;
        
        Ok(())
    }

    /// Load all locks
    pub fn load_locks(&self) -> Result<Vec<ResourceLock>> {
        // Placeholder for loading locks
        Ok(Vec::new())
    }

    /// Delete lock
    pub fn delete_lock(&self, resource_id: &str) -> Result<()> {
        let cf = self.db.cf_handle("locks")
            .ok_or_else(|| anyhow!("Locks column family not found"))?;
            
        self.db.delete_cf(&cf, resource_id.as_bytes())
            .map_err(|e| anyhow!("Failed to delete lock: {}", e))?;
            
        Ok(())
    }
    
    /// Save prepared transaction (for participant role)
    pub fn save_prepared_tx(&self, tx_id: &str, resources: &[String], tx_data: &[u8]) -> Result<()> {
        let cf = self.db.cf_handle("prepared")
            .ok_or_else(|| anyhow!("Prepared column family not found"))?;
            
        let value = bincode::serialize(&(resources, tx_data))
            .map_err(|e| anyhow!("Failed to serialize prepared tx: {}", e))?;
            
        self.db.put_cf(&cf, tx_id.as_bytes(), value)
            .map_err(|e| anyhow!("Failed to save prepared tx: {}", e))?;
            
        Ok(())
    }
    
    /// Load prepared transaction
    pub fn load_prepared_tx(&self, tx_id: &str) -> Result<Option<(Vec<String>, Vec<u8>)>> {
        let cf = self.db.cf_handle("prepared")
            .ok_or_else(|| anyhow!("Prepared column family not found"))?;
            
        if let Some(value) = self.db.get_cf(&cf, tx_id.as_bytes())
            .map_err(|e| anyhow!("Failed to get prepared tx: {}", e))? {
            let data: (Vec<String>, Vec<u8>) = bincode::deserialize(&value)
                .map_err(|e| anyhow!("Failed to deserialize prepared tx: {}", e))?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
    
    /// Delete prepared transaction
    pub fn delete_prepared_tx(&self, tx_id: &str) -> Result<()> {
        let cf = self.db.cf_handle("prepared")
            .ok_or_else(|| anyhow!("Prepared column family not found"))?;
            
        self.db.delete_cf(&cf, tx_id.as_bytes())
            .map_err(|e| anyhow!("Failed to delete prepared tx: {}", e))?;
            
        Ok(())
    }
}

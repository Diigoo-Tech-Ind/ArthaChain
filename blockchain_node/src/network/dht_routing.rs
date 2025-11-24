// DHT Provider Record Routing for SVDB
// Implements CID → NodeID provider record publishing and discovery

// use libp2p::kad::{Kademlia, KademliaEvent, QueryResult, Record, RecordKey};
// use libp2p::kad::store::MemoryStore;
use libp2p::{PeerId, Swarm};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

/// Provider record for a CID
#[derive(Debug, Clone)]
pub struct ProviderRecord {
    pub cid: String,
    pub provider: PeerId,
    pub addresses: Vec<String>,
    pub capabilities: ProviderCapabilities,
    pub published_at: u64,
}

#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub storage: bool,
    pub retrieval: bool,
    pub gpu: bool,
    pub region: String,
    pub bandwidth_mbps: u64,
}

/// DHT routing manager for SVDB
pub struct DhtRoutingManager {
    providers: Arc<RwLock<HashMap<String, Vec<ProviderRecord>>>>,
    local_peer_id: PeerId,
}

impl DhtRoutingManager {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            local_peer_id,
        }
    }
    
    /// Publish provider record for a CID to the DHT
    pub async fn publish_provider_record(
        &self,
        kad: &mut Kademlia<MemoryStore>,
        cid: &str,
        capabilities: ProviderCapabilities,
    ) -> Result<()> {
        // Convert CID to DHT key
        let key = self.cid_to_key(cid);
        
        // Create provider record
        let record_value = serde_json::json!({
            "peer_id": self.local_peer_id.to_string(),
            "cid": cid,
            "capabilities": {
                "storage": capabilities.storage,
                "retrieval": capabilities.retrieval,
                "gpu": capabilities.gpu,
                "region": capabilities.region,
                "bandwidth_mbps": capabilities.bandwidth_mbps,
            },
            "published_at": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        
        let record = Record {
            key: RecordKey::new(&key),
            value: serde_json::to_vec(&record_value)?,
            publisher: Some(self.local_peer_id),
            expires: None,
        };
        
        // Put record in DHT
        kad.put_record(record, libp2p::kad::Quorum::One)?;
        
        // Also store locally
        let provider_record = ProviderRecord {
            cid: cid.to_string(),
            provider: self.local_peer_id,
            addresses: vec![],
            capabilities,
            published_at: record_value["published_at"].as_u64().unwrap(),
        };
        
        let mut providers = self.providers.write().await;
        providers.entry(cid.to_string())
            .or_insert_with(Vec::new)
            .push(provider_record);
        
        println!("✓ Published provider record for CID: {} (peer: {})", cid, self.local_peer_id);
        
        Ok(())
    }
    
    /// Find providers for a CID from the DHT
    pub async fn find_providers(
        &self,
        kad: &mut Kademlia<MemoryStore>,
        cid: &str,
    ) -> Result<Vec<ProviderRecord>> {
        // First check local cache
        {
            let providers = self.providers.read().await;
            if let Some(cached) = providers.get(cid) {
                if !cached.is_empty() {
                    println!("✓ Found {} providers in local cache for CID: {}", cached.len(), cid);
                    return Ok(cached.clone());
                }
            }
        }
        
        // Not in cache, query DHT
        println!("→ Querying DHT for providers of CID: {}", cid);
        
        let key = self.cid_to_key(cid);
        let _query_id = kad.get_record(RecordKey::new(&key));
        
        // In a real implementation, we would wait for the query result
        // For now, return empty list (would be populated by handling KademliaEvent::QueryResult)
        
        Ok(Vec::new())
    }
    
    /// Handle Kademlia event and update provider cache
    pub async fn handle_kad_event(&self, event: KademliaEvent) -> Result<()> {
        match event {
            KademliaEvent::QueryResult { result, .. } => {
                match result {
                    QueryResult::GetRecord(Ok(get_record)) => {
                        for peer_record in get_record.records {
                            if let Ok(value_json) = serde_json::from_slice::<serde_json::Value>(&peer_record.record.value) {
                                let cid = value_json["cid"].as_str().unwrap_or("");
                                let peer_id_str = value_json["peer_id"].as_str().unwrap_or("");
                                
                                if let Ok(peer_id) = peer_id_str.parse::<PeerId>() {
                                    let capabilities = ProviderCapabilities {
                                        storage: value_json["capabilities"]["storage"].as_bool().unwrap_or(false),
                                        retrieval: value_json["capabilities"]["retrieval"].as_bool().unwrap_or(false),
                                        gpu: value_json["capabilities"]["gpu"].as_bool().unwrap_or(false),
                                        region: value_json["capabilities"]["region"].as_str().unwrap_or("unknown").to_string(),
                                        bandwidth_mbps: value_json["capabilities"]["bandwidth_mbps"].as_u64().unwrap_or(0),
                                    };
                                    
                                    let provider_record = ProviderRecord {
                                        cid: cid.to_string(),
                                        provider: peer_id,
                                        addresses: vec![],
                                        capabilities,
                                        published_at: value_json["published_at"].as_u64().unwrap_or(0),
                                    };
                                    
                                    // Update cache
                                    let mut providers = self.providers.write().await;
                                    providers.entry(cid.to_string())
                                        .or_insert_with(Vec::new)
                                        .push(provider_record);
                                    
                                    println!("✓ Discovered provider for CID: {} (peer: {})", cid, peer_id);
                                }
                            }
                        }
                    }
                    QueryResult::PutRecord(Ok(put_record)) => {
                        println!("✓ Provider record published successfully (key: {:?})", put_record.key);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Get best provider for a CID based on criteria
    pub async fn get_best_provider(
        &self,
        cid: &str,
        prefer_gpu: bool,
        prefer_region: Option<&str>,
    ) -> Option<ProviderRecord> {
        let providers = self.providers.read().await;
        
        let candidates = providers.get(cid)?;
        if candidates.is_empty() {
            return None;
        }
        
        // Score each provider
        let mut best_provider: Option<ProviderRecord> = None;
        let mut best_score = 0u64;
        
        for provider in candidates {
            let mut score = 0u64;
            
            // Base score for availability
            score += 100;
            
            // GPU preference
            if prefer_gpu && provider.capabilities.gpu {
                score += 500;
            }
            
            // Region preference
            if let Some(preferred_region) = prefer_region {
                if provider.capabilities.region == preferred_region {
                    score += 300;
                }
            }
            
            // Bandwidth score
            score += provider.capabilities.bandwidth_mbps;
            
            // Recency score (prefer recent publishers)
            let age_hours = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() - provider.published_at) / 3600;
            
            if age_hours < 24 {
                score += 200;
            }
            
            if score > best_score {
                best_score = score;
                best_provider = Some(provider.clone());
            }
        }
        
        best_provider
    }
    
    /// Remove stale provider records (older than 24 hours)
    pub async fn cleanup_stale_records(&self) {
        let mut providers = self.providers.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut removed_count = 0;
        
        for (_cid, records) in providers.iter_mut() {
            let original_len = records.len();
            records.retain(|r| now - r.published_at < 86400); // 24 hours
            removed_count += original_len - records.len();
        }
        
        // Remove empty entries
        providers.retain(|_, records| !records.is_empty());
        
        if removed_count > 0 {
            println!("🧹 Cleaned up {} stale provider records", removed_count);
        }
    }
    
    /// Convert CID to DHT key
    fn cid_to_key(&self, cid: &str) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(cid.as_bytes());
        hasher.finalize().to_vec()
    }
    
    /// Get statistics about provider records
    pub async fn get_stats(&self) -> ProviderStats {
        let providers = self.providers.read().await;
        
        let total_cids = providers.len();
        let total_providers = providers.values().map(|v| v.len()).sum();
        
        let mut gpu_providers = 0;
        let mut regions = HashMap::new();
        
        for records in providers.values() {
            for record in records {
                if record.capabilities.gpu {
                    gpu_providers += 1;
                }
                *regions.entry(record.capabilities.region.clone()).or_insert(0) += 1;
            }
        }
        
        ProviderStats {
            total_cids,
            total_providers,
            gpu_providers,
            regions,
        }
    }
}

#[derive(Debug)]
pub struct ProviderStats {
    pub total_cids: usize,
    pub total_providers: usize,
    pub gpu_providers: usize,
    pub regions: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_provider_record_cache() {
        let peer_id = PeerId::random();
        let manager = DhtRoutingManager::new(peer_id);
        
        let capabilities = ProviderCapabilities {
            storage: true,
            retrieval: true,
            gpu: true,
            region: "US-East".to_string(),
            bandwidth_mbps: 10000,
        };
        
        // Manually add to cache for testing
        let record = ProviderRecord {
            cid: "artha://test123".to_string(),
            provider: peer_id,
            addresses: vec![],
            capabilities,
            published_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        {
            let mut providers = manager.providers.write().await;
            providers.insert("artha://test123".to_string(), vec![record]);
        }
        
        // Test retrieval
        let best = manager.get_best_provider("artha://test123", true, Some("US-East")).await;
        assert!(best.is_some());
        
        let best_record = best.unwrap();
        assert_eq!(best_record.cid, "artha://test123");
        assert!(best_record.capabilities.gpu);
        assert_eq!(best_record.capabilities.region, "US-East");
    }
    
    #[tokio::test]
    async fn test_stats() {
        let peer_id = PeerId::random();
        let manager = DhtRoutingManager::new(peer_id);
        
        // Add some test records
        {
            let mut providers = manager.providers.write().await;
            for i in 0..5 {
                let capabilities = ProviderCapabilities {
                    storage: true,
                    retrieval: true,
                    gpu: i < 2,
                    region: if i < 3 { "US-East" } else { "EU-West" }.to_string(),
                    bandwidth_mbps: 10000,
                };
                
                let record = ProviderRecord {
                    cid: format!("artha://test{}", i),
                    provider: peer_id,
                    addresses: vec![],
                    capabilities,
                    published_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                
                providers.insert(format!("artha://test{}", i), vec![record]);
            }
        }
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_cids, 5);
        assert_eq!(stats.total_providers, 5);
        assert_eq!(stats.gpu_providers, 2);
        assert_eq!(*stats.regions.get("US-East").unwrap(), 3);
        assert_eq!(*stats.regions.get("EU-West").unwrap(), 2);
    }
}


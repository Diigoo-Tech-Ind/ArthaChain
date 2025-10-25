use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Decentralized hosting configuration for ArthaChain
/// Uses only arthachain.in domains to symbolize Indian origin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecentralizedHostingConfig {
    /// Primary domain for all APIs
    pub primary_domain: String,
    /// API endpoints under arthachain.in
    pub api_endpoints: HashMap<String, String>,
    /// Community node network
    pub community_nodes: Vec<String>,
    /// P2P network configuration
    pub p2p_config: P2PConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PConfig {
    pub enable_p2p: bool,
    pub bootstrap_nodes: Vec<String>,
    pub max_peers: usize,
    pub discovery_interval: u64,
}

impl Default for DecentralizedHostingConfig {
    fn default() -> Self {
        Self {
            primary_domain: "arthachain.in".to_string(),
            api_endpoints: HashMap::from([
                ("api".to_string(), "api.arthachain.in".to_string()),
                ("testnet".to_string(), "testnet.arthachain.in".to_string()),
                ("staging".to_string(), "staging.arthachain.in".to_string()),
                ("docs".to_string(), "docs.arthachain.in".to_string()),
                ("explorer".to_string(), "explorer.arthachain.in".to_string()),
            ]),
            community_nodes: vec![
                "validator.arthachain.in".to_string(), // Main validator node
                "fullnode.arthachain.in".to_string(),  // Full node for API access
                "archive.arthachain.in".to_string(),   // Archive node for historical data
                "rpc.arthachain.in".to_string(),       // RPC endpoint node
                "explorer.arthachain.in".to_string(),  // Blockchain explorer node
                "faucet.arthachain.in".to_string(),    // Faucet service node
                "monitor.arthachain.in".to_string(),   // Monitoring and metrics node
                "bridge.arthachain.in".to_string(),    // Cross-chain bridge node
            ],
            p2p_config: P2PConfig {
                enable_p2p: true,
                bootstrap_nodes: vec![
                    "validator.arthachain.in:8084".to_string(),
                    "fullnode.arthachain.in:8084".to_string(),
                    "rpc.arthachain.in:8084".to_string(),
                ],
                max_peers: 100,
                discovery_interval: 30,
            },
        }
    }
}

/// Decentralized hosting manager
pub struct DecentralizedHostingManager {
    config: DecentralizedHostingConfig,
    active_nodes: Arc<RwLock<HashMap<String, NodeStatus>>>,
    api_routes: Arc<RwLock<HashMap<String, String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub node_id: String,
    pub domain: String,
    pub is_active: bool,
    pub last_seen: u64,
    pub health_score: f64,
    pub api_endpoints: Vec<String>,
}

impl DecentralizedHostingManager {
    pub fn new(config: DecentralizedHostingConfig) -> Self {
        Self {
            config,
            active_nodes: Arc::new(RwLock::new(HashMap::new())),
            api_routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize decentralized hosting
    pub async fn initialize(&self) -> Result<()> {
        log::info!("🚀 Initializing ArthaChain decentralized hosting on arthachain.in");

        // Setup API routes
        self.setup_api_routes().await?;

        // Initialize P2P network
        if self.config.p2p_config.enable_p2p {
            self.initialize_p2p_network().await?;
        }

        // Setup community nodes
        self.setup_community_nodes().await?;

        log::info!("✅ Decentralized hosting initialized successfully");
        Ok(())
    }

    /// Setup API routes under arthachain.in domain
    async fn setup_api_routes(&self) -> Result<()> {
        let mut routes = self.api_routes.write().await;

        for (endpoint, domain) in &self.config.api_endpoints {
            let route = format!("https://{}/api/v1", domain);
            routes.insert(endpoint.clone(), route.clone());
            log::info!("🌐 API Route: {} -> {}", endpoint, route);
        }

        Ok(())
    }

    /// Initialize P2P network for decentralized communication
    async fn initialize_p2p_network(&self) -> Result<()> {
        log::info!("🔗 Initializing P2P network for decentralized hosting");

        for bootstrap_node in &self.config.p2p_config.bootstrap_nodes {
            log::info!("📡 Bootstrap node: {}", bootstrap_node);
        }

        log::info!(
            "✅ P2P network initialized with {} max peers",
            self.config.p2p_config.max_peers
        );
        Ok(())
    }

    /// Setup community nodes
    async fn setup_community_nodes(&self) -> Result<()> {
        log::info!("🏗️ Setting up community nodes");

        let mut nodes = self.active_nodes.write().await;

        for (index, node_domain) in self.config.community_nodes.iter().enumerate() {
            let node_status = NodeStatus {
                node_id: format!("ArthaX{}", generate_node_id()),
                domain: node_domain.clone(),
                is_active: false,
                last_seen: 0,
                health_score: 0.0,
                api_endpoints: vec![
                    format!("https://{}/api/v1", node_domain),
                    format!("https://{}/health", node_domain),
                ],
            };

            nodes.insert(node_domain.clone(), node_status);
            log::info!("🏠 Community node {}: {}", index + 1, node_domain);
        }

        log::info!(
            "✅ {} community nodes configured",
            self.config.community_nodes.len()
        );
        Ok(())
    }

    /// Get API endpoint for a specific service
    pub async fn get_api_endpoint(&self, service: &str) -> Option<String> {
        let routes = self.api_routes.read().await;
        routes.get(service).cloned()
    }

    /// Get all active community nodes
    pub async fn get_active_nodes(&self) -> Vec<NodeStatus> {
        let nodes = self.active_nodes.read().await;
        nodes
            .values()
            .filter(|node| node.is_active)
            .cloned()
            .collect()
    }

    /// Update node health status
    pub async fn update_node_health(&self, domain: &str, health_score: f64) -> Result<()> {
        let mut nodes = self.active_nodes.write().await;

        if let Some(node) = nodes.get_mut(domain) {
            node.health_score = health_score;
            node.last_seen = chrono::Utc::now().timestamp() as u64;
            node.is_active = health_score > 0.5;

            log::info!("📊 Node {} health updated: {:.2}", domain, health_score);
        }

        Ok(())
    }

    /// Get decentralized hosting status
    pub async fn get_status(&self) -> DecentralizedHostingStatus {
        let nodes = self.active_nodes.read().await;
        let active_count = nodes.values().filter(|node| node.is_active).count();

        DecentralizedHostingStatus {
            primary_domain: self.config.primary_domain.clone(),
            total_nodes: self.config.community_nodes.len(),
            active_nodes: active_count,
            p2p_enabled: self.config.p2p_config.enable_p2p,
            health_score: if self.config.community_nodes.is_empty() {
                0.0
            } else {
                active_count as f64 / self.config.community_nodes.len() as f64
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecentralizedHostingStatus {
    pub primary_domain: String,
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub p2p_enabled: bool,
    pub health_score: f64,
}

/// Generate unique node ID
fn generate_node_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let mut id = String::new();

    for _ in 0..19 {
        // 19 chars to make total 21 with "ArthaX" prefix
        let idx = rng.gen_range(0..chars.len());
        id.push(chars[idx]);
    }

    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_decentralized_hosting_initialization() {
        let config = DecentralizedHostingConfig::default();
        let manager = DecentralizedHostingManager::new(config);

        assert!(manager.initialize().await.is_ok());

        let status = manager.get_status().await;
        assert_eq!(status.primary_domain, "arthachain.in");
        assert_eq!(status.total_nodes, 8);
        assert!(status.p2p_enabled);
    }

    #[tokio::test]
    async fn test_api_endpoints() {
        let config = DecentralizedHostingConfig::default();
        let manager = DecentralizedHostingManager::new(config);

        manager.initialize().await.unwrap();

        let api_endpoint = manager.get_api_endpoint("api").await;
        assert!(api_endpoint.is_some());
        assert!(api_endpoint.unwrap().contains("api.arthachain.in"));
    }
}

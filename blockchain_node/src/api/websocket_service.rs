use crate::api::websocket::WebSocketManager;
use crate::consensus::consensus_manager::ConsensusManager;
use crate::ledger::state::State;
use crate::monitoring::metrics_collector::MetricsCollector;
use crate::network::p2p::P2PNetwork;
use crate::transaction::mempool::Mempool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};

/// WebSocket service for real-time blockchain event publishing
pub struct WebSocketService {
    /// WebSocket manager
    ws_manager: Arc<WebSocketManager>,
    /// Blockchain state
    state: Arc<RwLock<State>>,
    /// Mempool for transaction events
    mempool: Arc<RwLock<Mempool>>,
    /// P2P network for network status
    p2p_network: Arc<P2PNetwork>,
    /// Consensus manager for consensus events
    consensus_manager: Arc<ConsensusManager>,
    /// Metrics collector for performance data
    metrics_collector: Arc<MetricsCollector>,
    /// Service running flag
    running: Arc<RwLock<bool>>,
}

impl WebSocketService {
    /// Create a new WebSocket service
    pub fn new(
        state: Arc<RwLock<State>>,
        mempool: Arc<RwLock<Mempool>>,
        p2p_network: Arc<P2PNetwork>,
        consensus_manager: Arc<ConsensusManager>,
        metrics_collector: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            ws_manager: Arc::new(WebSocketManager::new()),
            state,
            mempool,
            p2p_network,
            consensus_manager,
            metrics_collector,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get the WebSocket manager
    pub fn ws_manager(&self) -> Arc<WebSocketManager> {
        self.ws_manager.clone()
    }

    /// Start the WebSocket service
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            return;
        }
        *running = true;

        // Start background tasks for real-time data publishing
        let ws_manager = self.ws_manager.clone();
        let state = self.state.clone();
        let p2p_network = self.p2p_network.clone();
        let consensus_manager = self.consensus_manager.clone();
        let metrics_collector = self.metrics_collector.clone();
        let running_flag = self.running.clone();

        // Network status publishing task
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(10)); // Every 10 seconds

            while *running_flag.read().await {
                interval.tick().await;

                if let Err(e) =
                    Self::publish_network_status(&ws_manager, &p2p_network, &state).await
                {
                    log::warn!("Failed to publish network status: {}", e);
                }
            }
        });

        // Mempool status publishing task
        let ws_manager = self.ws_manager.clone();
        let mempool = self.mempool.clone();
        let running_flag = self.running.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(5)); // Every 5 seconds

            while *running_flag.read().await {
                interval.tick().await;

                if let Err(e) = Self::publish_mempool_status(&ws_manager, &mempool).await {
                    log::warn!("Failed to publish mempool status: {}", e);
                }
            }
        });

        // Consensus status publishing task
        let ws_manager = self.ws_manager.clone();
        let consensus_manager = self.consensus_manager.clone();
        let running_flag = self.running.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(3)); // Every 3 seconds

            while *running_flag.read().await {
                interval.tick().await;

                if let Err(e) =
                    Self::publish_consensus_status(&ws_manager, &consensus_manager).await
                {
                    log::warn!("Failed to publish consensus status: {}", e);
                }
            }
        });

        // Performance metrics publishing task
        let ws_manager = self.ws_manager.clone();
        let metrics_collector = self.metrics_collector.clone();
        let running_flag = self.running.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(15)); // Every 15 seconds

            while *running_flag.read().await {
                interval.tick().await;

                if let Err(e) =
                    Self::publish_performance_metrics(&ws_manager, &metrics_collector).await
                {
                    log::warn!("Failed to publish performance metrics: {}", e);
                }
            }
        });

        log::info!("WebSocket service started with real-time data publishing");
    }

    /// Stop the WebSocket service
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        log::info!("WebSocket service stopped");
    }

    /// Publish real network status
    async fn publish_network_status(
        ws_manager: &WebSocketManager,
        p2p_network: &P2PNetwork,
        state: &Arc<RwLock<State>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event_manager = ws_manager.event_manager();

        // Get real network data
        let total_peers = p2p_network.get_peer_count().await.unwrap_or(0);
        let active_peers = p2p_network.get_active_connections().await.unwrap_or(0);
        let network_version = p2p_network
            .get_network_version()
            .await
            .unwrap_or_else(|_| "1.0.0".to_string());
        let chain_id = 201766; // ArthaChain mainnet

        let state_guard = state.read().await;
        let best_block_height = state_guard.get_height().unwrap_or(0);
        let sync_status = if let Ok(network_height) = p2p_network.get_network_best_height().await {
            if network_height == 0 {
                "synced".to_string()
            } else {
                let sync_percentage = (best_block_height as f64 / network_height as f64) * 100.0;
                if sync_percentage >= 99.9 {
                    "synced".to_string()
                } else if sync_percentage >= 90.0 {
                    "syncing".to_string()
                } else {
                    "behind".to_string()
                }
            }
        } else {
            "synced".to_string()
        };

        let network_difficulty = p2p_network
            .get_network_difficulty()
            .await
            .unwrap_or(1000000);

        // Publish network status event
        event_manager.publish_network_status(
            total_peers,
            active_peers,
            &network_version,
            chain_id,
            best_block_height,
            &sync_status,
            network_difficulty,
        );

        Ok(())
    }

    /// Publish real mempool status
    async fn publish_mempool_status(
        ws_manager: &WebSocketManager,
        mempool: &Arc<RwLock<Mempool>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event_manager = ws_manager.event_manager();
        let mempool_guard = mempool.read().await;

        // Get real mempool data
        let total_transactions = mempool_guard.get_size();
        let pending_transactions = mempool_guard.get_pending_count();
        let queued_transactions = mempool_guard.get_queued_count();
        let size_bytes = mempool_guard.get_memory_usage();

        // Get real gas price range
        let gas_prices = mempool_guard.get_gas_prices();
        let gas_price_range = crate::api::websocket::GasPriceRange {
            min: gas_prices.min,
            max: gas_prices.max,
            average: gas_prices.average,
            median: gas_prices.median,
        };

        // Get recent transaction hashes
        let recent_transactions: Vec<String> = mempool_guard
            .get_recent_transactions(10)
            .into_iter()
            .map(|tx| hex::encode(tx.hash()))
            .collect();

        // Publish mempool update event
        event_manager.publish_mempool_update(
            total_transactions,
            pending_transactions,
            queued_transactions,
            size_bytes,
            gas_price_range,
            recent_transactions,
        );

        Ok(())
    }

    /// Publish real consensus status
    async fn publish_consensus_status(
        ws_manager: &WebSocketManager,
        consensus_manager: &ConsensusManager,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event_manager = ws_manager.event_manager();

        // Get real consensus data
        let view = consensus_manager.get_current_view().await;
        let phase = consensus_manager.get_current_phase().await;
        let leader = consensus_manager.get_current_leader().await;
        let validator_count = consensus_manager.get_validator_count().await;
        let round = consensus_manager.get_current_round().await;
        let block_time = consensus_manager.get_block_time().await;
        let finality = consensus_manager.get_finality_type().await;

        // Publish consensus update event
        let leader_str = leader
            .as_ref()
            .map(|l| l.to_string())
            .unwrap_or_else(|| "none".to_string());
        event_manager.publish_consensus_update(
            view.unwrap_or(0),
            &phase.unwrap_or_else(|_| "unknown".to_string()),
            &leader_str,
            validator_count.unwrap_or(0),
            round.unwrap_or(0),
            block_time.unwrap_or(0),
            &finality.unwrap_or_else(|_| "unknown".to_string()),
        );

        Ok(())
    }

    /// Publish real performance metrics
    async fn publish_performance_metrics(
        ws_manager: &WebSocketManager,
        metrics_collector: &MetricsCollector,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event_manager = ws_manager.event_manager();

        // Get real performance metrics
        let metrics = metrics_collector.get_current_metrics().await?;

        // Publish performance event using consensus update with performance data
        let performance_phase = if metrics.transaction_throughput > 1000.0 {
            "high_performance"
        } else if metrics.transaction_throughput > 100.0 {
            "normal_performance"
        } else {
            "low_performance"
        };

        event_manager.publish_consensus_update(
            0, // view
            performance_phase,
            "performance_monitor",                // leader
            0,                                    // validator_count
            0,                                    // round
            metrics.block_production_rate as u64, // block_time
            "performance_metrics",
        );

        Ok(())
    }

    /// Publish new block event
    pub async fn publish_new_block(&self, block: &crate::ledger::block::Block) {
        self.ws_manager.event_manager().publish_new_block(block);
    }

    /// Publish new transaction event
    pub async fn publish_new_transaction(
        &self,
        transaction: &crate::ledger::transaction::Transaction,
    ) {
        self.ws_manager
            .event_manager()
            .publish_new_transaction(transaction);
    }

    /// Publish confirmed transaction event
    pub async fn publish_confirmed_transaction(
        &self,
        transaction: &crate::ledger::transaction::Transaction,
        block_hash: &crate::types::Hash,
        block_number: u64,
        transaction_index: u64,
        gas_used: u64,
        status: bool,
        logs: Vec<String>,
        contract_address: Option<crate::types::Address>,
    ) {
        self.ws_manager
            .event_manager()
            .publish_confirmed_transaction(
                transaction,
                block_hash,
                block_number,
                transaction_index,
                gas_used,
                status,
                logs,
                contract_address,
            );
    }

    /// Publish chain reorganization event
    pub async fn publish_chain_reorg(
        &self,
        old_block_hash: &crate::types::Hash,
        new_block_hash: &crate::types::Hash,
        common_ancestor_height: u64,
        reorg_depth: u64,
        affected_blocks: Vec<String>,
    ) {
        self.ws_manager.event_manager().publish_chain_reorg(
            old_block_hash,
            new_block_hash,
            common_ancestor_height,
            reorg_depth,
            affected_blocks,
        );
    }

    /// Publish validator update event
    pub async fn publish_validator_update(
        &self,
        address: &str,
        action: &str,
        stake: u64,
        commission_rate: f64,
        performance_score: f64,
        uptime: f64,
    ) {
        self.ws_manager.event_manager().publish_validator_update(
            address,
            action,
            stake,
            commission_rate,
            performance_score,
            uptime,
        );
    }
}

impl Default for WebSocketService {
    fn default() -> Self {
        // Create default instances for testing
        let state = Arc::new(RwLock::new(
            crate::ledger::state::State::new(&crate::config::Config::default()).unwrap(),
        ));
        let mempool = Arc::new(RwLock::new(crate::transaction::mempool::Mempool::new(
            10000,
        )));
        let p2p_network = Arc::new(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(crate::network::p2p::P2PNetwork::new(
                    crate::config::Config::default(),
                    state.clone(),
                    tokio::sync::mpsc::channel(1).0,
                ))
                .unwrap(),
        );
        let consensus_manager = Arc::new(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(crate::consensus::consensus_manager::ConsensusManager::new(
                    crate::consensus::consensus_manager::ConsensusConfig::default(),
                    crate::network::types::NodeId::from("default_node"),
                    std::collections::HashSet::new(),
                ))
                .unwrap(),
        );
        let metrics_collector =
            Arc::new(crate::monitoring::metrics_collector::MetricsCollector::new());

        Self::new(
            state,
            mempool,
            p2p_network,
            consensus_manager,
            metrics_collector,
        )
    }
}

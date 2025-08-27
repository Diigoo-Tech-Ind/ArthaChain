use axum::{
    extract::{Extension, WebSocketUpgrade},
    response::IntoResponse,
};
use futures::stream::BoxStream;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::ledger::block::Block;
use crate::ledger::state::State;
use crate::ledger::transaction::Transaction;
use crate::types::{Address, Hash};

/// WebSocket client connection information
#[derive(Debug, Clone)]
pub struct WebSocketClient {
    pub id: String,
    pub subscriptions: Vec<String>,
    pub connected_at: std::time::Instant,
    pub last_heartbeat: std::time::Instant,
}

/// Event types that can be sent to WebSocket clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketEvent {
    /// New block event
    #[serde(rename = "new_block")]
    NewBlock(BlockEvent),

    /// New transaction event
    #[serde(rename = "new_transaction")]
    NewTransaction(TransactionEvent),

    /// Transaction confirmed event
    #[serde(rename = "transaction_confirmed")]
    TransactionConfirmed(TransactionConfirmedEvent),

    /// Mempool update event
    #[serde(rename = "mempool_update")]
    MempoolUpdate(MempoolEvent),

    /// Consensus update event
    #[serde(rename = "consensus_update")]
    ConsensusUpdate(ConsensusEvent),

    /// Chain reorganization event
    #[serde(rename = "chain_reorg")]
    ChainReorg(ChainReorgEvent),

    /// Validator update event
    #[serde(rename = "validator_update")]
    ValidatorUpdate(ValidatorEvent),

    /// Network status event
    #[serde(rename = "network_status")]
    NetworkStatus(NetworkStatusEvent),

    /// Subscription confirmation
    #[serde(rename = "subscription")]
    Subscription(SubscriptionEvent),

    /// Error event
    #[serde(rename = "error")]
    Error(ErrorEvent),

    /// Heartbeat/ping event
    #[serde(rename = "ping")]
    Ping(PingEvent),
}

/// Data for a new block event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEvent {
    /// Block hash
    pub hash: String,
    /// Block height
    pub height: u64,
    /// Number of transactions
    pub tx_count: usize,
    /// Block size in bytes
    pub size: usize,
    /// Gas used
    pub gas_used: u64,
    /// Gas limit
    pub gas_limit: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Miner/validator address
    pub miner: String,
    /// Block reward
    pub reward: u64,
    /// Difficulty
    pub difficulty: u64,
    /// Total difficulty
    pub total_difficulty: u64,
    /// Parent hash
    pub parent_hash: String,
    /// Merkle root
    pub merkle_root: String,
    /// State root
    pub state_root: String,
    /// Receipts root
    pub receipts_root: String,
    /// Extra data
    pub extra_data: String,
}

/// Data for a new transaction event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEvent {
    /// Transaction hash
    pub hash: String,
    /// Sender address
    pub sender: String,
    /// Recipient address (if applicable)
    pub recipient: Option<String>,
    /// Transaction amount
    pub amount: u64,
    /// Gas price
    pub gas_price: u64,
    /// Gas limit
    pub gas_limit: u64,
    /// Nonce
    pub nonce: u64,
    /// Transaction type
    pub tx_type: String,
    /// Data (hex encoded)
    pub data: String,
    /// Signature
    pub signature: String,
    /// Timestamp
    pub timestamp: u64,
    /// Block hash (if confirmed)
    pub block_hash: Option<String>,
    /// Block number (if confirmed)
    pub block_number: Option<u64>,
    /// Transaction index (if confirmed)
    pub transaction_index: Option<u64>,
}

/// Data for a confirmed transaction event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionConfirmedEvent {
    /// Transaction hash
    pub hash: String,
    /// Block hash
    pub block_hash: String,
    /// Block number
    pub block_number: u64,
    /// Transaction index
    pub transaction_index: u64,
    /// Gas used
    pub gas_used: u64,
    /// Status (success/failure)
    pub status: bool,
    /// Logs
    pub logs: Vec<String>,
    /// Contract address (if contract creation)
    pub contract_address: Option<String>,
}

/// Data for mempool update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolEvent {
    /// Total transactions in mempool
    pub total_transactions: usize,
    /// Pending transactions
    pub pending_transactions: usize,
    /// Queued transactions
    pub queued_transactions: usize,
    /// Mempool size in bytes
    pub size_bytes: usize,
    /// Gas price range
    pub gas_price_range: GasPriceRange,
    /// Recent transactions added
    pub recent_transactions: Vec<String>,
}

/// Gas price range information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPriceRange {
    /// Minimum gas price
    pub min: u64,
    /// Maximum gas price
    pub max: u64,
    /// Average gas price
    pub average: u64,
    /// Median gas price
    pub median: u64,
}

/// Data for a consensus update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusEvent {
    /// View number
    pub view: u64,
    /// Phase
    pub phase: String,
    /// Leader
    pub leader: String,
    /// Validator count
    pub validator_count: usize,
    /// Consensus round
    pub round: u64,
    /// Block time
    pub block_time: u64,
    /// Finality
    pub finality: String,
}

/// Data for chain reorganization event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainReorgEvent {
    /// Old block hash
    pub old_block_hash: String,
    /// New block hash
    pub new_block_hash: String,
    /// Common ancestor height
    pub common_ancestor_height: u64,
    /// Reorg depth
    pub reorg_depth: u64,
    /// Affected blocks
    pub affected_blocks: Vec<String>,
}

/// Data for validator update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorEvent {
    /// Validator address
    pub address: String,
    /// Action (added/removed/updated)
    pub action: String,
    /// Stake amount
    pub stake: u64,
    /// Commission rate
    pub commission_rate: f64,
    /// Performance score
    pub performance_score: f64,
    /// Uptime percentage
    pub uptime: f64,
}

/// Data for network status event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatusEvent {
    /// Total peers
    pub total_peers: usize,
    /// Active peers
    pub active_peers: usize,
    /// Network version
    pub network_version: String,
    /// Chain ID
    pub chain_id: u64,
    /// Best block height
    pub best_block_height: u64,
    /// Sync status
    pub sync_status: String,
    /// Network difficulty
    pub network_difficulty: u64,
}

/// Data for a subscription confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEvent {
    /// Event types subscribed to
    pub events: Vec<String>,
    /// Success status
    pub success: bool,
    /// Client ID
    pub client_id: String,
    /// Message
    pub message: String,
}

/// Data for an error event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    /// Error code
    pub code: u32,
    /// Error message
    pub message: String,
    /// Error details
    pub details: Option<String>,
}

/// Data for a ping event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingEvent {
    /// Timestamp
    pub timestamp: u64,
    /// Client ID
    pub client_id: String,
}

/// Client message to subscribe/unsubscribe
#[derive(Debug, Deserialize)]
pub struct ClientMessage {
    /// Message ID for request/response correlation
    pub id: Option<String>,
    /// Action to perform
    pub action: String,
    /// Event types to subscribe to
    pub events: Option<Vec<String>>,
    /// Client ID
    pub client_id: Option<String>,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: Option<u64>,
}

/// WebSocket connection manager
pub struct WebSocketManager {
    /// Active connections
    connections: Arc<RwLock<HashMap<String, WebSocketClient>>>,
    /// Event manager
    event_manager: Arc<EventManager>,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_manager: Arc::new(EventManager::new()),
        }
    }

    /// Get the event manager
    pub fn event_manager(&self) -> Arc<EventManager> {
        self.event_manager.clone()
    }

    /// Add a new connection
    pub async fn add_connection(&self, client_id: String, subscriptions: Vec<String>) {
        let mut connections = self.connections.write().await;
        connections.insert(
            client_id.clone(),
            WebSocketClient {
                id: client_id,
                subscriptions,
                connected_at: std::time::Instant::now(),
                last_heartbeat: std::time::Instant::now(),
            },
        );
    }

    /// Remove a connection
    pub async fn remove_connection(&self, client_id: &str) {
        let mut connections = self.connections.write().await;
        connections.remove(client_id);
    }

    /// Update client heartbeat
    pub async fn update_heartbeat(&self, client_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(client) = connections.get_mut(client_id) {
            client.last_heartbeat = std::time::Instant::now();
        }
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Clean up stale connections
    pub async fn cleanup_stale_connections(&self, timeout_seconds: u64) {
        let mut connections = self.connections.write().await;
        let now = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_seconds);

        connections.retain(|_, client| now.duration_since(client.last_heartbeat) < timeout);
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Event manager for WebSocket events
#[derive(Clone)]
pub struct EventManager {
    /// Sender for new block events
    pub block_tx: broadcast::Sender<BlockEvent>,
    /// Sender for new transaction events
    pub transaction_tx: broadcast::Sender<TransactionEvent>,
    /// Sender for confirmed transaction events
    pub confirmed_tx_tx: broadcast::Sender<TransactionConfirmedEvent>,
    /// Sender for mempool update events
    pub mempool_tx: broadcast::Sender<MempoolEvent>,
    /// Sender for consensus update events
    pub consensus_tx: broadcast::Sender<ConsensusEvent>,
    /// Sender for chain reorg events
    pub reorg_tx: broadcast::Sender<ChainReorgEvent>,
    /// Sender for validator update events
    pub validator_tx: broadcast::Sender<ValidatorEvent>,
    /// Sender for network status events
    pub network_tx: broadcast::Sender<NetworkStatusEvent>,
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EventManager {
    /// Create a new event manager
    pub fn new() -> Self {
        let (block_tx, _) = broadcast::channel(1000);
        let (transaction_tx, _) = broadcast::channel(1000);
        let (confirmed_tx_tx, _) = broadcast::channel(1000);
        let (mempool_tx, _) = broadcast::channel(100);
        let (consensus_tx, _) = broadcast::channel(100);
        let (reorg_tx, _) = broadcast::channel(100);
        let (validator_tx, _) = broadcast::channel(100);
        let (network_tx, _) = broadcast::channel(100);

        Self {
            block_tx,
            transaction_tx,
            confirmed_tx_tx,
            mempool_tx,
            consensus_tx,
            reorg_tx,
            validator_tx,
            network_tx,
        }
    }

    /// Publish a new block event
    pub fn publish_new_block(&self, block: &Block) {
        let event = BlockEvent {
            hash: hex::encode(block.hash_bytes()),
            height: block.header.height,
            tx_count: block.transactions.len(),
            size: block
                .transactions
                .iter()
                .map(|tx| tx.data.len())
                .sum::<usize>()
                + 256, // Approximate size
            gas_used: 0,  // Not available in current BlockHeader
            gas_limit: 0, // Not available in current BlockHeader
            timestamp: block.header.timestamp,
            miner: hex::encode(block.header.producer.as_bytes()),
            reward: 0, // Not available in current BlockHeader
            difficulty: block.header.difficulty,
            total_difficulty: block.header.difficulty, // Same as difficulty for now
            parent_hash: hex::encode(block.header.previous_hash.as_ref()),
            merkle_root: hex::encode(block.header.merkle_root.as_ref()),
            state_root: hex::encode(block.header.merkle_root.as_ref()), // Use merkle root as state root
            receipts_root: hex::encode(block.header.merkle_root.as_ref()), // Use merkle root as receipts root
            extra_data: "0x".to_string(), // Not available in current BlockHeader
        };

        let _ = self.block_tx.send(event);
    }

    /// Publish a new transaction event
    pub fn publish_new_transaction(&self, tx: &Transaction) {
        let event = TransactionEvent {
            hash: hex::encode(tx.hash().as_ref()),
            sender: tx.sender.clone(),
            recipient: Some(tx.recipient.clone()),
            amount: tx.amount,
            gas_price: tx.gas_price,
            gas_limit: tx.gas_limit,
            nonce: tx.nonce,
            tx_type: format!("{:?}", tx.tx_type),
            data: hex::encode(&tx.data),
            signature: hex::encode(&tx.signature),
            timestamp: tx.timestamp,
            block_hash: None,
            block_number: None,
            transaction_index: None,
        };

        let _ = self.transaction_tx.send(event);
    }

    /// Publish a confirmed transaction event
    pub fn publish_confirmed_transaction(
        &self,
        tx: &Transaction,
        block_hash: &Hash,
        block_number: u64,
        transaction_index: u64,
        gas_used: u64,
        status: bool,
        logs: Vec<String>,
        contract_address: Option<Address>,
    ) {
        let event = TransactionConfirmedEvent {
            hash: hex::encode(tx.hash().as_ref()),
            block_hash: hex::encode(block_hash.as_ref()),
            block_number,
            transaction_index,
            gas_used,
            status,
            logs,
            contract_address: contract_address.map(|addr| addr.to_string()),
        };

        let _ = self.confirmed_tx_tx.send(event);
    }

    /// Publish a mempool update event
    pub fn publish_mempool_update(
        &self,
        total_transactions: usize,
        pending_transactions: usize,
        queued_transactions: usize,
        size_bytes: usize,
        gas_price_range: GasPriceRange,
        recent_transactions: Vec<String>,
    ) {
        let event = MempoolEvent {
            total_transactions,
            pending_transactions,
            queued_transactions,
            size_bytes,
            gas_price_range,
            recent_transactions,
        };

        let _ = self.mempool_tx.send(event);
    }

    /// Publish a consensus update event
    pub fn publish_consensus_update(
        &self,
        view: u64,
        phase: &str,
        leader: &str,
        validator_count: usize,
        round: u64,
        block_time: u64,
        finality: &str,
    ) {
        let event = ConsensusEvent {
            view,
            phase: phase.to_string(),
            leader: leader.to_string(),
            validator_count,
            round,
            block_time,
            finality: finality.to_string(),
        };

        let _ = self.consensus_tx.send(event);
    }

    /// Publish a chain reorganization event
    pub fn publish_chain_reorg(
        &self,
        old_block_hash: &Hash,
        new_block_hash: &Hash,
        common_ancestor_height: u64,
        reorg_depth: u64,
        affected_blocks: Vec<String>,
    ) {
        let event = ChainReorgEvent {
            old_block_hash: hex::encode(old_block_hash.as_ref()),
            new_block_hash: hex::encode(new_block_hash.as_ref()),
            common_ancestor_height,
            reorg_depth,
            affected_blocks,
        };

        let _ = self.reorg_tx.send(event);
    }

    /// Publish a validator update event
    pub fn publish_validator_update(
        &self,
        address: &str,
        action: &str,
        stake: u64,
        commission_rate: f64,
        performance_score: f64,
        uptime: f64,
    ) {
        let event = ValidatorEvent {
            address: address.to_string(),
            action: action.to_string(),
            stake,
            commission_rate,
            performance_score,
            uptime,
        };

        let _ = self.validator_tx.send(event);
    }

    /// Publish a network status event
    pub fn publish_network_status(
        &self,
        total_peers: usize,
        active_peers: usize,
        network_version: &str,
        chain_id: u64,
        best_block_height: u64,
        sync_status: &str,
        network_difficulty: u64,
    ) {
        let event = NetworkStatusEvent {
            total_peers,
            active_peers,
            network_version: network_version.to_string(),
            chain_id,
            best_block_height,
            sync_status: sync_status.to_string(),
            network_difficulty,
        };

        let _ = self.network_tx.send(event);
    }
}

/// The WebSocket handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: axum::extract::ws::WebSocket, state: Arc<RwLock<State>>) {
    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Create a channel for sending messages to the client
    let (tx, mut rx) = mpsc::channel::<WebSocketEvent>(1000);

    // Generate unique client ID
    let client_id = Uuid::new_v4().to_string();

    // Create event subscriptions
    let event_manager = EventManager::new();
    let mut subscriptions: HashMap<String, String> = HashMap::new();

    // Clone variables for tasks before they get moved
    let recv_client_id = client_id.clone();
    let recv_tx = tx.clone();
    let event_client_id = client_id.clone();
    let event_tx = tx.clone();

    // Send welcome message
    let welcome = WebSocketEvent::Subscription(SubscriptionEvent {
        events: vec![],
        success: true,
        client_id: client_id.clone(),
        message: "Connected to ArthaChain WebSocket API".to_string(),
    });
    let _ = tx.send(welcome).await;

    // Task to forward messages from the channel to the WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let msg = serde_json::to_string(&event).unwrap();
            if sender
                .send(axum::extract::ws::Message::Text(msg))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Process messages from the client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                axum::extract::ws::Message::Text(text) => {
                    // Parse client message
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        match client_msg.action.as_str() {
                            "subscribe" => {
                                if let Some(events) = client_msg.events {
                                    let mut subscribed_events = Vec::new();

                                    for event_type in events {
                                        if !subscriptions.contains_key(&event_type) {
                                            if event_type.as_str() == "new_block"
                                                || event_type.as_str() == "new_transaction"
                                                || event_type.as_str() == "transaction_confirmed"
                                                || event_type.as_str() == "mempool_update"
                                                || event_type.as_str() == "consensus_update"
                                                || event_type.as_str() == "chain_reorg"
                                                || event_type.as_str() == "validator_update"
                                                || event_type.as_str() == "network_status"
                                            {
                                                subscriptions
                                                    .insert(event_type.clone(), event_type.clone());
                                                subscribed_events.push(event_type);
                                            }
                                        }
                                    }

                                    // Send subscription confirmation
                                    let confirmation =
                                        WebSocketEvent::Subscription(SubscriptionEvent {
                                            events: subscribed_events,
                                            success: true,
                                            client_id: recv_client_id.clone(),
                                            message: "Subscriptions updated successfully"
                                                .to_string(),
                                        });
                                    let _ = recv_tx.send(confirmation).await;
                                }
                            }
                            "unsubscribe" => {
                                if let Some(events) = client_msg.events {
                                    for event_type in events {
                                        subscriptions.remove(&event_type);
                                    }
                                } else {
                                    // Unsubscribe from all
                                    subscriptions.clear();
                                }

                                // Send confirmation
                                let confirmation =
                                    WebSocketEvent::Subscription(SubscriptionEvent {
                                        events: Vec::new(),
                                        success: true,
                                        client_id: recv_client_id.clone(),
                                        message: "Unsubscribed successfully".to_string(),
                                    });
                                let _ = recv_tx.send(confirmation).await;
                            }
                            "ping" => {
                                // Send pong response
                                let ping = WebSocketEvent::Ping(PingEvent {
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                    client_id: recv_client_id.clone(),
                                });
                                let _ = recv_tx.send(ping).await;
                            }
                            _ => {
                                // Send error for unknown action
                                let error = WebSocketEvent::Error(ErrorEvent {
                                    code: 400,
                                    message: "Unknown action".to_string(),
                                    details: Some(format!(
                                        "Action '{}' not recognized",
                                        client_msg.action
                                    )),
                                });
                                let _ = recv_tx.send(error).await;
                            }
                        }
                    } else {
                        // Send error for invalid JSON
                        let error = WebSocketEvent::Error(ErrorEvent {
                            code: 400,
                            message: "Invalid JSON".to_string(),
                            details: None,
                        });
                        let _ = recv_tx.send(error).await;
                    }
                }
                axum::extract::ws::Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }

        // Client disconnected
        recv_tx.closed().await;
    });

    // Task to forward events from subscriptions to the client
    let mut event_task = tokio::spawn(async move {
        // For now, this is a simplified event task that just keeps the connection alive
        // In a real implementation, you would set up event listeners for each subscription type
        // and forward events to the client

        // Keep the task running to maintain the connection
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

            // Send a heartbeat to keep the connection alive
            let heartbeat = WebSocketEvent::Ping(PingEvent {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                client_id: event_client_id.clone(),
            });

            if event_tx.send(heartbeat).await.is_err() {
                break;
            }
        }
    });

    // Wait for any task to finish
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
            event_task.abort();
        },
        _ = &mut recv_task => {
            send_task.abort();
            event_task.abort();
        },
        _ = &mut event_task => {
            send_task.abort();
            recv_task.abort();
        }
    };
}

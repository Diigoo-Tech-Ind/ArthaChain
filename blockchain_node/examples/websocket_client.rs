use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::OnceLock;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

static EMPTY_VEC: OnceLock<Vec<Value>> = OnceLock::new();

/// WebSocket client for ArthaChain blockchain
pub struct ArthaChainWebSocketClient {
    /// WebSocket URL
    url: String,
    /// Client ID
    client_id: String,
    /// Subscriptions
    subscriptions: Vec<String>,
}

impl ArthaChainWebSocketClient {
    /// Create a new WebSocket client
    pub fn new(url: String) -> Self {
        Self {
            url,
            client_id: uuid::Uuid::new_v4().to_string(),
            subscriptions: Vec::new(),
        }
    }

    /// Connect to the WebSocket server
    pub async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let url = Url::parse(&self.url)?;
        let (ws_stream, _) = connect_async(url.as_str()).await?;

        println!("✅ Connected to ArthaChain WebSocket API");
        println!("🔗 URL: {}", self.url);
        println!("🆔 Client ID: {}", self.client_id);

        let (mut write, mut read) = ws_stream.split();

        // Send initial subscription for all events
        let subscribe_msg = json!({
            "id": "init",
            "action": "subscribe",
            "events": [
                "new_block",
                "new_transaction",
                "transaction_confirmed",
                "mempool_update",
                "consensus_update",
                "chain_reorg",
                "validator_update",
                "network_status"
            ],
            "client_id": self.client_id,
            "heartbeat_interval": 30
        });

        write
            .send(Message::Text(subscribe_msg.to_string().into()))
            .await?;
        println!("📡 Subscribed to all blockchain events");

        // Handle incoming messages
        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    self.handle_message(text.as_str()).await?;
                }
                Message::Close(_) => {
                    println!("🔌 WebSocket connection closed");
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle incoming WebSocket messages
    async fn handle_message(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let event: Value = serde_json::from_str(text)?;

        match event["type"].as_str() {
            Some("new_block") => {
                self.handle_new_block(&event["data"]).await?;
            }
            Some("new_transaction") => {
                self.handle_new_transaction(&event["data"]).await?;
            }
            Some("transaction_confirmed") => {
                self.handle_transaction_confirmed(&event["data"]).await?;
            }
            Some("mempool_update") => {
                self.handle_mempool_update(&event["data"]).await?;
            }
            Some("consensus_update") => {
                self.handle_consensus_update(&event["data"]).await?;
            }
            Some("chain_reorg") => {
                self.handle_chain_reorg(&event["data"]).await?;
            }
            Some("validator_update") => {
                self.handle_validator_update(&event["data"]).await?;
            }
            Some("network_status") => {
                self.handle_network_status(&event["data"]).await?;
            }
            Some("subscription") => {
                self.handle_subscription(&event["data"]).await?;
            }
            Some("error") => {
                self.handle_error(&event["data"]).await?;
            }
            Some("ping") => {
                self.handle_ping(&event["data"]).await?;
            }
            _ => {
                println!("❓ Unknown event type: {}", event["type"]);
            }
        }

        Ok(())
    }

    /// Handle new block events
    async fn handle_new_block(&self, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        println!("🆕 NEW BLOCK:");
        println!("   📏 Height: {}", data["height"]);
        println!("   🔗 Hash: {}", data["hash"]);
        println!("   📊 Transactions: {}", data["tx_count"]);
        println!("   📏 Size: {} bytes", data["size"]);
        println!("   ⛽ Gas Used: {}", data["gas_used"]);
        println!("   ⛽ Gas Limit: {}", data["gas_limit"]);
        println!("   ⏰ Timestamp: {}", data["timestamp"]);
        println!("   👷 Miner: {}", data["miner"]);
        println!("   💰 Reward: {} ArthaCoin", data["reward"]);
        println!("   🎯 Difficulty: {}", data["difficulty"]);
        println!("   📈 Total Difficulty: {}", data["total_difficulty"]);
        println!("   🔗 Parent Hash: {}", data["parent_hash"]);
        println!("   🌳 Merkle Root: {}", data["merkle_root"]);
        println!("   🏗️ State Root: {}", data["state_root"]);
        println!("   📋 Receipts Root: {}", data["receipts_root"]);
        println!("   📝 Extra Data: {}", data["extra_data"]);
        println!();
        Ok(())
    }

    /// Handle new transaction events
    async fn handle_new_transaction(&self, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        println!("🆕 NEW TRANSACTION:");
        println!("   🔗 Hash: {}", data["hash"]);
        println!("   👤 Sender: {}", data["sender"]);
        println!(
            "   👥 Recipient: {}",
            data["recipient"].as_str().unwrap_or("N/A")
        );
        println!("   💰 Amount: {} ArthaCoin", data["amount"]);
        println!("   ⛽ Gas Price: {} wei", data["gas_price"]);
        println!("   ⛽ Gas Limit: {}", data["gas_limit"]);
        println!("   🔢 Nonce: {}", data["nonce"]);
        println!("   🏷️ Type: {}", data["tx_type"]);
        println!(
            "   📝 Data: {} bytes",
            data["data"].as_str().unwrap_or("").len() / 2
        );
        println!(
            "   ✍️ Signature: {} bytes",
            data["signature"].as_str().unwrap_or("").len() / 2
        );
        println!("   ⏰ Timestamp: {}", data["timestamp"]);
        println!();
        Ok(())
    }

    /// Handle confirmed transaction events
    async fn handle_transaction_confirmed(
        &self,
        data: &Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("✅ TRANSACTION CONFIRMED:");
        println!("   🔗 Hash: {}", data["hash"]);
        println!("   🧱 Block Hash: {}", data["block_hash"]);
        println!("   📏 Block Number: {}", data["block_number"]);
        println!("   📊 Transaction Index: {}", data["transaction_index"]);
        println!("   ⛽ Gas Used: {}", data["gas_used"]);
        println!(
            "   📊 Status: {}",
            if data["status"].as_bool().unwrap_or(false) {
                "✅ Success"
            } else {
                "❌ Failed"
            }
        );
        println!(
            "   📋 Logs: {} entries",
            data["logs"]
                .as_array()
                .unwrap_or(EMPTY_VEC.get_or_init(Vec::new))
                .len()
        );
        if let Some(contract_addr) = data["contract_address"].as_str() {
            println!("   📜 Contract Address: {}", contract_addr);
        }
        println!();
        Ok(())
    }

    /// Handle mempool update events
    async fn handle_mempool_update(&self, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        println!("📊 MEMPOOL UPDATE:");
        println!("   📊 Total Transactions: {}", data["total_transactions"]);
        println!(
            "   ⏳ Pending Transactions: {}",
            data["pending_transactions"]
        );
        println!("   🚦 Queued Transactions: {}", data["queued_transactions"]);
        println!("   📏 Size: {} bytes", data["size_bytes"]);

        let gas_range = &data["gas_price_range"];
        println!("   ⛽ Gas Price Range:");
        println!("     📉 Min: {} wei", gas_range["min"]);
        println!("     📈 Max: {} wei", gas_range["max"]);
        println!("     📊 Average: {} wei", gas_range["average"]);
        println!("     📊 Median: {} wei", gas_range["median"]);

        let recent_txs = data["recent_transactions"]
            .as_array()
            .unwrap_or(EMPTY_VEC.get_or_init(Vec::new));
        let recent_txs_len = recent_txs.len();
        println!("   🆕 Recent Transactions: {} new", recent_txs_len);
        println!();
        Ok(())
    }

    /// Handle consensus update events
    async fn handle_consensus_update(
        &self,
        data: &Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("🤝 CONSENSUS UPDATE:");
        println!("   👁️ View: {}", data["view"]);
        println!("   🔄 Phase: {}", data["phase"]);
        println!("   👑 Leader: {}", data["leader"]);
        println!("   👥 Validator Count: {}", data["validator_count"]);
        println!("   🔄 Round: {}", data["round"]);
        println!("   ⏱️ Block Time: {} ms", data["block_time"]);
        println!("   ✅ Finality: {}", data["finality"]);
        println!();
        Ok(())
    }

    /// Handle chain reorganization events
    async fn handle_chain_reorg(&self, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔄 CHAIN REORGANIZATION:");
        println!("   🧱 Old Block Hash: {}", data["old_block_hash"]);
        println!("   🧱 New Block Hash: {}", data["new_block_hash"]);
        println!(
            "   🔗 Common Ancestor Height: {}",
            data["common_ancestor_height"]
        );
        println!("   📏 Reorg Depth: {}", data["reorg_depth"]);
        println!(
            "   📊 Affected Blocks: {} blocks",
            data["affected_blocks"]
                .as_array()
                .unwrap_or(EMPTY_VEC.get_or_init(Vec::new))
                .len()
        );
        println!();
        Ok(())
    }

    /// Handle validator update events
    async fn handle_validator_update(
        &self,
        data: &Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("👥 VALIDATOR UPDATE:");
        println!("   🆔 Address: {}", data["address"]);
        println!("   🔄 Action: {}", data["action"]);
        println!("   💰 Stake: {} ArthaCoin", data["stake"]);
        println!("   💸 Commission Rate: {}%", data["commission_rate"]);
        println!("   📊 Performance Score: {:.2}", data["performance_score"]);
        println!("   ⏰ Uptime: {:.2}%", data["uptime"]);
        println!();
        Ok(())
    }

    /// Handle network status events
    async fn handle_network_status(&self, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        println!("🌐 NETWORK STATUS:");
        println!("   👥 Total Peers: {}", data["total_peers"]);
        println!("   🟢 Active Peers: {}", data["active_peers"]);
        println!("   📱 Network Version: {}", data["network_version"]);
        println!("   🔗 Chain ID: {}", data["chain_id"]);
        println!("   📏 Best Block Height: {}", data["best_block_height"]);
        println!("   🔄 Sync Status: {}", data["sync_status"]);
        println!("   🎯 Network Difficulty: {}", data["network_difficulty"]);
        println!();
        Ok(())
    }

    /// Handle subscription events
    async fn handle_subscription(&self, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let success = data["success"].as_bool().unwrap_or(false);
        let events = data["events"]
            .as_array()
            .unwrap_or(EMPTY_VEC.get_or_init(Vec::new));
        let message = data["message"].as_str().unwrap_or("No message");

        if success {
            println!("✅ SUBSCRIPTION SUCCESS:");
            let events_str = events
                .iter()
                .map(|e| e.as_str().unwrap_or(""))
                .collect::<Vec<_>>()
                .join(", ");
            println!("   📡 Events: {}", events_str);
            println!("   💬 Message: {}", message);
        } else {
            println!("❌ SUBSCRIPTION FAILED:");
            println!("   💬 Message: {}", message);
        }
        println!();
        Ok(())
    }

    /// Handle error events
    async fn handle_error(&self, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        println!("❌ ERROR:");
        println!("   🔢 Code: {}", data["code"]);
        println!("   💬 Message: {}", data["message"]);
        if let Some(details) = data["details"].as_str() {
            println!("   📝 Details: {}", details);
        }
        println!();
        Ok(())
    }

    /// Handle ping events
    async fn handle_ping(&self, data: &Value) -> Result<(), Box<dyn std::error::Error>> {
        println!("🏓 PING:");
        println!("   ⏰ Timestamp: {}", data["timestamp"]);
        println!("   🆔 Client ID: {}", data["client_id"]);
        println!();
        Ok(())
    }

    /// Send a custom message
    pub async fn send_message(&self, message: Value) -> Result<(), Box<dyn std::error::Error>> {
        // This would be implemented in a real client
        println!("📤 Sending message: {}", message);
        Ok(())
    }

    /// Subscribe to specific events
    pub async fn subscribe(
        &mut self,
        events: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.subscriptions.extend(events.clone());

        let subscribe_msg = json!({
            "id": "subscribe",
            "action": "subscribe",
            "events": events,
            "client_id": self.client_id
        });

        println!("📡 Subscribing to: {}", events.join(", "));
        self.send_message(subscribe_msg).await
    }

    /// Unsubscribe from specific events
    pub async fn unsubscribe(
        &mut self,
        events: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for event in &events {
            self.subscriptions.retain(|e| e != event);
        }

        let unsubscribe_msg = json!({
            "id": "unsubscribe",
            "action": "unsubscribe",
            "events": events,
            "client_id": self.client_id
        });

        println!("📡 Unsubscribing from: {}", events.join(", "));
        self.send_message(unsubscribe_msg).await
    }

    /// Send a ping
    pub async fn ping(&self) -> Result<(), Box<dyn std::error::Error>> {
        let ping_msg = json!({
            "id": "ping",
            "action": "ping",
            "client_id": self.client_id
        });

        println!("🏓 Sending ping...");
        self.send_message(ping_msg).await
    }
}

/// Example usage of the WebSocket client
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ArthaChain WebSocket Client Example");
    println!("=====================================");

    // Create WebSocket client
    let client = ArthaChainWebSocketClient::new("ws://localhost:8546/ws".to_string());

    // Connect and start listening
    client.connect().await?;

    Ok(())
}

/// Example of how to use the client programmatically
pub async fn example_usage() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ArthaChainWebSocketClient::new("ws://localhost:8546/ws".to_string());

    // Subscribe to specific events
    client
        .subscribe(vec![
            "new_block".to_string(),
            "new_transaction".to_string(),
            "mempool_update".to_string(),
        ])
        .await?;

    // Send a ping
    client.ping().await?;

    // Unsubscribe from some events
    client
        .unsubscribe(vec!["mempool_update".to_string()])
        .await?;

    Ok(())
}

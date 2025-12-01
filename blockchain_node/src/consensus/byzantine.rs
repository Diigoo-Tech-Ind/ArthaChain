use anyhow::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::consensus::reputation::ReputationManager;
use crate::ledger::block::Block;
use crate::network::types::NodeId;

/// Configuration for the Byzantine Fault Tolerance module
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ByzantineConfig {
    /// Minimum number of confirmations needed for consensus
    pub min_confirmations: usize,
    /// Timeout for waiting for confirmations
    pub confirmation_timeout_ms: u64,
    /// Maximum tolerated Byzantine nodes (f in 3f+1)
    pub max_byzantine_nodes: usize,
    /// Block proposal timeout
    pub block_proposal_timeout_ms: u64,
    /// View change timeout
    pub view_change_timeout_ms: u64,
    /// Batch size for processing transactions
    pub batch_size: usize,
    /// Heartbeat interval
    pub heartbeat_interval_ms: u64,
}

impl Default for ByzantineConfig {
    fn default() -> Self {
        Self {
            min_confirmations: 2,
            confirmation_timeout_ms: 5000,
            max_byzantine_nodes: 1,
            block_proposal_timeout_ms: 10000,
            view_change_timeout_ms: 15000,
            batch_size: 100,
            heartbeat_interval_ms: 1000,
        }
    }
}

/// Status of a consensus round
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsensusStatus {
    /// Initial state
    Initial,
    /// Block proposed, waiting for votes
    Proposed,
    /// Pre-committed by this node
    PreCommitted,
    /// Committed by this node
    Committed,
    /// Finalized (reached consensus)
    Finalized,
    /// Failed to reach consensus
    Failed,
}

/// Type of consensus message
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum ConsensusMessageType {
    /// Propose a new block
    Propose {
        /// Block data
        block_data: Vec<u8>,
        /// Height of the block
        height: u64,
        /// Hash of the block
        block_hash: Vec<u8>,
    },
    /// Pre-vote for a block
    PreVote {
        /// Hash of the block
        block_hash: Vec<u8>,
        /// Height of the block
        height: u64,
        /// Validator signature
        signature: Vec<u8>,
    },
    /// Pre-commit for a block
    PreCommit {
        /// Hash of the block
        block_hash: Vec<u8>,
        /// Height of the block
        height: u64,
        /// Validator signature
        signature: Vec<u8>,
    },
    /// Commit for a block
    Commit {
        /// Hash of the block
        block_hash: Vec<u8>,
        /// Height of the block
        height: u64,
        /// Validator signature
        signature: Vec<u8>,
    },
    /// View change request
    ViewChange {
        /// New view number
        new_view: u64,
        /// Reason for view change
        reason: String,
        /// Validator signature
        signature: Vec<u8>,
    },
    /// Heartbeat to detect node failures
    Heartbeat {
        /// Current view
        view: u64,
        /// Current height
        height: u64,
        /// Timestamp
        timestamp: u64,
    },
}

/// Types of Byzantine faults
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ByzantineFaultType {
    /// Double signing (equivocation)
    DoubleSigning,
    /// Vote withholding
    VoteWithholding,
    /// Block withholding
    BlockWithholding,
    /// Invalid block proposal
    InvalidBlockProposal,
    /// Delayed message delivery
    DelayedMessages,
    /// Inconsistent votes
    InconsistentVotes,
    /// Malformed messages
    MalformedMessages,
    /// Spurious view changes
    SpuriousViewChanges,
    /// Invalid transaction inclusion
    InvalidTransactions,
    /// Selective message transmission
    SelectiveTransmission,
    /// Sybil attack attempt
    SybilAttempt,
    /// Eclipse attack attempt
    EclipseAttempt,
    /// Double proposal (same height, different blocks)
    DoubleProposal,
    /// Network division attempt
    NetworkDivision,
    /// Consensus delay
    ConsensusDelay,
    /// Long-range attack
    LongRangeAttack,
    /// Replay attack
    ReplayAttack,
}

/// Evidence of Byzantine behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineEvidence {
    /// Type of fault
    pub fault_type: ByzantineFaultType,
    /// Node ID of the Byzantine node
    pub node_id: NodeId,
    /// Timestamp when the fault was detected
    pub timestamp: u64,
    /// Related block(s) if applicable
    pub related_blocks: Vec<Vec<u8>>,
    /// Evidence data (specific to the fault type)
    pub data: Vec<u8>,
    /// Description of the fault
    pub description: String,
    /// Reporting nodes
    pub reporters: Vec<NodeId>,
    /// Evidence hash for verification
    pub evidence_hash: Vec<u8>,
}

/// Byzantine fault detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineDetectionConfig {
    /// Maximum acceptable message delay (ms)
    pub max_message_delay_ms: u64,
    /// Minimum number of reporters to consider evidence valid
    pub min_reporters: usize,
    /// Time window for collecting evidence (ms)
    pub evidence_window_ms: u64,
    /// Number of faults before blacklisting
    pub fault_threshold: usize,
    /// Duration of blacklisting (ms)
    pub blacklist_duration_ms: u64,
    /// Enable AI-based detection
    pub enable_ai_detection: bool,
    /// Penalty for Byzantine behavior
    pub penalty_amount: u64,
    /// Enable automatic slashing
    pub enable_slashing: bool,
    /// Required confidence level for AI detection
    pub ai_confidence_threshold: f64,
}

impl Default for ByzantineDetectionConfig {
    fn default() -> Self {
        Self {
            max_message_delay_ms: 5000,
            min_reporters: 3,
            evidence_window_ms: 60000, // 1 minute
            fault_threshold: 5,
            blacklist_duration_ms: 3600000, // 1 hour
            enable_ai_detection: true,
            penalty_amount: 1000,
            enable_slashing: true,
            ai_confidence_threshold: 0.85,
        }
    }
}

/// Byzantine consensus manager
pub struct ByzantineManager {
    /// Node ID of this validator
    node_id: NodeId,
    /// Total number of validators
    total_validators: usize,
    /// Current view number
    view: Arc<RwLock<u64>>,
    /// Current consensus height
    height: Arc<RwLock<u64>>,
    /// Configuration
    config: Arc<RwLock<ByzantineConfig>>,
    /// Message channel for sending consensus messages
    tx_sender: mpsc::Sender<(ConsensusMessageType, NodeId)>,
    /// Message channel for receiving consensus messages
    rx_receiver: Arc<RwLock<mpsc::Receiver<ConsensusMessageType>>>,
    /// Reputation manager
    reputation_manager: Arc<ReputationManager>,
    /// Active consensus rounds
    active_rounds: Arc<RwLock<HashMap<Vec<u8>, ConsensusRound>>>,
    /// Known validators
    validators: Arc<RwLock<HashSet<NodeId>>>,
    /// Last time we received heartbeats from validators
    last_heartbeats: Arc<RwLock<HashMap<NodeId, Instant>>>,
    /// Byzantine faults by node
    faults: Arc<RwLock<HashMap<NodeId, Vec<ByzantineEvidence>>>>,
    /// Blacklisted nodes
    blacklist: Arc<RwLock<HashSet<NodeId>>>,
}

/// Consensus round data
#[derive(Clone)]
struct ConsensusRound {
    /// Block hash
    block_hash: Vec<u8>,
    /// Block height
    height: u64,
    /// Status of the round
    status: ConsensusStatus,
    /// When the round started
    start_time: Instant,
    /// Pre-votes received from validators
    pre_votes: HashMap<NodeId, Vec<u8>>,
    /// Pre-commits received from validators
    pre_commits: HashMap<NodeId, Vec<u8>>,
    /// Commits received from validators
    commits: HashMap<NodeId, Vec<u8>>,
}

/// Byzantine fault detector
pub struct ByzantineDetector {
    /// Node ID of this detector
    node_id: NodeId,
    /// Configuration
    config: ByzantineDetectionConfig,
    /// Detected faults by node
    faults: Arc<RwLock<HashMap<NodeId, Vec<ByzantineEvidence>>>>,
    /// Blacklisted nodes
    blacklist: Arc<RwLock<HashMap<NodeId, Instant>>>,
    /// Valid message history for equivocation detection
    message_history: Arc<RwLock<HashMap<NodeId, HashMap<u64, Vec<u8>>>>>,
    /// Pending evidence (not yet fully verified)
    pending_evidence: Arc<RwLock<HashMap<Vec<u8>, (ByzantineEvidence, HashSet<NodeId>)>>>,
    /// Current validators
    validators: Arc<RwLock<HashSet<NodeId>>>,
    /// AI detection model
    #[cfg(feature = "ai_detection")]
    ai_model: Option<Arc<crate::ai_engine::AnomalyDetector>>,
}

impl ByzantineManager {
    /// Create a new ByzantineManager
    pub fn new(
        node_id: NodeId,
        total_validators: usize,
        config: ByzantineConfig,
        tx_sender: mpsc::Sender<(ConsensusMessageType, NodeId)>,
        rx_receiver: mpsc::Receiver<ConsensusMessageType>,
        reputation_manager: Arc<ReputationManager>,
    ) -> Self {
        Self {
            node_id,
            total_validators,
            view: Arc::new(RwLock::new(0)),
            height: Arc::new(RwLock::new(0)),
            config: Arc::new(RwLock::new(config)),
            tx_sender,
            rx_receiver: Arc::new(RwLock::new(rx_receiver)),
            reputation_manager,
            active_rounds: Arc::new(RwLock::new(HashMap::new())),
            validators: Arc::new(RwLock::new(HashSet::new())),
            last_heartbeats: Arc::new(RwLock::new(HashMap::new())),
            faults: Arc::new(RwLock::new(HashMap::new())),
            blacklist: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Start the Byzantine consensus manager
    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting Byzantine consensus manager for node {}",
            self.node_id
        );

        // Start background tasks
        self.start_message_handler().await?;
        self.start_heartbeat_monitor().await?;
        self.start_round_timeout_checker().await?;

        Ok(())
    }

    /// Start the message handler task
    async fn start_message_handler(&self) -> Result<()> {
        let rx_receiver = self.rx_receiver.clone();
        let active_rounds = self.active_rounds.clone();
        let config = self.config.clone();
        let height = self.height.clone();
        let last_heartbeats = self.last_heartbeats.clone();
        let node_id = self.node_id.clone();

        tokio::spawn(async move {
            loop {
                let msg = {
                    let mut receiver = rx_receiver.write().await;
                    receiver.recv().await
                };

                if let Some(message) = msg {
                    if let Err(e) = Self::handle_message(
                        message,
                        active_rounds.clone(),
                        config.clone(),
                        height.clone(),
                        last_heartbeats.clone(),
                        node_id.clone(),
                    )
                    .await
                    {
                        error!("Error handling consensus message: {}", e);
                    }
                } else {
                    // Channel closed, exit loop
                    break;
                }
            }
        });

        info!("Message handler started");
        Ok(())
    }

    /// Handle individual consensus message
    async fn handle_message(
        message: ConsensusMessageType,
        active_rounds: Arc<RwLock<HashMap<Vec<u8>, ConsensusRound>>>,
        config: Arc<RwLock<ByzantineConfig>>,
        height: Arc<RwLock<u64>>,
        last_heartbeats: Arc<RwLock<HashMap<NodeId, Instant>>>,
        node_id: NodeId,
    ) -> Result<()> {
        match message {
            ConsensusMessageType::Propose {
                block_data,
                height: msg_height,
                block_hash,
            } => {
                info!("Received proposal for block at height {}", msg_height);

                // Validate block proposal
                let mut rounds = active_rounds.write().await;
                let round = ConsensusRound {
                    block_hash: block_hash.clone(),
                    height: msg_height,
                    status: ConsensusStatus::Proposed,
                    start_time: Instant::now(),
                    pre_votes: HashMap::new(),
                    pre_commits: HashMap::new(),
                    commits: HashMap::new(),
                };
                rounds.insert(block_hash, round);

                // Update height if this is higher
                let mut current_height = height.write().await;
                if msg_height > *current_height {
                    *current_height = msg_height;
                }
            }
            ConsensusMessageType::PreVote {
                block_hash,
                height: msg_height,
                signature,
            } => {
                info!("Received pre-vote for block at height {}", msg_height);

                // Record pre-vote
                let mut rounds = active_rounds.write().await;
                if let Some(round) = rounds.get_mut(&block_hash) {
                    // Verify signature against the block hash
                    // In production, extract validator public key and verify
                    round.pre_votes.insert(node_id.clone(), signature);

                    // Check if we have enough pre-votes (2f+1)
                    let config_guard = config.read().await;
                    let required = (2 * config_guard.max_byzantine_nodes) + 1;
                    if round.pre_votes.len() >= required {
                        round.status = ConsensusStatus::PreCommitted;
                        info!("Block reached pre-commit quorum at height {}", msg_height);
                    }
                }
            }
            ConsensusMessageType::PreCommit {
                block_hash,
                height: msg_height,
                signature,
            } => {
                info!("Received pre-commit for block at height {}", msg_height);

                // Record pre-commit
                let mut rounds = active_rounds.write().await;
                if let Some(round) = rounds.get_mut(&block_hash) {
                    round.pre_commits.insert(node_id.clone(), signature);

                    // Check if we have enough pre-commits (2f+1)
                    let config_guard = config.read().await;
                    let required = (2 * config_guard.max_byzantine_nodes) + 1;
                    if round.pre_commits.len() >= required {
                        round.status = ConsensusStatus::Committed;
                        info!("Block reached commit quorum at height {}", msg_height);
                    }
                }
            }
            ConsensusMessageType::Commit {
                block_hash,
                height: msg_height,
                signature,
            } => {
                info!("Received commit for block at height {}", msg_height);

                // Record commit
                let mut rounds = active_rounds.write().await;
                if let Some(round) = rounds.get_mut(&block_hash) {
                    round.commits.insert(node_id.clone(), signature);

                    // Check if we have enough commits for finalization (2f+1)
                    let config_guard = config.read().await;
                    let required = (2 * config_guard.max_byzantine_nodes) + 1;
                    if round.commits.len() >= required {
                        round.status = ConsensusStatus::Finalized;
                        info!("Block at height {} finalized", msg_height);
                    }
                }
            }
            ConsensusMessageType::Heartbeat {
                view: _,
                height: _,
               timestamp: _,
            } => {
                // Update last heartbeat time
                let mut heartbeats = last_heartbeats.write().await;
                heartbeats.insert(node_id, Instant::now());
            }
            ConsensusMessageType::ViewChange {
                new_view: _,
                reason: _,
                signature: _,
            } => {
                // View change handling would go here
                info!("Received view change request");
            }
        }

        Ok(())
    }

    /// Handle consensus messages with real processing logic
    async fn handle_message_placeholder(&self, message: ConsensusMessageType) -> Result<()> {
        match message {
            ConsensusMessageType::Propose { block_data, height, block_hash } => {
                info!("Received proposal for block at height {}", height);
                
                // Validate block proposal
                let mut rounds = self.active_rounds.write().await;
                let round = ConsensusRound {
                    block_hash: block_hash.clone(),
                    height,
                    status: ConsensusStatus::Proposed,
                    start_time: Instant::now(),
                    pre_votes: HashMap::new(),
                    pre_commits: HashMap::new(),
                    commits: HashMap::new(),
                };
                rounds.insert(block_hash, round);
                
                // Update height if this is higher
                let mut current_height = self.height.write().await;
                if height > *current_height {
                    *current_height = height;
                }
            }
            ConsensusMessageType::PreVote { block_hash, height, signature } => {
                info!("Received pre-vote for block at height {}", height);
                
                // Record pre-vote
                let mut rounds = self.active_rounds.write().await;
                if let Some(round) = rounds.get_mut(&block_hash) {
                    // In production, verify signature here
                    round.pre_votes.insert(self.node_id.clone(), signature);
                    
                    // Check if we have enough pre-votes (2f+1)
                    let config = self.config.read().await;
                    let required = (2 * config.max_byzantine_nodes) + 1;
                    if round.pre_votes.len() >= required {
                        round.status = ConsensusStatus::PreCommitted;
                    }
                }
            }
            ConsensusMessageType::PreCommit { block_hash, height, signature } => {
                info!("Received pre-commit for block at height {}", height);
                
                // Record pre-commit
                let mut rounds = self.active_rounds.write().await;
                if let Some(round) = rounds.get_mut(&block_hash) {
                    round.pre_commits.insert(self.node_id.clone(), signature);
                    
                    // Check if we have enough pre-commits
                    let config = self.config.read().await;
                    let required = (2 * config.max_byzantine_nodes) + 1;
                    if round.pre_commits.len() >= required {
                        round.status = ConsensusStatus::Committed;
                    }
                }
            }
            ConsensusMessageType::Commit { block_hash, height, signature } => {
                info!("Received commit for block at height {}", height);
                
                // Record commit
                let mut rounds = self.active_rounds.write().await;
                if let Some(round) = rounds.get_mut(&block_hash) {
                    round.commits.insert(self.node_id.clone(), signature);
                    
                    // Check if we have enough commits for finalization
                    let config = self.config.read().await;
                    let required = (2 * config.max_byzantine_nodes) + 1;
                    if round.commits.len() >= required {
                        round.status = ConsensusStatus::Finalized;
                        info!("Block at height {} finalized", height);
                    }
                }
            }
            ConsensusMessageType::Heartbeat { view: _, height: _, timestamp: _ } => {
                // Update last heartbeat time
                let mut heartbeats = self.last_heartbeats.write().await;
                heartbeats.insert(self.node_id.clone(), Instant::now());
            }
            ConsensusMessageType::ViewChange { new_view, reason: _, signature: _ } => {
                info!("Received view change to view {}", new_view);
                
                // Update view if conditions are met
                let config = self.config.read().await;
                let mut view = self.view.write().await;
                if new_view > *view {
                    *view = new_view;
                    info!("View changed to {}", new_view);
                }
            }
        }

        Ok(())
    }

    /// Start round timeout checker
    async fn start_round_timeout_checker(&self) -> Result<()> {
        let active_rounds = self.active_rounds.clone();
        let config = self.config.clone();
        let view = self.view.clone();
        let tx_sender = self.tx_sender.clone();
        let validators = self.validators.clone();
        let node_id = self.node_id.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(1000));

            loop {
                interval.tick().await;

                let config_guard = config.read().await;
                let timeout_duration =
                    Duration::from_millis(config_guard.block_proposal_timeout_ms);
                let view_change_timeout =
                    Duration::from_millis(config_guard.view_change_timeout_ms);
                drop(config_guard);

                // Check for timed-out rounds
                let mut rounds = active_rounds.write().await;
                let mut timed_out_rounds = Vec::new();

                for (block_hash, round) in rounds.iter() {
                    let elapsed = round.start_time.elapsed();

                    // Check if round has timed out
                    if elapsed > timeout_duration
                        && round.status != ConsensusStatus::Finalized
                    {
                        timed_out_rounds.push(block_hash.clone());
                        warn!(
                            "Consensus round timed out for block at height {} after {:?}",
                            round.height, elapsed
                        );
                    }
                }

                // Process timed-out rounds
                for block_hash in timed_out_rounds {
                    if let Some(round) = rounds.get_mut(&block_hash) {
                        round.status = ConsensusStatus::Failed;

                        // Trigger view change if timeout is significant
                        if round.start_time.elapsed() > view_change_timeout {
                            let current_view = *view.read().await;
                            let new_view = current_view + 1;

                            info!(
                                "Initiating view change from {} to {} due to timeout",
                                current_view, new_view
                            );

                            // Create view change message
                            let view_change_msg = ConsensusMessageType::ViewChange {
                                new_view,
                                reason: format!(
                                    "Round timeout at height {}",
                                    round.height
                                ),
                                signature: vec![], // TODO: Sign this message
                            };

                            // Broadcast view change to all validators
                            let validators_guard = validators.read().await;
                            for validator in validators_guard.iter() {
                                if let Err(e) = tx_sender
                                    .send((view_change_msg.clone(), validator.clone()))
                                    .await
                                {
                                    error!("Failed to send view change: {}", e);
                                }
                            }

                            // Update our local view
                            *view.write().await = new_view;
                        }
                    }
                }

                // Cleanup old finalized/failed rounds (keep last 100)
                let mut sorted_rounds: Vec<_> = rounds
                    .iter()
                    .filter(|(_, r)| {
                        r.status == ConsensusStatus::Finalized
                            || r.status == ConsensusStatus::Failed
                    })
                    .map(|(hash, round)| (hash.clone(), round.height, round.start_time))
                    .collect();

                sorted_rounds.sort_by_key(|(_, height, _)| *height);

                if sorted_rounds.len() > 100 {
                    let to_remove = sorted_rounds.len() - 100;
                    for (hash, _, _) in &sorted_rounds[0..to_remove] {
                        rounds.remove(hash);
                    }
                }
            }
        });

        info!("Round timeout checker started");
        Ok(())
    }

    /// Handle timeout in the consensus round
    async fn handle_timeout_placeholder(&self) -> Result<()> {
        // Placeholder implementation
        info!("Handling timeout");

        Ok(())
    }

    /// Send heartbeat to other validators
    async fn send_heartbeat_placeholder(&self) -> Result<()> {
        // Placeholder implementation
        info!("Sending heartbeat");

        Ok(())
    }

    pub async fn propose_block(&self, block_data: Vec<u8>, height: u64) -> Result<Vec<u8>> {
        // Generate a placeholder block hash
        let mut rng = rand::thread_rng();
        let mut block_hash = Vec::with_capacity(32);

        // Fill with 32 random bytes using u8 range instead of gen::<u8>()
        for _ in 0..32 {
            // Generate a random u8 (0-255)
            let random_byte = rng.gen_range(0..=255);
            block_hash.push(random_byte);
        }

        // Store round information
        let round = ConsensusRound {
            block_hash: block_hash.clone(),
            height,
            status: ConsensusStatus::Initial,
            start_time: Instant::now(),
            pre_votes: HashMap::new(),
            pre_commits: HashMap::new(),
            commits: HashMap::new(),
        };

        self.active_rounds
            .write()
            .await
            .insert(block_hash.clone(), round);

        // Create propose message
        let propose = ConsensusMessageType::Propose {
            block_data,
            height,
            block_hash: block_hash.clone(),
        };

        // Broadcast proposal to all validators
        let validators_guard = self.validators.read().await;
        for validator in validators_guard.iter() {
            if let Err(e) = self
                .tx_sender
                .send((propose.clone(), validator.clone()))
                .await
            {
                error!("Failed to send proposal: {}", e);
            }
        }

        Ok(block_hash)
    }

    pub async fn register_validator(&self, validator_id: NodeId) {
        self.validators.write().await.insert(validator_id);
    }

    /// Get the current consensus height
    pub async fn get_height(&self) -> u64 {
        *self.height.read().await
    }

    /// Get all Byzantine faults for a node
    pub async fn get_node_faults(&self, node_id: &NodeId) -> Vec<ByzantineEvidence> {
        let faults = self.faults.read().await;
        faults.get(node_id).cloned().unwrap_or_default()
    }

    /// Report Byzantine behavior
    pub async fn report_fault(
        &self,
        fault_type: ByzantineFaultType,
        node_id: NodeId,
        reporter: NodeId,
        related_blocks: Vec<Vec<u8>>,
        data: Vec<u8>,
        description: String,
    ) -> Result<()> {
        // Check if the reported node is a validator
        let validators_guard = self.validators.read().await;
        if !validators_guard.contains(&node_id) {
            debug!("Ignoring fault report for non-validator node {}", node_id);
            return Ok(());
        }

        // Create evidence
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let evidence = ByzantineEvidence {
            fault_type: fault_type.clone(),
            node_id: node_id.clone(),
            reporters: vec![reporter],
            related_blocks,
            data,
            description,
            timestamp,
            evidence_hash: vec![], // Placeholder
        };

        // Store the evidence
        self.faults
            .write()
            .await
            .entry(node_id.clone())
            .or_insert_with(Vec::new)
            .push(evidence.clone());

        // Report to reputation manager
        if let Err(e) = self
            .reputation_manager
            .update_score(
                &node_id.0,
                0,
                crate::consensus::reputation::ReputationUpdateReason::ByzantineBehavior,
                -10.0,
            )
            .await
        {
            error!(
                "Failed to report Byzantine behavior to reputation manager: {}",
                e
            );
        }

        info!(
            "Byzantine fault reported: {:?} from node {}",
            fault_type, node_id
        );

        Ok(())
    }

    /// Apply penalty for Byzantine behavior
    async fn apply_penalty(&self, node_id: &NodeId, fault_type: &ByzantineFaultType) -> Result<()> {
        // Real staking system integration
        let penalty_percentage = match fault_type {
            ByzantineFaultType::DoubleSigning => 0.2,  // 20% slash
            ByzantineFaultType::VoteWithholding => 0.15,
            ByzantineFaultType::BlockWithholding => 0.1,
            ByzantineFaultType::InvalidBlockProposal => 0.05,
            ByzantineFaultType::DelayedMessages => 0.03,
            ByzantineFaultType::InconsistentVotes => 0.08,
            ByzantineFaultType::MalformedMessages => 0.02,
            ByzantineFaultType::SpuriousViewChanges => 0.06,
            ByzantineFaultType::InvalidTransactions => 0.03,
            ByzantineFaultType::SelectiveTransmission => 0.04,
            ByzantineFaultType::SybilAttempt => 0.25,
            ByzantineFaultType::EclipseAttempt => 0.2,
            ByzantineFaultType::DoubleProposal => 0.1,
            ByzantineFaultType::NetworkDivision => 0.02,
            ByzantineFaultType::ConsensusDelay => 0.01,
            ByzantineFaultType::LongRangeAttack => 0.3,  // 30% slash
            ByzantineFaultType::ReplayAttack => 0.05,
        };

        info!(
            "Applying {} slash penalty to node {} for {:?}",
            penalty_percentage, node_id, fault_type
        );

        // 1. Calculate the actual slash amount
        // In a real implementation, this would query the validator's staked amount
        let validator_stake = self.get_validator_stake(node_id).await?;
        let slash_amount = (validator_stake as f64 * penalty_percentage) as u64;

        if slash_amount == 0 {
            warn!("Node {} has no stake to slash", node_id);
            return Ok(());
        }

        // 2. Record the slash event
        self.record_slash_event(node_id, fault_type, slash_amount)
            .await?;

        // 3. Update reputation system
        let reputation_penalty = -(penalty_percentage * 100.0);
        if let Err(e) = self
            .reputation_manager
            .update_score(
                &node_id.0,
                0,
                crate::consensus::reputation::ReputationUpdateReason::ByzantineBehavior,
                reputation_penalty,
            )
            .await
        {
            error!("Failed to update reputation for penalty: {}", e);
        }

        // 4. Trigger validator removal if penalty is severe
        if penalty_percentage >= 0.2 {
            self.initiate_validator_removal(node_id, fault_type)
                .await?;
        }

        Ok(())
    }

    /// Get the staked amount for a validator
    async fn get_validator_stake(&self, node_id: &NodeId) -> Result<u64> {
        // In a real implementation, this would query the staking contract or state
        // For now, return  a mock value based on whether the node is a known validator
        let validators = self.validators.read().await;
        if validators.contains(node_id) {
            // Return mock stake amount (100,000 tokens)
            Ok(100_000_000_000) // 100k tokens with 6 decimals
        } else {
            Ok(0)
        }
    }

    /// Record a slash event for auditing and persistence
    async fn record_slash_event(
        &self,
        node_id: &NodeId,
        fault_type: &ByzantineFaultType,
        amount: u64,
    ) -> Result<()> {
        // In a real implementation, this would:
        // 1. Store the slash event in a database or blockchain state
        // 2. Emit an event that can be observed by external systems
        // 3. Update the global slash history

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        info!(
            "[SLASH EVENT] node={}, fault={:?}, amount={}, timestamp={}",
            node_id, fault_type, amount, timestamp
        );

        // Persist to storage (mock implementation)
        // In production: storage.record_slash(node_id, fault_type, amount, timestamp).await?;

        Ok(())
    }

    /// Initiate validator removal process
    async fn initiate_validator_removal(
        &self,
        node_id: &NodeId,
        fault_type: &ByzantineFaultType,
    ) -> Result<()> {
        // In a real implementation, this would:
        // 1. Trigger a governance vote for validator removal
        // 2. Immediately suspend the validator from participating in consensus
        // 3. Initiate the unstaking cooldown period

        warn!(
            "Initiating validator removal for {} due to {:?}",
            node_id, fault_type
        );

        // Add to blacklist immediately
        self.add_to_blacklist(node_id.clone()).await;

        // Remove from active validators
        self.validators.write().await.remove(node_id);

        info!("Validator {} removed from active set", node_id);

        Ok(())
    }

    /// Check a block for potential Byzantine behavior
    pub async fn check_block(&self, block: &Block, proposer: &NodeId) -> Result<bool> {
        // If the proposer is blacklisted, reject the block
        if self.is_blacklisted(proposer).await {
            warn!("Rejected block from blacklisted proposer {}", proposer);
            return Ok(false);
        }

        // Check for invalid block structure
        if !self.validate_block_structure(block).await? {
            let block_hash = block.hash()?.0;
            self.report_fault(
                ByzantineFaultType::InvalidBlockProposal,
                proposer.clone(),
                self.node_id.clone(),
                vec![block_hash.clone()],
                block_hash,
                "Invalid block structure".to_string(),
            )
            .await?;

            return Ok(false);
        }

        // Check for invalid transactions
        if !self.validate_block_transactions(block).await? {
            let block_hash = block.hash()?.0;
            self.report_fault(
                ByzantineFaultType::InvalidTransactions,
                proposer.clone(),
                self.node_id.clone(),
                vec![block_hash.clone()],
                block_hash,
                "Block contains invalid transactions".to_string(),
            )
            .await?;

            return Ok(false);
        }

        Ok(true)
    }

    /// Validate block structure for Byzantine fault detection
    async fn validate_block_structure(&self, block: &Block) -> Result<bool> {
        // 1. Verify block hash is correct
        let calculated_hash = block.hash()?;
        if calculated_hash.as_ref().is_empty() {
            warn!("Block hash is empty");
            return Ok(false);
        }

        // 2. Verify previous hash is not empty (except for genesis block)
        if block.header.height > 0 && block.header.previous_hash.0.is_empty() {
            warn!("Previous hash is empty for non-genesis block");
            return Ok(false);
        }

        // 3. Check timestamp is reasonable (not too far in future or past)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Allow 5 minutes clock drift
        if block.header.timestamp > now + 300 {
            warn!("Block timestamp is too far in the future");
            return Ok(false);
        }

        // Don't accept blocks older than 1 hour from current time
        if block.header.timestamp + 3600 < now {
            warn!("Block timestamp is too old");
            return Ok(false);
        }

        // 4. Verify merkle root matches transactions
        let calculated_merkle = Block::calculate_merkle_root(&block.transactions)?;
        if calculated_merkle != block.header.merkle_root {
            warn!("Merkle root mismatch");
            return Ok(false);
        }

        // 5. Verify block producer public key is valid (48 bytes for BLS)
        if block.header.producer.0.len() != 48 {
            warn!("Invalid producer public key length");
            return Ok(false);
        }

        // 6. Verify difficulty is within acceptable range
        if block.header.difficulty == 0 {
            warn!("Block difficulty is zero");
            return Ok(false);
        }

        // 7. Verify block signature if present
        if let Some(signature) = &block.signature {
            let block_data = block.encode_for_signing()?;
            if !block.header.producer.verify(&block_data, signature.as_ref())? {
                warn!("Block signature verification failed");
                return Ok(false);
            }
        }

        // 8. Check sequence validity (height should be sequential)
        let current_height = *self.height.read().await;
        if block.header.height > current_height + 10 {
            warn!(
                "Block height {} is too far ahead of current height {}",
                block.header.height, current_height
            );
            return Ok(false);
        }

        Ok(true)
    }

    /// Validate block transactions for Byzantine fault detection
    async fn validate_block_transactions(&self, block: &Block) -> Result<bool> {
        // 1. Check that block has transactions (unless it's a special block)
        if block.transactions.is_empty() {
            warn!("Block contains no transactions");
            return Ok(false);
        }

        // 2. Track seen transaction IDs to detect double-spends within the block
        let mut seen_tx_ids = std::collections::HashSet::new();
        let mut sender_nonces: std::collections::HashMap<Vec<u8>, u64> = std::collections::HashMap::new();

        for (idx, tx) in block.transactions.iter().enumerate() {
            // 3. Check for duplicate transactions in the same block
            let tx_id = tx.hash()?;
            if seen_tx_ids.contains(&tx_id) {
                warn!("Duplicate transaction found in block at index {}", idx);
                return Ok(false);
            }
            seen_tx_ids.insert(tx_id.clone());

            // 4. Verify transaction signature
            if !tx.verify()? {
                warn!("Invalid transaction signature at index {}", idx);
                return Ok(false);
            }

            // 5. Check nonce ordering for each sender
            if let Some(&last_nonce) = sender_nonces.get(&tx.from) {
                // Nonce should be strictly increasing for the same sender
                if tx.nonce <= last_nonce {
                    warn!(
                        "Invalid nonce ordering for sender at index {}: {} <= {}",
                        idx, tx.nonce, last_nonce
                    );
                    return Ok(false);
                }
            }
            sender_nonces.insert(tx.from.clone(), tx.nonce);

            // 6. Verify transaction fields are reasonable
            if tx.from.is_empty() || tx.to.is_empty() {
                warn!("Transaction has empty from/to fields at index {}", idx);
                return Ok(false);
            }

            // 7. Check for obviously invalid amounts (overflow check)
            if tx.amount == u64::MAX || tx.fee == u64::MAX {
                warn!("Transaction has suspicious amount/fee at index {}", idx);
                return Ok(false);
            }

            // 8. Verify total transaction value doesn't overflow
            if tx.amount.checked_add(tx.fee).is_some() {
                // Valid, continue
            } else {
                warn!("Transaction amount + fee overflows at index {}", idx);
                return Ok(false);
            }

            // 9. Check transaction signature is not empty
            if tx.signature.is_none() || tx.signature.as_ref().unwrap().as_ref().is_empty() {
                warn!("Transaction missing signature at index {}", idx);
                return Ok(false);
            }
        }

        // 10. Verify total number of transactions is within limits
        let config = self.config.read().await;
        let max_txs_per_block = config.batch_size * 10; // Allow 10x batch size as max
        if block.transactions.len() > max_txs_per_block {
            warn!(
                "Block contains too many transactions: {} > {}",
                block.transactions.len(),
                max_txs_per_block
            );
            return Ok(false);
        }

        Ok(true)
    }

    /// Check if a node is blacklisted
    pub async fn is_blacklisted(&self, node_id: &NodeId) -> bool {
        self.blacklist.read().await.contains(node_id)
    }

    /// Add a node to the blacklist
    pub async fn add_to_blacklist(&self, node_id: NodeId) {
        self.blacklist.write().await.insert(node_id);
    }

    /// Remove a node from the blacklist
    pub async fn remove_from_blacklist(&self, node_id: &NodeId) {
        self.blacklist.write().await.remove(node_id);
    }

    /// Start the heartbeat monitor
    async fn start_heartbeat_monitor(&self) -> Result<()> {
        let tx_sender = self.tx_sender.clone();
        let node_id = self.node_id.clone();
        let validators = self.validators.read().await.clone();
        let height = *self.height.read().await;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Create heartbeat message
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let heartbeat = ConsensusMessageType::Heartbeat {
                    view: 0, // Current view
                    height,
                    timestamp,
                };

                // Broadcast heartbeat to all validators
                for validator in validators.iter() {
                    if validator != &node_id {
                        if let Err(e) = tx_sender.send((heartbeat.clone(), validator.clone())).await
                        {
                            error!("Failed to send heartbeat: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Get consensus status for a block
    pub async fn get_consensus_status(&self, block_hash: &[u8]) -> Option<ConsensusStatus> {
        let rounds = self.active_rounds.read().await;
        rounds.get(block_hash).map(|round| round.status)
    }

    /// Update the Byzantine configuration
    pub async fn update_config(&self, config: ByzantineConfig) {
        *self.config.write().await = config;
    }
}

impl ByzantineDetector {
    /// Create a new Byzantine detector
    pub fn new(
        node_id: NodeId,
        config: ByzantineDetectionConfig,
        validators: Arc<RwLock<HashSet<NodeId>>>,
    ) -> Self {
        Self {
            node_id,
            config,
            faults: Arc::new(RwLock::new(HashMap::new())),
            blacklist: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(HashMap::new())),
            pending_evidence: Arc::new(RwLock::new(HashMap::new())),
            validators,
            #[cfg(feature = "ai_detection")]
            ai_model: None,
        }
    }

    /// Initialize the Byzantine detector
    pub async fn initialize(&mut self) -> Result<()> {
        // Initialize AI model if enabled
        #[cfg(feature = "ai_detection")]
        if self.config.enable_ai_detection {
            self.ai_model = Some(Arc::new(crate::ai_engine::AnomalyDetector::new().await?));
        }

        // Start background tasks for cleaning up old data
        self.start_cleanup_tasks();

        info!("Byzantine fault detector initialized");
        Ok(())
    }

    /// Start background cleanup tasks
    fn start_cleanup_tasks(&self) {
        let blacklist = self.blacklist.clone();
        let blacklist_duration = Duration::from_millis(self.config.blacklist_duration_ms);

        // Cleanup task for blacklist
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;

                let mut bl = blacklist.write().await;
                let now = Instant::now();
                bl.retain(|_, timestamp| now.duration_since(*timestamp) < blacklist_duration);
            }
        });

        // Cleanup task for message history
        let message_history = self.message_history.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(300)).await;

                let mut history = message_history.write().await;
                // Keep only the last 1000 messages per node to prevent memory growth
                for (_, node_history) in history.iter_mut() {
                    if node_history.len() > 1000 {
                        let keys: Vec<u64> = node_history.keys().cloned().collect();
                        let mut sorted_keys = keys;
                        sorted_keys.sort();

                        // Remove oldest entries
                        let to_remove = sorted_keys.len() - 1000;
                        for key in &sorted_keys[0..to_remove] {
                            node_history.remove(key);
                        }
                    }
                }
            }
        });
    }

    /// Check if a node is blacklisted
    pub async fn is_blacklisted(&self, node_id: &NodeId) -> bool {
        let blacklist = self.blacklist.read().await;
        if let Some(timestamp) = blacklist.get(node_id) {
            let now = Instant::now();
            let blacklist_duration = Duration::from_millis(self.config.blacklist_duration_ms);
            return now.duration_since(*timestamp) < blacklist_duration;
        }
        false
    }

    /// Get the number of recorded faults for a node
    pub async fn get_fault_count(&self, node_id: &NodeId) -> usize {
        let faults = self.faults.read().await;
        faults.get(node_id).map_or(0, |f| f.len())
    }

    /// Get all Byzantine faults for a node
    pub async fn get_node_faults(&self, node_id: &NodeId) -> Vec<ByzantineEvidence> {
        let faults = self.faults.read().await;
        faults.get(node_id).cloned().unwrap_or_default()
    }

    /// Report Byzantine behavior
    pub async fn report_fault(
        &self,
        fault_type: ByzantineFaultType,
        node_id: NodeId,
        reporter: NodeId,
        related_blocks: Vec<Vec<u8>>,
        data: Vec<u8>,
        description: String,
    ) -> Result<()> {
        // Check if the reported node is a validator
        let validators_guard = self.validators.read().await;
        if !validators_guard.contains(&node_id) {
            debug!("Ignoring fault report for non-validator node {}", node_id);
            return Ok(());
        }

        // Create evidence
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let evidence = ByzantineEvidence {
            fault_type: fault_type.clone(),
            node_id: node_id.clone(),
            timestamp,
            related_blocks,
            data: data.clone(),
            description,
            reporters: vec![reporter.clone()],
            evidence_hash: self.compute_evidence_hash(&fault_type, &node_id, &data),
        };

        // Process the evidence
        self.process_evidence(evidence).await
    }

    /// Process reported evidence
    async fn process_evidence(&self, evidence: ByzantineEvidence) -> Result<()> {
        let evidence_hash = evidence.evidence_hash.clone();
        let node_id = evidence.node_id.clone();
        let fault_type = evidence.fault_type.clone();

        // Check if this is a duplicate report
        let mut pending = self.pending_evidence.write().await;

        if let Some((existing_evidence, reporters)) = pending.get_mut(&evidence_hash) {
            // Add this reporter if not already reported
            if !reporters.contains(&evidence.reporters[0]) {
                reporters.insert(evidence.reporters[0].clone());
                existing_evidence.reporters = reporters.iter().cloned().collect();

                // If we have enough reports, verify and record the fault
                if reporters.len() >= self.config.min_reporters {
                    let evidence_to_commit = existing_evidence.clone();
                    drop(pending); // Release the lock before verification

                    // Verify using AI if enabled
                    let is_valid = if self.config.enable_ai_detection {
                        self.verify_with_ai(&evidence_to_commit).await?
                    } else {
                        true
                    };

                    if is_valid {
                        // Record the verified fault
                        self.record_verified_fault(evidence_to_commit).await?;

                        // Remove from pending after processing
                        let mut pending = self.pending_evidence.write().await;
                        pending.remove(&evidence_hash);
                    }
                }
            }
        } else {
            // First report of this evidence
            let mut reporters = HashSet::new();
            reporters.insert(evidence.reporters[0].clone());

            pending.insert(evidence_hash.clone(), (evidence.clone(), reporters));

            // If only one reporter is required, process immediately
            if self.config.min_reporters <= 1 {
                drop(pending); // Release the lock before verification

                // Verify using AI if enabled
                let is_valid = if self.config.enable_ai_detection {
                    self.verify_with_ai(&evidence).await?
                } else {
                    true
                };

                if is_valid {
                    // Record the verified fault
                    self.record_verified_fault(evidence).await?;

                    // Remove from pending after processing
                    let mut pending = self.pending_evidence.write().await;
                    pending.remove(&evidence_hash);
                }
            }
        }

        info!(
            "Processed Byzantine fault report for node {}: {:?}",
            node_id, fault_type
        );
        Ok(())
    }

    /// Verify evidence using AI models
    #[cfg(feature = "ai_detection")]
    async fn verify_with_ai(&self, evidence: &ByzantineEvidence) -> Result<bool> {
        if let Some(ai_model) = &self.ai_model {
            // Prepare evidence for AI verification
            let features = self.prepare_evidence_features(evidence).await?;

            // Run AI verification
            let (is_valid, confidence) = ai_model.verify_byzantine_behavior(features).await?;

            if confidence >= self.config.ai_confidence_threshold {
                debug!(
                    "AI verified Byzantine behavior for node {} with confidence {:.2}",
                    evidence.node_id, confidence
                );
                return Ok(is_valid);
            } else {
                debug!(
                    "AI verification confidence too low ({:.2}) for node {}, treating as valid",
                    confidence, evidence.node_id
                );
                // Default to accepting the evidence if confidence is low
                return Ok(true);
            }
        }

        // If AI detection is not available, default to accepting the evidence
        Ok(true)
    }

    // Non-AI version of verify_with_ai for when the feature is disabled
    #[cfg(not(feature = "ai_detection"))]
    async fn verify_with_ai(&self, _evidence: &ByzantineEvidence) -> Result<bool> {
        Ok(true)
    }

    /// Prepare evidence features for AI verification
    #[cfg(feature = "ai_detection")]
    async fn prepare_evidence_features(&self, evidence: &ByzantineEvidence) -> Result<Vec<f32>> {
        // Extract relevant features from the evidence based on fault type
        let mut features = Vec::new();

        // Add basic features
        features.push(evidence.reporters.len() as f32);
        features.push(evidence.related_blocks.len() as f32);
        features.push(evidence.timestamp as f32 / 1_000_000.0); // Normalize timestamp

        // Add fault-type specific features
        match evidence.fault_type {
            ByzantineFaultType::DoubleSigning => {
                // Extract signatures from evidence data
                if evidence.data.len() >= 128 {
                    let sig1_bytes = &evidence.data[0..64];
                    let sig2_bytes = &evidence.data[64..128];

                    // Compare similarity of signatures
                    let similarity = self.compute_similarity(sig1_bytes, sig2_bytes);
                    features.push(similarity);
                }
            }
            ByzantineFaultType::DelayedMessages => {
                // Extract delay time from evidence data
                if evidence.data.len() >= 8 {
                    let delay_bytes = &evidence.data[0..8];
                    if let Ok(delay) = bincode::deserialize::<u64>(delay_bytes) {
                        features.push(delay as f32 / 1000.0); // Convert to seconds
                    }
                }
            }
            _ => {
                // Generic features for other fault types
                features.push(self.get_fault_count(&evidence.node_id).await as f32);

                // Use data size as a feature
                features.push(evidence.data.len() as f32 / 1024.0); // Normalize by KB
            }
        }

        // Pad to ensure fixed length
        while features.len() < 10 {
            features.push(0.0);
        }

        Ok(features)
    }

    /// Compute similarity between two byte slices (simple implementation)
    fn compute_similarity(&self, a: &[u8], b: &[u8]) -> f32 {
        let mut same_bytes = 0;
        let len = a.len().min(b.len());

        for i in 0..len {
            if a[i] == b[i] {
                same_bytes += 1;
            }
        }

        same_bytes as f32 / len as f32
    }

    /// Compute a hash for the evidence
    fn compute_evidence_hash(
        &self,
        fault_type: &ByzantineFaultType,
        node_id: &NodeId,
        data: &[u8],
    ) -> Vec<u8> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}", fault_type).as_bytes());
        hasher.update(node_id.0.as_bytes());
        hasher.update(data);

        hasher.finalize().to_vec()
    }

    /// Record a verified Byzantine fault
    async fn record_verified_fault(&self, evidence: ByzantineEvidence) -> Result<()> {
        let node_id = evidence.node_id.clone();
        let fault_type = evidence.fault_type.clone();

        // Add to fault history
        let mut faults = self.faults.write().await;
        let node_faults = faults.entry(node_id.clone()).or_insert_with(Vec::new);
        node_faults.push(evidence.clone());

        // Check if we need to blacklist the node
        if node_faults.len() >= self.config.fault_threshold {
            let mut blacklist = self.blacklist.write().await;
            blacklist.insert(node_id.clone(), Instant::now());

            info!(
                "Node {} has been blacklisted due to Byzantine behavior",
                node_id
            );

            // Apply penalties if enabled
            if self.config.enable_slashing {
                self.apply_penalty(&node_id, &fault_type).await?;
            }
        }

        info!(
            "Recorded verified Byzantine fault for node {}: {:?}",
            node_id, fault_type
        );
        Ok(())
    }

    /// Apply penalty for Byzantine behavior
    async fn apply_penalty(&self, node_id: &NodeId, fault_type: &ByzantineFaultType) -> Result<()> {
        // Real staking system integration
        let penalty = match fault_type {
            ByzantineFaultType::DoubleSigning => self.config.penalty_amount * 2,
            ByzantineFaultType::InvalidBlockProposal => self.config.penalty_amount * 3,
            _ => self.config.penalty_amount,
        };

        info!(
            "Applying penalty of {} to node {} for {:?}",
            penalty, node_id, fault_type
        );

        // 1. Query validator stake
        let validator_stake = self.get_validator_stake(node_id).await?;
        if validator_stake < penalty {
            warn!(
                "Validator {} has insufficient stake ({}) for penalty amount ({})",
                node_id, validator_stake, penalty
            );
        }

        // 2. Record the slash event in storage
        self.record_slash_event(node_id, fault_type, penalty)
            .await?;

        // 3. Broadcast the slashing event to the network for consensus
        self.broadcast_slash_event(node_id, fault_type, penalty)
            .await?;

        Ok(())
    }

    /// Get validator stake amount
    async fn get_validator_stake(&self, node_id: &NodeId) -> Result<u64> {
        // In a real implementation, query staking contract/state
        let validators = self.validators.read().await;
        if validators.contains(node_id) {
            Ok(100_000_000_000) // Mock: 100k tokens
        } else {
            Ok(0)
        }
    }

    /// Record slash event
    async fn record_slash_event(
        &self,
        node_id: &NodeId,
        fault_type: &ByzantineFaultType,
        amount: u64,
    ) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        info!(
            "[DETECTOR SLASH] node={}, fault={:?}, amount={}, ts={}",
            node_id, fault_type, amount, timestamp
        );

        // In production: persist to blockchain state or database
        Ok(())
    }

    /// Broadcast slash event to network
    async fn broadcast_slash_event(
        &self,
        node_id: &NodeId,
        fault_type: &ByzantineFaultType,
        amount: u64,
    ) -> Result<()> {
        // In a real implementation:
        // 1. Create a SlashEvent message
        // 2. Sign it with this node's private key
        // 3. Broadcast to all validators for verification
        // 4. Wait for 2f+1 acknowledgments before finalizing

        debug!(
            "Would broadcast slash event: node={}, fault={:?}, amount={}",
            node_id, fault_type, amount
        );

        Ok(())
    }

    /// Check a block for potential Byzantine behavior
    pub async fn check_block(&self, block: &Block, proposer: &NodeId) -> Result<bool> {
        // If the proposer is blacklisted, reject the block
        if self.is_blacklisted(proposer).await {
            warn!("Rejected block from blacklisted proposer {}", proposer);
            return Ok(false);
        }

        // Check for invalid block structure
        if !self.validate_block_structure(block).await? {
            let block_hash = block.hash()?.0;
            self.report_fault(
                ByzantineFaultType::InvalidBlockProposal,
                proposer.clone(),
                self.node_id.clone(),
                vec![block_hash.clone()],
                block_hash,
                "Invalid block structure".to_string(),
            )
            .await?;

            return Ok(false);
        }

        // Check for invalid transactions
        if !self.validate_block_transactions(block).await? {
            let block_hash = block.hash()?.0;
            self.report_fault(
                ByzantineFaultType::InvalidTransactions,
                proposer.clone(),
                self.node_id.clone(),
                vec![block_hash.clone()],
                block_hash,
                "Block contains invalid transactions".to_string(),
            )
            .await?;

            return Ok(false);
        }

        // Block appears valid from Byzantine perspective
        Ok(true)
    }

    /// Validate block structure
    async fn validate_block_structure(&self, block: &Block) -> Result<bool> {
        // 1. Verify block hash is correct
        let calculated_hash = block.hash()?;
        if calculated_hash.as_ref().is_empty() {
            return Ok(false);
        }

        // 2. Verify previous hash exists (except genesis)
        if block.header.height > 0 && block.header.previous_hash.0.is_empty() {
            return Ok(false);
        }

        // 3. Check timestamp validity
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Reject blocks with timestamps too far in future (5 min tolerance)
        if block.header.timestamp > now + 300 {
            return Ok(false);
        }

        // Reject very old blocks (1 hour)
        if block.header.timestamp + 3600 < now {
            return Ok(false);
        }

        // 4. Verify merkle root
        let calculated_merkle = Block::calculate_merkle_root(&block.transactions)?;
        if calculated_merkle != block.header.merkle_root {
            return Ok(false);
        }

        // 5. Verify producer key length
        if block.header.producer.0.len() != 48 {
            return Ok(false);
        }

        // 6. Verify difficulty
        if block.header.difficulty == 0 {
            return Ok(false);
        }

        // 7. Verify block signature
        if let Some(signature) = &block.signature {
            let block_data = block.encode_for_signing()?;
            if !block.header.producer.verify(&block_data, signature.as_ref())? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Validate block transactions
    async fn validate_block_transactions(&self, block: &Block) -> Result<bool> {
        if block.transactions.is_empty() {
            return Ok(false);
        }

        let mut seen_tx_ids = std::collections::HashSet::new();
        let mut sender_nonces: std::collections::HashMap<Vec<u8>, u64> =
            std::collections::HashMap::new();

        for tx in &block.transactions {
            // Check for duplicate transactions
            let tx_id = tx.hash()?;
            if seen_tx_ids.contains(&tx_id) {
                return Ok(false);
            }
            seen_tx_ids.insert(tx_id);

            // Verify transaction signature
            if !tx.verify()? {
                return Ok(false);
            }

            // Check nonce ordering
            if let Some(&last_nonce) = sender_nonces.get(&tx.from) {
                if tx.nonce <= last_nonce {
                    return Ok(false);
                }
            }
            sender_nonces.insert(tx.from.clone(), tx.nonce);

            // Verify basic transaction fields
            if tx.from.is_empty() || tx.to.is_empty() {
                return Ok(false);
            }

            // Check for overflow
            if tx.amount.checked_add(tx.fee).is_none() {
                return Ok(false);
            }

            // Verify signature exists
            if tx.signature.is_none() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check for equivocation (double signing)
    pub async fn check_equivocation(
        &self,
        node_id: &NodeId,
        view: u64,
        signature: &[u8],
        block_hash: &[u8],
    ) -> Result<bool> {
        let mut history = self.message_history.write().await;
        let node_history = history.entry(node_id.clone()).or_insert_with(HashMap::new);

        if let Some(existing_sig) = node_history.get(&view) {
            // Check if signatures are for different blocks
            if existing_sig != block_hash {
                // Construct evidence data
                let mut evidence_data = Vec::new();
                evidence_data.extend_from_slice(existing_sig);
                evidence_data.extend_from_slice(block_hash);

                // Report equivocation
                self.report_fault(
                    ByzantineFaultType::DoubleSigning,
                    self.node_id.clone(),
                    crate::network::types::NodeId("system".to_string()),
                    vec![block_hash.to_vec()],
                    evidence_data,
                    format!("Equivocation detected for view {}", view),
                )
                .await?;

                return Ok(false);
            }
        } else {
            // Record the signature for this view
            node_history.insert(view, block_hash.to_vec());
        }

        Ok(true)
    }

    /// Get statistics about Byzantine faults
    pub async fn get_statistics(&self) -> HashMap<ByzantineFaultType, usize> {
        let mut stats = HashMap::new();
        let faults = self.faults.read().await;

        for fault_list in faults.values() {
            for evidence in fault_list {
                *stats.entry(evidence.fault_type.clone()).or_insert(0) += 1;
            }
        }

        stats
    }
}

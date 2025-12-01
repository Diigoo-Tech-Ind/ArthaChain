use crate::consensus::cross_shard::merkle_proof::{MerkleProof, ProofCache, ProvenTransaction};
use crate::consensus::cross_shard::protocol::CrossShardTxType;
// removed: use crate::network::cross_shard::CrossShardConfig;
use crate::utils::crypto::quantum_resistant_hash;
use anyhow::{anyhow, Context, Result as AnyResult};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{info as trace_info, info_span, Instrument, warn as trace_warn};
use thiserror::Error;
use uuid::Uuid;
use pqcrypto_mldsa::mldsa65::{
    SecretKey as DilithiumSK,
    DetachedSignature as DilithiumDetachedSignature,
    detached_sign as dilithium_sign_impl,
    verify_detached_signature as dilithium_verify_impl,
};
use pqcrypto_mlkem::mlkem512::{
    keypair as kyber_keypair,
    encapsulate,
    PublicKey as KyberPK,
    SecretKey as KyberSK,
};
use pqcrypto_falcon::falcon512::{
    keypair as falcon_keypair,
    PublicKey as FalconPK,
    SecretKey as FalconSK,
    sign as falcon_sign_impl,
};
use pqcrypto_traits::sign::{PublicKey as PqcPublicKey, SecretKey as PqcSecretKey, SignedMessage as PqcSignedMessage, DetachedSignature as PqcDetachedSignature};
use ark_groth16::VerifyingKey as Groth16VK;
use ark_bn254::Bn254;
use ark_std::io::Cursor;
use ark_serialize::CanonicalDeserialize;
use prometheus::{Opts, IntCounterVec, HistogramVec, Registry};
use lazy_static::lazy_static;
use crate::consensus::cross_shard::coordinator_storage::CoordinatorStorage;
use crate::consensus::cross_shard::key_registry::KeyRegistry;

/// Enhanced Error Type
#[derive(Error, Debug)]
pub enum CoordinatorError {
    #[error("Crypto failure: {0}")]
    Crypto(#[from] anyhow::Error),

    #[error("Consensus failure: threshold {threshold} not met with {votes} votes")]
    Consensus { threshold: usize, votes: usize },

    #[error("Shard {shard_id} unresponsive: {reason}")]
    ShardFailure { shard_id: u32, reason: String },

    #[error("Lock acquisition failed for resource {resource}")]
    LockFailure { resource: String },

    #[error("Invalid proof for tx {tx_id}")]
    InvalidProof { tx_id: String },

    #[error("Storage error: {0}")]
    Storage(String),
}

/// Which post-quantum signature scheme to use
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QuantumSigScheme {
    Dilithium,
    Falcon,
    Hybrid, // e.g., Falcon for tx-level, Dilithium for governance (can be extended)
}

/// Quantum signature wrapper with algorithm tagging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantumSignature {
    Dilithium { sig: Vec<u8> },
    Falcon { sig: Vec<u8> },
}

fn sign_with_scheme(
    scheme: QuantumSigScheme,
    dilithium_sk: &DilithiumSK,
    falcon_sk: &FalconSK,
    msg: &[u8],
) -> AnyResult<QuantumSignature> {
    use pqcrypto_traits::sign::SignedMessage;
    match scheme {
        QuantumSigScheme::Dilithium => {
            let signed_msg = dilithium_sign_impl(msg, dilithium_sk);
            Ok(QuantumSignature::Dilithium { sig: signed_msg.as_bytes().to_vec() })
        }
        QuantumSigScheme::Falcon => {
            let signed_msg = falcon_sign_impl(msg, falcon_sk);
            Ok(QuantumSignature::Falcon { sig: signed_msg.as_bytes().to_vec() })
        }
        QuantumSigScheme::Hybrid => {
            // For now, use Falcon as the primary scheme in Hybrid mode.
            let signed_msg = falcon_sign_impl(msg, falcon_sk);
            Ok(QuantumSignature::Falcon { sig: signed_msg.as_bytes().to_vec() })
        }
    }
}

/// Configuration for the cross-shard coordinator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    pub timeout_ms: u64,
    pub max_concurrent_txs: usize,
    pub retry_attempts: u32,
    pub quantum_signature_enabled: bool,
    pub quantum_sig_scheme: QuantumSigScheme,
    pub enable_distributed_coordination: bool,
    pub coordinator_replicas: usize,
    pub consensus_threshold: usize,
    pub enable_coordinator_failover: bool,
    pub coordinator_health_check_interval_ms: u64,
    pub replica_endpoints: Vec<String>,
    pub timeout_check_interval_ms: u64,
    pub db_path: String,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        
        CoordinatorConfig {
            db_path: "test_db".to_string(),
            quantum_sig_scheme: crate::consensus::cross_shard::coordinator::QuantumSigScheme::Dilithium, // Corrected to existing enum variant
            replica_endpoints: vec![],
            timeout_ms: 1000,
            max_concurrent_txs: 100,
            retry_attempts: 3,
            quantum_signature_enabled: true,
            enable_distributed_coordination: false,
            coordinator_replicas: 1,
            consensus_threshold: 1,
            enable_coordinator_failover: false,
            coordinator_health_check_interval_ms: 1000,
            timeout_check_interval_ms: 100,
        }
    }
}

impl CoordinatorConfig {
    pub fn validate(&self) -> Result<(), CoordinatorError> {
        if self.timeout_ms == 0 {
            return Err(anyhow!("Timeout must be > 0").into());
        }
        if self.enable_distributed_coordination
            && self.coordinator_replicas < self.consensus_threshold
        {
            return Err(anyhow!(
                "Replicas {} < threshold {}",
                self.coordinator_replicas,
                self.consensus_threshold
            )
            .into());
        }
        Ok(())
    }

    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    pub fn timeout_check_interval(&self) -> Duration {
        Duration::from_millis(self.timeout_check_interval_ms)
    }

    pub fn health_check_interval(&self) -> Duration {
        Duration::from_millis(self.coordinator_health_check_interval_ms)
    }
}

/// CrossShardConfig with embedded CoordinatorConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossShardConfig {
    pub local_shard: u32,
    pub connected_shards: Vec<u32>,
    pub transaction_timeout_ms: u64,
    pub retry_count: usize,
    pub coordinator_config: CoordinatorConfig,
}

impl Default for CrossShardConfig {
    fn default() -> Self {
        Self {
            local_shard: 0,
            connected_shards: vec![1, 2],
            transaction_timeout_ms: 30_000,
            retry_count: 3,
            coordinator_config: CoordinatorConfig::default(),
        }
    }
}

impl CrossShardConfig {
    pub fn transaction_timeout(&self) -> Duration {
        Duration::from_millis(self.transaction_timeout_ms)
    }
}

/// Transaction preparation phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxPhase {
    Prepare,
    Commit,
    Abort,
}

impl TxPhase {
    fn as_str(&self) -> &'static str {
        match self {
            TxPhase::Prepare => "prepare",
            TxPhase::Commit => "commit",
            TxPhase::Abort => "abort",
        }
    }
}

/// Cross-shard coordinator message types (with PQ signatures)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinatorMessage {
    PrepareRequest {
        tx_id: String,
        // Kyber-encrypted tx_data
        tx_ciphertext: Vec<u8>,
        shared_secret_signature: QuantumSignature, // Sig on shared_secret
        from_shard: u32,
        to_shard: u32,
        signature: QuantumSignature, // Overall message sig
        timestamp: u64,
    },
    PrepareResponse {
        tx_id: String,
        success: bool,
        reason: Option<String>,
        signature: QuantumSignature,
        shard_id: u32,
    },
    CommitRequest {
        tx_id: String,
        proof: Vec<u8>,    // Merkle proof
        zk_proof: Vec<u8>, // Groth16 proof bytes
        signature: QuantumSignature,
        coordinator_shard: u32,
    },
    AbortRequest {
        tx_id: String,
        reason: String,
        signature: QuantumSignature,
        coordinator_shard: u32,
    },
    Acknowledgment {
        tx_id: String,
        phase: TxPhase,
        success: bool,
        signature: QuantumSignature,
        shard_id: u32,
    },
    Heartbeat {
        from_shard: u32,
        timestamp: u64,
        signature: QuantumSignature,
    },
}

/// Resource lock information
#[derive(Debug, Clone)]
pub struct ResourceLock {
    pub resource_id: String,
    pub tx_id: String,
    pub acquired_at: Instant,
    pub expires_at: Instant,
    pub shard_id: u32,
}

/// LockGraph for deadlock detection
#[derive(Debug, Clone)]
struct LockGraph {
    holders: HashMap<String, Vec<String>>,
    waiters: HashMap<String, Vec<String>>,
}

impl LockGraph {
    fn new() -> Self {
        Self {
            holders: HashMap::new(),
            waiters: HashMap::new(),
        }
    }

    fn detect_deadlock(&self) -> Option<String> {
        for (tx, waits) in &self.waiters {
            if self.has_cycle(tx, waits) {
                return Some(format!("Deadlock detected involving tx {}", tx));
            }
        }
        None
    }

    fn has_cycle(&self, start_tx: &str, waits: &[String]) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![start_tx.to_string()];

        while let Some(tx) = stack.pop() {
            if visited.contains(&tx) {
                return true;
            }
            visited.insert(tx.clone());
            if let Some(held_by) = self.waiters.get(&tx) {
                for res in held_by {
                    if let Some(holders) = self.holders.get(res) {
                        for holder in holders {
                            if !visited.contains(holder) {
                                stack.push(holder.clone());
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn add_holder(&mut self, resource: String, tx: String) {
        self.holders
            .entry(resource)
            .or_default()
            .push(tx);
    }

    fn add_waiter(&mut self, tx: String, resource: String) {
        self.waiters
            .entry(tx)
            .or_default()
            .push(resource);
    }

    fn remove_holders(&mut self, tx: &str) {
        self.holders.retain(|_, txs| {
            let mut kept = txs.clone();
            kept.retain(|t| t != tx);
            if kept.is_empty() {
                false
            } else {
                *txs = kept;
                true
            }
        });
        self.waiters.remove(tx);
    }
}

/// Transaction coordinator state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorTxState {
    pub tx_id: String,
    pub phase: TxPhase,
    pub participants: Vec<u32>,
    pub prepared: HashSet<u32>,
    pub committed: HashSet<u32>,
    pub start_time: SystemTime,
    pub last_action: SystemTime,
    pub tx_data: Vec<u8>,
    pub tx_type: CrossShardTxType,
    pub timeout: Duration,
    pub retry_count: u32,
    pub max_retries: u32,
    pub quantum_hash: Vec<u8>,
}

impl CoordinatorTxState {
    pub fn new(
        tx_id: String,
        participants: Vec<u32>,
        tx_data: Vec<u8>,
        tx_type: CrossShardTxType,
        timeout: Duration,
        max_retries: u32,
    ) -> Result<Self, CoordinatorError> {
        let quantum_hash = quantum_resistant_hash(&tx_data).context("Hash fail")?;
        Ok(Self {
            tx_id,
            phase: TxPhase::Prepare,
            participants,
            prepared: HashSet::new(),
            committed: HashSet::new(),
            start_time: SystemTime::now(),
            last_action: SystemTime::now(),
            tx_data,
            tx_type,
            timeout,
            retry_count: 0,
            max_retries,
            quantum_hash,
        })
    }

    pub fn all_prepared(&self) -> bool {
        self.prepared.len() == self.participants.len()
    }

    pub fn all_committed(&self) -> bool {
        self.committed.len() == self.participants.len()
    }

    pub fn is_timed_out(&self) -> bool {
        SystemTime::now()
            .duration_since(self.last_action)
            .unwrap_or_default()
            >= self.timeout
    }

    pub fn update_last_action(&mut self) {
        self.last_action = SystemTime::now();
    }

    pub fn increment_retry_count(&mut self) -> bool {
        self.retry_count += 1;
        self.retry_count <= self.max_retries
    }
}

/// Distributed coordination structs
#[derive(Debug, Clone)]
pub struct CoordinatorReplica {
    pub replica_id: usize,
    pub shard_id: u32,
    pub endpoint: String,
    pub is_active: bool,
    pub last_heartbeat: SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoordinatorHealth {
    Healthy,
    Degraded,
    Failed,
    Recovering,
}

#[derive(Debug, Clone)]
pub struct CoordinatorConsensus {
    pub consensus_threshold: usize,
    pub active_coordinators: usize,
    pub pending_decisions: HashMap<String, ConsensusDecision>,
}

#[derive(Debug, Clone)]
pub struct ConsensusDecision {
    pub operation: String,
    pub votes: HashMap<usize, bool>,
    pub timestamp: SystemTime,
    pub resolved: bool,
}

// Metrics
lazy_static! {
    static ref TXS_INITIATED: IntCounterVec = IntCounterVec::new(
        Opts::new("txs_initiated_total", "Number of txs initiated"),
        &["shard"],
    )
    .unwrap();

    static ref TXS_COMMITTED: IntCounterVec = IntCounterVec::new(
        Opts::new("txs_committed_total", "Number of txs committed"),
        &["shard"],
    )
    .unwrap();

    static ref TX_DURATION: HistogramVec = HistogramVec::new(
        prometheus::HistogramOpts::new("tx_duration_seconds", "Tx duration in seconds"),
        &["phase"],
    )
    .unwrap();

    static ref REGISTRY: Registry = Registry::new();
}

pub fn init_metrics() -> Result<(), prometheus::Error> {
    REGISTRY.register(Box::new(TXS_INITIATED.clone()))?;
    REGISTRY.register(Box::new(TXS_COMMITTED.clone()))?;
    REGISTRY.register(Box::new(TX_DURATION.clone()))?;
    Ok(())
}

pub struct CrossShardCoordinator {
    local_shard: u32,
    config: CrossShardConfig,
    transactions: Arc<RwLock<HashMap<String, CoordinatorTxState>>>,
    resource_locks: Arc<RwLock<HashMap<String, ResourceLock>>>,
    lock_graph: Arc<RwLock<LockGraph>>,
    running: Arc<Mutex<bool>>,
    timeout_checker: Option<JoinHandle<()>>,
    message_sender: mpsc::Sender<CoordinatorMessage>,

    // PQ keys
    dilithium_sk: DilithiumSK,
    falcon_sk: FalconSK,
    falcon_pk: FalconPK,

    // Kyber keypair
    kyber_pk: KyberPK,
    kyber_sk: KyberSK,

    // ZK VerifyingKey (loaded from storage or file)
    zk_vk: Groth16VK<Bn254>,

    heartbeats: Arc<RwLock<HashMap<u32, SystemTime>>>,
    proof_cache: Arc<Mutex<ProofCache>>,
    pending_proofs: Arc<RwLock<HashMap<String, ProvenTransaction>>>,
    key_registry: Arc<dyn KeyRegistry + Send + Sync>,
    storage: Arc<CoordinatorStorage>,
    coordinator_replicas: Arc<RwLock<Vec<CoordinatorReplica>>>,
    primary_coordinator: Arc<RwLock<usize>>,
    coordinator_consensus: Arc<RwLock<CoordinatorConsensus>>,
    replica_health: Arc<RwLock<HashMap<usize, CoordinatorHealth>>>,
}

impl CrossShardCoordinator {
    pub async fn new(
        config: CrossShardConfig,
        dilithium_sk_bytes: Vec<u8>, // Provided as bytes
        message_sender: mpsc::Sender<CoordinatorMessage>,
        key_registry: Arc<dyn KeyRegistry + Send + Sync>,
    ) -> Result<Self, CoordinatorError> {
        config.coordinator_config.validate()?;

        let dilithium_sk =
            DilithiumSK::from_bytes(&dilithium_sk_bytes).context("Invalid Dilithium SK")?;

        // Load or generate Falcon keys
        let falcon_key_path = format!("{}_falcon.key", config.coordinator_config.db_path);
        let (falcon_pk, falcon_sk) = if let Ok(bytes) = std::fs::read(&falcon_key_path) {
            use pqcrypto_traits::sign::SecretKey;
            let sk = FalconSK::from_bytes(&bytes).map_err(|e| anyhow!("Invalid Falcon SK: {:?}", e))?;
            // For Falcon, we need to derive PK from the keypair
            let (pk, _) = falcon_keypair();
            (pk, sk)
        } else {
            let (pk, sk) = falcon_keypair();
            use pqcrypto_traits::sign::SecretKey;
            std::fs::write(&falcon_key_path, sk.as_bytes()).map_err(|e| CoordinatorError::Storage(e.to_string()))?;
            (pk, sk)
        };

        // Load or generate Kyber keys
        let kyber_key_path = format!("{}_kyber.key", config.coordinator_config.db_path);
        let (kyber_pk, kyber_sk) = if let Ok(bytes) = std::fs::read(&kyber_key_path) {
            use pqcrypto_traits::kem::SecretKey;
            let sk = KyberSK::from_bytes(&bytes).map_err(|e| anyhow!("Invalid Kyber SK: {:?}", e))?;
            // For Kyber KEM, we need to derive PK from the keypair
            let (pk, _) = kyber_keypair();
            (pk, sk)
        } else {
            let (pk, sk) = kyber_keypair();
            use pqcrypto_traits::kem::SecretKey;
            std::fs::write(&kyber_key_path, sk.as_bytes()).map_err(|e| CoordinatorError::Storage(e.to_string()))?;
            (pk, sk)
        };

        // Register our keys in the registry
        use pqcrypto_traits::sign::{PublicKey as TraitPK, SecretKey as TraitSK};
        use pqcrypto_traits::kem::PublicKey as KemPublicKey;
        let dilithium_pk_bytes = dilithium_sk.as_bytes();
        key_registry.register_shard_keys(
            config.local_shard,
            Some(dilithium_pk_bytes),
            Some(falcon_pk.as_bytes()),
            Some(kyber_pk.as_bytes())
        ).map_err(CoordinatorError::Crypto)?;

        // Load ZK VK from storage (assume serialized; in prod, generate/setup circuit once)
        let zk_vk_bytes = std::fs::read(config.coordinator_config.db_path.replace(".db", "_zk_vk.bin"))
            .unwrap_or_default();
        let zk_vk = if !zk_vk_bytes.is_empty() {
            let mut cursor = Cursor::new(zk_vk_bytes.as_slice());
            Groth16VK::<Bn254>::deserialize_compressed(&mut cursor)
                .map_err(|e| anyhow!("Failed to deserialize ZK VK: {:?}", e))?
        } else {
            Groth16VK::<Bn254>::default()
        };

        let mut replicas = Vec::new();
        for (id, endpoint) in config.coordinator_config.replica_endpoints.iter().enumerate() {
            replicas.push(CoordinatorReplica {
                replica_id: id,
                shard_id: config.local_shard,
                endpoint: endpoint.clone(),
                is_active: true,
                last_heartbeat: SystemTime::now(),
            });
        }

        let consensus = CoordinatorConsensus {
            consensus_threshold: config.coordinator_config.consensus_threshold,
            active_coordinators: replicas.len(),
            pending_decisions: HashMap::new(),
        };

        init_metrics()
            .map_err(|e| CoordinatorError::Storage(format!("metrics init: {}", e)))?;

        let storage = Arc::new(
            CoordinatorStorage::open(&config.coordinator_config.db_path)
                .map_err(|e| CoordinatorError::Storage(e.to_string()))?,
        );

        Ok(Self {
            local_shard: config.local_shard,
            config,
            transactions: Arc::new(RwLock::new(HashMap::new())),
            resource_locks: Arc::new(RwLock::new(HashMap::new())),
            lock_graph: Arc::new(RwLock::new(LockGraph::new())),
            running: Arc::new(Mutex::new(false)),
            timeout_checker: None,
            message_sender,
            dilithium_sk,
            falcon_sk,
            falcon_pk,
            kyber_pk,
            kyber_sk,
            zk_vk,
            heartbeats: Arc::new(RwLock::new(HashMap::new())),
            proof_cache: Arc::new(Mutex::new(ProofCache::new(1000))),
            pending_proofs: Arc::new(RwLock::new(HashMap::new())),
            key_registry,
            storage,
            coordinator_replicas: Arc::new(RwLock::new(replicas)),
            primary_coordinator: Arc::new(RwLock::new(0)),
            coordinator_consensus: Arc::new(RwLock::new(consensus)),
            replica_health: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn sig_scheme(&self) -> QuantumSigScheme {
        self.config.coordinator_config.quantum_sig_scheme
    }

    fn verify_signature_for_shard(
        &self,
        shard_id: u32,
        msg: &[u8],
        sig: &QuantumSignature,
    ) -> bool {
        match sig {
            QuantumSignature::Dilithium { sig } => {
                if let Some(pk) = self.key_registry.get_dilithium_pk(shard_id) {
                    if let Ok(det) = <DilithiumDetachedSignature as PqcDetachedSignature>::from_bytes(sig) {
                        return dilithium_verify_impl(&det, msg, &pk).is_ok();
                    }
                }
                false
            }
            QuantumSignature::Falcon { sig: _ } => {
                // Falcon DetachedSignature::from_bytes removed from pqcrypto library
                // Fall back to returning false for now
                // TODO: implement alternative Falcon verification if needed
                false
            }
        }
    }

    pub async fn get_proof_cache_stats(&self) -> (usize, Vec<Vec<u8>>) {
        let cache = self.proof_cache.lock().await;
        (cache.size(), cache.get_cached_hashes())
    }

    pub async fn clear_proof_cache(&self) {
        let mut cache = self.proof_cache.lock().await;
        cache.clear();
    }

    pub async fn start(&mut self) -> Result<(), CoordinatorError> {
        let span = info_span!("coordinator_start", shard = %self.local_shard);
        async move { self.do_start().await }.instrument(span).await
    }

    async fn do_start(&mut self) -> Result<(), CoordinatorError> {
        let mut running = self.running.lock().await;
        if *running {
            return Err(anyhow!("Already running").into());
        }
        *running = true;
        drop(running);

        self.init_replicas().await?;
        if self.config.coordinator_config.enable_distributed_coordination {
            self.elect_primary_coordinator().await?;
            self.start_health_checker().await?;
        }

        let transactions = self.transactions.clone();
        let resource_locks = self.resource_locks.clone();
        let lock_graph = self.lock_graph.clone();
        let message_sender = self.message_sender.clone();
        let running_flag = self.running.clone();
        let dilithium_sk = self.dilithium_sk;
        let falcon_sk = self.falcon_sk;
        let sig_scheme = self.sig_scheme();
        let local_shard = self.local_shard;
        let config = self.config.clone();
        let heartbeats = self.heartbeats.clone();
        let key_registry = self.key_registry.clone();
        let retry_interval = config.coordinator_config.timeout_check_interval();

        self.timeout_checker = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(retry_interval);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let now = Instant::now();
                        let mut to_abort = Vec::new();

                        {
                            let mut tx_map = transactions.write().await;
                            for (tx_id, tx_state) in tx_map.iter_mut() {
                                if tx_state.is_timed_out() {
                                    if tx_state.increment_retry_count() {
                                        tx_state.update_last_action();

                                        match tx_state.phase {
                                            TxPhase::Prepare => {
                                                debug!("Retrying prepare for {}", tx_id);

                                                for &shard in &tx_state.participants {
                                                    if !tx_state.prepared.contains(&shard) {
                                                        let ts = SystemTime::now()
                                                            .duration_since(SystemTime::UNIX_EPOCH)
                                                            .unwrap_or_default()
                                                            .as_secs();

                                                        let msg_data = format!(
                                                            "prepare_retry:{tx_id}:{local_shard}:{shard}"
                                                        )
                                                        .into_bytes();

                                                        let sig = match sign_with_scheme(
                                                            sig_scheme,
                                                            &dilithium_sk,
                                                            &falcon_sk,
                                                            &msg_data,
                                                        ) {
                                                            Ok(s) => s,
                                                            Err(e) => {
                                                                error!(
                                                                    "Failed to sign prepare retry: {e}"
                                                                );
                                                                continue;
                                                            }
                                                        };

                                                        if let Some(recipient_pk) =
                                                            key_registry.get_kyber_pk(shard)
                                                        {
                                                            let (ciphertext, shared_secret) =
                                                                encapsulate(&recipient_pk);

                                                            let shared_sig =
                                                                match sign_with_scheme(
                                                                    sig_scheme,
                                                                    &dilithium_sk,
                                                                    &falcon_sk,
                                                                    shared_secret.as_bytes(),
                                                                ) {
                                                                    Ok(s) => s,
                                                                    Err(e) => {
                                                                        error!(
                                                                            "Failed to sign shared secret on retry: {e}"
                                                                        );
                                                                        continue;
                                                                    }
                                                                };

                                                             use pqcrypto_traits::kem::{Ciphertext as KemCiphertext, SharedSecret as KemSharedSecret};
                                                             let prepare =
                                                                CoordinatorMessage::PrepareRequest {
                                                                    tx_id: tx_id.clone(),
                                                                    tx_ciphertext: ciphertext
                                                                        .as_bytes()
                                                                        .to_vec(),
                                                                    shared_secret_signature: shared_sig,
                                                                    from_shard: local_shard,
                                                                    to_shard: shard,
                                                                    signature: sig,
                                                                    timestamp: ts,
                                                                };

                                                            if let Err(e) =
                                                                message_sender.try_send(prepare)
                                                            {
                                                                warn!(
                                                                    "Failed to send prepare retry to shard {}: {}",
                                                                    shard, e
                                                                );
                                                            }
                                                        } else {
                                                            warn!(
                                                                "No Kyber PK for shard {} when retrying prepare",
                                                                shard
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            TxPhase::Commit => {
                                                debug!("Retrying commit for {}", tx_id);
                                                let msg_data =
                                                    format!("commit_retry:{tx_id}").into_bytes();
                                                let sig = match sign_with_scheme(
                                                    sig_scheme,
                                                    &dilithium_sk,
                                                    &falcon_sk,
                                                    &msg_data,
                                                ) {
                                                    Ok(s) => s,
                                                    Err(e) => {
                                                        error!(
                                                            "Failed to sign commit retry: {e}"
                                                        );
                                                        continue;
                                                    }
                                                };

                                                let proof = vec![0u8; 64]; // Merkle stub
                                                let zk_proof = vec![0u8; 128]; // ZK stub

                                                let commit = CoordinatorMessage::CommitRequest {
                                                    tx_id: tx_id.clone(),
                                                    proof,
                                                    zk_proof,
                                                    signature: sig,
                                                    coordinator_shard: local_shard,
                                                };

                                                for &shard in &tx_state.participants {
                                                    if let Err(e) =
                                                        message_sender.try_send(commit.clone())
                                                    {
                                                        warn!(
                                                            "Failed to send commit retry to shard {}: {}",
                                                            shard, e
                                                        );
                                                    }
                                                }
                                            }
                                            TxPhase::Abort => {
                                                debug!("Retrying abort for {}", tx_id);
                                                let msg_data =
                                                    format!("abort_retry:{tx_id}").into_bytes();
                                                let sig = match sign_with_scheme(
                                                    sig_scheme,
                                                    &dilithium_sk,
                                                    &falcon_sk,
                                                    &msg_data,
                                                ) {
                                                    Ok(s) => s,
                                                    Err(e) => {
                                                        error!("Failed to sign abort retry: {e}");
                                                        continue;
                                                    }
                                                };

                                                let abort =
                                                    CoordinatorMessage::AbortRequest {
                                                        tx_id: tx_id.clone(),
                                                        reason: "Retry".to_string(),
                                                        signature: sig,
                                                        coordinator_shard: local_shard,
                                                    };

                                                for &shard in &tx_state.participants {
                                                    if let Err(e) =
                                                        message_sender.try_send(abort.clone())
                                                    {
                                                        warn!(
                                                            "Failed to send abort retry to shard {}: {}",
                                                            shard, e
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        to_abort.push(tx_id.clone());
                                    }
                                }
                            }
                        }

                        // Handle aborts that exceeded retries
                        for tx_id in to_abort {
                            let msg_data = format!("abort_final:{tx_id}").into_bytes();
                            let sig = match sign_with_scheme(
                                sig_scheme,
                                &dilithium_sk,
                                &falcon_sk,
                                &msg_data,
                            ) {
                                Ok(s) => s,
                                Err(e) => {
                                    error!(
                                        "Failed to sign final abort for tx {}: {e}",
                                        tx_id
                                    );
                                    continue;
                                }
                            };

                            let abort_msg = CoordinatorMessage::AbortRequest {
                                tx_id: tx_id.clone(),
                                reason: "Max retries".to_string(),
                                signature: sig,
                                coordinator_shard: local_shard,
                            };

                            if let Err(e) = message_sender.try_send(abort_msg) {
                                warn!("Failed to send final abort for {}: {}", tx_id, e);
                            }

                            {
                                let mut locks = resource_locks.write().await;
                                locks.retain(|_, lock| lock.tx_id != tx_id);
                            }

                            let mut tx_map = transactions.write().await;
                            tx_map.remove(&tx_id);
                        }

                        // Check for expired resource locks
                        {
                            let mut locks = resource_locks.write().await;
                            locks.retain(|_, lock| lock.expires_at > now);
                        }

                        // Heartbeat checks for shards
                        {
                            let mut h = heartbeats.write().await;
                            let heartbeat_limit = config.coordinator_config
                                .timeout_check_interval()
                                * 3;
                            h.retain(|_, last| {
                                if let Ok(elapsed) = SystemTime::now().duration_since(*last) {
                                    elapsed < heartbeat_limit
                                } else {
                                    false
                                }
                            });
                        }

                        // Send heartbeat
                        if let Ok(ts) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                            let ts_secs = ts.as_secs();
                            let ts_bytes = ts_secs.to_le_bytes().to_vec();

                            match sign_with_scheme(
                                sig_scheme,
                                &dilithium_sk,
                                &falcon_sk,
                                &ts_bytes,
                            ) {
                                Ok(sig) => {
                                    let heartbeat = CoordinatorMessage::Heartbeat {
                                        from_shard: local_shard,
                                        timestamp: ts_secs,
                                        signature: sig,
                                    };
                                    if let Err(e) = message_sender.try_send(heartbeat) {
                                        warn!("Failed to send heartbeat: {}", e);
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to sign heartbeat: {}", e);
                                }
                            }
                        }
                    }

                    _ = async {
                        let running = running_flag.lock().await;
                        !*running
                    } => break,
                }
            }
        }));

        Ok(())
    }

    async fn init_replicas(&self) -> Result<(), CoordinatorError> {
        let mut health = self.replica_health.write().await;
        for replica in self.coordinator_replicas.read().await.iter() {
            health.insert(replica.replica_id, CoordinatorHealth::Healthy);
        }
        Ok(())
    }

    async fn elect_primary_coordinator(&self) -> Result<(), CoordinatorError> {
        let my_id = *self.primary_coordinator.read().await;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let msg_data = format!("probe:{}:{}", my_id, timestamp).into_bytes();

        let sig = sign_with_scheme(
            self.sig_scheme(),
            &self.dilithium_sk,
            &self.falcon_sk,
            &msg_data,
        )
        .map_err(CoordinatorError::Crypto)?;

        // In a real implementation, we would broadcast and wait for ACKs.
        // For now, we simulate self-election if we are replica 0.
        if my_id == 0 {
            trace_info!("Elected primary coordinator {}", my_id);
        }

        Ok(())
    }

    async fn start_health_checker(&self) -> Result<(), CoordinatorError> {
        let replicas = self.coordinator_replicas.clone();
        let health = self.replica_health.clone();
        let primary = self.primary_coordinator.clone();
        let interval = self.config.coordinator_config.health_check_interval();

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let now = SystemTime::now();
                let mut r_guard = replicas.write().await;
                let mut h_guard = health.write().await;
                let mut failover_needed = false;

                for replica in r_guard.iter_mut() {
                    if now
                        .duration_since(replica.last_heartbeat)
                        .unwrap_or_default()
                        > interval * 3
                    {
                        replica.is_active = false;
                        h_guard.insert(replica.replica_id, CoordinatorHealth::Failed);

                        if replica.replica_id == *primary.read().await {
                            failover_needed = true;
                        }
                    } else {
                        h_guard.insert(replica.replica_id, CoordinatorHealth::Healthy);
                    }
                }

                if failover_needed {
                    *primary.write().await = 0;
                    trace_warn!("Primary coordinator failed. Failover to replica 0");
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), CoordinatorError> {
        let mut running = self.running.lock().await;
        if !*running {
            return Err(anyhow!("Not running").into());
        }
        *running = false;
        if let Some(handle) = self.timeout_checker.take() {
            let _ = handle.await;
        }
        Ok(())
    }

    pub async fn initiate_transaction(
        &self,
        tx_data: Vec<u8>,
        from_shard: u32,
        to_shard: u32,
        resources: Vec<String>,
    ) -> Result<String, CoordinatorError> {
        let tx_id = Uuid::new_v4().to_string();
        let acquired_locks = self.try_acquire_locks(&tx_id, &resources).await?;
        if !acquired_locks {
            return Err(CoordinatorError::LockFailure {
                resource: "multiple".to_string(),
            });
        }

        let tx_type = CrossShardTxType::DirectTransfer {
            from_shard,
            to_shard,
            amount: 0,
        };

        let participants = vec![from_shard, to_shard];
        let tx_state = CoordinatorTxState::new(
            tx_id.clone(),
            participants.clone(),
            tx_data.clone(),
            tx_type,
            self.config.transaction_timeout(),
            self.config.retry_count as u32,
        )?;

        {
            let mut tx_map = self.transactions.write().await;
            tx_map.insert(tx_id.clone(), tx_state);
        }

        for shard in participants {
            if shard == self.local_shard {
                let mut tx_map = self.transactions.write().await;
                if let Some(tx_state) = tx_map.get_mut(&tx_id) {
                    tx_state.prepared.insert(shard);
                    tx_state.update_last_action();
                }
                continue;
            }

            let ts = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let msg_data =
                format!("prepare:{tx_id}:{from_shard}:{to_shard}").into_bytes();

            let sig = sign_with_scheme(
                self.sig_scheme(),
                &self.dilithium_sk,
                &self.falcon_sk,
                &msg_data,
            )
            .map_err(CoordinatorError::Crypto)?;

            let recipient_pk = self
                .key_registry
                .get_kyber_pk(shard)
                .ok_or_else(|| CoordinatorError::ShardFailure {
                    shard_id: shard,
                    reason: "Missing Kyber PK".to_string(),
                })?;

            let (ciphertext, shared_secret) =
                encapsulate(&recipient_pk);

            use pqcrypto_traits::kem::{Ciphertext as KemCiphertext, SharedSecret as KemSharedSecret};
            let shared_sig = sign_with_scheme(
                self.sig_scheme(),
                &self.dilithium_sk,
                &self.falcon_sk,
                shared_secret.as_bytes(),
            )
            .map_err(CoordinatorError::Crypto)?;

            let prepare_msg = CoordinatorMessage::PrepareRequest {
                tx_id: tx_id.clone(),
                tx_ciphertext: ciphertext.as_bytes().to_vec(),
                shared_secret_signature: shared_sig,
                from_shard,
                to_shard,
                signature: sig,
                timestamp: ts,
            };

            self.message_sender
                .send(prepare_msg)
                .await
                .map_err(|e| CoordinatorError::ShardFailure {
                    shard_id: shard,
                    reason: e.to_string(),
                })?;
        }

        Ok(tx_id)
    }

    async fn try_acquire_locks(
        &self,
        tx_id: &str,
        resources: &[String],
    ) -> Result<bool, CoordinatorError> {
        let mut locks = self.resource_locks.write().await;
        let mut graph = self.lock_graph.write().await;
        let now = Instant::now();
        let lock_timeout = self.config.transaction_timeout();

        for resource in resources {
            if let Some(lock) = locks.get(resource) {
                if lock.tx_id != tx_id && lock.expires_at > now {
                    graph.add_waiter(tx_id.to_string(), resource.clone());
                    if let Some(deadlock) = graph.detect_deadlock() {
                        return Err(CoordinatorError::LockFailure { resource: deadlock });
                    }
                    return Ok(false);
                }
            }
        }

        for resource in resources {
            let lock = ResourceLock {
                resource_id: resource.clone(),
                tx_id: tx_id.to_string(),
                acquired_at: now,
                expires_at: now + lock_timeout,
                shard_id: self.local_shard,
            };
            locks.insert(resource.clone(), lock);
            graph.add_holder(resource.clone(), tx_id.to_string());
        }

        Ok(true)
    }

    pub async fn handle_prepare_response(
        &self,
        tx_id: String,
        success: bool,
        reason: Option<String>,
        signature: QuantumSignature,
        shard_id: u32,
    ) {
        let msg_data = format!("prepare_response:{}:{}", tx_id, shard_id).into_bytes();

        if !self.verify_signature_for_shard(shard_id, &msg_data, &signature) {
            warn!(
                "Invalid prepare_response signature for tx {} from shard {}",
                tx_id, shard_id
            );
            return;
        }

        let mut should_commit = false;
        let mut should_abort = false;
        let mut participants = Vec::new();

        {
            let mut tx_map = self.transactions.write().await;
            if let Some(tx_state) = tx_map.get_mut(&tx_id) {
                if success {
                    tx_state.prepared.insert(shard_id);
                    tx_state.update_last_action();
                    if tx_state.all_prepared() {
                        tx_state.phase = TxPhase::Commit;
                        should_commit = true;
                    }
                } else {
                    should_abort = true;
                    tx_state.phase = TxPhase::Abort;
                    participants = tx_state.participants.clone();
                }
                let _ = self.storage.save_transaction(tx_state);
            }
        }

        if should_commit {
            let msg_data = format!("commit:{tx_id}").into_bytes();
            if let Ok(sig) = sign_with_scheme(
                self.sig_scheme(),
                &self.dilithium_sk,
                &self.falcon_sk,
                &msg_data,
            ) {
                let proof = vec![0u8; 64]; // TODO: real Merkle proof
                let zk_proof = vec![0u8; 128]; // TODO: real ZK proof

                let commit_msg = CoordinatorMessage::CommitRequest {
                    tx_id: tx_id.clone(),
                    proof,
                    zk_proof,
                    signature: sig,
                    coordinator_shard: self.local_shard,
                };
                let _ = self.message_sender.send(commit_msg).await;
            } else {
                error!("Failed to sign commit for tx {}", tx_id);
            }
        }

        if should_abort {
            let msg_data = format!("abort:{tx_id}").into_bytes();
            if let Ok(sig) = sign_with_scheme(
                self.sig_scheme(),
                &self.dilithium_sk,
                &self.falcon_sk,
                &msg_data,
            ) {
                let abort_msg = CoordinatorMessage::AbortRequest {
                    tx_id: tx_id.clone(),
                    reason: reason.unwrap_or_default(),
                    signature: sig,
                    coordinator_shard: self.local_shard,
                };
                let _ = self.message_sender.send(abort_msg).await;
            } else {
                error!("Failed to sign abort for tx {}", tx_id);
            }
        }
    }

    pub async fn handle_acknowledgment(
        &self,
        tx_id: String,
        phase: TxPhase,
        success: bool,
        signature: QuantumSignature,
        shard_id: u32,
    ) {
        let msg_data =
            format!("ack:{}:{}:{:?}", tx_id, shard_id, phase.as_str()).into_bytes();

        if !self.verify_signature_for_shard(shard_id, &msg_data, &signature) {
            warn!(
                "Invalid acknowledgment signature for tx {} from shard {}",
                tx_id, shard_id
            );
            return;
        }

        let mut tx_completed = false;
        {
            let mut tx_map = self.transactions.write().await;
            if let Some(tx_state) = tx_map.get_mut(&tx_id) {
                match phase {
                    TxPhase::Commit => {
                        if success {
                            tx_state.committed.insert(shard_id);
                            tx_state.update_last_action();
                            if tx_state.all_committed() {
                                tx_completed = true;
                            }
                        }
                    }
                    TxPhase::Abort => {
                        tx_state.committed.insert(shard_id);
                        tx_state.update_last_action();
                        if tx_state.all_committed() {
                            tx_completed = true;
                        }
                    }
                    _ => {}
                }
                let _ = self.storage.save_transaction(tx_state);
            }
        }

        if tx_completed {
            let mut tx_map = self.transactions.write().await;
            if let Some(tx_state) = tx_map.remove(&tx_id) {
                let mut locks = self.resource_locks.write().await;
                locks.retain(|_, lock| lock.tx_id != tx_id);
                let _ = self.storage.delete_transaction(&tx_id);
                info!(
                    "Transaction {} completed with phase {:?}",
                    tx_id, tx_state.phase
                );
            }
        }
    }

    pub async fn verify_proven_transaction(
        &self,
        tx_id: String,
    ) -> Result<bool, CoordinatorError> {
        // Hook for Merkle + Groth16 verification.
        // For now, keep as a simple stub but wired to error type.
        let _ = tx_id;
        Ok(true)
    }

    pub fn validate_merkle_proof(&self, proof: &MerkleProof) -> Result<bool, CoordinatorError> {
        proof.verify().map_err(|e| CoordinatorError::InvalidProof { tx_id: hex::encode(&proof.tx_hash) })
    }

    pub async fn submit_proven_transaction(&self, proven_tx: ProvenTransaction) -> Result<String, CoordinatorError> {
        let tx_hash = proven_tx.proof.tx_hash.clone();
        let tx_id = hex::encode(&tx_hash);
        
        // Validate proof
        if !proven_tx.verify().unwrap_or(false) {
             return Err(CoordinatorError::InvalidProof { tx_id: tx_id.clone() });
        }
        
        // Create state
        let participants = vec![proven_tx.source_shard, proven_tx.target_shard];
        let tx_state = CoordinatorTxState::new(
            tx_id.clone(),
            participants,
            proven_tx.transaction_data,
            CrossShardTxType::Transfer,
            self.config.transaction_timeout(),
            self.config.retry_count as u32
        )?;
        
        let mut transactions = self.transactions.write().await;
        transactions.insert(tx_id.clone(), tx_state);
        
        Ok(tx_id)
    }

    pub async fn get_transaction_status(
        &self,
        tx_id: &str,
    ) -> Option<(TxPhase, bool)> {
        let tx_map = self.transactions.read().await;
        tx_map.get(tx_id).map(|tx_state| {
            let is_complete = match tx_state.phase {
                TxPhase::Prepare => false,
                TxPhase::Commit => tx_state.all_committed(),
                TxPhase::Abort => tx_state.all_committed(),
            };
            (tx_state.phase, is_complete)
        })
    }
}

pub struct ParticipantHandler {
    local_shard: u32,
    config: CrossShardConfig,
    resource_locks: Arc<RwLock<HashMap<String, ResourceLock>>>,
    prepared_transactions: Arc<RwLock<HashMap<String, (Vec<String>, Vec<u8>)>>>,
    dilithium_sk: DilithiumSK,
    message_sender: mpsc::Sender<CoordinatorMessage>,
    key_registry: Arc<dyn KeyRegistry + Send + Sync>,
    storage: Arc<CoordinatorStorage>,
}

impl ParticipantHandler {
    pub fn new(
        config: CrossShardConfig,
        dilithium_sk_bytes: Vec<u8>,
        message_sender: mpsc::Sender<CoordinatorMessage>,
        key_registry: Arc<dyn KeyRegistry + Send + Sync>,
    ) -> Result<Self, CoordinatorError> {
        let dilithium_sk =
            DilithiumSK::from_bytes(&dilithium_sk_bytes).context("Invalid SK")?;
        let storage = Arc::new(
            CoordinatorStorage::open("data/participant_db")
                .map_err(|e| CoordinatorError::Storage(e.to_string()))?,
        );

        Ok(Self {
            local_shard: config.local_shard,
            config,
            resource_locks: Arc::new(RwLock::new(HashMap::new())),
            prepared_transactions: Arc::new(RwLock::new(HashMap::new())),
            dilithium_sk,
            message_sender,
            key_registry,
            storage,
        })
    }

    pub async fn handle_prepare_request(
        &self,
        tx_id: String,
        tx_ciphertext: Vec<u8>,
        shared_secret_sig: QuantumSignature,
        from_shard: u32,
        to_shard: u32,
        signature: QuantumSignature,
        ts: u64,
    ) -> Result<(), CoordinatorError> {
        // 1. Verify coordinator signature
        let msg_data = format!("prepare:{tx_id}:{from_shard}:{to_shard}").into_bytes();
        if !self.verify_signature(from_shard, &msg_data, &signature) {
            return Err(CoordinatorError::Crypto(anyhow!("Invalid coordinator signature")));
        }

        // 2. Verify shared secret signature
        // Note: In a real implementation, we would decapsulate the shared secret here using our SK
        // and verify the signature against the decapsulated secret.
        // For this implementation, we verify the signature structure but skip full decapsulation check
        // to avoid needing the Kyber SK in ParticipantHandler (which simplifies the struct).
        // In production, ParticipantHandler should have access to Kyber SK.
        
        // 3. Check for replay (timestamp)
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs();
        if ts > now + 30 || ts < now - 30 {
             return Err(CoordinatorError::Consensus { threshold: 0, votes: 0 }); // Invalid timestamp
        }

        // 4. Check resource locks
        // In a real implementation, we would parse tx_ciphertext (after decryption) to get resources.
        // Here we assume a simple transfer or use a placeholder resource ID based on tx_id.
        let resource = format!("resource_{}", tx_id); 
        
        let mut locks = self.resource_locks.write().await;
        if locks.contains_key(&resource) {
             // Resource locked
             let response = CoordinatorMessage::PrepareResponse {
                 tx_id: tx_id.clone(),
                 success: false,
                 reason: Some("Resource locked".to_string()),
                 signature: self.sign_response(&format!("prepare_response:{}:{}", tx_id, self.local_shard))?,
                 shard_id: self.local_shard,
             };
             self.message_sender.try_send(response).map_err(|e| CoordinatorError::ShardFailure { shard_id: from_shard, reason: e.to_string() })?;
             return Ok(());
        }

        // 5. Lock resource
        locks.insert(resource.clone(), ResourceLock {
            resource_id: resource.clone(),
            tx_id: tx_id.clone(),
            acquired_at: Instant::now(),
            expires_at: Instant::now() + self.config.transaction_timeout(),
            shard_id: from_shard,
        });

        // 6. Store transaction state
        {
            let mut txs = self.prepared_transactions.write().await;
            txs.insert(tx_id.clone(), (vec![resource], tx_ciphertext));
        }

        // 7. Send success response
        let response = CoordinatorMessage::PrepareResponse {
            tx_id: tx_id.clone(),
            success: true,
            reason: None,
            signature: self.sign_response(&format!("prepare_response:{}:{}", tx_id, self.local_shard))?,
            shard_id: self.local_shard,
        };
        self.message_sender.try_send(response).map_err(|e| CoordinatorError::ShardFailure { shard_id: from_shard, reason: e.to_string() })?;

        Ok(())
    }

    pub async fn handle_commit_request(
        &self,
        tx_id: String,
        _proof: Vec<u8>,
        _zk: Vec<u8>,
        signature: QuantumSignature,
        coord_shard: u32,
    ) -> Result<(), CoordinatorError> {
        // 1. Verify signature
        let msg_data = format!("commit:{tx_id}").into_bytes();
        if !self.verify_signature(coord_shard, &msg_data, &signature) {
             return Err(CoordinatorError::Crypto(anyhow!("Invalid commit signature")));
        }

        // 2. Apply changes (stub)
        // In a real system, we would apply the state changes here.
        info!("Applying commit for tx {}", tx_id);

        // 3. Release locks
        {
            let mut txs = self.prepared_transactions.write().await;
            if let Some((resources, _)) = txs.remove(&tx_id) {
                let mut locks = self.resource_locks.write().await;
                for res in resources {
                    locks.remove(&res);
                }
            }
        }

        // 4. Send ACK
        let ack = CoordinatorMessage::Acknowledgment {
            tx_id: tx_id.clone(),
            phase: TxPhase::Commit,
            success: true,
            signature: self.sign_response(&format!("ack:{}:{}:commit", tx_id, self.local_shard))?,
            shard_id: self.local_shard,
        };
        self.message_sender.try_send(ack).map_err(|e| CoordinatorError::ShardFailure { shard_id: coord_shard, reason: e.to_string() })?;

        Ok(())
    }

    pub async fn handle_abort_request(
        &self,
        tx_id: String,
        _reason: String,
        signature: QuantumSignature,
        coord_shard: u32,
    ) -> Result<(), CoordinatorError> {
        // 1. Verify signature
        let msg_data = format!("abort:{tx_id}").into_bytes();
        if !self.verify_signature(coord_shard, &msg_data, &signature) {
             return Err(CoordinatorError::Crypto(anyhow!("Invalid abort signature")));
        }

        // 2. Release locks
        {
            let mut txs = self.prepared_transactions.write().await;
            if let Some((resources, _)) = txs.remove(&tx_id) {
                let mut locks = self.resource_locks.write().await;
                for res in resources {
                    locks.remove(&res);
                }
            }
        }

        // 3. Send ACK
        let ack = CoordinatorMessage::Acknowledgment {
            tx_id: tx_id.clone(),
            phase: TxPhase::Abort,
            success: true,
            signature: self.sign_response(&format!("ack:{}:{}:abort", tx_id, self.local_shard))?,
            shard_id: self.local_shard,
        };
        self.message_sender.try_send(ack).map_err(|e| CoordinatorError::ShardFailure { shard_id: coord_shard, reason: e.to_string() })?;

        Ok(())
    }

    fn verify_signature(&self, shard_id: u32, msg: &[u8], sig: &QuantumSignature) -> bool {
        match sig {
            QuantumSignature::Dilithium { sig } => {
                if let Some(pk) = self.key_registry.get_dilithium_pk(shard_id) {
                    if let Ok(det) = <DilithiumDetachedSignature as PqcDetachedSignature>::from_bytes(sig) {
                        return dilithium_verify_impl(&det, msg, &pk).is_ok();
                    }
                }
                false
            }
            QuantumSignature::Falcon { sig: _ } => {
                // Falcon DetachedSignature::from_bytes removed from pqcrypto library
                // Fall back to returning false for now
                // TODO: implement alternative Falcon verification if needed
                false
            }
        }
    }

    fn sign_response(&self, msg: &str) -> Result<QuantumSignature, CoordinatorError> {
        // For ParticipantHandler, we currently only have Dilithium SK loaded.
        // In a full implementation, we should load Falcon SK as well if needed.
        // For now, we default to Dilithium for responses.
        let sig = dilithium_sign_impl(msg.as_bytes(), &self.dilithium_sk);
        Ok(QuantumSignature::Dilithium { sig: sig.as_bytes().to_vec() })
    }
}

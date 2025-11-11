use clap::{Parser, Subcommand};
use reqwest::blocking::{Client, multipart};
use std::fs;
use std::io::Write;

#[derive(Parser)]
#[command(name = "arthai", version, about = "ArthaChain CLI for SVDB public storage")] 
struct Cli {
    /// Base URL of the node HTTP API
    #[arg(long, env = "ARTHA_NODE", default_value = "http://127.0.0.1:8080")] 
    node: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload a file/directory archive to SVDB
    StoragePush {
        /// Path to a file to upload (raw bytes)
        input: String,
        /// Replicas (informational for deal creation flow)
        #[arg(long, default_value_t = 1u32)]
        replicas: u32,
        /// Months (informational for deal creation flow)
        #[arg(long, default_value_t = 1u32)]
        months: u32,
        /// Optional envelope JSON file to pass as X-Artha-Envelope
        #[arg(long)]
        envelope: Option<String>,
    },
    /// Set access policy for a CID (private/public/allowlist/token)
    AccessPolicy {
        /// CID URI artha://<base64>
        cid: String,
        /// Make private? (true/false)
        #[arg(long)] private: bool,
        /// Comma-separated DIDs for allowlist (optional)
        #[arg(long, default_value = "")] allow: String,
        /// Token for token-gated (optional)
        #[arg(long, default_value = "")] token: String,
    },
    /// Add DID to allowlist for a CID
    AccessAllowAdd { cid: String, #[arg(long)] did: String },
    /// Remove DID from allowlist for a CID
    AccessAllowRemove { cid: String, #[arg(long)] did: String },
    /// Download by CID to an output file
    StorageGet {
        /// CID URI artha://<base64>
        cid: String,
        /// Output file path
        #[arg(short, long)]
        output: String,
    },
    /// Show manifest info for a CID
    StorageInfo {
        /// CID URI artha://<base64>
        cid: String,
    },
    /// Create a storage deal for a manifest CID
    StoragePin {
        /// CID URI artha://<base64>
        cid: String,
        /// Total size in bytes
        #[arg(long)]
        size: u64,
        /// Replicas
        #[arg(long, default_value_t = 1u32)]
        replicas: u32,
        /// Months
        #[arg(long, default_value_t = 1u32)]
        months: u32,
        /// Max price per GB-month (in ARTH wei)
        #[arg(long)]
        max_price: f64,
    },
    /// Quote retrieval price and nonce
    Quote {
        /// Provider address (hex 0x...)
        #[arg(long)] provider: String,
        /// CID URI artha://<base>
        cid: String,
    },
    /// Settle retrieval micro-fees
    Settle {
        #[arg(long)] rpc_url: String,
        #[arg(long)] chain_id: u64,
        #[arg(long)] private_key: String,
        #[arg(long)] deal_market: String,
        #[arg(long)] manifest_root: String,
        #[arg(long)] bytes_served: u64,
        #[arg(long)] provider: String,
        #[arg(long)] total_wei: u64,
        #[arg(long, default_value_t = 1_000_000_000u64)] gas_price: u64,
        #[arg(long, default_value_t = 200_000u64)] gas_limit: u64,
    },
    /// Pin and unpin helpers
    Pin { cid: String },
    Unpin { cid: String },
    
    // ============================================================================
    // Identity & AI Commands
    // ============================================================================
    
    /// Create a new DID
    IdentityDidCreate {
        /// Ed25519 auth key (hex)
        #[arg(long)]
        auth_key: String,
        /// X25519 encryption key (hex)
        #[arg(long)]
        enc_key: String,
        /// Metadata CID (artha://...)
        #[arg(long)]
        meta_cid: String,
    },
    
    /// Get DID document
    IdentityDidGet {
        /// DID (did:artha:...)
        did: String,
    },
    
    /// Rotate DID keys
    IdentityDidRotate {
        /// DID (did:artha:...)
        did: String,
        /// New auth key (hex)
        #[arg(long)]
        new_auth_key: String,
        /// New enc key (hex)
        #[arg(long)]
        new_enc_key: String,
    },
    
    /// Revoke a DID
    IdentityDidRevoke {
        /// DID (did:artha:...)
        did: String,
    },
    
    /// Issue a Verifiable Credential
    IdentityVcIssue {
        /// Issuer DID
        #[arg(long)]
        issuer: String,
        /// Subject DID
        #[arg(long)]
        subject: String,
        /// Claim hash (keccak256 of claim)
        #[arg(long)]
        claim_hash: String,
        /// Doc CID (artha://...)
        #[arg(long)]
        doc_cid: String,
        /// Expiration timestamp (unix epoch, 0 = never)
        #[arg(long, default_value_t = 0u64)]
        expires_at: u64,
    },
    
    /// Revoke a VC
    IdentityVcRevoke {
        /// VC hash
        vc_hash: String,
    },
    
    /// Verify a VC
    IdentityVcVerify {
        /// VC hash
        vc_hash: String,
    },
    
    /// Get VCs for a subject
    IdentityVcList {
        /// Subject DID
        subject: String,
    },
    
    /// Create an AI Identity
    IdentityAiidCreate {
        /// Owner DID
        #[arg(long)]
        owner: String,
        /// Model CID
        #[arg(long)]
        model_cid: String,
        /// Dataset ID
        #[arg(long)]
        dataset_id: String,
        /// Code hash
        #[arg(long)]
        code_hash: String,
        /// Version string
        #[arg(long)]
        version: String,
    },
    
    /// Get AIID document
    IdentityAiidGet {
        /// AIID (aiid:artha:...)
        aiid: String,
    },
    
    /// Rotate AIID (new version)
    IdentityAiidRotate {
        /// AIID (aiid:artha:...)
        aiid: String,
        /// New model CID
        #[arg(long)]
        new_model_cid: String,
        /// New version
        #[arg(long)]
        new_version: String,
    },
    
    /// Register a Node Certificate
    NodecertRegister {
        /// Node public key (hex)
        #[arg(long)]
        node_pubkey: String,
        /// Owner DID
        #[arg(long)]
        owner_did: String,
        /// Role (validator/sp/retriever/gpu)
        #[arg(long)]
        role: String,
        /// Region (ISO-3166-1 alpha-2)
        #[arg(long)]
        region: String,
        /// Capabilities (comma-separated, e.g. "gpu:a100,storage:1tb")
        #[arg(long)]
        capabilities: String,
    },
    
    /// Send node heartbeat
    NodecertHeartbeat {
        /// Node public key (hex)
        node_pubkey: String,
    },
    
    /// Submit a job
    JobSubmit {
        /// AIID
        #[arg(long)]
        aiid: String,
        /// Dataset ID
        #[arg(long)]
        dataset_id: String,
        /// Parameters hash
        #[arg(long)]
        params_hash: String,
        /// Submitter DID
        #[arg(long)]
        submitter_did: String,
    },
    
    /// Get job status
    JobStatus {
        /// Job ID
        job_id: String,
    },
    
    // ============================================================================
    // AI Operations (ArthaAIN v1)
    // ============================================================================
    
    /// Register a dataset
    DatasetRegister {
        /// Root CID (artha://...)
        root_cid: String,
        /// License CID (artha://...)
        #[arg(long)]
        license: String,
        /// Tags (comma-separated, e.g. "nlp,english,gpt")
        #[arg(long, default_value = "")]
        tags: String,
    },
    
    /// List datasets
    DatasetList {
        /// Filter by owner DID
        #[arg(long)]
        owner: Option<String>,
    },
    
    /// Get dataset info
    DatasetInfo {
        /// Dataset ID
        dataset_id: String,
    },
    
    /// Register a model
    ModelRegister {
        /// Model CID (artha://...)
        model_cid: String,
        /// Architecture (e.g. "llama", "gpt", "vit")
        #[arg(long)]
        arch: String,
        /// Base model ID (if fine-tuned)
        #[arg(long, default_value = "")]
        base_model: String,
        /// Dataset ID used for training
        #[arg(long)]
        dataset: String,
        /// Code hash (hash of training script)
        #[arg(long)]
        code_hash: String,
        /// Version (e.g. "v1.0")
        #[arg(long)]
        version: String,
        /// License CID (artha://...)
        #[arg(long, default_value = "")]
        license: String,
    },
    
    /// List models
    ModelList {
        /// Filter by owner DID
        #[arg(long)]
        owner: Option<String>,
    },
    
    /// Get model lineage (parent chain)
    ModelLineage {
        /// Model ID
        model_id: String,
    },
    
    /// Publish model checkpoint
    ModelPublish {
        /// Model ID
        model_id: String,
        /// Checkpoint CID (artha://...)
        #[arg(long)]
        checkpoint: String,
    },
    
    /// Submit training job
    Train {
        /// Model ID to train
        #[arg(long)]
        model: String,
        /// Dataset ID to train on
        #[arg(long)]
        data: String,
        /// Number of epochs
        #[arg(long, default_value_t = 1)]
        epochs: u32,
        /// Batch size
        #[arg(long, default_value_t = 32)]
        batch: u32,
        /// Learning rate
        #[arg(long, default_value_t = 0.001)]
        lr: f64,
        /// Optimizer (adam/sgd/adamw)
        #[arg(long, default_value = "adam")]
        optimizer: String,
        /// Budget in ARTH wei
        #[arg(long, default_value_t = 1000)]
        budget: u64,
        /// Output directory for checkpoints
        #[arg(long, default_value = "./checkpoints")]
        output: String,
    },
    
    /// Submit inference job
    Infer {
        /// Model ID to use
        #[arg(long)]
        model: String,
        /// Input file path
        #[arg(long)]
        input: String,
        /// Mode (realtime/batch/stream)
        #[arg(long, default_value = "realtime")]
        mode: String,
        /// Max tokens for generation
        #[arg(long, default_value_t = 1024)]
        max_tokens: u32,
        /// Budget in ARTH wei
        #[arg(long, default_value_t = 100)]
        budget: u64,
        /// Output file
        #[arg(short, long, default_value = "output.json")]
        out: String,
    },
    
    /// Run autonomous agent
    AgentRun {
        /// AIID of the agent
        #[arg(long)]
        aiid: String,
        /// Goal/instruction for the agent
        #[arg(long)]
        goal: String,
        /// Tools (comma-separated: svdb,web,code,tx)
        #[arg(long, default_value = "svdb")]
        tools: String,
        /// Memory policy (ephemeral/persistent)
        #[arg(long, default_value = "ephemeral")]
        memory: String,
        /// Budget in ARTH wei
        #[arg(long, default_value_t = 500)]
        budget: u64,
    },
    
    /// Start federated learning round
    FederatedStart {
        /// Model ID
        #[arg(long)]
        model: String,
        /// Dataset IDs (comma-separated)
        #[arg(long)]
        datasets: String,
        /// Number of rounds
        #[arg(long, default_value_t = 10)]
        rounds: u32,
        /// Enable differential privacy
        #[arg(long)]
        dp: bool,
        /// Budget in ARTH wei
        #[arg(long, default_value_t = 5000)]
        budget: u64,
    },
    
    /// Start evolutionary search
    EvolutionStart {
        /// Search space CID (artha://...)
        #[arg(long)]
        space: String,
        /// Population size
        #[arg(long, default_value_t = 50)]
        population: u32,
        /// Number of generations
        #[arg(long, default_value_t = 30)]
        generations: u32,
        /// Budget in ARTH wei
        #[arg(long, default_value_t = 2000)]
        budget: u64,
    },
    
    /// Deploy model for inference
    Deploy {
        /// Model ID to deploy
        #[arg(long)]
        model: String,
        /// Endpoint path (e.g. "/generate")
        #[arg(long, default_value = "/generate")]
        endpoint: String,
        /// Number of replicas
        #[arg(long, default_value_t = 1)]
        replicas: u32,
        /// Max tokens per request
        #[arg(long, default_value_t = 4096)]
        max_tokens: u32,
    },
    
    /// Get job logs
    JobLogs {
        /// Job ID
        job_id: String,
        /// Follow logs (tail -f)
        #[arg(long)]
        follow: bool,
    },
    
    /// Cancel job
    JobCancel {
        /// Job ID
        job_id: String,
    },
    
    /// Detect missing shards for a manifest CID
    RepairDetect {
        cid: String,
    },
    /// Post a repair task on-chain (RepairAuction.createTask)
    RepairPost {
        #[arg(long)] rpc_url: String,
        #[arg(long)] chain_id: u64,
        #[arg(long)] private_key: String,
        #[arg(long)] repair_auction: String,
        #[arg(long)] manifest_root: String,
        #[arg(long)] shard_index: u64,
        #[arg(long)] bounty_wei: u64,
    },
    /// Claim a repair task on-chain (RepairAuction.claim)
    RepairClaim {
        #[arg(long)] rpc_url: String,
        #[arg(long)] chain_id: u64,
        #[arg(long)] private_key: String,
        #[arg(long)] repair_auction: String,
        #[arg(long)] manifest_root: String,
        #[arg(long)] shard_index: u64,
        #[arg(long)] leaf: String,
        /// Comma-separated hex bytes32 branch
        #[arg(long)] branch: String,
        #[arg(long)] index: u64,
    },
    /// Register dataset on-chain (DatasetRegistry.registerDataset)
    RegistryDatasetOnchain {
        #[arg(long)] rpc_url: String,
        #[arg(long)] chain_id: u64,
        #[arg(long)] private_key: String,
        #[arg(long)] dataset_registry: String,
        #[arg(long)] cid_root: String,
        #[arg(long)] size: u64,
        #[arg(long, default_value = "")] license: String,
    },
    /// Register model on-chain (ModelRegistry.registerModel)
    RegistryModelOnchain {
        #[arg(long)] rpc_url: String,
        #[arg(long)] chain_id: u64,
        #[arg(long)] private_key: String,
        #[arg(long)] model_registry: String,
        #[arg(long)] model_cid_root: String,
        #[arg(long)] dataset_cid_root: String,
        #[arg(long)] code_hash: String,
        #[arg(long, default_value = "")] version: String,
    },
    /// Governance: read price from PriceOracle
    GovPrice { #[arg(long)] rpc_url: String, #[arg(long)] price_oracle: String },
    /// Governance: read offer from OfferBook
    GovOffer { #[arg(long)] rpc_url: String, #[arg(long)] offer_book: String, #[arg(long)] provider: String },
    /// Governance: read provider reputation multiplier from node
    GovReputation { #[arg(long)] provider: String },
    /// Submit epoch payout: build branch then streamPayout on-chain
    PayoutSubmit {
        /// CID URI artha://<base64>
        cid: String,
        /// leaf index to challenge
        #[arg(long)] index: u64,
        #[arg(long)] rpc_url: String,
        #[arg(long)] chain_id: u64,
        #[arg(long)] private_key: String,
        #[arg(long)] deal_market: String,
        /// next nonce for the signer
        #[arg(long)] nonce: u64,
        /// gas price in wei
        #[arg(long, default_value_t = 1_000_000_000u64)] gas_price: u64,
        /// gas limit
        #[arg(long, default_value_t = 300_000u64)] gas_limit: u64,
    },
}

fn main() {
    let cli = Cli::parse();
    let client = Client::new();

    match cli.command {
        Commands::Quote { provider, cid } => {
            let url = format!("{}/svdb/retrieval/quote", cli.node);
            let payload = serde_json::json!({"provider": provider, "cid": cid});
            let resp = client.post(url).json(&payload).send().expect("quote");
            if !resp.status().is_success() { panic!("quote failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::Settle { rpc_url, chain_id, private_key, deal_market, manifest_root, bytes_served, provider, total_wei, gas_price, gas_limit } => {
            let url = format!("{}/svdb/retrieval/settle", cli.node);
            let payload = serde_json::json!({
                "rpcUrl": rpc_url, "chainId": chain_id, "privateKey": private_key,
                "dealMarket": deal_market, "manifestRoot": manifest_root,
                "bytesServed": bytes_served, "provider": provider, "totalWei": total_wei,
                "gasPrice": gas_price, "gasLimit": gas_limit
            });
            let resp = client.post(url).json(&payload).send().expect("settle");
            if !resp.status().is_success() { panic!("settle failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::Pin { cid } => {
            let url = format!("{}/svdb/pin", cli.node);
            let payload = serde_json::json!({"cid": cid});
            let resp = client.post(url).json(&payload).send().expect("pin");
            if !resp.status().is_success() { panic!("pin failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::Unpin { cid } => {
            let url = format!("{}/svdb/unpin", cli.node);
            let payload = serde_json::json!({"cid": cid});
            let resp = client.post(url).json(&payload).send().expect("unpin");
            if !resp.status().is_success() { panic!("unpin failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::StoragePush { input, envelope, .. } => {
            let data = fs::read(&input).expect("read input");
            let part = multipart::Part::bytes(data);
            let form = multipart::Form::new().part("file", part);
            let url = format!("{}/svdb/upload", cli.node);
            let mut req = client.post(url).multipart(form);
            if let Some(path) = envelope {
                let content = fs::read_to_string(path).expect("read envelope json");
                req = req.header("X-Artha-Envelope", content);
            }
            let resp = req.send().expect("upload");
            if !resp.status().is_success() { panic!("upload failed: {}", resp.status()); }
            let txt = resp.text().expect("body");
            println!("{}", txt);
        }
        Commands::StorageGet { cid, output } => {
            let cid_b64 = cid.trim_start_matches("artha://").to_string();
            let url = format!("{}/svdb/download/{}", cli.node, cid_b64);
            let mut resp = client.get(url).send().expect("download");
            if !resp.status().is_success() && resp.status().as_u16() != 206 { panic!("download failed: {}", resp.status()); }
            let mut file = fs::File::create(&output).expect("create output");
            let bytes = resp.bytes().expect("bytes");
            file.write_all(&bytes).expect("write");
            println!("wrote {} bytes to {}", bytes.len(), output);
        }
        Commands::StorageInfo { cid } => {
            let cid_b64 = cid.trim_start_matches("artha://").to_string();
            let url = format!("{}/svdb/info/{}", cli.node, cid_b64);
            let resp = client.get(url).send().expect("info");
            if !resp.status().is_success() { panic!("info failed: {}", resp.status()); }
            let txt = resp.text().expect("body");
            println!("{}", txt);
        }
        Commands::StoragePin { cid, size, replicas, months, max_price } => {
            let url = format!("{}/svdb/deals", cli.node);
            let payload = serde_json::json!({
                "cid": cid,
                "size": size,
                "replicas": replicas,
                "months": months,
                "maxPrice": max_price,
            });
            let resp = client.post(url).json(&payload).send().expect("deal");
            if !resp.status().is_success() { panic!("deal failed: {}", resp.status()); }
            let txt = resp.text().expect("body");
            println!("{}", txt);
        }
        Commands::AccessPolicy { cid, private, allow, token } => {
            let url = format!("{}/svdb/access/policy", cli.node);
            let allowed: Vec<&str> = if allow.trim().is_empty() { vec![] } else { allow.split(',').map(|s| s.trim()).collect() };
            let payload = serde_json::json!({
                "cid": cid,
                "private": private,
                "allowedDids": allowed,
                "token": if token.trim().is_empty() { serde_json::Value::Null } else { serde_json::Value::String(token) }
            });
            let resp = client.post(url).json(&payload).send().expect("access policy");
            if !resp.status().is_success() { panic!("access policy failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::AccessAllowAdd { cid, did } => {
            let url = format!("{}/svdb/access/allowlist/add", cli.node);
            let payload = serde_json::json!({ "cid": cid, "did": did });
            let resp = client.post(url).json(&payload).send().expect("allow add");
            if !resp.status().is_success() { panic!("allow add failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::AccessAllowRemove { cid, did } => {
            let url = format!("{}/svdb/access/allowlist/remove", cli.node);
            let payload = serde_json::json!({ "cid": cid, "did": did });
            let resp = client.post(url).json(&payload).send().expect("allow remove");
            if !resp.status().is_success() { panic!("allow remove failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::RepairDetect { cid } => {
            let url = format!("{}/svdb/repair/detect", cli.node);
            let payload = serde_json::json!({ "cid": cid });
            let resp = client.post(url).json(&payload).send().expect("detect");
            if !resp.status().is_success() { panic!("detect failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::RepairPost { rpc_url, chain_id, private_key, repair_auction, manifest_root, shard_index, bounty_wei } => {
            let url = format!("{}/svdb/repair/post", cli.node);
            let payload = serde_json::json!({
                "rpcUrl": rpc_url, "chainId": chain_id, "privateKey": private_key,
                "repairAuction": repair_auction, "manifestRoot": manifest_root,
                "shardIndex": shard_index, "bountyWei": bounty_wei
            });
            let resp = client.post(url).json(&payload).send().expect("post");
            if !resp.status().is_success() { panic!("post failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::RepairClaim { rpc_url, chain_id, private_key, repair_auction, manifest_root, shard_index, leaf, branch, index } => {
            let url = format!("{}/svdb/repair/claim", cli.node);
            let branch_vec: Vec<serde_json::Value> = branch.split(',').map(|s| serde_json::Value::String(s.trim().to_string())).collect();
            let payload = serde_json::json!({
                "rpcUrl": rpc_url, "chainId": chain_id, "privateKey": private_key,
                "repairAuction": repair_auction, "manifestRoot": manifest_root,
                "shardIndex": shard_index, "leaf": leaf, "branch": branch_vec, "index": index
            });
            let resp = client.post(url).json(&payload).send().expect("claim");
            if !resp.status().is_success() { panic!("claim failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::RegistryDatasetOnchain { rpc_url, chain_id, private_key, dataset_registry, cid_root, size, license } => {
            let url = format!("{}/svdb/registry/dataset/onchain", cli.node);
            let payload = serde_json::json!({
                "rpcUrl": rpc_url, "chainId": chain_id, "privateKey": private_key,
                "datasetRegistry": dataset_registry, "cidRoot": cid_root, "size": size, "license": license
            });
            let resp = client.post(url).json(&payload).send().expect("dataset onchain");
            if !resp.status().is_success() { panic!("dataset onchain failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::RegistryModelOnchain { rpc_url, chain_id, private_key, model_registry, model_cid_root, dataset_cid_root, code_hash, version } => {
            let url = format!("{}/svdb/registry/model/onchain", cli.node);
            let payload = serde_json::json!({
                "rpcUrl": rpc_url, "chainId": chain_id, "privateKey": private_key,
                "modelRegistry": model_registry, "modelCidRoot": model_cid_root, "datasetCidRoot": dataset_cid_root, "codeHash": code_hash, "version": version
            });
            let resp = client.post(url).json(&payload).send().expect("model onchain");
            if !resp.status().is_success() { panic!("model onchain failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::GovPrice { rpc_url, price_oracle } => {
            let url = format!("{}/svdb/governance/price?rpcUrl={}&priceOracle={}", cli.node, urlencoding::encode(&rpc_url), urlencoding::encode(&price_oracle));
            let resp = client.get(url).send().expect("gov price");
            if !resp.status().is_success() { panic!("gov price failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::GovOffer { rpc_url, offer_book, provider } => {
            let url = format!("{}/svdb/governance/offer?rpcUrl={}&offerBook={}&provider={}", cli.node, urlencoding::encode(&rpc_url), urlencoding::encode(&offer_book), urlencoding::encode(&provider));
            let resp = client.get(url).send().expect("gov offer");
            if !resp.status().is_success() { panic!("gov offer failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::GovReputation { provider } => {
            let url = format!("{}/svdb/governance/reputation?provider={}", cli.node, urlencoding::encode(&provider));
            let resp = client.get(url).send().expect("gov reputation");
            if !resp.status().is_success() { panic!("gov reputation failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::PayoutSubmit { cid, index, rpc_url, chain_id, private_key, deal_market, nonce, gas_price, gas_limit } => {
            // 1) Fetch branch
            let url = format!("{}/svdb/proofs/branch", cli.node);
            let payload = serde_json::json!({ "cid": cid, "index": index });
            let resp = client.post(url).json(&payload).send().expect("branch");
            if !resp.status().is_success() { panic!("branch failed: {}", resp.status()); }
            let json: serde_json::Value = serde_json::from_str(&resp.text().unwrap()).expect("json");
            let root = json.get("root").and_then(|v| v.as_str()).expect("root");
            let leaf = json.get("leaf").and_then(|v| v.as_str()).expect("leaf");
            let branch = json.get("branch").and_then(|v| v.as_array()).expect("branch");
            let branch_vals: Vec<serde_json::Value> = branch.iter().map(|v| serde_json::Value::String(v.as_str().unwrap().to_string())).collect();
            // 2) Submit streamPayout
            let url2 = format!("{}/svdb/proofs/submit", cli.node);
            let payload2 = serde_json::json!({
                "rpcUrl": rpc_url,
                "chainId": chain_id,
                "privateKey": private_key,
                "nonce": nonce,
                "gasPrice": gas_price,
                "gasLimit": gas_limit,
                "dealMarket": deal_market,
                "root": root,
                "leaf": leaf,
                "index": index,
                "branch": branch_vals
            });
            let resp2 = client.post(url2).json(&payload2).send().expect("submit payout");
            if !resp2.status().is_success() { panic!("submit failed: {}", resp2.status()); }
            println!("{}", resp2.text().unwrap());
        }
        
        // Identity & AI Commands
        Commands::IdentityDidCreate { auth_key, enc_key, meta_cid } => {
            let url = format!("{}/identity/did/create", cli.node);
            let payload = serde_json::json!({"authKey": auth_key, "encKey": enc_key, "metaCid": meta_cid});
            let resp = client.post(&url).json(&payload).send().expect("create DID");
            if !resp.status().is_success() { panic!("create DID failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::IdentityDidGet { did } => {
            let url = format!("{}/identity/did/{}", cli.node, urlencoding::encode(&did));
            let resp = client.get(&url).send().expect("get DID");
            if !resp.status().is_success() { panic!("get DID failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::IdentityDidRotate { did, new_auth_key, new_enc_key } => {
            let url = format!("{}/identity/did/rotate", cli.node);
            let payload = serde_json::json!({"did": did, "newAuthKey": new_auth_key, "newEncKey": new_enc_key});
            let resp = client.post(&url).json(&payload).send().expect("rotate keys");
            if !resp.status().is_success() { panic!("rotate keys failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::IdentityDidRevoke { did } => {
            let url = format!("{}/identity/did/revoke", cli.node);
            let payload = serde_json::json!({"did": did});
            let resp = client.post(&url).json(&payload).send().expect("revoke DID");
            if !resp.status().is_success() { panic!("revoke DID failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::IdentityVcIssue { issuer, subject, claim_hash, doc_cid, expires_at } => {
            let url = format!("{}/identity/vc/issue", cli.node);
            let payload = serde_json::json!({"issuerDid": issuer, "subjectDid": subject, "claimHash": claim_hash, "docCid": doc_cid, "expiresAt": expires_at});
            let resp = client.post(&url).json(&payload).send().expect("issue VC");
            if !resp.status().is_success() { panic!("issue VC failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::IdentityVcRevoke { vc_hash } => {
            let url = format!("{}/identity/vc/revoke", cli.node);
            let payload = serde_json::json!({"vcHash": vc_hash});
            let resp = client.post(&url).json(&payload).send().expect("revoke VC");
            if !resp.status().is_success() { panic!("revoke VC failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::IdentityVcVerify { vc_hash } => {
            let url = format!("{}/identity/vc/{}", cli.node, urlencoding::encode(&vc_hash));
            let resp = client.get(&url).send().expect("verify VC");
            if !resp.status().is_success() { panic!("verify VC failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::IdentityVcList { subject } => {
            let url = format!("{}/identity/vc/subject/{}", cli.node, urlencoding::encode(&subject));
            let resp = client.get(&url).send().expect("list VCs");
            if !resp.status().is_success() { panic!("list VCs failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::IdentityAiidCreate { owner, model_cid, dataset_id, code_hash, version } => {
            let url = format!("{}/identity/aiid/create", cli.node);
            let payload = serde_json::json!({"ownerDid": owner, "modelCid": model_cid, "datasetId": dataset_id, "codeHash": code_hash, "version": version});
            let resp = client.post(&url).json(&payload).send().expect("create AIID");
            if !resp.status().is_success() { panic!("create AIID failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::IdentityAiidGet { aiid } => {
            let url = format!("{}/identity/aiid/{}", cli.node, urlencoding::encode(&aiid));
            let resp = client.get(&url).send().expect("get AIID");
            if !resp.status().is_success() { panic!("get AIID failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::IdentityAiidRotate { aiid, new_model_cid, new_version } => {
            let url = format!("{}/identity/aiid/rotate", cli.node);
            let payload = serde_json::json!({"aiid": aiid, "newModelCid": new_model_cid, "newVersion": new_version});
            let resp = client.post(&url).json(&payload).send().expect("rotate AIID");
            if !resp.status().is_success() { panic!("rotate AIID failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::NodecertRegister { node_pubkey, owner_did, role, region, capabilities } => {
            let url = format!("{}/nodecert/register", cli.node);
            let caps: Vec<String> = capabilities.split(',').map(|s| s.trim().to_string()).collect();
            let payload = serde_json::json!({"nodePubkey": node_pubkey, "ownerDid": owner_did, "role": role, "region": region, "capabilities": caps});
            let resp = client.post(&url).json(&payload).send().expect("register node");
            if !resp.status().is_success() { panic!("register node failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::NodecertHeartbeat { node_pubkey } => {
            let url = format!("{}/nodecert/heartbeat", cli.node);
            let payload = serde_json::json!({"nodePubkey": node_pubkey});
            let resp = client.post(&url).json(&payload).send().expect("heartbeat");
            if !resp.status().is_success() { panic!("heartbeat failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::JobSubmit { aiid, dataset_id, params_hash, submitter_did } => {
            let url = format!("{}/job/submit", cli.node);
            let payload = serde_json::json!({"aiid": aiid, "datasetId": dataset_id, "paramsHash": params_hash, "submitterDid": submitter_did});
            let resp = client.post(&url).json(&payload).send().expect("submit job");
            if !resp.status().is_success() { panic!("submit job failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        Commands::JobStatus { job_id } => {
            let url = format!("{}/job/{}", cli.node, urlencoding::encode(&job_id));
            let resp = client.get(&url).send().expect("get job status");
            if !resp.status().is_success() { panic!("get job status failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        
        // ========================================================================
        // AI Operations Handlers (ArthaAIN v1)
        // ========================================================================
        
        Commands::DatasetRegister { root_cid, license, tags } => {
            let url = format!("{}/ai/dataset/register", cli.node);
            let tag_vec: Vec<String> = if tags.is_empty() {
                vec![]
            } else {
                tags.split(',').map(|s| s.trim().to_string()).collect()
            };
            let payload = serde_json::json!({
                "rootCid": root_cid,
                "licenseCid": license,
                "tags": tag_vec,
            });
            let resp = client.post(url).json(&payload).send().expect("register dataset");
            if !resp.status().is_success() { panic!("register dataset failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        
        Commands::DatasetList { owner } => {
            let url = if let Some(owner_did) = owner {
                format!("{}/ai/dataset/list?owner={}", cli.node, urlencoding::encode(&owner_did))
            } else {
                format!("{}/ai/dataset/list", cli.node)
            };
            let resp = client.get(url).send().expect("list datasets");
            if !resp.status().is_success() { panic!("list datasets failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        
        Commands::DatasetInfo { dataset_id } => {
            let url = format!("{}/ai/dataset/{}", cli.node, urlencoding::encode(&dataset_id));
            let resp = client.get(url).send().expect("get dataset info");
            if !resp.status().is_success() { panic!("get dataset info failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        
        Commands::ModelRegister { model_cid, arch, base_model, dataset, code_hash, version, license } => {
            let url = format!("{}/ai/model/register", cli.node);
            let payload = serde_json::json!({
                "modelCid": model_cid,
                "architecture": arch,
                "baseModelId": if base_model.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(base_model) },
                "datasetId": dataset,
                "codeHash": code_hash,
                "version": version,
                "licenseCid": if license.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(license) },
            });
            let resp = client.post(url).json(&payload).send().expect("register model");
            if !resp.status().is_success() { panic!("register model failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        
        Commands::ModelList { owner } => {
            let url = if let Some(owner_did) = owner {
                format!("{}/ai/model/list?owner={}", cli.node, urlencoding::encode(&owner_did))
            } else {
                format!("{}/ai/model/list", cli.node)
            };
            let resp = client.get(url).send().expect("list models");
            if !resp.status().is_success() { panic!("list models failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        
        Commands::ModelLineage { model_id } => {
            let url = format!("{}/ai/model/{}/lineage", cli.node, urlencoding::encode(&model_id));
            let resp = client.get(url).send().expect("get model lineage");
            if !resp.status().is_success() { panic!("get model lineage failed: {}", resp.status()); }
            println!("{}", resp.text().unwrap());
        }
        
        Commands::ModelPublish { model_id, checkpoint } => {
            println!("📦 Publishing model checkpoint...");
            println!("  Model:     {}", model_id);
            println!("  Checkpoint: {}", checkpoint);
            
            let url = format!("{}/ai/model/{}/publish", cli.node, urlencoding::encode(&model_id));
            let payload = serde_json::json!({
                "checkpointCid": checkpoint,
            });
            
            let resp = client.post(url).json(&payload).send().expect("publish model");
            if !resp.status().is_success() {
                panic!("publish model failed: {}", resp.status());
            }
            
            let result: serde_json::Value = resp.json().expect("parse response");
            println!("\n✅ Model checkpoint published!");
            
            if let Some(published_cid) = result.get("publishedCid") {
                println!("   Published CID: {}", published_cid);
            }
            if let Some(version) = result.get("version") {
                println!("   Version: {}", version);
            }
            if let Some(lineage) = result.get("lineage") {
                println!("   Lineage: {} checkpoints", lineage.as_array().map(|a| a.len()).unwrap_or(0));
            }
            
            println!("\n📝 Next steps:");
            println!("   1. Deploy: arthai deploy --model {} --endpoint /generate", model_id);
            println!("   2. Verify: arthai model lineage {}", model_id);
            println!("   3. Share: Model available at artha://{}", checkpoint);
        }
        
        Commands::Train { model, data, epochs, batch, lr, optimizer, budget, output } => {
            println!("🚀 Submitting training job...");
            println!("  Model:      {}", model);
            println!("  Dataset:    {}", data);
            println!("  Epochs:     {}", epochs);
            println!("  Batch Size: {}", batch);
            println!("  LR:         {}", lr);
            println!("  Budget:     {} ARTH", budget);
            
            let url = format!("{}/ai/train", cli.node);
            let payload = serde_json::json!({
                "modelId": model,
                "datasetId": data,
                "submitterDid": "did:artha:cli", // Should come from auth
                "params": {
                    "epochs": epochs,
                    "batchSize": batch,
                    "learningRate": lr,
                    "optimizer": optimizer,
                    "checkpointInterval": 500,
                },
                "budget": budget,
            });
            
            let resp = client.post(url).json(&payload).send().expect("submit train job");
            if !resp.status().is_success() {
                panic!("train job failed: {}", resp.status());
            }
            
            let result: serde_json::Value = resp.json().expect("parse response");
            let job_id = result["jobId"].as_str().unwrap();
            
            println!("\n✅ Training job submitted!");
            println!("   Job ID: {}", job_id);
            if let Some(status) = result.get("status") {
                println!("   Status: {}", status);
            }
            if let Some(cost) = result.get("estimatedCost") {
                println!("   Estimated Cost: {} ARTH", cost);
            }
            if let Some(duration) = result.get("estimatedDurationSecs") {
                println!("   Estimated Duration: {}s", duration);
            }
            
            if let Some(output_path) = output {
                println!("\n⏳ Polling for completion (Ctrl+C to stop)...");
                poll_job_status(&client, &cli.node, job_id, Some(&output_path));
            } else {
                println!("\nMonitor with: arthai job-status {}", job_id);
                println!("Get logs with: arthai job-logs {}", job_id);
            }
        }
        
        Commands::Infer { model, input, mode, max_tokens, budget, out } => {
            println!("🔮 Submitting inference job...");
            println!("  Model:      {}", model);
            println!("  Input:      {}", input);
            println!("  Mode:       {}", mode);
            println!("  Max Tokens: {}", max_tokens);
            
            // Read input file
            let input_text = fs::read_to_string(&input).expect("read input file");
            
            let url = format!("{}/ai/infer", cli.node);
            let payload = serde_json::json!({
                "modelId": model,
                "inlineInput": input_text,
                "submitterDid": "did:artha:cli",
                "mode": mode,
                "maxTokens": max_tokens,
                "budget": budget,
            });
            
            let resp = client.post(url).json(&payload).send().expect("submit infer job");
            if !resp.status().is_success() {
                panic!("infer job failed: {}", resp.status());
            }
            
            let result: serde_json::Value = resp.json().expect("parse response");
            let job_id = result["jobId"].as_str().unwrap();
            
            println!("\n✅ Inference job submitted!");
            println!("   Job ID: {}", job_id);
            if let Some(status) = result.get("status") {
                println!("   Status: {}", status);
            }
            
            if let Some(output_path) = out {
                println!("\n⏳ Waiting for inference result...");
                poll_job_status(&client, &cli.node, job_id, Some(&output_path));
            } else {
                println!("\nMonitor with: arthai job-status {}", job_id);
                println!("Get logs with: arthai job-logs {}", job_id);
            }
        }
        
        Commands::AgentRun { aiid, goal, tools, memory, budget } => {
            println!("🤖 Starting autonomous agent...");
            println!("  AIID:   {}", aiid);
            println!("  Goal:   {}", goal);
            println!("  Tools:  {}", tools);
            println!("  Memory: {}", memory);
            
            let url = format!("{}/ai/agent", cli.node);
            let tool_vec: Vec<String> = tools.split(',').map(|s| s.trim().to_string()).collect();
            let payload = serde_json::json!({
                "agentSpecCid": aiid,
                "submitterDid": "did:artha:cli",
                "goal": goal,
                "tools": tool_vec,
                "memoryPolicy": memory,
                "budget": budget,
            });
            
            let resp = client.post(url).json(&payload).send().expect("submit agent job");
            if !resp.status().is_success() {
                panic!("agent job failed: {}", resp.status());
            }
            
            let result: serde_json::Value = resp.json().expect("parse response");
            let job_id = result["jobId"].as_str().unwrap();
            
            println!("\n✅ Agent started!");
            println!("   Job ID: {}", job_id);
            println!("\nMonitor with: arthai job-status {}", job_id);
        }
        
        Commands::FederatedStart { model, datasets, rounds, dp, budget } => {
            println!("🔗 Starting federated learning...");
            println!("  Model:   {}", model);
            println!("  Rounds:  {}", rounds);
            println!("  DP:      {}", if dp { "enabled" } else { "disabled" });
            
            let url = format!("{}/ai/federated/start", cli.node);
            let dataset_vec: Vec<String> = datasets.split(',').map(|s| s.trim().to_string()).collect();
            let payload = serde_json::json!({
                "modelId": model,
                "datasetIds": dataset_vec,
                "rounds": rounds,
                "dp": dp,
                "budget": budget,
            });
            
            let resp = client.post(url).json(&payload).send().expect("start federated");
            if !resp.status().is_success() {
                panic!("federated start failed: {}", resp.status());
            }
            println!("{}", resp.text().unwrap());
        }
        
        Commands::EvolutionStart { space, population, generations, budget } => {
            println!("🧬 Starting evolutionary search...");
            println!("  Population:  {}", population);
            println!("  Generations: {}", generations);
            
            let url = format!("{}/ai/evolve/start", cli.node);
            let payload = serde_json::json!({
                "searchSpaceCid": space,
                "population": population,
                "generations": generations,
                "budget": budget,
            });
            
            let resp = client.post(url).json(&payload).send().expect("start evolution");
            if !resp.status().is_success() {
                panic!("evolution start failed: {}", resp.status()); 
            }
            println!("{}", resp.text().unwrap());
        }
        
        Commands::Deploy { model, endpoint, replicas, max_tokens } => {
            println!("🚀 Deploying model for inference...");
            println!("  Model:      {}", model);
            println!("  Endpoint:   {}", endpoint);
            println!("  Replicas:   {}", replicas);
            println!("  Max Tokens: {}", max_tokens);
            
            let url = format!("{}/ai/deploy", cli.node);
            let payload = serde_json::json!({
                "modelId": model,
                "endpoint": endpoint,
                "replicas": replicas,
                "maxTokens": max_tokens,
            });
            
            let resp = client.post(url).json(&payload).send().expect("deploy model");
            if !resp.status().is_success() {
                panic!("deploy failed: {}", resp.status());
            }
            
            let result: serde_json::Value = resp.json().expect("parse response");
            let deployment_id = result["deploymentId"].as_str().unwrap_or("unknown");
            
            println!("\n✅ Model deployed!");
            println!("   Deployment ID: {}", deployment_id);
            
            // Generate endpoint URL
            let endpoint_url = if endpoint.starts_with('/') {
                format!("{}{}", cli.node, endpoint)
            } else {
                format!("{}/{}", cli.node, endpoint)
            };
            println!("   Endpoint URL: {}", endpoint_url);
            println!("   API Endpoint: {}/ai/deployment/{}/status", cli.node, deployment_id);
            
            // Wait for deployment to be ready
            println!("\n⏳ Waiting for deployment to be ready...");
            for _ in 0..60 {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let status_url = format!("{}/ai/deployment/{}/status", cli.node, deployment_id);
                if let Ok(status_resp) = client.get(&status_url).send() {
                    if status_resp.status().is_success() {
                        if let Ok(status_data) = status_resp.json::<serde_json::Value>() {
                            let status = status_data["status"].as_str().unwrap_or("unknown");
                            if status == "active" || status == "ready" {
                                println!("   ✅ Deployment ready!");
                                if let Some(endpoint_val) = status_data.get("endpoint") {
                                    println!("   🌐 Live Endpoint: {}", endpoint_val);
                                }
                                break;
                            }
                            print!(".");
                            std::io::stdout().flush().unwrap();
                        }
                    }
                }
            }
            println!("\n");
        }
        
        Commands::JobLogs { job_id, follow } => {
            if follow {
                println!("📜 Following logs for job {}... (Ctrl+C to stop)", job_id);
                // In production: WebSocket stream
                loop {
                    let url = format!("{}/ai/job/{}/logs", cli.node, urlencoding::encode(&job_id));
                    let resp = client.get(&url).send().expect("get logs");
                    if resp.status().is_success() {
                        if let Ok(logs) = resp.json::<Vec<String>>() {
                            for log in logs {
                                println!("{}", log);
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            } else {
                let url = format!("{}/ai/job/{}/logs", cli.node, urlencoding::encode(&job_id));
                let resp = client.get(url).send().expect("get logs");
                if !resp.status().is_success() {
                    panic!("get logs failed: {}", resp.status());
                }
                println!("{}", resp.text().unwrap());
            }
        }
        
        Commands::JobCancel { job_id } => {
            println!("❌ Cancelling job {}...", job_id);
            let url = format!("{}/ai/job/{}/cancel", cli.node, urlencoding::encode(&job_id));
            let resp = client.post(url).send().expect("cancel job");
            if !resp.status().is_success() {
                panic!("cancel failed: {}", resp.status());
            }
            println!("✅ Job cancelled");
        }
    }
}

fn poll_job_status(client: &Client, node: &str, job_id: &str, output_path: Option<&str>) {
    use std::thread;
    use std::time::Duration;
    
    let mut last_log_count = 0;
    
    loop {
        thread::sleep(Duration::from_secs(5));
        
        let url = format!("{}/ai/job/{}/status", node, urlencoding::encode(job_id));
        let resp = match client.get(&url).send() {
            Ok(r) => r,
            Err(_) => {
                print!("\r   ⚠️  Connection error, retrying...");
                std::io::stdout().flush().unwrap();
                continue;
            }
        };
        
        if !resp.status().is_success() {
            continue;
        }
        
        let status: serde_json::Value = match resp.json() {
            Ok(s) => s,
            Err(_) => continue,
        };
        
        let job = &status["job"];
        let state = job["status"].as_str().unwrap_or("unknown");
        let progress = job["progress"].as_f64().unwrap_or(0.0);
        
        // Get new logs
        let logs_url = format!("{}/ai/job/{}/logs", node, urlencoding::encode(job_id));
        if let Ok(logs_resp) = client.get(&logs_url).send() {
            if logs_resp.status().is_success() {
                if let Ok(logs) = logs_resp.json::<Vec<String>>() {
                    for i in last_log_count..logs.len() {
                        println!("\n   {}", logs[i]);
                    }
                    last_log_count = logs.len();
                }
            }
        }
        
        print!("\r   Status: {:<12} | Progress: {:>5.1}%", state, progress * 100.0);
        std::io::stdout().flush().unwrap();
        
        match state {
            "Completed" => {
                println!("\n\n✅ Job completed!");
                
                // Download artifacts if output path provided
                if let Some(output) = output_path {
                    if let Some(output_cid) = job["outputCid"].as_str() {
                        println!("   Downloading output from {}...", output_cid);
                        let cid_clean = output_cid.replace("artha://", "");
                        let download_url = format!("{}/svdb/download/{}", node, cid_clean);
                        match client.get(&download_url).send() {
                            Ok(mut resp) => {
                                if resp.status().is_success() {
                                    match std::fs::File::create(output) {
                                        Ok(mut file) => {
                                            if std::io::copy(&mut resp, &mut file).is_ok() {
                                                println!("   ✅ Output saved to: {}", output);
                                            }
                                        }
                                        Err(e) => println!("   ⚠️  Failed to create file: {}", e),
                                    }
                                }
                            }
                            Err(e) => println!("   ⚠️  Failed to download: {}", e),
                        }
                    }
                    
                    // Also download artifacts
                    if let Some(artifacts) = status["artifacts"].as_array() {
                        println!("   Artifacts available: {}", artifacts.len());
                    }
                }
                break;
            }
            "Failed" | "Cancelled" => {
                println!("\n\n❌ Job {}: {}", job_id, state);
                if let Some(reason) = job.get("reason").and_then(|r| r.as_str()) {
                    println!("   Reason: {}", reason);
                }
                break;
            }
            _ => continue,
        }
    }
}



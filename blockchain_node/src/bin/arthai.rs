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
    },
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
        Commands::StoragePush { input, .. } => {
            let data = fs::read(&input).expect("read input");
            let part = multipart::Part::bytes(data);
            let form = multipart::Form::new().part("file", part);
            let url = format!("{}/svdb/upload", cli.node);
            let resp = client.post(url).multipart(form).send().expect("upload");
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
    }
}



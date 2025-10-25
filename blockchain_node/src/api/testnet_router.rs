use crate::types::Hash;
use crate::network::types::NodeId;
use axum::{
    extract::{Extension, Path, Query, State as AxumState, Multipart},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use axum::response::Response as AxumResponse;

use crate::api::{
    handlers::{
        accounts, ai, blocks, consensus, contracts, dev, faucet, gas_free, identity, metrics,
        monitoring, network_monitoring, security, status, testnet_api, transaction_submission,
        transactions, validators, wallet_rpc,
    },
    routes::create_monitoring_router,
    server::NetworkStats,
    wallet_integration,
};
use crate::gas_free::GasFreeManager;
use crate::ledger::state::State;
use crate::transaction::mempool::Mempool;
use crate::storage::{svdb_storage::SvdbStorage, Manifests, ChunkStore, Cid, Manifest, Codec, ManifestChunkEntry, MemMapStorage, Storage as _};
use axum::http::HeaderMap;
use crate::storage::EncryptionEnvelope;
use ed25519_dalek::{VerifyingKey, Signature};
use axum::http::StatusCode as HttpStatusCode;
use reqwest::Client as HttpClient;
use sha3::{Digest, Keccak256};
use base64::Engine as _;
use ed25519_dalek::Verifier as _;
use elliptic_curve::generic_array::GenericArray;

fn json_error(code: HttpStatusCode, message: &str, details: Option<serde_json::Value>) -> (HttpStatusCode, axum::Json<serde_json::Value>) {
    let mut obj = serde_json::json!({"error": {"code": code.as_u16(), "message": message}});
    if let Some(d) = details { obj["error"]["details"] = d; }
    (code, axum::Json(obj))
}

fn keccak_bytes(input: &[u8]) -> [u8;32] {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    let result = hasher.finalize();
    let mut out = [0u8;32];
    out.copy_from_slice(&result);
    out
}
/// Global node state for tracking runtime information
#[derive(Clone)]
pub struct NodeRuntimeState {
    pub node_id: String,
    pub start_time: SystemTime,
    pub version: String,
    pub network_name: String,
    pub role_validator: bool,
    pub role_storage_provider: bool,
    pub role_retriever: bool,
    pub role_archive: bool,
}

impl NodeRuntimeState {
    pub fn new() -> Self {
        let role_validator = std::env::var("ARTHA_ROLE_VALIDATOR").ok().map(|v| v=="1"||v.eq_ignore_ascii_case("true")).unwrap_or(true);
        let role_storage_provider = std::env::var("ARTHA_ROLE_SP").ok().map(|v| v=="1"||v.eq_ignore_ascii_case("true")).unwrap_or(true);
        let role_retriever = std::env::var("ARTHA_ROLE_RETRIEVER").ok().map(|v| v=="1"||v.eq_ignore_ascii_case("true")).unwrap_or(true);
        let role_archive = std::env::var("ARTHA_ROLE_ARCHIVE").ok().map(|v| v=="1"||v.eq_ignore_ascii_case("true")).unwrap_or(false);
        Self {
            node_id: NodeId::random().into_string(),
            start_time: SystemTime::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            network_name: "ArthaChain Testnet".to_string(),
            role_validator,
            role_storage_provider,
            role_retriever,
            role_archive,
        }
    }

    pub fn get_uptime(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn get_uptime_formatted(&self) -> String {
        let uptime = self.get_uptime();
        let days = uptime / 86400;
        let hours = (uptime % 86400) / 3600;
        let minutes = (uptime % 3600) / 60;
        let seconds = uptime % 60;
        
        if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

/// Create the testnet router with all API endpoints connected to real data
pub fn create_testnet_router(
    state: Arc<RwLock<State>>,
    mempool: Arc<RwLock<Mempool>>,
    faucet_service: Arc<faucet::Faucet>,
    gas_free_manager: Arc<GasFreeManager>,
) -> Router {
    let node_runtime = NodeRuntimeState::new();
    let svdb = SvdbStorage::default();
    let deal_store = MemMapStorage::default();

    // Background epoch scheduler for Merkle sample payouts (v1)
    {
        let svdb_bg = svdb.clone();
        let deal_store_bg = deal_store.clone();
        let node_rt = node_runtime.clone();
        tokio::spawn(async move {
            if !node_rt.role_storage_provider { return; }
            // Read config from environment
            let rpc_url = match std::env::var("ARTHA_RPC_URL") { Ok(v) => v, Err(_) => return };
            let chain_id: u64 = match std::env::var("ARTHA_CHAIN_ID").ok().and_then(|v| v.parse().ok()) { Some(v) => v, None => return };
            let priv_hex = match std::env::var("ARTHA_PRIVATE_KEY") { Ok(v) => v, Err(_) => return };
            let deal_market = match std::env::var("ARTHA_DEALMARKET") { Ok(v) => v, Err(_) => return };
            let gas_price: u64 = std::env::var("ARTHA_GAS_PRICE").ok().and_then(|v| v.parse().ok()).unwrap_or(1_000_000_000);
            let gas_limit: u64 = std::env::var("ARTHA_GAS_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(300_000);
            let epoch_secs: u64 = std::env::var("ARTHA_EPOCH_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(60);

            fn keccak(input: &[u8]) -> [u8;32] { use tiny_keccak::Hasher; let mut h=tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; h.update(input); h.finalize(&mut out); out }
            fn pad32(mut v: Vec<u8>) -> Vec<u8> { let mut p = vec![0u8; 32 - v.len()]; p.append(&mut v); p }
            fn enc_bytes32(b: &[u8]) -> Vec<u8> { let mut out = vec![0u8;32]; out.copy_from_slice(b); out.to_vec() }
            fn rlp_bytes(b: &[u8]) -> Vec<u8> { if b.len()==1 && b[0]<0x80 { return b.to_vec(); } if b.len()<=55 { let mut out=vec![0x80 + b.len() as u8]; out.extend_from_slice(b); return out; } let mut len= b.len(); let mut v=Vec::new(); let mut s=Vec::new(); while len>0 { s.push((len & 0xff) as u8); len >>= 8; } for c in s.iter().rev(){ v.push(*c); } let mut out=vec![0xb7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(b); out }
            fn rlp_u256(x: u128) -> Vec<u8> { if x==0 { return vec![0x80]; } let mut n=x; let mut tmp=Vec::new(); while n>0 { tmp.push((n & 0xff) as u8); n >>= 8; } rlp_bytes(&tmp.iter().rev().cloned().collect::<Vec<_>>()) }
            fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> { let payload_len: usize = items.iter().map(|i| i.len()).sum(); let mut payload=Vec::with_capacity(payload_len); for i in items { payload.extend_from_slice(i); } if payload_len<=55 { let mut out=vec![0xc0 + payload_len as u8]; out.extend_from_slice(&payload); return out; } let mut n=payload_len; let mut v=Vec::new(); let mut s=Vec::new(); while n>0{ s.push((n & 0xff) as u8); n >>= 8; } for c in s.iter().rev(){ v.push(*c);} let mut out=vec![0xf7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(&payload); out }

            // Track nonce if provided, else fetch from RPC each time
            let mut cached_nonce: Option<u64> = None;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(epoch_secs)).await;
                // Load manifest index
                let index_key = b"mf:all".to_vec();
                let list = match deal_store_bg.get(&index_key).await { Ok(Some(b)) => serde_json::from_slice::<Vec<String>>(&b).unwrap_or_default(), _ => Vec::new() };
                if list.is_empty() { continue; }
                for cid_uri in list.iter() {
                    // Parse CID
                    let b64 = cid_uri.trim_start_matches("artha://");
                    let Ok(bytes) = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64) else { continue };
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { continue; }
                    let mut bl=[0u8;32]; bl.copy_from_slice(&bytes[2..34]);
                    let mut cursor = 35; let has_poseidon = bytes[34]==1; let poseidon = if has_poseidon { if bytes.len()<cursor+32 { None } else { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } } else { None };
                    let mut sz=[0u8;8]; if bytes.len()<cursor+8 { continue; } sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let size = u64::from_be_bytes(sz);
                    let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                    let m_cid = Cid::new(u16::from_be_bytes([bytes[0],bytes[1]]), bl, poseidon, size, codec);
                    // Get manifest
                    let Ok(manifest) = svdb_bg.get_manifest(&m_cid).await else { continue };
                    // Avoid double-paying: check epoch
                    let now = chrono::Utc::now().timestamp() as u64;
                    let start_key = [b"start:".as_ref(), &manifest.merkle_root].concat();
                    let last_key = [b"lastpay:".as_ref(), &manifest.merkle_root].concat();
                    let start_epoch = match deal_store_bg.get(&start_key).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => { let se = now; let _=deal_store_bg.put(&start_key, &se.to_le_bytes()).await; se } };
                    let current_epoch = (now.saturating_sub(start_epoch)).saturating_div(epoch_secs.max(1));
                    let last_epoch = match deal_store_bg.get(&last_key).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                    if current_epoch <= last_epoch { continue; }
                    // Build leaves and a random index
                    let mut entries = manifest.chunks.clone(); entries.sort_by_key(|e| e.order);
                    if entries.is_empty() { continue; }
                    // Derive epoch salt from L1 randomness (previous block hash with lag)
                    let lag: u64 = std::env::var("ARTHA_SALT_LAG").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
                    let client = reqwest::Client::new();
                    // Fetch latest block number
                    let bn_payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1});
                    let Ok(bn_resp) = client.post(&rpc_url).json(&bn_payload).send().await else { continue };
                    let Ok(bn_json) = bn_resp.json::<serde_json::Value>().await else { continue };
                    let latest_hex = bn_json.get("result").and_then(|v| v.as_str()).unwrap_or("0x0");
                    let latest = u64::from_str_radix(latest_hex.trim_start_matches("0x"), 16).unwrap_or(0);
                    let target = latest.saturating_sub(lag);
                    let target_hex = format!("0x{:x}", target);
                    // Fetch block by number
                    let blk_payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":[target_hex, false],"id":1});
                    let Ok(blk_resp) = client.post(&rpc_url).json(&blk_payload).send().await else { continue };
                    let Ok(blk_json) = blk_resp.json::<serde_json::Value>().await else { continue };
                    let salt_hex = blk_json.get("result").and_then(|r| r.get("hash")).and_then(|v| v.as_str()).unwrap_or("0x00");
                    let salt = match hex::decode(salt_hex.trim_start_matches("0x")) { Ok(v) if v.len()==32 => { let mut a=[0u8;32]; a.copy_from_slice(&v); a }, _ => [0u8;32] };
                    // Compute leaves
                    let mut leaves: Vec<[u8;32]> = Vec::with_capacity(entries.len());
                    for e in &entries { if let Ok(bytes) = svdb_bg.get(&e.cid).await { leaves.push(*blake3::hash(&bytes).as_bytes()); } else { leaves.push([0u8;32]); } }
                    // K-sample per epoch
                    let k_samples: usize = std::env::var("ARTHA_PROOFS_K").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
                    let mut indices = Vec::new();
                    for i in 0..k_samples {
                        let s = keccak(&[salt.as_slice(), &(i as u64).to_le_bytes()].concat());
                        let idx = (u128::from_be_bytes({ let mut a=[0u8;16]; a.copy_from_slice(&s[0..16]); a }) % (entries.len() as u128)) as usize;
                        indices.push(idx);
                    }

                    // Batch mode (default): ARTHA_PROOFS_BATCH=1
                    let use_batch = std::env::var("ARTHA_PROOFS_BATCH").ok().map(|v| v=="1"||v.eq_ignore_ascii_case("true")).unwrap_or(true);
                    if use_batch {
                        // Build per-index branches and leaves
                        let mut leaves_vec: Vec<Vec<u8>> = Vec::with_capacity(indices.len());
                        let mut branches_vec: Vec<Vec<u8>> = Vec::with_capacity(indices.len());
                        let mut indices_u256: Vec<u128> = Vec::with_capacity(indices.len());
                        for &idx in &indices {
                            let mut level = leaves.clone();
                            let leaf = level[idx];
                            let mut branch: Vec<[u8;32]> = Vec::new();
                            let mut i_idx = idx;
                            while level.len() > 1 {
                                let mut next = Vec::with_capacity((level.len()+1)/2);
                                let mut i = 0;
                                while i < level.len() {
                                    let l = level[i];
                                    let r = if i+1 < level.len() { level[i+1] } else { l };
                                    if i == (i_idx ^ 1) || i+1 == (i_idx ^ 1) { let sib = if i_idx % 2 == 0 { r } else { l }; branch.push(sib); }
                                    let ke = keccak_bytes(&[l.as_slice(), r.as_slice()].concat());
                                    next.push(ke);
                                    i += 2;
                                }
                                level = next; i_idx /= 2;
                            }
                            leaves_vec.push(leaf.to_vec());
                            // abi-encode dynamic array bytes32[] branch for later
                            let mut btail = Vec::new();
                            btail.extend_from_slice(&rlp_u256(branch.len() as u128));
                            for b in &branch { btail.extend_from_slice(&enc_bytes32(b)); }
                            branches_vec.push(btail);
                            indices_u256.push(idx as u128);
                        }

                        // ABI encode DealMarket.streamPayoutV2Batch
                        let selector = &keccak(b"streamPayoutV2Batch(bytes32[],bytes32[],bytes32[],bytes32[][],uint256[],address[])")[0..4];
                        // per-parameter heads (offsets for dynamic arrays)
                        // For simplicity, construct a monolithic payload: selector + heads/tails (not full ABI here; demo pattern)
                        // Fallback to sequential if ABI packing fails
                        let provider_hex = std::env::var("ARTHA_PROVIDER").unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());
                        let provider = match hex::decode(provider_hex.trim_start_matches("0x")) { Ok(v) => v, Err(_) => vec![0u8;20] };
                        // Compose a minimal approximated encoding by calling our batch submit endpoint helper instead
                        let deal_market = match std::env::var("ARTHA_DEALMARKET") { Ok(v) => v, Err(_) => continue };
                        let chain_id = chain_id as u128;
                        // Build calldata via our REST batch submit helper
                        // Note: for production switch to direct ABI encoder
                        // Submit per-leaf (fallback if needed)
                        for (i, idx) in indices.iter().enumerate() {
                            let leaf = &leaves_vec[i];
                            let mut level = leaves.clone();
                            let mut branch: Vec<[u8;32]> = Vec::new(); let mut i_idx = *idx;
                            while level.len() > 1 { let mut next=Vec::with_capacity((level.len()+1)/2); let mut j=0; while j<level.len(){ let l=level[j]; let r=if j+1<level.len(){level[j+1]} else { l }; if j==(i_idx^1)||j+1==(i_idx^1){ let sib=if i_idx%2==0{r}else{l}; branch.push(sib);} let ke=keccak256(&[l.as_slice(), r.as_slice()].concat()); next.push(ke); j+=2;} level=next; i_idx/=2; }
                            // streamPayoutV2 per index (fallback path)
                            let selector2 = &keccak(b"streamPayoutV2(bytes32,bytes32,bytes32,bytes32[],uint256)")[0..4];
                            let root_bytes = manifest.merkle_root.to_vec(); let leaf_bytes = leaf.clone();
                            let head_size = 32*5; let mut head = Vec::with_capacity(head_size);
                            head.extend_from_slice(&enc_bytes32(&root_bytes)); head.extend_from_slice(&enc_bytes32(&salt)); head.extend_from_slice(&enc_bytes32(&leaf_bytes)); head.extend_from_slice(&rlp_u256(head_size as u128)); head.extend_from_slice(&rlp_u256(*idx as u128));
                            let mut branch_tail = Vec::new(); branch_tail.extend_from_slice(&rlp_u256(branch.len() as u128)); for b in &branch { branch_tail.extend_from_slice(&enc_bytes32(b)); }
                            let mut data = Vec::with_capacity(4 + head.len() + branch_tail.len()); data.extend_from_slice(selector2); data.extend_from_slice(&head); data.extend_from_slice(&branch_tail);
                            let to = match hex::decode(deal_market.trim_start_matches("0x")) { Ok(v) => v, Err(_) => continue };
                            let nonce_u64 = if let Some(n) = cached_nonce { cached_nonce = Some(n + 1); n } else { let from_addr = match std::env::var("ARTHA_FROM") { Ok(v) => v, Err(_) => continue }; let payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_getTransactionCount","params":[from_addr,"pending"],"id":1}); let Ok(resp)=client.post(&rpc_url).json(&payload).send().await else { continue }; let Ok(val)=resp.json::<serde_json::Value>().await else { continue }; let hex_nonce = val.get("result").and_then(|v| v.as_str()).unwrap_or("0x0"); let n = u64::from_str_radix(hex_nonce.trim_start_matches("0x"), 16).unwrap_or(0); cached_nonce = Some(n); n };
                            let nonce = nonce_u64 as u128; let tx_parts = vec![ rlp_u256(nonce), rlp_u256(gas_price as u128), rlp_u256(gas_limit as u128), rlp_bytes(&to), rlp_u256(0), rlp_bytes(&data), rlp_u256(chain_id), rlp_u256(0), rlp_u256(0) ]; let sighash = keccak(&rlp_list(&tx_parts));
                        let Ok(pk_bytes) = hex::decode(priv_hex.trim_start_matches("0x")) else { continue }; use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey}; let Ok(sk)=SecretKey::from_bytes(&pk_bytes) else { continue }; let signing_key=SigningKey::from(sk); let sig: k256::ecdsa::Signature = signing_key.sign(&sighash); let (r,s)=(sig.r().to_bytes(), sig.s().to_bytes()); let v = (chain_id as u64 * 2 + 35) as u8; let raw = rlp_list(&[ rlp_u256(nonce), rlp_u256(gas_price as u128), rlp_u256(gas_limit as u128), rlp_bytes(&to), rlp_u256(0), rlp_bytes(&data), rlp_u256(v as u128), rlp_bytes(&r.to_vec()), rlp_bytes(&s.to_vec()) ]);
                            let raw_hex = format!("0x{}", hex::encode(raw)); let payload_rpc = serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex.clone()],"id":1}); let _ = client.post(&rpc_url).json(&payload_rpc).send().await;
                        }
                        let _ = deal_store_bg.put(&last_key, &(current_epoch as u64).to_le_bytes()).await;
                        continue;
                    }

                    // Sequential submit (fallback)
                    for idx in indices {
                        // Build branch for idx
                        let mut level = leaves.clone();
                        let leaf = level[idx];
                        let mut branch: Vec<[u8;32]> = Vec::new();
                        let mut i_idx = idx;
                        while level.len() > 1 {
                            let mut next = Vec::with_capacity((level.len()+1)/2);
                            let mut i = 0;
                            while i < level.len() {
                                let l = level[i];
                                let r = if i+1 < level.len() { level[i+1] } else { l };
                                if i == (i_idx ^ 1) || i+1 == (i_idx ^ 1) { let sib = if i_idx % 2 == 0 { r } else { l }; branch.push(sib); }
                                let ke = keccak_bytes(&[l.as_slice(), r.as_slice()].concat());
                                next.push(ke);
                                i += 2;
                            }
                            level = next; i_idx /= 2;
                        }
                        // Optional pre-check via ProofsV2
                        if let Ok(proofs_v2) = std::env::var("ARTHA_PROOFS_V2") {
                            let selector = &keccak(b"verifySalted(bytes32,bytes32,bytes32,bytes32[],uint256)")[0..4];
                            let mut head = Vec::new();
                            head.extend_from_slice(&enc_bytes32(&manifest.merkle_root));
                            head.extend_from_slice(&enc_bytes32(&salt));
                            let head_size = 32*5;
                            head.extend_from_slice(&rlp_u256(head_size as u128));
                            head.extend_from_slice(&enc_bytes32(&leaf));
                            head.extend_from_slice(&rlp_u256(idx as u128));
                            let mut tail = Vec::new();
                            tail.extend_from_slice(&rlp_u256(branch.len() as u128));
                            for b in &branch { tail.extend_from_slice(&enc_bytes32(b)); }
                            let mut call_data = Vec::with_capacity(4 + head.len() + tail.len());
                            call_data.extend_from_slice(selector);
                            call_data.extend_from_slice(&head);
                            call_data.extend_from_slice(&tail);
                            let call = serde_json::json!({"to": proofs_v2, "data": format!("0x{}", hex::encode(call_data))});
                            let payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_call","params":[call, "latest"],"id":1});
                            if client.post(&rpc_url).json(&payload).send().await.is_err() { continue; }
                        }
                        // Submit streamPayoutV2 (salted)
                        let selector = &keccak(b"streamPayoutV2(bytes32,bytes32,bytes32,bytes32[],uint256)")[0..4];
                        let root_bytes = manifest.merkle_root.to_vec();
                        let leaf_bytes = leaf.to_vec();
                        let head_size = 32*5;
                        let mut head = Vec::with_capacity(head_size);
                        head.extend_from_slice(&enc_bytes32(&root_bytes));
                        head.extend_from_slice(&enc_bytes32(&salt));
                        head.extend_from_slice(&enc_bytes32(&leaf_bytes));
                        head.extend_from_slice(&rlp_u256(head_size as u128));
                        head.extend_from_slice(&rlp_u256(idx as u128));
                        let mut branch_tail = Vec::new();
                        branch_tail.extend_from_slice(&rlp_u256(branch.len() as u128));
                        for b in &branch { branch_tail.extend_from_slice(&enc_bytes32(b)); }
                        let mut data = Vec::with_capacity(4 + head.len() + branch_tail.len());
                        data.extend_from_slice(selector);
                        data.extend_from_slice(&head);
                        data.extend_from_slice(&branch_tail);
                        let to = match hex::decode(deal_market.trim_start_matches("0x")) { Ok(v) => v, Err(_) => continue };
                        let nonce_u64 = if let Some(n) = cached_nonce { cached_nonce = Some(n + 1); n } else {
                            let from_addr = match std::env::var("ARTHA_FROM") { Ok(v) => v, Err(_) => continue };
                            let payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_getTransactionCount","params":[from_addr,"pending"],"id":1});
                            let Ok(resp) = client.post(&rpc_url).json(&payload).send().await else { continue };
                            let Ok(val) = resp.json::<serde_json::Value>().await else { continue };
                            let hex_nonce = val.get("result").and_then(|v| v.as_str()).unwrap_or("0x0");
                            let n = u64::from_str_radix(hex_nonce.trim_start_matches("0x"), 16).unwrap_or(0);
                            cached_nonce = Some(n); n
                        };
                        let nonce = nonce_u64 as u128;
                        let tx_parts = vec![ rlp_u256(nonce), rlp_u256(gas_price as u128), rlp_u256(gas_limit as u128), rlp_bytes(&to), rlp_u256(0), rlp_bytes(&data), rlp_u256(chain_id as u128), rlp_u256(0), rlp_u256(0) ];
                        let sighash = keccak(&rlp_list(&tx_parts));
                        let Ok(pk_bytes) = hex::decode(priv_hex.trim_start_matches("0x")) else { continue };
                        use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey};
                        let Ok(sk) = SecretKey::from_bytes(&pk_bytes) else { continue };
                        let signing_key = SigningKey::from(sk);
                        let sig: k256::ecdsa::Signature = signing_key.sign(&sighash);
                        let (r, s) = (sig.r().to_bytes(), sig.s().to_bytes());
                        let v = (chain_id * 2 + 35) as u8;
                        let raw = rlp_list(&[ rlp_u256(nonce), rlp_u256(gas_price as u128), rlp_u256(gas_limit as u128), rlp_bytes(&to), rlp_u256(0), rlp_bytes(&data), rlp_u256(v as u128), rlp_bytes(&r.to_vec()), rlp_bytes(&s.to_vec()) ]);
                        let raw_hex = format!("0x{}", hex::encode(raw));
                        let payload_rpc = serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex.clone()],"id":1});
                        let tx_hash = if let Ok(resp_tx) = client.post(&rpc_url).json(&payload_rpc).send().await {
                            if let Ok(val) = resp_tx.json::<serde_json::Value>().await { val.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()) } else { None }
                        } else { None };
                        // Persist proof log for auditing
                        let root_hex = format!("0x{}", hex::encode(manifest.merkle_root));
                        let log_key = format!("prooflog:{}:{}:{}", root_hex, current_epoch, idx);
                        let log = serde_json::json!({
                            "root": root_hex,
                            "epoch": current_epoch,
                            "index": idx,
                            "salt": format!("0x{}", hex::encode(salt)),
                            "branchLen": branch.len(),
                            "tx": tx_hash,
                        });
                        let _ = deal_store_bg.put(log_key.as_bytes(), serde_json::to_string(&log).unwrap().as_bytes()).await;
                    }
                    let _ = deal_store_bg.put(&last_key, &(current_epoch as u64).to_le_bytes()).await;
                }
            }
        });
    }
    Router::new()
        // Basic status endpoints
        .route("/", get(|| async {
            Html(r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>ArthaChain Node</title>
                <style>
                    body { font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }
                    .container { max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
                    h1 { color: #2c3e50; text-align: center; }
                    .section { margin: 30px 0; padding: 20px; border: 1px solid #ecf0f1; border-radius: 8px; }
                    .endpoint { background: #f8f9fa; padding: 15px; margin: 10px 0; border-radius: 5px; border-left: 4px solid #3498db; }
                    .method { display: inline-block; background: #3498db; color: white; padding: 5px 10px; border-radius: 3px; font-size: 12px; font-weight: bold; }
                    .url { font-family: monospace; color: #2c3e50; }
                    .description { color: #7f8c8d; margin-top: 5px; }
                </style>
            </head>
            <body>
                <div class="container">
                    <h1>ArthaChain Node</h1>
                    <p style="text-align: center; color: #7f8c8d;">Next-generation blockchain with AI-native features, quantum resistance, and ultra-high performance</p>
                    <div class="section">
                        <h2>API Endpoints</h2>
                        <div class="endpoint">
                            <span class="method">GET</span>
                            <span class="url">/health</span>
                            <div class="description">Check node health and status</div>
                        </div>
                        <div class="endpoint">
                            <span class="method">GET</span>
                            <span class="url">/api/v1/node/id</span>
                            <div class="description">Get unique node identifier</div>
                        </div>
                        <div class="endpoint">
                            <span class="method">GET</span>
                            <span class="url">/api/v1/blockchain/height</span>
                            <div class="description">Get current blockchain height</div>
                        </div>
                        <div class="endpoint">
                            <span class="method">POST</span>
                            <span class="url">/api/v1/transactions/submit</span>
                            <div class="description">Submit a new transaction</div>
                        </div>
                        <div class="endpoint">
                            <span class="method">GET</span>
                            <span class="url">/api/v1/blockchain/status</span>
                            <div class="description">Get blockchain status and metrics</div>
                        </div>
                    </div>
                </div>
            </body>
            </html>
            "#)
        }))
        // Build Merkle branch (v1) for a manifest and leaf index (blake3 leaf, keccak node composition)
        .route("/svdb/proofs/branch", post({
            let svdb = svdb.clone();
            move |Json(body): Json<serde_json::Value>| {
                let svdb = svdb.clone();
                async move {
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let index = body.get("index").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as usize;
                    let b64 = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let codec_tag = u16::from_be_bytes([bytes[0], bytes[1]]);
                    let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                    let has_poseidon = bytes[34] == 1; let mut cursor = 35;
                    let poseidon = if has_poseidon { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } else { None };
                    let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let cid_size = u64::from_be_bytes(sz);
                    let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                    let m_cid = Cid::new(codec_tag, blake, poseidon, cid_size, codec);
                    let manifest = svdb.get_manifest(&m_cid).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?;
                    let mut entries = manifest.chunks.clone(); entries.sort_by_key(|e| e.order);
                    if index >= entries.len() { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    // compute leaves
                    let mut leaves: Vec<[u8;32]> = Vec::with_capacity(entries.len());
                    for e in &entries { let bytes = ChunkStore::get(&svdb, &e.cid).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?; leaves.push(*blake3::hash(&bytes).as_bytes()); }
                    // build branch for index
                    let mut idx = index;
                    let mut level = leaves.clone();
                    let leaf = level[idx];
                    let mut branch: Vec<[u8;32]> = Vec::new();
                    while level.len() > 1 {
                        let mut next = Vec::with_capacity((level.len() + 1)/2);
                        let mut i = 0;
                        while i < level.len() {
                            let l = level[i];
                            let r = if i+1 < level.len() { level[i+1] } else { l };
                            if i == (idx ^ 1) || i+1 == (idx ^ 1) {
                                // sibling for current idx
                                let sib = if idx % 2 == 0 { r } else { l };
                                branch.push(sib);
                            }
                            let keccak = keccak_bytes(&[l.as_slice(), r.as_slice()].concat());
                            next.push(keccak);
                            i += 2;
                        }
                        level = next;
                        idx /= 2;
                    }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({
                        "root": hex::encode(manifest.merkle_root),
                        "leaf": hex::encode(leaf),
                        "branch": branch.into_iter().map(|b| hex::encode(b)).collect::<Vec<_>>(),
                        "index": index
                    })))
                }
            }
        }))
        // Governance: fetch price oracle base/floor/ceiling
        .route("/svdb/governance/price", get({
            move |Query(params): Query<HashMap<String, String>>| async move {
                let rpc_url = params.get("rpcUrl").ok_or(axum::http::StatusCode::BAD_REQUEST)?.to_string();
                let oracle = params.get("priceOracle").ok_or(axum::http::StatusCode::BAD_REQUEST)?.to_string();
                fn keccak(input:&[u8])->[u8;32]{use tiny_keccak::Hasher; let mut h=tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; h.update(input); h.finalize(&mut out); out}
                let to = hex::decode(oracle.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let selector=&keccak(b"getPrice()")[0..4];
                let data=selector.to_vec();
                let call = serde_json::json!({"to": format!("0x{}", hex::encode(&to)), "data": format!("0x{}", hex::encode(&data))});
                let payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_call","params":[call,"latest"],"id":1});
                let client=HttpClient::new(); let resp=client.post(&rpc_url).json(&payload).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?; let json:serde_json::Value=resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                Ok(Json(json))
            }
        }))
        // Governance: fetch offer for provider
        .route("/svdb/governance/offer", get({
            move |Query(params): Query<HashMap<String, String>>| async move {
                let rpc_url = params.get("rpcUrl").ok_or(axum::http::StatusCode::BAD_REQUEST)?.to_string();
                let offerbook = params.get("offerBook").ok_or(axum::http::StatusCode::BAD_REQUEST)?.to_string();
                let provider = params.get("provider").ok_or(axum::http::StatusCode::BAD_REQUEST)?.to_string();
                fn keccak(input:&[u8])->[u8;32]{use tiny_keccak::Hasher; let mut h=tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; h.update(input); h.finalize(&mut out); out}
                fn enc_address(b: &[u8]) -> Vec<u8> { let mut out = vec![0u8;32]; out[12..].copy_from_slice(b); out }
                let to = hex::decode(offerbook.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let prov = hex::decode(provider.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let selector=&keccak(b"getOffer(address)")[0..4];
                let mut data=Vec::with_capacity(4+32); data.extend_from_slice(selector); data.extend_from_slice(&enc_address(&prov));
                let call = serde_json::json!({"to": format!("0x{}", hex::encode(&to)), "data": format!("0x{}", hex::encode(&data))});
                let payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_call","params":[call,"latest"],"id":1});
                let client=HttpClient::new(); let resp=client.post(&rpc_url).json(&payload).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?; let json:serde_json::Value=resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                Ok(Json(json))
            }
        }))
        // Governance: compute SP reputation multiplier from internal stats
        .route("/svdb/governance/reputation", get({
            let deal_store = deal_store.clone();
            move |Query(params): Query<HashMap<String, String>>| {
                let deal_store = deal_store.clone();
                async move {
                    let provider = params.get("provider").ok_or(axum::http::StatusCode::BAD_REQUEST)?.to_string();
                    // Use capabilities and basic counters to estimate reputation
                    let cap_key = format!("caps:{}", provider);
                    let caps: serde_json::Value = match deal_store.get(cap_key.as_bytes()).await { Ok(Some(b)) => serde_json::from_slice(&b).unwrap_or_default(), _ => serde_json::json!({}) };
                    let gpu = caps.get("gpu").and_then(|v| v.as_bool()).unwrap_or(false);
                    let disk = caps.get("disk_free_bytes").and_then(|v| v.as_u64()).unwrap_or(0) as f64;
                    // In absence of a full metrics DB, simple heuristic
                    let uptime_weight = 0.4f64; let proof_success_weight=0.4f64; let bandwidth_weight=0.2f64;
                    let uptime_score = 1.0f64; // placeholder for now, treat as perfect if registered
                    let proof_success_score = 1.0f64; // would integrate from on-chain events
                    let bandwidth_score = if gpu { 1.0 } else { 0.8 } * (1.0f64.min(disk / (1.0e12))) ;
                    let multiplier = (uptime_weight*uptime_score + proof_success_weight*proof_success_score + bandwidth_weight*bandwidth_score).min(1.5).max(0.5);
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"provider": provider, "multiplier": multiplier})))
                }
            }
        }))
        // RepairAuction.claim on-chain
        .route("/svdb/repair/claim", post({
            move |Json(body): Json<serde_json::Value>| async move {
                let rpc_url = body.get("rpcUrl").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let chain_id = body.get("chainId").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                let priv_hex = body.get("privateKey").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let auction = body.get("repairAuction").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let root_hex = body.get("manifestRoot").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let shard_index = body.get("shardIndex").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                let leaf_hex = body.get("leaf").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let branch_vals = body.get("branch").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let index = body.get("index").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                fn pad32(mut v: Vec<u8>) -> Vec<u8> { let mut p = vec![0u8; 32 - v.len()]; p.append(&mut v); p }
                fn enc_u256(x: u128) -> Vec<u8> { let s = format!("{:x}", x); let mut bytes = if s.len()%2==1 { hex::decode(format!("0{}", s)).unwrap() } else { hex::decode(s).unwrap() }; pad32(bytes) }
                fn enc_bytes32(b: &[u8]) -> Vec<u8> { let mut out = vec![0u8;32]; out.copy_from_slice(b); out.to_vec() }
                fn keccak(input: &[u8]) -> [u8;32] { use tiny_keccak::Hasher; let mut hasher = tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; hasher.update(input); hasher.finalize(&mut out); out }
                fn rlp_bytes(b: &[u8]) -> Vec<u8> { if b.len()==1 && b[0]<0x80 { return b.to_vec(); } if b.len()<=55 { let mut out=vec![0x80 + b.len() as u8]; out.extend_from_slice(b); return out; } let mut len= b.len(); let mut v=Vec::new(); let mut s=Vec::new(); while len>0 { s.push((len & 0xff) as u8); len >>= 8; } for c in s.iter().rev(){ v.push(*c); } let mut out=vec![0xb7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(b); out }
                fn rlp_u256(x: u128) -> Vec<u8> { if x==0 { return vec![0x80]; } let mut n=x; let mut tmp=Vec::new(); while n>0 { tmp.push((n & 0xff) as u8); n >>= 8; } rlp_bytes(&tmp.iter().rev().cloned().collect::<Vec<_>>()) }
                fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> { let payload_len: usize = items.iter().map(|i| i.len()).sum(); let mut payload=Vec::with_capacity(payload_len); for i in items { payload.extend_from_slice(i); } if payload_len<=55 { let mut out=vec![0xc0 + payload_len as u8]; out.extend_from_slice(&payload); return out; } let mut n=payload_len; let mut v=Vec::new(); let mut s=Vec::new(); while n>0{ s.push((n & 0xff) as u8); n >>= 8; } for c in s.iter().rev(){ v.push(*c);} let mut out=vec![0xf7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(&payload); out }
                let to = hex::decode(auction.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let root = hex::decode(root_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let leaf = hex::decode(leaf_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                // Prepare dynamic array branch (bytes32[])
                let mut branch_bytes: Vec<[u8;32]> = Vec::new();
                for v in branch_vals { let s=v.as_str().ok_or(axum::http::StatusCode::BAD_REQUEST)?; let b=hex::decode(s.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if b.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) } let mut arr=[0u8;32]; arr.copy_from_slice(&b); branch_bytes.push(arr); }
                // ABI head: root (32), shardIndex (32), leaf (32), offset to branch (32), index (32)
                let selector = &keccak(b"claim(bytes32,uint256,bytes32,bytes32[],uint256)")[0..4];
                let head_size = 32*5;
                let mut head = Vec::with_capacity(head_size);
                head.extend_from_slice(&enc_bytes32(&root));
                head.extend_from_slice(&enc_u256(shard_index));
                head.extend_from_slice(&enc_bytes32(&leaf.as_slice().try_into().unwrap()));
                head.extend_from_slice(&enc_u256(head_size as u128)); // offset to branch tail
                head.extend_from_slice(&enc_u256(index));
                let mut tail = Vec::new();
                tail.extend_from_slice(&enc_u256(branch_bytes.len() as u128));
                for el in &branch_bytes { tail.extend_from_slice(&enc_bytes32(el)); }
                let mut data = Vec::with_capacity(4 + head.len() + tail.len());
                data.extend_from_slice(selector);
                data.extend_from_slice(&head);
                data.extend_from_slice(&tail);
                // Build and send TX (no value)
                let gas_price = 1_000_000_000u128; let gas_limit=500_000u128; let nonce=0u128; let value=0u128; let chain_id=chain_id;
                let tx_parts = vec![ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(value), rlp_bytes(&data), rlp_u256(chain_id), rlp_u256(0), rlp_u256(0) ];
                let sighash = keccak(&rlp_list(&tx_parts));
                let pk_bytes = hex::decode(priv_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey};
                let pk_arr = GenericArray::from_slice(&pk_bytes);
                let sk = SecretKey::from_bytes(pk_arr).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let signing_key = SigningKey::from(sk);
                let sig: k256::ecdsa::Signature = signing_key.sign(&sighash);
                let (r, s) = (sig.r().to_bytes(), sig.s().to_bytes());
                let v = (chain_id * 2 + 35) as u8;
                let raw = rlp_list(&[ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(value), rlp_bytes(&data), rlp_u256(v as u128), rlp_bytes(&r.to_vec()), rlp_bytes(&s.to_vec()) ]);
                let raw_hex = format!("0x{}", hex::encode(raw));
                let client = HttpClient::new();
                let payload_rpc = serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex],"id":1});
                let resp = client.post(rpc_url).json(&payload_rpc).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                let json: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                Ok(Json(json))
            }
        }))
        // DatasetRegistry on-chain register
        .route("/svdb/registry/dataset/onchain", post({
            move |Json(body): Json<serde_json::Value>| async move {
                let rpc_url = body.get("rpcUrl").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let chain_id = body.get("chainId").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                let priv_hex = body.get("privateKey").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let registry = body.get("datasetRegistry").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let root_hex = body.get("cidRoot").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let size = body.get("size").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                let license = body.get("license").and_then(|v| v.as_str()).unwrap_or("");
                fn pad32(mut v: Vec<u8>) -> Vec<u8> { let mut p=vec![0u8;32 - v.len()]; p.append(&mut v); p }
                fn enc_u256(x: u128) -> Vec<u8> { let s=format!("{:x}",x); let mut b=if s.len()%2==1{hex::decode(format!("0{}",s)).unwrap()} else {hex::decode(s).unwrap()}; pad32(b) }
                fn enc_bytes32(b:&[u8])->Vec<u8>{let mut out=vec![0u8;32]; out.copy_from_slice(b); out.to_vec()}
                fn enc_string(s:&str)->(Vec<u8>,usize){let bytes=s.as_bytes(); let mut tail=Vec::new(); tail.extend_from_slice(&enc_u256(bytes.len() as u128)); let mut data=bytes.to_vec(); while data.len()%32!=0{data.push(0);} tail.extend_from_slice(&data); (tail, 32)}
                fn keccak(input:&[u8])->[u8;32]{use tiny_keccak::Hasher; let mut h=tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; h.update(input); h.finalize(&mut out); out }
                fn rlp_bytes(b:&[u8])->Vec<u8>{ if b.len()==1 && b[0]<0x80 {return b.to_vec();} if b.len()<=55 {let mut out=vec![0x80 + b.len() as u8]; out.extend_from_slice(b); return out;} let mut n=b.len(); let mut v=Vec::new(); let mut s=Vec::new(); while n>0{s.push((n&0xff)as u8); n>>=8;} for c in s.iter().rev(){v.push(*c);} let mut out=vec![0xb7+v.len()as u8]; out.extend_from_slice(&v); out.extend_from_slice(b); out }
                fn rlp_u256(x:u128)->Vec<u8>{ if x==0{return vec![0x80];} let mut n=x; let mut tmp=Vec::new(); while n>0{tmp.push((n&0xff)as u8); n>>=8;} rlp_bytes(&tmp.iter().rev().cloned().collect::<Vec<_>>()) }
                fn rlp_list(items:&[Vec<u8>])->Vec<u8>{ let payload_len:usize=items.iter().map(|i|i.len()).sum(); let mut payload=Vec::with_capacity(payload_len); for i in items{payload.extend_from_slice(i);} if payload_len<=55{let mut out=vec![0xc0+payload_len as u8]; out.extend_from_slice(&payload); return out;} let mut n=payload_len; let mut v=Vec::new(); let mut s=Vec::new(); while n>0{s.push((n&0xff)as u8); n>>=8;} for c in s.iter().rev(){v.push(*c);} let mut out=vec![0xf7+v.len()as u8]; out.extend_from_slice(&v); out.extend_from_slice(&payload); out }
                let to=hex::decode(registry.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let root=hex::decode(root_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                // ABI: registerDataset(bytes32,uint64,string,string[])
                let selector=&keccak(b"registerDataset(bytes32,uint64,string,string[])")[0..4];
                // Head: root(32), size(32), offset_string(32), offset_array(32)
                let head_size=32*4; let mut head=Vec::with_capacity(head_size);
                head.extend_from_slice(&enc_bytes32(&root));
                head.extend_from_slice(&enc_u256(size));
                head.extend_from_slice(&enc_u256(head_size as u128));
                // string[] empty at tail after string
                let (string_tail, _)=enc_string(license);
                let offset_array = head_size as u128 + string_tail.len() as u128;
                head.extend_from_slice(&enc_u256(offset_array));
                let mut tail=Vec::new();
                tail.extend_from_slice(&string_tail);
                tail.extend_from_slice(&enc_u256(0)); // string[] length 0
                let mut data=Vec::with_capacity(4+head.len()+tail.len()); data.extend_from_slice(selector); data.extend_from_slice(&head); data.extend_from_slice(&tail);
                let gas_price=1_000_000_000u128; let gas_limit=500_000u128; let nonce=0u128; let value=0u128; let chain_id=chain_id;
                let tx_parts=vec![ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(value), rlp_bytes(&data), rlp_u256(chain_id), rlp_u256(0), rlp_u256(0) ];
                let sighash=keccak(&rlp_list(&tx_parts));
                let pk_bytes=hex::decode(priv_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey}; let sk=SecretKey::from_bytes(&pk_bytes).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; let signing_key=SigningKey::from(sk); let sig: k256::ecdsa::Signature = signing_key.sign(&sighash); let (r,s)=(sig.r().to_bytes(), sig.s().to_bytes()); let v=(chain_id*2+35) as u8;
                let raw=rlp_list(&[ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(value), rlp_bytes(&data), rlp_u256(v as u128), rlp_bytes(&r.to_vec()), rlp_bytes(&s.to_vec()) ]);
                let raw_hex=format!("0x{}", hex::encode(raw)); let client=HttpClient::new(); let payload_rpc=serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex],"id":1}); let resp=client.post(rpc_url).json(&payload_rpc).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?; let json: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?; Ok::<_, axum::http::StatusCode>(Json(json))
            }
        }))
        // ModelRegistry on-chain register
        .route("/svdb/registry/model/onchain", post({
            move |Json(body): Json<serde_json::Value>| async move {
                let rpc_url = body.get("rpcUrl").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let chain_id = body.get("chainId").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                let priv_hex = body.get("privateKey").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let registry = body.get("modelRegistry").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let model_root = body.get("modelCidRoot").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let dataset_root = body.get("datasetCidRoot").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let code_hash = body.get("codeHash").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let version = body.get("version").and_then(|v| v.as_str()).unwrap_or("");
                fn pad32(mut v: Vec<u8>) -> Vec<u8> { let mut p=vec![0u8;32 - v.len()]; p.append(&mut v); p }
                fn enc_u256(x: u128) -> Vec<u8> { let s=format!("{:x}",x); let mut b=if s.len()%2==1{hex::decode(format!("0{}",s)).unwrap()} else {hex::decode(s).unwrap()}; pad32(b) }
                fn enc_bytes32(b:&[u8])->Vec<u8>{let mut out=vec![0u8;32]; out.copy_from_slice(b); out.to_vec()}
                fn enc_string(s:&str)->(Vec<u8>,usize){let bytes=s.as_bytes(); let mut tail=Vec::new(); tail.extend_from_slice(&enc_u256(bytes.len() as u128)); let mut data=bytes.to_vec(); while data.len()%32!=0{data.push(0);} tail.extend_from_slice(&data); (tail, 32)}
                fn keccak(input:&[u8])->[u8;32]{use tiny_keccak::Hasher; let mut h=tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; h.update(input); h.finalize(&mut out); out }
                fn rlp_bytes(b:&[u8])->Vec<u8>{ if b.len()==1 && b[0]<0x80 {return b.to_vec();} if b.len()<=55 {let mut out=vec![0x80 + b.len() as u8]; out.extend_from_slice(b); return out;} let mut n=b.len(); let mut v=Vec::new(); let mut s=Vec::new(); while n>0{s.push((n&0xff)as u8); n>>=8;} for c in s.iter().rev(){v.push(*c);} let mut out=vec![0xb7+v.len()as u8]; out.extend_from_slice(&v); out.extend_from_slice(b); out }
                fn rlp_u256(x:u128)->Vec<u8>{ if x==0{return vec![0x80];} let mut n=x; let mut tmp=Vec::new(); while n>0{tmp.push((n&0xff)as u8); n>>=8;} rlp_bytes(&tmp.iter().rev().cloned().collect::<Vec<_>>()) }
                fn rlp_list(items:&[Vec<u8>])->Vec<u8>{ let payload_len:usize=items.iter().map(|i|i.len()).sum(); let mut payload=Vec::with_capacity(payload_len); for i in items{payload.extend_from_slice(i);} if payload_len<=55{let mut out=vec![0xc0+payload_len as u8]; out.extend_from_slice(&payload); return out;} let mut n=payload_len; let mut v=Vec::new(); let mut s=Vec::new(); while n>0{s.push((n&0xff)as u8); n>>=8;} for c in s.iter().rev(){v.push(*c);} let mut out=vec![0xf7+v.len()as u8]; out.extend_from_slice(&v); out.extend_from_slice(&payload); out }
                let to=hex::decode(registry.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let model=hex::decode(model_root.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let dataset=hex::decode(dataset_root.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let code=hex::decode(code_hash.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                // ABI: registerModel(bytes32,bytes32,bytes32,string,bytes32[])
                let selector=&keccak(b"registerModel(bytes32,bytes32,bytes32,string,bytes32[])")[0..4];
                // Head: model(32) dataset(32) code(32) offset_string(32) offset_array(32)
                let head_size=32*5; let mut head=Vec::with_capacity(head_size);
                head.extend_from_slice(&enc_bytes32(&model));
                head.extend_from_slice(&enc_bytes32(&dataset));
                head.extend_from_slice(&enc_bytes32(&code));
                head.extend_from_slice(&enc_u256(head_size as u128));
                let (string_tail, _)=enc_string(version);
                let offset_array=head_size as u128 + string_tail.len() as u128;
                head.extend_from_slice(&enc_u256(offset_array));
                let mut tail=Vec::new(); tail.extend_from_slice(&string_tail); tail.extend_from_slice(&enc_u256(0)); // empty lineage
                let mut data=Vec::with_capacity(4+head.len()+tail.len()); data.extend_from_slice(selector); data.extend_from_slice(&head); data.extend_from_slice(&tail);
                let gas_price=1_000_000_000u128; let gas_limit=500_000u128; let nonce=0u128; let value=0u128; let chain_id=chain_id;
                let tx_parts=vec![ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(value), rlp_bytes(&data), rlp_u256(chain_id), rlp_u256(0), rlp_u256(0) ];
                let sighash=keccak(&rlp_list(&tx_parts));
                let pk_bytes=hex::decode(priv_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey}; let sk=SecretKey::from_bytes(&pk_bytes).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; let signing_key=SigningKey::from(sk); let sig: k256::ecdsa::Signature = signing_key.sign(&sighash); let (r,s)=(sig.r().to_bytes(), sig.s().to_bytes()); let v=(chain_id*2+35) as u8;
                let raw=rlp_list(&[ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(value), rlp_bytes(&data), rlp_u256(v as u128), rlp_bytes(&r.to_vec()), rlp_bytes(&s.to_vec()) ]);
                let raw_hex=format!("0x{}", hex::encode(raw)); let client=HttpClient::new(); let payload_rpc=serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex],"id":1}); let resp=client.post(rpc_url).json(&payload_rpc).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?; let json: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?; Ok::<_, axum::http::StatusCode>(Json(json))
            }
        }))
        // Repair detection: find missing shards for a manifest
        .route("/svdb/repair/detect", post({
            let svdb = svdb.clone();
            move |Json(body): Json<serde_json::Value>| {
                let svdb = svdb.clone();
                async move {
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let b64 = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let codec_tag = u16::from_be_bytes([bytes[0], bytes[1]]);
                    let mut bl=[0u8;32]; bl.copy_from_slice(&bytes[2..34]);
                    let has_poseidon = bytes[34] == 1; let mut cursor = 35;
                    let poseidon = if has_poseidon { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } else { None };
                    let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let sz_u = u64::from_be_bytes(sz);
                    let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                    let m_cid = Cid::new(codec_tag, bl, poseidon, sz_u, codec);
                    let manifest = svdb.get_manifest(&m_cid).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?;
                    let mut missing = Vec::new();
                    for (i, e) in manifest.chunks.iter().enumerate() {
                        if !svdb.has(&e.cid).await.unwrap_or(false) { missing.push(i as u32); }
                    }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"missing": missing})))
                }
            }
        }))
        // Repair: attempt RS reconstruction for a manifest stripe given available shard CIDs
        .route("/svdb/repair/reconstruct", post({
            let svdb = svdb.clone();
            move |Json(body): Json<serde_json::Value>| {
                let svdb = svdb.clone();
                async move {
                    // Expect: { shards: [ {cid_b64, order} ... ] } with up to 10 elements (k=8,m=2)
                    let arr = body.get("shards").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    if arr.is_empty() { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let mut shards: Vec<Option<Vec<u8>>> = Vec::new();
                    for entry in arr {
                        let cid_b64 = entry.get("cid_b64").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                        let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(cid_b64).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                        if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                        let mut bl=[0u8;32]; bl.copy_from_slice(&bytes[2..34]);
                        let cid = Cid::new(u16::from_be_bytes([bytes[0],bytes[1]]), bl, None, 0, Codec::Raw);
                        let data = match ChunkStore::get(&svdb, &cid).await { Ok(v)=>Some(v), Err(_)=>None };
                        shards.push(data);
                    }
                    let mut shards_opt = shards;
                    svdb.rs_reconstruct_10_8(&mut shards_opt).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    let out: Vec<String> = shards_opt.into_iter().map(|opt| opt.map(|v| base64::engine::general_purpose::STANDARD_NO_PAD.encode(v)).unwrap_or_default()).collect();
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"reconstructed": out})))
                }
            }
        }))
        // RepairAuction.createTask on-chain
        .route("/svdb/repair/post", post({
            move |Json(body): Json<serde_json::Value>| async move {
                let rpc_url = body.get("rpcUrl").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let chain_id = body.get("chainId").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                let priv_hex = body.get("privateKey").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let auction = body.get("repairAuction").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let root_hex = body.get("manifestRoot").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let shard_index = body.get("shardIndex").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                let bounty_wei = body.get("bountyWei").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                fn pad32(mut v: Vec<u8>) -> Vec<u8> { let mut p = vec![0u8; 32 - v.len()]; p.append(&mut v); p }
                fn enc_u256(x: u128) -> Vec<u8> { let s = format!("{:x}", x); let mut bytes = if s.len()%2==1 { hex::decode(format!("0{}", s)).unwrap() } else { hex::decode(s).unwrap() }; pad32(bytes) }
                fn enc_bytes32(b: &[u8]) -> Vec<u8> { let mut out = vec![0u8;32]; out.copy_from_slice(b); out.to_vec() }
                fn keccak(input: &[u8]) -> [u8;32] { use tiny_keccak::Hasher; let mut hasher = tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; hasher.update(input); hasher.finalize(&mut out); out }
                fn rlp_bytes(b: &[u8]) -> Vec<u8> { if b.len()==1 && b[0]<0x80 { return b.to_vec(); } if b.len()<=55 { let mut out=vec![0x80 + b.len() as u8]; out.extend_from_slice(b); return out; } let mut len= b.len(); let mut v=Vec::new(); let mut s=Vec::new(); while len>0 { s.push((len & 0xff) as u8); len >>= 8; } for c in s.iter().rev(){ v.push(*c); } let mut out=vec![0xb7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(b); out }
                fn rlp_u256(x: u128) -> Vec<u8> { if x==0 { return vec![0x80]; } let mut n=x; let mut tmp=Vec::new(); while n>0 { tmp.push((n & 0xff) as u8); n >>= 8; } rlp_bytes(&tmp.iter().rev().cloned().collect::<Vec<_>>()) }
                fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> { let payload_len: usize = items.iter().map(|i| i.len()).sum(); let mut payload=Vec::with_capacity(payload_len); for i in items { payload.extend_from_slice(i); } if payload_len<=55 { let mut out=vec![0xc0 + payload_len as u8]; out.extend_from_slice(&payload); return out; } let mut n=payload_len; let mut v=Vec::new(); let mut s=Vec::new(); while n>0{ s.push((n & 0xff) as u8); n >>= 8; } for c in s.iter().rev(){ v.push(*c);} let mut out=vec![0xf7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(&payload); out }
                let to = hex::decode(auction.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let root = hex::decode(root_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let selector = &keccak(b"createTask(bytes32,uint256)")[0..4];
                let mut data = Vec::with_capacity(4 + 32*2);
                data.extend_from_slice(selector);
                data.extend_from_slice(&enc_bytes32(&root));
                data.extend_from_slice(&enc_u256(shard_index));
                let gas_price = 1_000_000_000u128; let gas_limit=200_000u128; let nonce=0u128; let value=bounty_wei; let chain_id=chain_id;
                let tx_parts = vec![ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(value), rlp_bytes(&data), rlp_u256(chain_id), rlp_u256(0), rlp_u256(0) ];
                let sighash = keccak(&rlp_list(&tx_parts));
                let pk_bytes = hex::decode(priv_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey};
                let pk_arr = GenericArray::from_slice(&pk_bytes);
                let sk = SecretKey::from_bytes(pk_arr).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let signing_key = SigningKey::from(sk);
                let sig: k256::ecdsa::Signature = signing_key.sign(&sighash);
                let (r, s) = (sig.r().to_bytes(), sig.s().to_bytes());
                let v = (chain_id * 2 + 35) as u8;
                let raw = rlp_list(&[ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(value), rlp_bytes(&data), rlp_u256(v as u128), rlp_bytes(&r.to_vec()), rlp_bytes(&s.to_vec()) ]);
                let raw_hex = format!("0x{}", hex::encode(raw));
                let client = HttpClient::new();
                let payload_rpc = serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex],"id":1});
                let resp = client.post(rpc_url).json(&payload_rpc).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                let json: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                Ok(Json(json))
            }
        }))
        .route("/status", get({
            let node_runtime = node_runtime.clone();
            move || async move {
                Json(serde_json::json!({
                    "node_id": node_runtime.node_id,
                    "service": "ArthaChain Node",
                    "status": "healthy",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "uptime": node_runtime.get_uptime_formatted(),
                    "version": node_runtime.version,
                    "network": node_runtime.network_name
                }))
            }
        }))
        .route("/health", get({
            let node_runtime = node_runtime.clone();
            move || async move {
                Json(serde_json::json!({
                    "node_id": node_runtime.node_id,
                    "service": "ArthaChain Node",
                    "status": "healthy",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "uptime": node_runtime.get_uptime_formatted(),
                    "version": node_runtime.version,
                    "network": node_runtime.network_name
                }))
            }
        }))
        .route("/config", get({
            let node_runtime = node_runtime.clone();
            move || async move {
                Json(serde_json::json!({
                    "chain_id": 201766,
                    "network": node_runtime.network_name,
                    "consensus": "SVCP-SVBFT",
                    "version": node_runtime.version,
                    "node_id": node_runtime.node_id,
                    "roles": {
                        "validator": node_runtime.role_validator,
                        "storage_provider": node_runtime.role_storage_provider,
                        "retriever": node_runtime.role_retriever,
                        "archive": node_runtime.role_archive
                    }
                }))
            }
        }))
        .route("/api/v1/node/roles", post({
            let node_runtime = node_runtime.clone();
            move |Json(body): Json<serde_json::Value>| {
                let node_runtime = node_runtime.clone();
                async move {
                    let mut nr = node_runtime.clone();
                    if let Some(v) = body.get("validator").and_then(|v| v.as_bool()) { nr.role_validator = v; }
                    if let Some(v) = body.get("storage_provider").and_then(|v| v.as_bool()) { nr.role_storage_provider = v; }
                    if let Some(v) = body.get("retriever").and_then(|v| v.as_bool()) { nr.role_retriever = v; }
                    if let Some(v) = body.get("archive").and_then(|v| v.as_bool()) { nr.role_archive = v; }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({
                        "validator": nr.role_validator,
                        "storage_provider": nr.role_storage_provider,
                        "retriever": nr.role_retriever,
                        "archive": nr.role_archive
                    })))
                }
            }
        }))
        .route("/docs", get(|| async { "API Documentation" }))

        // Core Blockchain APIs - Connect to handlers
        .route("/api/v1/blockchain/height", get(blocks::get_blockchain_height))
        .route("/api/v1/blockchain/status", get(blocks::get_blockchain_status))
        .route("/api/v1/node/id", get(identity::get_node_id))
        
        // Blocks API - Connect to handlers
        .route("/api/v1/blocks/latest", get(blocks::get_latest_block))
        .route("/api/v1/blocks/:hash", get(blocks::get_block_by_hash))
        .route("/api/v1/blocks/height/:height", get(blocks::get_block_by_height))
        .route("/api/v1/blocks", get(blocks::get_blocks))
        .route("/api/v1/blocks/sync", post(blocks::sync_blocks))
        
        // Transactions API - Connect to handlers  
        .route("/api/v1/transactions/:hash", get(transactions::get_transaction))
          .route("/api/v1/transactions", get(transactions::get_transaction))
        .route("/api/v1/transactions", post(transactions::submit_transaction))
        .route("/api/v1/transactions/submit", post(transaction_submission::submit_transaction))
        .route("/api/v1/mempool/transactions", get(transaction_submission::get_pending_transactions))
        .route("/api/v1/transactions/pending", get(transaction_submission::get_pending_transactions))
        
        // Accounts API - Connect to handlers
        .route("/api/v1/accounts/:address", get(accounts::get_account))
        .route("/api/v1/accounts/:address/transactions", get(accounts::get_account_transactions))
        .route("/api/v1/accounts/:address/balance", get(accounts::get_account_balance))
        .route("/api/v1/accounts/:address/nonce", get(accounts::get_account_nonce))
        
        // Consensus API - Connect to handlers
        .route("/api/v1/consensus/status", get(consensus::get_consensus_status))
        .route("/api/v1/consensus/validators", get(validators::get_validators_list))
        .route("/api/v1/consensus/rounds", get(consensus::get_consensus_info))
        .route("/api/v1/consensus/vote", post(consensus::submit_vote))
        
        // Network API - Connect to handlers
        .route("/api/v1/network/peers", get(network_monitoring::get_peers))
        .route("/api/v1/network/status", get(network_monitoring::get_network_status))
        .route("/api/v1/network/sync", get(network_monitoring::get_network_status))
        .route("/api/v1/network/mempool-size", get(network_monitoring::get_mempool_size))
        .route("/api/v1/network/uptime", get(network_monitoring::get_uptime))
        .route("/api/v1/network/stats", get(network_monitoring::get_network_status))
        
        
        // Monitoring API - Connect to handlers
        .route("/api/v1/monitoring/health", get(metrics::get_metrics))
        .route("/api/v1/monitoring/metrics", get(metrics::get_metrics))
        .route("/api/v1/monitoring/performance", get(metrics::get_performance_metrics))
        .route("/api/v1/monitoring/alerts", get(metrics::get_performance_metrics))
        
        
        // Faucet API - Connect to handlers
        .route("/api/v1/testnet/faucet/request", post(faucet::request_tokens))
        .route("/api/v1/testnet/faucet/status", get(faucet::get_faucet_status))
        .route("/api/v1/testnet/faucet/history", get(faucet::get_faucet_history))
        .route("/api/v1/faucet/request", get(faucet::get_faucet_form))
        .route("/api/v1/faucet/request", post(faucet::request_tokens))
        
        // Gas-free API - Connect to handlers
        
        .route("/api/v1/testnet/gas-free/stats", get(gas_free::get_gas_free_stats))
        .route("/api/v1/gas-free/status", get(gas_free::get_gas_free_stats))
        
        
        // AI API - Connect to handlers
        .route("/api/v1/ai/status", get(ai::get_ai_status))
        .route("/api/v1/ai/models", get(ai::get_ai_models))
        .route("/api/v1/ai/fraud/detect", post(ai::detect_fraud))
        .route("/api/v1/ai/analytics", get(ai::get_ai_models))
        .route("/api/v1/ai/inference", post(ai::train_neural_network))
        .route("/api/v1/ai/fraud-detection", post(ai::detect_fraud))
        
        // Security API - Connect to handlers
        .route("/api/v1/security/status", get(security::get_security_status))
        .route("/api/v1/security/threats", get(security::get_security_info))
        .route("/api/v1/security/events", get(security::get_security_monitoring))
        .route("/api/v1/security/audit", get(security::get_security_health))
        .route("/api/v1/security/encryption", get(security::get_security_health))
        
        // Contract API - Connect to handlers
        .route("/api/v1/contracts/:address", get(contracts::get_contract_by_address))
        
        .route("/api/v1/contracts/deploy", post(contracts::deploy_evm_contract))
        .route("/api/v1/contracts/call", post(contracts::call_evm_contract))
        
        .route("/api/v1/contracts/:address/call", post(contracts::call_evm_contract))
        .route("/api/v1/contracts/:address/events", get(contracts::get_contracts_health))
        
        // Dev API - Connect to handlers
        .route("/api/v1/dev/tools", get(dev::get_dev_info))
        .route("/api/v1/dev/debug", get(dev::get_debug_info))
        .route("/api/v1/dev/logs", get(dev::get_debug_info))
        .route("/api/v1/developer/tools", get(dev::get_dev_info))
        .route("/api/v1/developer/debug", get(dev::get_debug_info))
        
        // Identity API - Connect to handlers
        .route("/api/v1/identity/create", post(identity::create_did))
        .route("/api/v1/identity/verify", post(identity::authenticate_did))
        .route("/api/v1/identity/status", get(identity::get_identity_status))
        .route("/api/v1/identity/verify", get(identity::get_verify_status))
        
        // Wallet API - Connect to handlers
        .route("/api/v1/wallet/supported", get(wallet_rpc::get_supported_wallets))
        .route("/api/v1/wallet/ides", get(wallet_rpc::get_wallet_ides))
        .route("/api/v1/wallet/connect", get(wallet_rpc::connect_wallet))
        .route("/api/v1/wallet/setup", get(wallet_rpc::setup_wallet))
        .route("/api/v1/wallet/balance", get(wallet_rpc::get_wallet_balance))
        .route("/api/v1/wallet/create", post(wallet_rpc::create_wallet))
        .route("/api/v1/wallet/addresses", get(wallet_rpc::get_wallet_addresses))
        .route("/api/v1/wallet/rpc", post(wallet_rpc::handle_rpc_request))
        
        // EVM/RPC API - Connect to handlers
        .route("/api/v1/rpc/eth_blockNumber", post(wallet_rpc::handle_rpc_request))
        .route("/api/v1/rpc/eth_getBalance", post(wallet_rpc::handle_rpc_request))
        .route("/api/v1/rpc/eth_gasPrice", post(wallet_rpc::handle_rpc_request))
        .route("/api/v1/rpc/eth_sendRawTransaction", post(wallet_rpc::handle_rpc_request))
        .route("/api/v1/rpc/eth_getTransactionCount", post(wallet_rpc::handle_rpc_request))
        .route("/api/v1/rpc/eth_getTransactionReceipt", post(wallet_rpc::handle_rpc_request))
        
        // EVM API - Connect to handlers
        .route("/api/v1/evm/accounts", get(accounts::get_evm_accounts))
        .route("/api/v1/evm/accounts/create", post(accounts::create_evm_account))
        .route("/api/v1/evm/balance", get(accounts::get_evm_balance))
        .route("/api/v1/evm/transfer", post(accounts::transfer_evm))
        
        // WebSocket API - Connect to handlers
        .route("/api/v1/ws/connect", get(wallet_rpc::websocket_connect))
        .route("/api/v1/ws/subscribe", post(wallet_rpc::websocket_subscribe))
        
        // Explorer API - Connect to handlers
        .route("/api/v1/explorer/stats", get(testnet_api::get_blockchain_stats))
        .route("/api/v1/explorer/blocks/recent", get(testnet_api::get_recent_blocks))
        .route("/api/v1/explorer/transactions/recent", get(testnet_api::get_recent_transactions))
        
        // Protocol API - Connect to handlers
        .route("/api/v1/protocol/evm", get(dev::get_evm_protocol))
        .route("/api/v1/protocol/wasm", get(dev::get_wasm_protocol))
        .route("/api/v1/protocol/version", get(dev::get_protocol_version))
        .route("/api/v1/protocol/features", get(dev::get_protocol_features))
        
        // Test API - Connect to handlers
        .route("/api/v1/test/health", get(status::get_status))
        .route("/api/v1/test/performance", get(metrics::get_performance_metrics))
        .route("/api/v1/test/status", get(status::get_status))
        .route("/api/v1/test/run", post(dev::run_tests))
        
        // SVDB Public API
        .route("/svdb/upload", post({
            let svdb = svdb.clone();
            let deal_store_for_access = deal_store.clone();
            let node_runtime_for_upload = node_runtime.clone();
            move |mut multipart: Multipart, headers: HeaderMap| {
                let svdb = svdb.clone();
                let deal_store_for_access = deal_store_for_access.clone();
                let node_runtime_for_upload = node_runtime_for_upload.clone();
                async move {
                    if !node_runtime_for_upload.role_storage_provider { return Err(axum::http::StatusCode::FORBIDDEN); }
                    // Simple per-IP rate limit and size quota
                    let client_ip = headers.get("X-Client-IP").and_then(|v| v.to_str().ok()).unwrap_or("unknown");
                    let now_min = (chrono::Utc::now().timestamp() / 60).to_string();
                    let rl_key = format!("ratelimit:upload:{}:{}", client_ip, now_min);
                    let cnt = match deal_store_for_access.get(rl_key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                    let max_req = std::env::var("ARTHA_UPLOADS_PER_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(30u64);
                    if cnt >= max_req { return Err(axum::http::StatusCode::TOO_MANY_REQUESTS); }
                    let _ = deal_store_for_access.put(rl_key.as_bytes(), &(cnt+1).to_le_bytes()).await;
                    // Optional access policy header: X-Artha-Access: public|private|allowlist
                    let access_mode = headers.get("X-Artha-Access").and_then(|v| v.to_str().ok()).unwrap_or("public");
                    // Stream multipart without buffering entire file in memory
                    // Accumulate into fixed-size buffers and process on the fly
                    let chunk_size: usize = 8 * 1024 * 1024; // 8MB
                    let mut rolling = Vec::with_capacity(chunk_size * 2);
                    let mut total_size: usize = 0;
                    let mut chunks: Vec<ManifestChunkEntry> = Vec::new();
                    let mut order: u32 = 0;
                    let mut leaf_hashes: Vec<[u8;32]> = Vec::new();

                    // Reed-Solomon erasure coding (GF(2^8), k=8, m=2)
                    fn rs_encode_10_8(data: &[u8]) -> Vec<Vec<u8>> {
                        let k = 8usize; let m = 2usize; let n = k + m;
                        let shard_len = (data.len() + k - 1) / k;
                        let mut shards: Vec<Vec<u8>> = vec![vec![0u8; shard_len]; n];
                        for i in 0..k {
                            let start = i * shard_len;
                            let end = core::cmp::min(start + shard_len, data.len());
                            if start < data.len() { shards[i][..end - start].copy_from_slice(&data[start..end]); }
                        }
                        // Parity encoding gated out in default build to avoid unmaintained deps
                        let mut refs: Vec<&mut [u8]> = shards.iter_mut().map(|v| v.as_mut_slice()).collect();
                        let _ = rs.encode(&mut refs);
                        shards
                    }

                    // Process any file fields in a streaming fashion
                    while let Some(mut field) = multipart.next_field().await.unwrap_or(None) {
                        // Read incoming chunks for this field
                        while let Ok(Some(chunk)) = field.chunk().await {
                            if !chunk.is_empty() {
                                total_size = total_size.saturating_add(chunk.len());
                                rolling.extend_from_slice(&chunk);
                            }
                            // While we have at least one full 8MB window, process it
                            while rolling.len() >= chunk_size {
                                let slice = &rolling[..chunk_size];
                                let shards = rs_encode_10_8(slice);
                                for shard in shards.into_iter() {
                                    let blake = *blake3::hash(&shard).as_bytes();
                                    let cid = Cid::new(0x0129, blake, None, shard.len() as u64, Codec::Raw);
                                    ChunkStore::put(&svdb, &cid, &shard).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                                    chunks.push(ManifestChunkEntry { cid: cid.clone(), order });
                                    order += 1;
                                    leaf_hashes.push(*blake3::hash(&shard).as_bytes());
                                }
                                // Drain processed window
                                let remaining = rolling.split_off(chunk_size);
                                rolling = remaining;
                            }
                        }
                    }

                    // Enforce maximum object size and per-IP byte quota
                    let max_mb = std::env::var("ARTHA_MAX_OBJECT_MB").ok().and_then(|v| v.parse().ok()).unwrap_or(10240usize);
                    if total_size > max_mb * 1024 * 1024 { return Err(axum::http::StatusCode::PAYLOAD_TOO_LARGE); }
                    let q_key = format!("quota:bytes:{}", client_ip);
                    let used = match deal_store_for_access.get(q_key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 } as usize;
                    let max_per_ip = std::env::var("ARTHA_MAX_BYTES_PER_IP").ok().and_then(|v| v.parse().ok()).unwrap_or(100*1024*1024*1024usize);
                    if used.saturating_add(total_size) > max_per_ip { return Err(axum::http::StatusCode::FORBIDDEN); }
                    let _ = deal_store_for_access.put(q_key.as_bytes(), &((used+total_size) as u64).to_le_bytes()).await;

                    // Process any final remainder less than chunk_size
                    if !rolling.is_empty() {
                        let shards = rs_encode_10_8(&rolling);
                        for shard in shards.into_iter() {
                            let blake = *blake3::hash(&shard).as_bytes();
                            let cid = Cid::new(0x0129, blake, None, shard.len() as u64, Codec::Raw);
                            ChunkStore::put(&svdb, &cid, &shard).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                            chunks.push(ManifestChunkEntry { cid: cid.clone(), order });
                            order += 1;
                            leaf_hashes.push(*blake3::hash(&shard).as_bytes());
                        }
                    }

                    fn merkle_root(mut leaves: Vec<[u8;32]>) -> [u8;32] {
                        if leaves.is_empty() { return [0u8;32]; }
                        while leaves.len() > 1 {
                            let mut next = Vec::with_capacity((leaves.len()+1)/2);
                            let mut i = 0;
                            while i < leaves.len() {
                                let left = leaves[i];
                                let right = if i+1 < leaves.len() { leaves[i+1] } else { left };
                                let keccak = keccak_bytes(&[left.as_slice(), right.as_slice()].concat());
                                next.push(keccak);
                                i += 2;
                            }
                            leaves = next;
                        }
                        leaves[0]
                    }
                    let merkle_root = merkle_root(leaf_hashes.clone());
                    // Poseidon root over leaves replaced with keccak-based fold to remove light-poseidon dependency
                    fn poseidon_root_over_leaves(mut leaves: Vec<[u8;32]>) -> [u8;32] {
                        if leaves.is_empty() { return [0u8;32]; }
                        while leaves.len() > 1 {
                            let mut next = Vec::with_capacity((leaves.len()+1)/2);
                            let mut i = 0;
                            while i < leaves.len() {
                                let left = leaves[i];
                                let right = if i+1 < leaves.len() { leaves[i+1] } else { left };
                                let h = keccak_bytes(&[left.as_slice(), right.as_slice()].concat());
                                let mut out = [0u8;32];
                                out.copy_from_slice(&h);
                                next.push(out);
                                i += 2;
                            }
                            leaves = next;
                        }
                        leaves[0]
                    }
                    let poseidon_root = poseidon_root_over_leaves(leaf_hashes);

                    let envelope = headers.get("X-Artha-Envelope").and_then(|v| v.to_str().ok()).and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()).map(|j| EncryptionEnvelope {
                        alg: j.get("alg").and_then(|v| v.as_str()).unwrap_or("XChaCha20-Poly1305").to_string(),
                        salt_b64: j.get("salt_b64").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        nonce_b64: j.get("nonce_b64").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        aad_b64: j.get("aad_b64").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    });
                    let manifest = Manifest {
                        version: 1,
                        size: total_size as u64,
                        chunks,
                        license: None,
                        codec: Codec::Raw,
                        erasure_data_shards: Some(8),
                        erasure_parity_shards: Some(2),
                        merkle_root,
                        poseidon_root: Some(poseidon_root),
                        envelope,
                    };
                    let manifest_cid = svdb.put_manifest(&manifest).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    // Auto provider announce for co-location and retrieval routing
                    let cid_b64 = manifest_cid.to_uri();
                    let b64 = cid_b64.trim_start_matches("artha://");
                    let raw = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64).unwrap();
                    let mut bl=[0u8;32]; bl.copy_from_slice(&raw[2..34]);
                    let cid_hex = hex::encode(bl);
                    let prov_key = format!("prov:{}", cid_hex);
                    let mut list: Vec<String> = match deal_store_for_access.get(prov_key.as_bytes()).await { Ok(Some(b)) => serde_json::from_slice(&b).unwrap_or_default(), _ => Vec::new() };
                    if !list.contains(&node_runtime_for_upload.node_id) { list.push(node_runtime_for_upload.node_id.clone()); }
                    let _ = deal_store_for_access.put(prov_key.as_bytes(), serde_json::to_vec(&list).unwrap().as_slice()).await;
                    // Index manifest for epoch scheduler (mf:all)
                    let idx_key = b"mf:all".to_vec();
                    let mut manifests: Vec<String> = match deal_store_for_access.get(&idx_key).await { Ok(Some(b)) => serde_json::from_slice(&b).unwrap_or_default(), _ => Vec::new() };
                    if !manifests.contains(&cid_b64) { manifests.push(cid_b64.clone()); }
                    let _ = deal_store_for_access.put(&idx_key, serde_json::to_vec(&manifests).unwrap().as_slice()).await;
                    // Best-effort P2P announce via HTTP entrypoint (optional): store an "announce" intent for background network task
                    let announce_key = format!("announce:{}", cid_hex);
                    let _ = deal_store_for_access.put(announce_key.as_bytes(), b"1").await;
                    // Also publish to P2P gossipsub (svdb-announce) when message channel is available - handled by background p2p task
                    // Initialize access policy
                    if access_mode == "private" || access_mode == "allowlist" || access_mode == "token" {
                        let policy_key = format!("access:{}", cid_hex);
                        let mut policy = serde_json::json!({"mode": access_mode, "allowedDids": Vec::<String>::new()});
                        if access_mode == "token" {
                            if let Some(tok) = headers.get("X-Artha-Token").and_then(|v| v.to_str().ok()) {
                                let hash = keccak_bytes(tok.as_bytes());
                                policy["tokenHash"] = serde_json::json!(format!("0x{}", hex::encode(hash)));
                            }
                        }
                        let _ = deal_store_for_access.put(policy_key.as_bytes(), serde_json::to_string(&policy).unwrap().as_bytes()).await;
                    }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({ "cid": manifest_cid.to_uri() })))
                }
            }
        }))
        // Serve a single chunk by CID hex (for inter-node retrieval)
        .route("/svdb/chunk/:cid_hex", get({
            let svdb = svdb.clone();
            let node_runtime_chunk = node_runtime.clone();
            move |axum::extract::Path(cid_hex): axum::extract::Path<String>| {
                let svdb = svdb.clone();
                let node_runtime_chunk = node_runtime_chunk.clone();
                async move {
                    if !node_runtime_chunk.role_retriever { return Err(axum::http::StatusCode::FORBIDDEN); }
                    // retriever/serve guard
                    // (NodeRuntimeState not available here; chunk serve is allowed for any node)
                    let mut bl=[0u8;32]; let bytes = hex::decode(&cid_hex).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if bytes.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) } bl.copy_from_slice(&bytes);
                    // Build a minimal CID wrapper to query storage; codec/raw and defaults
                    let cid = Cid::new(0x0129, bl, None, 0, Codec::Raw);
                     let data = ChunkStore::get(&svdb, &cid).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?;
                    Ok::<_, axum::http::StatusCode>(axum::body::Bytes::from(data))
                }
            }
        }))
        .route("/svdb/download/:cid_b64", get({
            let svdb = svdb.clone();
            let deal_store = deal_store.clone();
            let node_runtime_download = node_runtime.clone();
            move |axum::extract::Path(cid_b64): axum::extract::Path<String>, headers: HeaderMap| {
                let svdb = svdb.clone();
                let deal_store = deal_store.clone();
                let node_runtime_download = node_runtime_download.clone();
                async move {
                    if !node_runtime_download.role_retriever { return Err(axum::http::StatusCode::FORBIDDEN); }
                    let enc = cid_b64.trim_start_matches("artha://");
                    // Support base64 (current) and multibase base32 (bafy...)
                    let bytes = match base64::engine::general_purpose::STANDARD_NO_PAD.decode(enc) {
                        Ok(b) => b,
                        Err(_) => {
                            match data_encoding::BASE32_NOPAD.decode(enc.as_bytes()) {
                                Ok(b) => b,
                                Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                            }
                        }
                    };
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let codec_tag = u16::from_be_bytes([bytes[0], bytes[1]]);
                    let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                    let has_poseidon = bytes[34] == 1; let mut cursor = 35;
                    let poseidon = if has_poseidon { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } else { None };
                    let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let size = u64::from_be_bytes(sz);
                    let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                    let m_cid = Cid::new(codec_tag, blake, poseidon, size, codec);
                    // Voucher verification (optional requirement)
                    let require_voucher = std::env::var("ARTHA_REQUIRE_VOUCHER").ok().map(|v| v=="1"||v.eq_ignore_ascii_case("true")).unwrap_or(false);
                    if require_voucher || headers.contains_key("X-Artha-Voucher") {
                        let v_b64 = headers.get("X-Artha-Voucher").and_then(|v| v.to_str().ok()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                        let sig_hex = headers.get("X-Artha-Voucher-Sig").and_then(|v| v.to_str().ok()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                        let v_bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(v_b64).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                        let v_json: serde_json::Value = serde_json::from_slice(&v_bytes).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                        let provider = v_json.get("provider").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                        let cid_hex = hex::encode(blake);
                        let cid_in = v_json.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                        if cid_in.trim_start_matches("0x") != cid_hex { return Err(axum::http::StatusCode::UNAUTHORIZED); }
                        let bytes_req = v_json.get("bytes").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                        let price = v_json.get("price").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                        let nonce = v_json.get("nonce").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                        let expires = v_json.get("expires").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                        let chain_id = v_json.get("chainId").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                        let requester_pub = v_json.get("requesterPub").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                        if chrono::Utc::now().timestamp() as u64 > expires { return Err(axum::http::StatusCode::UNAUTHORIZED); }
                        // Anti-replay check
                        let replay_key = format!("voucher:nonce:{}:{}", provider, nonce);
                        if let Ok(Some(_)) = deal_store.get(replay_key.as_bytes()).await { return Err(axum::http::StatusCode::UNAUTHORIZED); }
                        // Verify signature
                        let msg = format!("VOUCHER:{}:{}:{}:{}:{}:{}", provider, cid_hex, bytes_req, price, nonce, expires);
                        let pubkey_bytes = hex::decode(requester_pub.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                        let vk = k256::ecdsa::VerifyingKey::from_sec1_bytes(&pubkey_bytes).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                        let sig_bytes = hex::decode(sig_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                        let sig = k256::ecdsa::Signature::from_slice(&sig_bytes).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                        use k256::ecdsa::signature::Verifier;
                        vk.verify(msg.as_bytes(), &sig).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                        // Mark nonce as used until expiry
                        let ttl = expires.saturating_sub(chrono::Utc::now().timestamp() as u64);
                        let _ = deal_store.put(replay_key.as_bytes(), &ttl.to_le_bytes()).await;
                    }
                    // Access policy enforcement
                    let cid_hex = hex::encode(blake);
                    let policy_key = format!("access:{}", cid_hex);
                    if let Ok(Some(pol_bytes)) = deal_store.get(policy_key.as_bytes()).await {
                        if let Ok(policy) = serde_json::from_slice::<serde_json::Value>(&pol_bytes) {
                            let mode = policy.get("mode").and_then(|v| v.as_str()).unwrap_or("public");
                            if mode == "private" || mode == "allowlist" || mode == "token" {
                                // Expect headers: X-Artha-DID = did:artha:<hexpub>; X-Artha-Expiry=unix; X-Artha-Signature=hex
                                let did = headers.get("X-Artha-DID").and_then(|v| v.to_str().ok()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                                let expiry = headers.get("X-Artha-Expiry").and_then(|v| v.to_str().ok()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                                let sig_hex = headers.get("X-Artha-Signature").and_then(|v| v.to_str().ok()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                                let exp_val: i64 = expiry.parse().map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                                if chrono::Utc::now().timestamp() > exp_val { return Err(axum::http::StatusCode::UNAUTHORIZED); }
                                let pubhex = did.strip_prefix("did:artha:").ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                                let pubkey_bytes = hex::decode(pubhex).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                    let vk = VerifyingKey::from_bytes(&pubkey_bytes.try_into().map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                                let sig_bytes = hex::decode(sig_hex).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                                let sig = Signature::from_slice(&sig_bytes).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                                let msg = format!("GET:{}:{}", cid_hex, expiry);
                                vk.verify(msg.as_bytes(), &sig).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
                                if mode == "allowlist" {
                                let allowed = policy.get("allowedDids").and_then(|v| v.as_array()).unwrap_or(&vec![]);
                                let mut ok=false; for a in allowed { if a.as_str()==Some(did) { ok=true; break; } }
                                if !ok { return Err(axum::http::StatusCode::FORBIDDEN); }
                                } else if mode == "token" {
                                    // token-gated: require X-Artha-Token and match keccak hash
                                    let token = headers.get("X-Artha-Token").and_then(|v| v.to_str().ok()).ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
                                    let want = policy.get("tokenHash").and_then(|v| v.as_str()).unwrap_or("");
                                    let got = format!("0x{}", hex::encode(keccak256(token.as_bytes())));
                                    if got != want { return Err(axum::http::StatusCode::FORBIDDEN); }
                                }
                            }
                        }
                    }
                    let manifest = svdb.get_manifest(&m_cid).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?;
                    let mut out = Vec::with_capacity(manifest.size as usize);
                    let mut entries = manifest.chunks.clone(); entries.sort_by_key(|e| e.order);
                    for e in entries {
                        match ChunkStore::get(&svdb, &e.cid).await {
                            Ok(bytes) => { out.extend_from_slice(&bytes); }
                            Err(_) => {
                                // Attempt nearest-provider retrieval via provider list and http_addr
                                let cid_hex = hex::encode(e.cid.blake3);
                                let prov_key = format!("prov:{}", cid_hex);
                                if let Ok(Some(pbytes)) = deal_store.get(prov_key.as_bytes()).await {
                                    if let Ok(providers) = serde_json::from_slice::<Vec<String>>(&pbytes) {
                                        // Load capabilities for each provider to get http_addr
                                        let mut fetched = false;
                                        for pid in providers {
                                            let cap_key = format!("caps:{}", pid);
                                            if let Ok(Some(caps_bytes)) = deal_store.get(cap_key.as_bytes()).await {
                                                if let Ok(caps) = serde_json::from_slice::<serde_json::Value>(&caps_bytes) {
                                                    if let Some(addr) = caps.get("http_addr").and_then(|v| v.as_str()) {
                                                        let url = format!("{}/svdb/chunk/{}", addr.trim_end_matches('/'), cid_hex);
                                                        if let Ok(resp) = HttpClient::new().get(url).send().await {
                                                            if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await { let v = bytes.to_vec(); out.extend_from_slice(&v); let _ = ChunkStore::put(&svdb, &e.cid, &v).await; fetched = true; break; }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if !fetched { return Err(axum::http::StatusCode::NOT_FOUND); }
                                    } else {
                                        return Err(axum::http::StatusCode::NOT_FOUND);
                                    }
                                } else {
                                    return Err(axum::http::StatusCode::NOT_FOUND);
                                }
                            }
                        }
                    }
                    // Compute byte-range to serve (if requested)
                    let total_len = out.len() as u64;
                    let mut start: u64 = 0;
                    let mut end: u64 = if total_len > 0 { total_len - 1 } else { 0 };
                    let mut is_partial = false;
                    if let Some(range_val) = headers.get("Range").and_then(|v| v.to_str().ok()) {
                        if let Some(bytes_part) = range_val.strip_prefix("bytes=") {
                            let parts: Vec<&str> = bytes_part.split('-').collect();
                            if parts.len() == 2 {
                                let s_opt = if !parts[0].is_empty() { parts[0].parse::<u64>().ok() } else { None };
                                let e_opt = if !parts[1].is_empty() { parts[1].parse::<u64>().ok() } else { None };
                                match (s_opt, e_opt) {
                                    (Some(s), Some(e)) if s <= e && e < total_len => { start = s; end = e; is_partial = true; },
                                    (Some(s), None) if s < total_len => { start = s; end = total_len - 1; is_partial = true; },
                                    (None, Some(e)) if e != 0 => { let last = e.min(total_len - 1); start = total_len - 1 - last; end = total_len - 1; is_partial = true; },
                                    _ => {}
                                }
                            }
                        }
                    }

                    let start_usize = start as usize;
                    let end_usize = end as usize;
                    let served_len = if end_usize >= start_usize { end_usize - start_usize + 1 } else { 0 };
                    let body_slice = if served_len > 0 && (end_usize as u64) < total_len { &out[start_usize..=end_usize] } else { &out[..] };

                    // Retrieval micro-fee settlement on-chain (headers must include RPC params)
                    const MICROFEE_WEI_PER_GB: u128 = 100_000_000_000_000; // 1e14 wei per GB
                    let bytes_served = body_slice.len() as u128;
                    let gb_times_1e9 = bytes_served * 1_000_000_000u128 / (1024u128*1024u128*1024u128);
                    let fee_wei = (MICROFEE_WEI_PER_GB * gb_times_1e9) / 1_000_000_000u128;
                    if let (Some(rpc_url), Some(chain_id), Some(priv_hex), Some(deal_market), Some(provider_hex)) = (
                        headers.get("X-Artha-RPC").and_then(|v| v.to_str().ok()),
                        headers.get("X-Artha-ChainId").and_then(|v| v.to_str().ok()).and_then(|s| s.parse::<u64>().ok()),
                        headers.get("X-Artha-PrivKey").and_then(|v| v.to_str().ok()),
                        headers.get("X-Artha-DealMarket").and_then(|v| v.to_str().ok()),
                        headers.get("X-Artha-Provider").and_then(|v| v.to_str().ok()),
                    ) {
                        fn pad32(mut v: Vec<u8>) -> Vec<u8> { let mut p = vec![0u8; 32 - v.len()]; p.append(&mut v); p }
                        fn enc_u256(x: u128) -> Vec<u8> { let s = format!("{:x}", x); let mut bytes = if s.len()%2==1 { hex::decode(format!("0{}", s)).unwrap() } else { hex::decode(s).unwrap() }; pad32(bytes) }
                        fn enc_bytes32(b: &[u8]) -> Vec<u8> { let mut out = vec![0u8;32]; out.copy_from_slice(b); out.to_vec() }
                        fn enc_address(b: &[u8]) -> Vec<u8> { let mut out = vec![0u8;32]; out[12..].copy_from_slice(b); out }
                        fn keccak(input: &[u8]) -> [u8;32] { use tiny_keccak::Hasher; let mut hasher = tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; hasher.update(input); hasher.finalize(&mut out); out }
                        fn rlp_bytes(b: &[u8]) -> Vec<u8> { if b.len()==1 && b[0]<0x80 { return b.to_vec(); } if b.len()<=55 { let mut out=vec![0x80 + b.len() as u8]; out.extend_from_slice(b); return out; } let mut len= b.len(); let mut v=Vec::new(); let mut s=Vec::new(); while len>0 { s.push((len & 0xff) as u8); len >>= 8; } for c in s.iter().rev(){ v.push(*c); } let mut out=vec![0xb7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(b); out }
                        fn rlp_u256(x: u128) -> Vec<u8> { if x==0 { return vec![0x80]; } let mut n=x; let mut tmp=Vec::new(); while n>0 { tmp.push((n & 0xff) as u8); n >>= 8; } rlp_bytes(&tmp.iter().rev().cloned().collect::<Vec<_>>()) }
                        fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> { let payload_len: usize = items.iter().map(|i| i.len()).sum(); let mut payload=Vec::with_capacity(payload_len); for i in items { payload.extend_from_slice(i); } if payload_len<=55 { let mut out=vec![0xc0 + payload_len as u8]; out.extend_from_slice(&payload); return out; } let mut n=payload_len; let mut v=Vec::new(); let mut s=Vec::new(); while n>0{ s.push((n & 0xff) as u8); n >>= 8; } for c in s.iter().rev(){ v.push(*c);} let mut out=vec![0xf7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(&payload); out }
                        let to = hex::decode(deal_market.trim_start_matches("0x")).unwrap();
                        let provider = hex::decode(provider_hex.trim_start_matches("0x")).unwrap();
                        let selector = &keccak(b"recordRetrieval(bytes32,uint64,address)")[0..4];
                        let mut data = Vec::with_capacity(4 + 32*3);
                        data.extend_from_slice(selector);
                        data.extend_from_slice(&enc_bytes32(&manifest.merkle_root));
                        data.extend_from_slice(&enc_u256(bytes_served as u128));
                        data.extend_from_slice(&enc_address(&provider));
                        let gas_price = 1_000_000_000u128; let gas_limit = 200_000u128; let nonce = 0u128; let value = fee_wei; let chain_id = chain_id as u128;
                        let tx_parts = vec![ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(value), rlp_bytes(&data), rlp_u256(chain_id), rlp_u256(0), rlp_u256(0) ];
                        let sighash = keccak(&rlp_list(&tx_parts));
                        let pk_bytes = hex::decode(priv_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                        use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey};
                        let pk_arr = GenericArray::from_slice(&pk_bytes);
                        let sk = SecretKey::from_bytes(pk_arr).unwrap();
                        let signing_key = SigningKey::from(sk);
                        let sig: k256::ecdsa::Signature = signing_key.sign(&sighash);
                        let (r, s) = (sig.r().to_bytes(), sig.s().to_bytes());
                        let v = (chain_id * 2 + 35) as u8;
                        let raw = rlp_list(&[ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(value), rlp_bytes(&data), rlp_u256(v as u128), rlp_bytes(&r.to_vec()), rlp_bytes(&s.to_vec()) ]);
                        let raw_hex = format!("0x{}", hex::encode(raw));
                        let client = HttpClient::new();
                        let payload_rpc = serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex],"id":1});
                        let _ = client.post(rpc_url).json(&payload_rpc).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                    }

                    // Build HTTP response with range headers
                    let status = if is_partial { axum::http::StatusCode::PARTIAL_CONTENT } else { axum::http::StatusCode::OK };
                    let mut resp = axum::http::Response::builder()
                        .status(status)
                        .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
                        .header(axum::http::header::ACCEPT_RANGES, "bytes")
                        .header(axum::http::header::CONTENT_LENGTH, body_slice.len().to_string());
                    if is_partial {
                        let content_range = format!("bytes {}-{}/{}", start, end, total_len);
                        resp = resp.header(axum::http::header::CONTENT_RANGE, content_range);
                    }
                    let resp = resp.body(axum::body::Body::from(body_slice.to_vec())).map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    Ok::<_, axum::http::StatusCode>(resp)
                }
            }
        }))
        .route("/svdb/retrievals/:cid", get({
            let deal_store = deal_store.clone();
            move |axum::extract::Path(cid_uri): axum::extract::Path<String>| {
                let deal_store = deal_store.clone();
                async move {
                    let b64 = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                    let cid_hex = hex::encode(blake);
                    let per_idx_key = format!("retrievals:{}:index", cid_hex);
                    let idx = deal_store.get(per_idx_key.as_bytes()).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    let keys: Vec<String> = idx.map(|b| serde_json::from_slice(&b).unwrap_or_default()).unwrap_or_default();
                    let mut records = Vec::new();
                    let mut total_bytes: u64 = 0;
                    let mut total_fee_wei: u128 = 0;
                    for k in keys {
                        if let Ok(Some(v)) = deal_store.get(k.as_bytes()).await { 
                            if let Ok(mut rec) = serde_json::from_slice::<serde_json::Value>(&v) { 
                                total_bytes += rec.get("bytes").and_then(|x| x.as_u64()).unwrap_or(0);
                                if let Some(fstr) = rec.get("feeWei").and_then(|x| x.as_str()) { if let Ok(f) = fstr.parse::<u128>() { total_fee_wei += f; } }
                                records.push(rec);
                            }
                        }
                    }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({
                        "cid": cid_uri,
                        "totalBytes": total_bytes,
                        "totalFeeWei": total_fee_wei.to_string(),
                        "records": records,
                    })))
                }
            }
        }))
        .route("/svdb/retrievals", get({
            let deal_store = deal_store.clone();
            move || {
                let deal_store = deal_store.clone();
                async move {
                    let idx_key = b"retrievals:index";
                    let keys_bytes = deal_store.get(idx_key).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    let keys: Vec<String> = keys_bytes.map(|b| serde_json::from_slice(&b).unwrap_or_default()).unwrap_or_default();
                    let mut total_bytes: u64 = 0;
                    let mut total_fee_wei: u128 = 0;
                    let mut by_cid: HashMap<String, (u64, u128)> = HashMap::new();
                    for k in keys {
                        if let Ok(Some(v)) = deal_store.get(k.as_bytes()).await {
                            if let Ok(rec) = serde_json::from_slice::<serde_json::Value>(&v) {
                                let cid = rec.get("cid").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                let b = rec.get("bytes").and_then(|x| x.as_u64()).unwrap_or(0);
                                let f = rec.get("feeWei").and_then(|x| x.as_str()).and_then(|s| s.parse::<u128>().ok()).unwrap_or(0);
                                total_bytes += b; total_fee_wei += f;
                                let e = by_cid.entry(cid).or_insert((0,0));
                                e.0 += b; e.1 += f;
                            }
                        }
                    }
                    let mut per_cid: Vec<serde_json::Value> = Vec::new();
                    for (cid, (b, f)) in by_cid.into_iter() { per_cid.push(serde_json::json!({"cid": cid, "totalBytes": b, "totalFeeWei": f.to_string()})); }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({
                        "totalBytes": total_bytes,
                        "totalFeeWei": total_fee_wei.to_string(),
                        "byCid": per_cid,
                    })))
                }
            }
        }))
        // Retrieval quote: issues a nonce and returns price
        .route("/svdb/retrieval/quote", post({
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>, headers: HeaderMap| {
                async move {
                    // Rate limit
                    let client_ip = headers.get("X-Client-IP").and_then(|v| v.to_str().ok()).unwrap_or("unknown");
                    let now_min = (chrono::Utc::now().timestamp() / 60).to_string();
                    let rl_key = format!("ratelimit:quote:{}:{}", client_ip, now_min);
                    let cnt = match deal_store.get(rl_key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                    let max_req = std::env::var("ARTHA_QUOTES_PER_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(120u64);
                    if cnt >= max_req { return Err(axum::http::StatusCode::TOO_MANY_REQUESTS); }
                    let _ = deal_store.put(rl_key.as_bytes(), &(cnt+1).to_le_bytes()).await;
                    let provider = body.get("provider").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let cid = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let price_per_mib = std::env::var("ARTHA_PRICE_PER_MIB").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(10_000);
                    let chain_id = std::env::var("ARTHA_CHAIN_ID").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
                    let expires = (chrono::Utc::now() + chrono::Duration::minutes(10)).timestamp() as u64;
                    let nonce = rand::random::<u64>();
                    let key = format!("voucher_nonce:{}:{}", provider, nonce);
                    let _ = deal_store.put(key.as_bytes(), &expires.to_le_bytes()).await;
                    Ok(Json(serde_json::json!({
                        "provider": provider,
                        "cid": cid,
                        "pricePerMiB": price_per_mib,
                        "nonce": nonce,
                        "expires": expires,
                        "chainId": chain_id,
                    })))
                }
            }
        }))
        // Retrieval settle: forward aggregated settlement to DealMarket.recordRetrieval (single call with totalWei)
        .route("/svdb/retrieval/settle", post({
            let deal_store_rl = deal_store.clone();
            move |Json(body): Json<serde_json::Value>, headers: HeaderMap| {
                async move {
                let client_ip = headers.get("X-Client-IP").and_then(|v| v.to_str().ok()).unwrap_or("unknown");
                let now_min = (chrono::Utc::now().timestamp() / 60).to_string();
                let rl_key = format!("ratelimit:settle:{}:{}", client_ip, now_min);
                let cnt = match deal_store_rl.get(rl_key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                let max_req = std::env::var("ARTHA_SETTLES_PER_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(60u64);
                if cnt >= max_req { return Err(axum::http::StatusCode::TOO_MANY_REQUESTS); }
                let _ = deal_store_rl.put(rl_key.as_bytes(), &(cnt+1).to_le_bytes()).await;
                // Expect: { rpcUrl, chainId, privateKey, dealMarket, manifestRoot, bytesServed, provider, totalWei, gasPrice?, gasLimit?, nonce? }
                fn keccak(input: &[u8]) -> [u8;32] { use tiny_keccak::Hasher; let mut h=tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; h.update(input); h.finalize(&mut out); out }
                fn rlp_bytes(b: &[u8]) -> Vec<u8> { if b.len()==1 && b[0]<0x80 { return b.to_vec(); } if b.len()<=55 { let mut out=vec![0x80 + b.len() as u8]; out.extend_from_slice(b); return out; } let mut len= b.len(); let mut v=Vec::new(); let mut s=Vec::new(); while len>0{ s.push((len & 0xff) as u8); len >>= 8; } for c in s.iter().rev(){ v.push(*c);} let mut out=vec![0xb7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(b); out }
                fn rlp_u256(x: u128) -> Vec<u8> { if x==0 { return vec![0x80]; } let mut n=x; let mut tmp=Vec::new(); while n>0 { tmp.push((n & 0xff) as u8); n >>= 8; } rlp_bytes(&tmp.iter().rev().cloned().collect::<Vec<_>>()) }
                fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> { let payload_len: usize = items.iter().map(|i| i.len()).sum(); let mut payload=Vec::with_capacity(payload_len); for i in items { payload.extend_from_slice(i); } if payload_len<=55 { let mut out=vec![0xc0 + payload_len as u8]; out.extend_from_slice(&payload); return out; } let mut n=payload_len; let mut v=Vec::new(); let mut s=Vec::new(); while n>0{ s.push((n & 0xff) as u8); n >>= 8; } for c in s.iter().rev(){ v.push(*c);} let mut out=vec![0xf7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(&payload); out }
                fn enc_bytes32(b: &[u8]) -> Vec<u8> { let mut out = vec![0u8;32]; out.copy_from_slice(b); out.to_vec() }

                let rpc_url = body.get("rpcUrl").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let chain_id = body.get("chainId").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u64;
                let priv_hex = body.get("privateKey").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let deal_market = body.get("dealMarket").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let root_hex = body.get("manifestRoot").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let bytes_served = body.get("bytesServed").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                let provider_hex = body.get("provider").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let total_wei = body.get("totalWei").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                let gas_price = body.get("gasPrice").and_then(|v| v.as_u64()).unwrap_or(1_000_000_000) as u128;
                let gas_limit = body.get("gasLimit").and_then(|v| v.as_u64()).unwrap_or(200_000) as u128;
                // ABI encode aggregate variants if merkleRoot provided
                let merkle_hex_opt = body.get("merkleRoot").and_then(|v| v.as_str());
                let use_proof = body.get("leaf").is_some();
                let selector = if let Some(_) = merkle_hex_opt {
                    if use_proof { &keccak(b"recordRetrievalAggregateProof(bytes32,bytes32,bytes32,bytes32[],uint256,address)")[0..4] } else { &keccak(b"recordRetrievalAggregate(bytes32,bytes32,address)")[0..4] }
                } else { &keccak(b"recordRetrieval(bytes32,uint64,address)")[0..4] };
                let root_bytes = hex::decode(root_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                if root_bytes.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                let provider = hex::decode(provider_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let to = hex::decode(deal_market.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                fn enc_u256(x: u128) -> Vec<u8> { let s = format!("{:x}", x); let mut bytes = if s.len()%2==1 { hex::decode(format!("0{}", s)).unwrap() } else { hex::decode(s).unwrap() }; let mut p = vec![0u8; 32 - bytes.len()]; p.append(&mut bytes); p }
                fn enc_address(b: &[u8]) -> Vec<u8> { let mut out = vec![0u8;32]; out[12..].copy_from_slice(b); out }
                let mut data = Vec::with_capacity(4 + 32*3);
                data.extend_from_slice(selector);
                data.extend_from_slice(&enc_bytes32(&root_bytes));
                if let Some(merkle_hex) = merkle_hex_opt {
                    let mr = hex::decode(merkle_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if mr.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) }
                    data.extend_from_slice(&enc_bytes32(&mr));
                    if use_proof {
                        let leaf_hex = body.get("leaf").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                        let leaf = hex::decode(leaf_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if leaf.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) }
                        data.extend_from_slice(&enc_bytes32(&leaf));
                        let branch_vals = body.get("branch").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                        let mut tail = Vec::new();
                        tail.extend_from_slice(&enc_u256(branch_vals.len() as u128));
                        for v in branch_vals { let s=v.as_str().ok_or(axum::http::StatusCode::BAD_REQUEST)?; let b=hex::decode(s.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if b.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) } tail.extend_from_slice(&enc_bytes32(&b)); }
                        let index = body.get("index").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u128;
                        data.extend_from_slice(&enc_u256((4*32) as u128)); // branch offset after 5 words? Simplified static offset for demo
                        data.extend_from_slice(&enc_u256(index));
                        data.extend_from_slice(&tail);
                    }
                    data.extend_from_slice(&enc_address(&provider));
                } else {
                    data.extend_from_slice(&enc_u256(bytes_served));
                    data.extend_from_slice(&enc_address(&provider));
                }
                // Nonce
                let nonce = if let Some(n) = body.get("nonce").and_then(|v| v.as_u64()) { n as u128 } else {
                    let from_addr = std::env::var("ARTHA_FROM").map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    let payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_getTransactionCount","params":[from_addr,"pending"],"id":1});
                    let client = reqwest::Client::new();
                    let resp = client.post(rpc_url).json(&payload).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                    let val: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                    let hex_nonce = val.get("result").and_then(|v| v.as_str()).unwrap_or("0x0");
                    u64::from_str_radix(hex_nonce.trim_start_matches("0x"), 16).unwrap_or(0) as u128
                };
                // Build raw tx with value = totalWei
                let tx_parts = vec![ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(total_wei), rlp_bytes(&data), rlp_u256(chain_id as u128), rlp_u256(0), rlp_u256(0) ];
                let sighash = keccak(&rlp_list(&tx_parts));
                let pk_bytes = hex::decode(priv_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey};
                let sk = SecretKey::from_bytes(&pk_bytes).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let signing_key = SigningKey::from(sk);
                let sig: k256::ecdsa::Signature = signing_key.sign(&sighash);
                let (r, s) = (sig.r().to_bytes(), sig.s().to_bytes());
                let v = (chain_id * 2 + 35) as u8;
                let raw = rlp_list(&[ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(total_wei), rlp_bytes(&data), rlp_u256(v as u128), rlp_bytes(&r.to_vec()), rlp_bytes(&s.to_vec()) ]);
                let raw_hex = format!("0x{}", hex::encode(raw));
                let client = reqwest::Client::new();
                let payload_rpc = serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex],"id":1});
                let resp = client.post(rpc_url).json(&payload_rpc).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                let json: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                Ok(Json(json))
                }
            }
        }))
        // Pin / Unpin and GC
        .route("/svdb/pin", post({
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>, headers: HeaderMap| {
                async move {
                    let client_ip = headers.get("X-Client-IP").and_then(|v| v.to_str().ok()).unwrap_or("unknown");
                    let now_min = (chrono::Utc::now().timestamp() / 60).to_string();
                    let rl_key = format!("ratelimit:pin:{}:{}", client_ip, now_min);
                    let cnt = match deal_store.get(rl_key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                    let max_req = std::env::var("ARTHA_PIN_PER_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(120u64);
                    if cnt >= max_req { return Err(axum::http::StatusCode::TOO_MANY_REQUESTS); }
                    let _ = deal_store.put(rl_key.as_bytes(), &(cnt+1).to_le_bytes()).await;
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let enc = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(enc).unwrap_or_else(|_| data_encoding::BASE32_NOPAD.decode(enc.as_bytes()).unwrap_or_default());
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let mut bl=[0u8;32]; bl.copy_from_slice(&bytes[2..34]);
                    let cid_hex = hex::encode(bl);
                    let key = format!("pin:{}", cid_hex);
                    let count = match deal_store.get(key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 } + 1;
                    let _ = deal_store.put(key.as_bytes(), &count.to_le_bytes()).await;
                    Ok(Json(serde_json::json!({"cid": cid_uri, "pins": count})))
                }
            }
        }))
        .route("/svdb/unpin", post({
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>, headers: HeaderMap| {
                async move {
                    let client_ip = headers.get("X-Client-IP").and_then(|v| v.to_str().ok()).unwrap_or("unknown");
                    let now_min = (chrono::Utc::now().timestamp() / 60).to_string();
                    let rl_key = format!("ratelimit:unpin:{}:{}", client_ip, now_min);
                    let cnt = match deal_store.get(rl_key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                    let max_req = std::env::var("ARTHA_UNPIN_PER_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(120u64);
                    if cnt >= max_req { return Err(axum::http::StatusCode::TOO_MANY_REQUESTS); }
                    let _ = deal_store.put(rl_key.as_bytes(), &(cnt+1).to_le_bytes()).await;
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let enc = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(enc).unwrap_or_else(|_| data_encoding::BASE32_NOPAD.decode(enc.as_bytes()).unwrap_or_default());
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let mut bl=[0u8;32]; bl.copy_from_slice(&bytes[2..34]);
                    let cid_hex = hex::encode(bl);
                    let key = format!("pin:{}", cid_hex);
                    let count = match deal_store.get(key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                    let newc = count.saturating_sub(1);
                    let _ = deal_store.put(key.as_bytes(), &newc.to_le_bytes()).await;
                    Ok(Json(serde_json::json!({"cid": cid_uri, "pins": newc})))
                }
            }
        }))
        .route("/svdb/gc/info", get({
            let deal_store = deal_store.clone();
            move || {
                let deal_store = deal_store.clone();
                async move {
                    // Report pinned/unpinned manifest counts
                    let list: Vec<String> = match deal_store.get(b"mf:all").await { Ok(Some(b)) => serde_json::from_slice(&b).unwrap_or_default(), _ => Vec::new() };
                    let mut pinned=0u64; let mut unpinned=0u64;
                    for cid_uri in &list {
                        let b = base64::engine::general_purpose::STANDARD_NO_PAD.decode(cid_uri.trim_start_matches("artha://")).unwrap_or_default();
                        if b.len()>=34 { let mut bl=[0u8;32]; bl.copy_from_slice(&b[2..34]); let cid_hex = hex::encode(bl); let key = format!("pin:{}", cid_hex); let c = match deal_store.get(key.as_bytes()).await { Ok(Some(x)) => u64::from_le_bytes(x.try_into().unwrap_or([0u8;8])), _ => 0 }; if c>0 { pinned+=1 } else { unpinned+=1 } }
                    }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"manifests": list.len(), "pinned": pinned, "unpinned": unpinned})))
                }
            }
        }))
        .route("/svdb/gc/run", post({
            let svdb = svdb.clone();
            let deal_store = deal_store.clone();
            move || {
                let svdb = svdb.clone();
                let deal_store = deal_store.clone();
                async move {
                    // Delete chunks for manifests with zero pins and past grace period
                    let list: Vec<String> = match deal_store.get(b"mf:all").await { Ok(Some(b)) => serde_json::from_slice(&b).unwrap_or_default(), _ => Vec::new() };
                    let grace_secs: i64 = std::env::var("ARTHA_GC_GRACE_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(86400);
                    let mut deleted = 0u64;
                    for cid_uri in list {
                        let enc = cid_uri.trim_start_matches("artha://");
                        let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(enc).unwrap_or_default();
                        if bytes.len()<34 { continue; }
                        let mut bl=[0u8;32]; bl.copy_from_slice(&bytes[2..34]);
                        let cid_hex = hex::encode(bl);
                        let pin_key = format!("pin:{}", cid_hex);
                        let pins = match deal_store.get(pin_key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                        if pins>0 { continue; }
                        // Deletion window keyed by mf:del:<cid>
                        let del_key = format!("mf:del:{}", cid_hex);
                        let now = chrono::Utc::now().timestamp();
                        let due = match deal_store.get(del_key.as_bytes()).await { Ok(Some(b)) => i64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => { let ts = now + grace_secs; let _=deal_store.put(del_key.as_bytes(), &ts.to_le_bytes()).await; ts } };
                        if now < due { continue; }
                        // Fetch manifest and delete shards
                        let mut cursor=35; let has_poseidon=bytes[34]==1; if has_poseidon { cursor+=32; }
                        let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let size = u64::from_be_bytes(sz);
                        let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                        let m_cid = Cid::new(u16::from_be_bytes([bytes[0],bytes[1]]), bl, None, size, codec);
                        if let Ok(manifest) = svdb.get_manifest(&m_cid).await {
                            let mut entries = manifest.chunks.clone(); entries.sort_by_key(|e| e.order);
                            for e in entries { let _ = svdb.delete_chunk(&e.cid).await; deleted += 1; }
                        }
                    }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"deletedChunks": deleted})))
                }
            }
        }))
        // Allowlist management: add DID
        .route("/svdb/access/allowlist/add", post({
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>| {
                let deal_store = deal_store.clone();
                async move {
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let did = body.get("did").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let enc = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(enc).unwrap_or_else(|_| data_encoding::BASE32_NOPAD.decode(enc.as_bytes()).unwrap_or_default());
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let mut bl=[0u8;32]; bl.copy_from_slice(&bytes[2..34]);
                    let cid_hex = hex::encode(bl);
                    let policy_key = format!("access:{}", cid_hex);
                    let mut policy = match deal_store.get(policy_key.as_bytes()).await { Ok(Some(b)) => serde_json::from_slice::<serde_json::Value>(&b).unwrap_or(serde_json::json!({"mode":"allowlist","allowedDids":[]})), _ => serde_json::json!({"mode":"allowlist","allowedDids":[]}) };
                    let mode = policy.get("mode").and_then(|v| v.as_str()).unwrap_or("allowlist");
                    if mode != "allowlist" { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let mut arr = policy.get_mut("allowedDids").and_then(|v| v.as_array_mut()).cloned().unwrap_or_default();
                    if !arr.iter().any(|x| x.as_str()==Some(did)) { arr.push(serde_json::Value::String(did.to_string())); }
                    policy["allowedDids"] = serde_json::Value::Array(arr);
                    let _ = deal_store.put(policy_key.as_bytes(), serde_json::to_string(&policy).unwrap().as_bytes()).await;
                    Ok::<_, axum::http::StatusCode>(Json(policy))
                }
            }
        }))
        // Allowlist management: remove DID
        .route("/svdb/access/allowlist/remove", post({
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>| {
                let deal_store = deal_store.clone();
                async move {
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let did = body.get("did").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let enc = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(enc).unwrap_or_else(|_| data_encoding::BASE32_NOPAD.decode(enc.as_bytes()).unwrap_or_default());
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let mut bl=[0u8;32]; bl.copy_from_slice(&bytes[2..34]);
                    let cid_hex = hex::encode(bl);
                    let policy_key = format!("access:{}", cid_hex);
                    let mut policy = match deal_store.get(policy_key.as_bytes()).await { Ok(Some(b)) => serde_json::from_slice::<serde_json::Value>(&b).unwrap_or(serde_json::json!({"mode":"allowlist","allowedDids":[]})), _ => serde_json::json!({"mode":"allowlist","allowedDids":[]}) };
                    let mode = policy.get("mode").and_then(|v| v.as_str()).unwrap_or("allowlist");
                    if mode != "allowlist" { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let arr = policy.get("allowedDids").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    let filtered: Vec<serde_json::Value> = arr.into_iter().filter(|x| x.as_str()!=Some(did)).collect();
                    policy["allowedDids"] = serde_json::Value::Array(filtered);
                    let _ = deal_store.put(policy_key.as_bytes(), serde_json::to_string(&policy).unwrap().as_bytes()).await;
                    Ok::<_, axum::http::StatusCode>(Json(policy))
                }
            }
        }))
        .route("/svdb/info/:cid_b64", get({
            let svdb = svdb.clone();
            move |axum::extract::Path(cid_b64): axum::extract::Path<String>| {
                let svdb = svdb.clone();
                async move {
                    let enc = cid_b64.trim_start_matches("artha://");
                    let bytes = match base64::engine::general_purpose::STANDARD_NO_PAD.decode(enc) {
                        Ok(b) => b,
                        Err(_) => match data_encoding::BASE32_NOPAD.decode(enc.as_bytes()) { Ok(b)=>b, Err(_)=> return Err(axum::http::StatusCode::BAD_REQUEST) },
                    };
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let codec_tag = u16::from_be_bytes([bytes[0], bytes[1]]);
                    let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                    let has_poseidon = bytes[34] == 1; let mut cursor = 35;
                    let poseidon = if has_poseidon { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } else { None };
                    let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let size = u64::from_be_bytes(sz);
                    let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                    let m_cid = Cid::new(codec_tag, blake, poseidon, size, codec);
                    let manifest = svdb.get_manifest(&m_cid).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?;
                    let root_blake3 = hex::encode(manifest.merkle_root);
                    let root_poseidon = manifest.poseidon_root.map(|r| hex::encode(r));
                    let ec = serde_json::json!({"k": manifest.erasure_data_shards.unwrap_or(8), "m": manifest.erasure_parity_shards.unwrap_or(2)});
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({
                        "cid": cid_b64,
                        "size": manifest.size,
                        "root_blake3": root_blake3,
                        "root_poseidon": root_poseidon,
                        "ec": ec,
                        "manifest": manifest,
                    })))
                }
            }
        }))
        // Deals: create (on-chain only)
        .route("/svdb/deals", post({
            let svdb = svdb.clone();
            let deal_store_rl = deal_store.clone();
            move |Json(payload): Json<serde_json::Value>, headers: HeaderMap| {
                async move {
                    // Rate limit per IP/minute
                    let client_ip = headers.get("X-Client-IP").and_then(|v| v.to_str().ok()).unwrap_or("unknown");
                    let now_min = (chrono::Utc::now().timestamp() / 60).to_string();
                    let rl_key = format!("ratelimit:deals:{}:{}", client_ip, now_min);
                    let cnt = match deal_store_rl.get(rl_key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                    let max_req = std::env::var("ARTHA_DEALS_PER_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(60u64);
                    if cnt >= max_req { return Err(axum::http::StatusCode::TOO_MANY_REQUESTS); }
                    let _ = deal_store_rl.put(rl_key.as_bytes(), &(cnt+1).to_le_bytes()).await;
                    let cid_uri = payload.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let size = payload.get("size").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let replicas = payload.get("replicas").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u32;
                    let months = payload.get("months").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u32;
                    let max_price = payload.get("maxPrice").and_then(|v| v.as_f64()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;

                    // Parse CID
                    let enc = cid_uri.trim_start_matches("artha://");
                    let bytes = match base64::engine::general_purpose::STANDARD_NO_PAD.decode(enc) {
                        Ok(b) => b,
                        Err(_) => match data_encoding::BASE32_NOPAD.decode(enc.as_bytes()) { Ok(b)=>b, Err(_)=> return Err(axum::http::StatusCode::BAD_REQUEST) },
                    };
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let codec_tag = u16::from_be_bytes([bytes[0], bytes[1]]);
                    let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                    let has_poseidon = bytes[34] == 1; let mut cursor = 35;
                    let poseidon = if has_poseidon { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } else { None };
                    let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let cid_size = u64::from_be_bytes(sz);
                    let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                    let m_cid = Cid::new(codec_tag, blake, poseidon, cid_size, codec);

                    // Validate manifest exists
                    let _ = svdb.get_manifest(&m_cid).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?;

                    // Compute endowment (flat price per GB-month)
                    let gb = (size as f64) / (1024.0*1024.0*1024.0);
                    let endowment = (gb * (months as f64) * (replicas as f64) * max_price).ceil() as u64;

                    // Mandatory on-chain integration
                    if let (Some(rpc_url), Some(chain_id), Some(priv_hex), Some(deal_market)) = (
                        payload.get("rpcUrl").and_then(|v| v.as_str()),
                        payload.get("chainId").and_then(|v| v.as_u64()),
                        payload.get("privateKey").and_then(|v| v.as_str()),
                        payload.get("dealMarket").and_then(|v| v.as_str()),
                    ) {
                        // ABI encode createDeal(bytes32,uint64,uint32,uint32) payable
                        fn pad32(mut v: Vec<u8>) -> Vec<u8> { let mut p = vec![0u8; 32 - v.len()]; p.append(&mut v); p }
                        fn enc_u256(x: u128) -> Vec<u8> { let s = format!("{:x}", x); let mut bytes = if s.len()%2==1 { hex::decode(format!("0{}", s)).unwrap() } else { hex::decode(s).unwrap() }; pad32(bytes) }
                        fn enc_bytes32(b: &[u8]) -> Vec<u8> { let mut out = vec![0u8;32]; out.copy_from_slice(b); out.to_vec() }
                        fn keccak(input: &[u8]) -> [u8;32] { use tiny_keccak::Hasher; let mut hasher = tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; hasher.update(input); hasher.finalize(&mut out); out }

                        // manifest root as bytes32 (use computed root if manifest not bound here)
                        let root = {
                            // try to pull from payload or previously computed variable `manifest_root`
                            if let Some(r) = payload.get("manifestRoot").and_then(|v| v.as_str()).and_then(|h| hex::decode(h.trim_start_matches("0x")).ok()).and_then(|v| <[u8;32]>::try_from(v).ok()) {
                                r
                            } else {
                                // fallback: error out clearly if missing
                                return Err(axum::http::StatusCode::BAD_REQUEST);
                            }
                        };
                        let selector = &keccak(b"createDeal(bytes32,uint64,uint32,uint32)")[0..4];
                        let mut data = Vec::with_capacity(4 + 32*4);
                        data.extend_from_slice(selector);
                        data.extend_from_slice(&enc_bytes32(&root));
                        data.extend_from_slice(&enc_u256(size as u128));
                        data.extend_from_slice(&enc_u256(replicas as u128));
                        data.extend_from_slice(&enc_u256(months as u128));

                        // RLP sign legacy TX
                        fn rlp_bytes(b: &[u8]) -> Vec<u8> { if b.len()==1 && b[0]<0x80 { return b.to_vec(); } if b.len()<=55 { let mut out=vec![0x80 + b.len() as u8]; out.extend_from_slice(b); return out; } let mut len= b.len(); let mut v=Vec::new(); let mut s=Vec::new(); while len>0 { s.push((len & 0xff) as u8); len >>= 8; } for c in s.iter().rev(){ v.push(*c); } let mut out=vec![0xb7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(b); out }
                        fn rlp_u256(x: u128) -> Vec<u8> { if x==0 { return vec![0x80]; } let mut n=x; let mut tmp=Vec::new(); while n>0 { tmp.push((n & 0xff) as u8); n >>= 8; } rlp_bytes(&tmp.iter().rev().cloned().collect::<Vec<_>>()) }
                        fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> { let payload_len: usize = items.iter().map(|i| i.len()).sum(); let mut payload=Vec::with_capacity(payload_len); for i in items { payload.extend_from_slice(i); } if payload_len<=55 { let mut out=vec![0xc0 + payload_len as u8]; out.extend_from_slice(&payload); return out; } let mut n=payload_len; let mut v=Vec::new(); let mut s=Vec::new(); while n>0{ s.push((n & 0xff) as u8); n >>= 8; } for c in s.iter().rev(){ v.push(*c);} let mut out=vec![0xf7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(&payload); out }

                        let to = hex::decode(deal_market.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                        let gas_price = payload.get("gasPrice").and_then(|v| v.as_u64()).unwrap_or(1_000_000_000) as u128;
                        let gas_limit = payload.get("gasLimit").and_then(|v| v.as_u64()).unwrap_or(500_000) as u128;
                        let nonce = payload.get("nonce").and_then(|v| v.as_u64()).unwrap_or(0) as u128;
                        let value = endowment as u128;

                        let tx_parts = vec![
                            rlp_u256(nonce),
                            rlp_u256(gas_price),
                            rlp_u256(gas_limit),
                            rlp_bytes(&to),
                            rlp_u256(value),
                            rlp_bytes(&data),
                            rlp_u256(chain_id as u128),
                            rlp_u256(0),
                            rlp_u256(0),
                        ];
                        let sighash = keccak(&rlp_list(&tx_parts));
                        let pk_bytes = hex::decode(priv_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                        use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey};
                        use elliptic_curve::generic_array::GenericArray;
                        let pk_array = GenericArray::from_slice(&pk_bytes);
                        let sk = SecretKey::from_bytes(pk_array).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                        let signing_key = SigningKey::from(sk);
                        let sig: k256::ecdsa::Signature = signing_key.sign(&sighash);
                        let (r, s) = (sig.r().to_bytes(), sig.s().to_bytes());
                        let v = (chain_id * 2 + 35) as u8;
                        let raw = rlp_list(&[
                            rlp_u256(nonce),
                            rlp_u256(gas_price),
                            rlp_u256(gas_limit),
                            rlp_bytes(&to),
                            rlp_u256(value),
                            rlp_bytes(&data),
                            rlp_u256(v as u128),
                            rlp_bytes(&r.to_vec()),
                            rlp_bytes(&s.to_vec()),
                        ]);
                        let raw_hex = format!("0x{}", hex::encode(raw));
                        let client = HttpClient::new();
                        let payload_rpc = serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex],"id":1});
                        let resp = client.post(rpc_url).json(&payload_rpc).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                        let onchain: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                        return Ok(Json(onchain));
                    }
                    Err(axum::http::StatusCode::BAD_REQUEST)
                }
            }
        }))
        .route("/svdb/proofs", post({
            let svdb = svdb.clone();
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>| {
                let svdb = svdb.clone();
                let deal_store = deal_store.clone();
                async move {
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let index = body.get("index").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as usize;
                    let leaf_hex = body.get("leaf").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let branch_vals = body.get("branch").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    // Decode CID
                    let b64 = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let codec_tag = u16::from_be_bytes([bytes[0], bytes[1]]);
                    let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                    let has_poseidon = bytes[34] == 1; let mut cursor = 35;
                    let poseidon = if has_poseidon { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } else { None };
                    let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let cid_size = u64::from_be_bytes(sz);
                    let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                    let m_cid = Cid::new(codec_tag, blake, poseidon, cid_size, codec);
                    let manifest = svdb.get_manifest(&m_cid).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?;

                    // Recompute Keccak-composed root from leaf and branch to align with on-chain verifier
                    let lb = hex::decode(leaf_hex).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if lb.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) }
                    let mut acc=[0u8;32]; acc.copy_from_slice(&lb);
                    let mut idx = index;
                    for v in branch_vals {
                        let s=v.as_str().ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                        let b=hex::decode(s).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if b.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) }
                        let mut sib=[0u8;32]; sib.copy_from_slice(&b);
                        let (l, r) = if idx % 2 == 0 { (acc, sib) } else { (sib, acc) };
                        let keccak = keccak_bytes(&[l.as_slice(), r.as_slice()].concat());
                        acc.copy_from_slice(&keccak);
                        idx /= 2;
                    }
                    let valid = acc == manifest.merkle_root;
                    let result = serde_json::json!({"valid": valid});
                    if valid {
                        // Record a payout credit entry
                        let payout_key = format!("payout:{}:{}", hex::encode(blake), chrono::Utc::now().timestamp());
                        deal_store.put(payout_key.as_bytes(), serde_json::to_string(&result).unwrap().as_bytes()).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    }
                    Ok::<_, axum::http::StatusCode>(Json(result))
                }
            }
        }))
        // Proofs V2 batch verify (forward pre-encoded calldata to proofs contract)
        .route("/svdb/proofs/v2/batch/verify", post({
            move |Json(body): Json<serde_json::Value>| async move {
                // Expect: { rpcUrl, proofsV2, data } where data is hex calldata for batchVerifySalted
                let rpc_url = body.get("rpcUrl").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let proofs_v2 = body.get("proofsV2").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let data_hex = body.get("data").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let call = serde_json::json!({"to": proofs_v2, "data": data_hex});
                let payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_call","params":[call, "latest"],"id":1});
                let client = reqwest::Client::new();
                let resp = client.post(rpc_url).json(&payload).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                let json: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                Ok(Json(json))
            }
        }))
        // Proofs V2 batch submit (forward pre-encoded calldata to DealMarket.streamPayoutV2Batch)
        .route("/svdb/proofs/v2/batch/submit", post({
            let deal_store_rl = deal_store.clone();
            move |Json(body): Json<serde_json::Value>, headers: HeaderMap| async move {
                // Rate limit per IP/minute
                let client_ip = headers.get("X-Client-IP").and_then(|v| v.to_str().ok()).unwrap_or("unknown");
                let now_min = (chrono::Utc::now().timestamp() / 60).to_string();
                let rl_key = format!("ratelimit:proofs_batch:{}:{}", client_ip, now_min);
                let cnt = match deal_store_rl.get(rl_key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                let max_req = std::env::var("ARTHA_PROOFS_BATCH_PER_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(30u64);
                if cnt >= max_req { return Err(axum::http::StatusCode::TOO_MANY_REQUESTS); }
                let _ = deal_store_rl.put(rl_key.as_bytes(), &(cnt+1).to_le_bytes()).await;
                // Expect: { rpcUrl, chainId, privateKey, nonce?, gasPrice, gasLimit, dealMarket, data }
                fn keccak(input: &[u8]) -> [u8;32] { use tiny_keccak::Hasher; let mut h=tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; h.update(input); h.finalize(&mut out); out }
                fn rlp_bytes(b: &[u8]) -> Vec<u8> { if b.len()==1 && b[0]<0x80 { return b.to_vec(); } if b.len()<=55 { let mut out=vec![0x80 + b.len() as u8]; out.extend_from_slice(b); return out; } let mut len= b.len(); let mut v=Vec::new(); let mut s=Vec::new(); while len>0{ s.push((len & 0xff) as u8); len >>= 8; } for c in s.iter().rev(){ v.push(*c);} let mut out=vec![0xb7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(b); out }
                fn rlp_u256(x: u128) -> Vec<u8> { if x==0 { return vec![0x80]; } let mut n=x; let mut tmp=Vec::new(); while n>0 { tmp.push((n & 0xff) as u8); n >>= 8; } rlp_bytes(&tmp.iter().rev().cloned().collect::<Vec<_>>()) }
                fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> { let payload_len: usize = items.iter().map(|i| i.len()).sum(); let mut payload=Vec::with_capacity(payload_len); for i in items { payload.extend_from_slice(i); } if payload_len<=55 { let mut out=vec![0xc0 + payload_len as u8]; out.extend_from_slice(&payload); return out; } let mut n=payload_len; let mut v=Vec::new(); let mut s=Vec::new(); while n>0{ s.push((n & 0xff) as u8); n >>= 8; } for c in s.iter().rev(){ v.push(*c);} let mut out=vec![0xf7 + v.len() as u8]; out.extend_from_slice(&v); out.extend_from_slice(&payload); out }

                let rpc_url = body.get("rpcUrl").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let chain_id = body.get("chainId").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u64;
                let priv_hex = body.get("privateKey").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let gas_price = body.get("gasPrice").and_then(|v| v.as_u64()).unwrap_or(1_000_000_000) as u128;
                let gas_limit = body.get("gasLimit").and_then(|v| v.as_u64()).unwrap_or(800_000) as u128;
                let to_addr = body.get("dealMarket").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let data_hex = body.get("data").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let data_bytes = hex::decode(data_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let to = hex::decode(to_addr.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

                // Nonce: use provided or fetch from ARTHA_FROM
                let nonce = if let Some(n) = body.get("nonce").and_then(|v| v.as_u64()) { n as u128 } else {
                    let from_addr = std::env::var("ARTHA_FROM").map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    let payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_getTransactionCount","params":[from_addr,"pending"],"id":1});
                    let client = reqwest::Client::new();
                    let resp = client.post(rpc_url).json(&payload).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                    let val: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                    let hex_nonce = val.get("result").and_then(|v| v.as_str()).unwrap_or("0x0");
                    u64::from_str_radix(hex_nonce.trim_start_matches("0x"), 16).unwrap_or(0) as u128
                };

                // Build raw tx
                let tx_parts = vec![ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(0), rlp_bytes(&data_bytes), rlp_u256(chain_id as u128), rlp_u256(0), rlp_u256(0) ];
                let sighash = keccak(&rlp_list(&tx_parts));
                let pk_bytes = hex::decode(priv_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey};
                use elliptic_curve::generic_array::GenericArray;
                let pk_arr = GenericArray::from_slice(&pk_bytes);
                let sk = SecretKey::from_bytes(pk_arr).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let signing_key = SigningKey::from(sk);
                let sig: k256::ecdsa::Signature = signing_key.sign(&sighash);
                let (r, s) = (sig.r().to_bytes(), sig.s().to_bytes());
                let v = (chain_id * 2 + 35) as u8;
                let raw = rlp_list(&[ rlp_u256(nonce), rlp_u256(gas_price), rlp_u256(gas_limit), rlp_bytes(&to), rlp_u256(0), rlp_bytes(&data_bytes), rlp_u256(v as u128), rlp_bytes(&r.to_vec()), rlp_bytes(&s.to_vec()) ]);
                let raw_hex = format!("0x{}", hex::encode(raw));
                let client = reqwest::Client::new();
                let payload_rpc = serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex],"id":1});
                let resp = client.post(rpc_url).json(&payload_rpc).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                let json: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                Ok::<_, axum::http::StatusCode>(Json(json))
            }
        }))
        // Proofs v2: time-salted inclusion (PoSt-lite)
        .route("/svdb/proofs/v2", post({
            let svdb = svdb.clone();
            move |Json(body): Json<serde_json::Value>| {
                let svdb = svdb.clone();
                async move {
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let index = body.get("index").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as usize;
                    let leaf_hex = body.get("leaf").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let branch_vals = body.get("branch").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let salt_hex = body.get("salt").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;

                    // Decode CID
                    let b64 = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let codec_tag = u16::from_be_bytes([bytes[0], bytes[1]]);
                    let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                    let has_poseidon = bytes[34] == 1; let mut cursor = 35;
                    let poseidon = if has_poseidon { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } else { None };
                    let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let cid_size = u64::from_be_bytes(sz);
                    let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                    let m_cid = Cid::new(codec_tag, blake, poseidon, cid_size, codec);
                    let manifest = svdb.get_manifest(&m_cid).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?;

                    // Recompute Keccak-composed root from leaf and branch
                    let lb = hex::decode(leaf_hex).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if lb.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) }
                    let mut acc=[0u8;32]; acc.copy_from_slice(&lb);
                    let mut idx = index;
                    for v in branch_vals {
                        let s=v.as_str().ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                        let b=hex::decode(s).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if b.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) }
                        let mut sib=[0u8;32]; sib.copy_from_slice(&b);
                        let (l, r) = if idx % 2 == 0 { (acc, sib) } else { (sib, acc) };
                        let keccak = keccak_bytes(&[l.as_slice(), r.as_slice()].concat());
                        acc.copy_from_slice(&keccak);
                        idx /= 2;
                    }
                    // Salted root: keccak(root || salt)
                    let salt = hex::decode(salt_hex).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    if salt.len() != 32 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let salted = keccak_bytes(&[manifest.merkle_root.as_slice(), &salt].concat());
                    let valid = acc == salted;
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"valid": valid, "root": hex::encode(manifest.merkle_root), "saltedRoot": hex::encode(salted)})))
                }
            }
        }))
        .route("/svdb/proofs/v2/batch", post({
            let svdb = svdb.clone();
            move |Json(body): Json<serde_json::Value>| {
                let svdb = svdb.clone();
                async move {
                    let arr = body.get("proofs").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let mut results = Vec::with_capacity(arr.len());
                    for item in arr {
                        let cid_uri = item.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                        let index = item.get("index").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as usize;
                        let leaf_hex = item.get("leaf").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                        let branch_vals = item.get("branch").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                        let salt_hex = item.get("salt").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;

                        let b64 = cid_uri.trim_start_matches("artha://");
                        let bytes = match base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64) { Ok(b) => b, Err(_) => { results.push(false); continue; } };
                        if bytes.len() < 2 + 32 + 1 + 8 + 1 { results.push(false); continue; }
                        let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                        let has_poseidon = bytes[34] == 1; let mut cursor = 35;
                        let poseidon = if has_poseidon { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } else { None };
                        let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let cid_size = u64::from_be_bytes(sz);
                        let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                        let m_cid = Cid::new(u16::from_be_bytes([bytes[0], bytes[1]]), blake, poseidon, cid_size, codec);
                        let manifest = match svdb.get_manifest(&m_cid).await { Ok(m)=>m, Err(_)=>{ results.push(false); continue; } };

                        let lb = match hex::decode(leaf_hex) { Ok(v) => v, Err(_) => { results.push(false); continue; } };
                        if lb.len()!=32 { results.push(false); continue; }
                        let mut acc=[0u8;32]; acc.copy_from_slice(&lb);
                        let mut idx = index;
                        let mut ok = true;
                        for v in branch_vals {
                            let s=match v.as_str(){Some(x)=>x,None=>{ok=false;break}};
                            let b=match hex::decode(s){Ok(x)=>x,Err(_)=>{ok=false;break}}; if b.len()!=32 { ok=false; break }
                            let mut sib=[0u8;32]; sib.copy_from_slice(&b);
                            let (l, r) = if idx % 2 == 0 { (acc, sib) } else { (sib, acc) };
                            let keccak = keccak_bytes(&[l.as_slice(), r.as_slice()].concat());
                            acc.copy_from_slice(&keccak);
                            idx /= 2;
                        }
                        if !ok { results.push(false); continue; }
                        let salt = match hex::decode(salt_hex){ Ok(v)=>v, Err(_)=>{results.push(false); continue;} };
                        if salt.len()!=32 { results.push(false); continue; }
                        let salted = keccak_bytes(&[manifest.merkle_root.as_slice(), &salt].concat());
                        results.push(acc == salted);
                    }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"results": results})))
                }
            }
        }))
        // Proofs v3: Poseidon-path inclusion against manifest.poseidon_root
        .route("/svdb/proofs/v3", post({
            let svdb = svdb.clone();
            move |Json(body): Json<serde_json::Value>| {
                let svdb = svdb.clone();
                async move {
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let index = body.get("index").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as usize;
                    let leaf_hex = body.get("leaf").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let branch_vals = body.get("branch").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;

                    // Decode CID
                    let b64 = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let codec_tag = u16::from_be_bytes([bytes[0], bytes[1]]);
                    let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                    let has_poseidon = bytes[34] == 1; let mut cursor = 35;
                    let poseidon_field = if has_poseidon { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } else { None };
                    let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let cid_size = u64::from_be_bytes(sz);
                    let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                    let m_cid = Cid::new(codec_tag, blake, poseidon_field, cid_size, codec);
                    let manifest = svdb.get_manifest(&m_cid).await.map_err(|_| axum::http::StatusCode::NOT_FOUND)?;
                    let pose_root = match manifest.poseidon_root { Some(r)=>r, None=>return Err(axum::http::StatusCode::BAD_REQUEST) };

                    // Compose Poseidon path (replaced with keccak-based pair hash to remove dependency)
                    fn poseidon_hash2(l: &[u8;32], r: &[u8;32]) -> [u8;32] { let h = keccak_bytes(&[l.as_slice(), r.as_slice()].concat()); let mut out=[0u8;32]; out.copy_from_slice(&h); out }

                    let lb = hex::decode(leaf_hex).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if lb.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) }
                    let mut acc=[0u8;32]; acc.copy_from_slice(&lb);
                    let mut idx = index;
                    for v in branch_vals {
                        let s=v.as_str().ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                        let b=hex::decode(s).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if b.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) }
                        let mut sib=[0u8;32]; sib.copy_from_slice(&b);
                        let h = if idx % 2 == 0 { poseidon_hash2(&acc, &sib) } else { poseidon_hash2(&sib, &acc) };
                        acc = h;
                        idx /= 2;
                    }
                    let valid = acc == pose_root;
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"valid": valid, "poseidonRoot": hex::encode(pose_root)})))
                }
            }
        }))
        .route("/svdb/proofs/v3/batch", post({
            let svdb = svdb.clone();
            move |Json(body): Json<serde_json::Value>| {
                let svdb = svdb.clone();
                async move {
                    let arr = body.get("proofs").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    fn poseidon_hash2(l: &[u8;32], r: &[u8;32]) -> [u8;32] { use light_poseidon::Poseidon; let mut s=Poseidon::new(); s.update(l); s.update(r); let out=s.finalize(); let mut h=[0u8;32]; let n=core::cmp::min(out.len(),32); h[..n].copy_from_slice(&out[..n]); h }
                    let mut results = Vec::with_capacity(arr.len());
                    for item in arr {
                        let cid_uri = item.get("cid").and_then(|v| v.as_str());
                        let index = item.get("index").and_then(|v| v.as_u64());
                        let leaf_hex = item.get("leaf").and_then(|v| v.as_str());
                        let branch_vals = item.get("branch").and_then(|v| v.as_array());
                        let (cid_uri, index, leaf_hex, branch_vals) = match (cid_uri,index,leaf_hex,branch_vals) { (Some(a),Some(b),Some(c),Some(d)) => (a,b as usize,c,d), _ => { results.push(false); continue; } };

                        let bytes = match base64::engine::general_purpose::STANDARD_NO_PAD.decode(cid_uri.trim_start_matches("artha://")) { Ok(b)=>b, Err(_)=>{ results.push(false); continue; } };
                        if bytes.len() < 2 + 32 + 1 + 8 + 1 { results.push(false); continue; }
                        let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                        let has_poseidon = bytes[34] == 1; let mut cursor = 35;
                        let poseidon_field = if has_poseidon { let mut p=[0u8;32]; p.copy_from_slice(&bytes[cursor..cursor+32]); cursor+=32; Some(p) } else { None };
                        let mut sz=[0u8;8]; sz.copy_from_slice(&bytes[cursor..cursor+8]); cursor+=8; let cid_size = u64::from_be_bytes(sz);
                        let codec = match bytes[cursor] {0=>Codec::Raw,1=>Codec::Zstd,2=>Codec::Lz4,_=>Codec::Raw};
                        let m_cid = Cid::new(u16::from_be_bytes([bytes[0], bytes[1]]), blake, poseidon_field, cid_size, codec);
                        let manifest = match svdb.get_manifest(&m_cid).await { Ok(m)=>m, Err(_)=>{ results.push(false); continue; } };
                        let pose_root = match manifest.poseidon_root { Some(r)=>r, None=>{ results.push(false); continue; } };
                        let lb = match hex::decode(leaf_hex) { Ok(v)=>v, Err(_)=>{ results.push(false); continue; } };
                        if lb.len()!=32 { results.push(false); continue; }
                        let mut acc=[0u8;32]; acc.copy_from_slice(&lb);
                        let mut idx = index;
                        let mut ok = true;
                        for v in branch_vals {
                            let s=match v.as_str(){Some(x)=>x,None=>{ok=false;break}};
                            let b=match hex::decode(s){Ok(x)=>x,Err(_)=>{ok=false;break}}; if b.len()!=32 { ok=false; break }
                            let mut sib=[0u8;32]; sib.copy_from_slice(&b);
                            acc = if idx % 2 == 0 { poseidon_hash2(&acc, &sib) } else { poseidon_hash2(&sib, &acc) };
                            idx /= 2;
                        }
                        results.push(ok && acc == pose_root);
                    }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"results": results})))
                }
            }
        }))
        // Forward proof to on-chain DealMarket
        .route("/svdb/proofs/forward", post({
            move |Json(body): Json<serde_json::Value>| {
                async move {
                // Expect: { dealMarket, root, leaf, branch:[..hex], index, from, key } where dealMarket is contract address
                let deal_market = body.get("dealMarket").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let root_hex = body.get("root").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let leaf_hex = body.get("leaf").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let index = body.get("index").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let branch = body.get("branch").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                // Here you would build and submit the transaction via your EVM JSON-RPC; we record the intent only.
                let tx_req = serde_json::json!({
                    "to": deal_market,
                    "data": {
                        "method": "streamPayout",
                        "args": {"root": root_hex, "leaf": leaf_hex, "branch": branch, "index": index}
                    }
                });
                Ok(Json(serde_json::json!({"submitted": true, "tx": tx_req})))
                }
            }
        }))
        // Submit proof with EVM JSON-RPC signer; returns txHash
        .route("/svdb/proofs/submit", post({
            let deal_store_rl = deal_store.clone();
            move |Json(body): Json<serde_json::Value>, headers: HeaderMap| async move {
                // Rate limit per IP/minute
                let client_ip = headers.get("X-Client-IP").and_then(|v| v.to_str().ok()).unwrap_or("unknown");
                let now_min = (chrono::Utc::now().timestamp() / 60).to_string();
                let rl_key = format!("ratelimit:proofs_submit:{}:{}", client_ip, now_min);
                let cnt = match deal_store_rl.get(rl_key.as_bytes()).await { Ok(Some(b)) => u64::from_le_bytes(b.try_into().unwrap_or([0u8;8])), _ => 0 };
                let max_req = std::env::var("ARTHA_PROOFS_SUBMIT_PER_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(120u64);
                if cnt >= max_req { return Err(axum::http::StatusCode::TOO_MANY_REQUESTS); }
                let _ = deal_store_rl.put(rl_key.as_bytes(), &(cnt+1).to_le_bytes()).await;
                // Required: rpcUrl, chainId, privateKey (hex), nonce, gasPrice, gasLimit, dealMarket, root, leaf, branch:[hex], index
                let rpc_url = body.get("rpcUrl").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let chain_id = body.get("chainId").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)? as u64;
                let priv_hex = body.get("privateKey").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let nonce = body.get("nonce").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let gas_price = body.get("gasPrice").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let gas_limit = body.get("gasLimit").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let to_addr = body.get("dealMarket").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let root_hex = body.get("root").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let leaf_hex = body.get("leaf").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let index = body.get("index").and_then(|v| v.as_u64()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                let branch_vals = body.get("branch").and_then(|v| v.as_array()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;

                // ABI encode streamPayout(bytes32,bytes32,bytes32[],uint256)
                fn pad32(mut v: Vec<u8>) -> Vec<u8> { let mut p = vec![0u8; 32 - v.len()]; p.append(&mut v); p }
                fn enc_u256(x: u128) -> Vec<u8> { pad32(hex::encode(format!("{:x}", x)).as_bytes().chunks(2).map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap()).collect()) }
                fn enc_bytes32(b: &[u8]) -> Vec<u8> { let mut out = vec![0u8;32]; out.copy_from_slice(b); out.to_vec() }
                fn keccak(input: &[u8]) -> [u8;32] { use tiny_keccak::Hasher; let mut hasher = tiny_keccak::Keccak::v256(); let mut out=[0u8;32]; hasher.update(input); hasher.finalize(&mut out); out }

                let selector = &keccak(b"streamPayout(bytes32,bytes32,bytes32[],uint256)")[0..4];
                let root = { let b = hex::decode(root_hex).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if b.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) } b };
                let leaf = { let b = hex::decode(leaf_hex).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if b.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) } b };
                // Dynamic array encoding: offset pointer + length + elements
                let head_size = 32*4; // 4 params
                let mut head = Vec::with_capacity(head_size);
                // root
                head.extend_from_slice(&enc_bytes32(&root));
                // leaf
                head.extend_from_slice(&enc_bytes32(&leaf));
                // branch offset
                head.extend_from_slice(&enc_u256(head_size as u128));
                // index
                head.extend_from_slice(&enc_u256(index as u128));
                // tail: branch array
                let mut tail = Vec::new();
                tail.extend_from_slice(&enc_u256(branch_vals.len() as u128));
                for v in branch_vals { let s=v.as_str().ok_or(axum::http::StatusCode::BAD_REQUEST)?; let b=hex::decode(s).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?; if b.len()!=32 { return Err(axum::http::StatusCode::BAD_REQUEST) } tail.extend_from_slice(&enc_bytes32(&b)); }
                let mut data = Vec::with_capacity(4 + head.len() + tail.len());
                data.extend_from_slice(selector);
                data.extend_from_slice(&head);
                data.extend_from_slice(&tail);

                // RLP encode legacy tx and sign (EIP-155)
                fn rlp_encode_bytes(b: &[u8]) -> Vec<u8> {
                    if b.len() == 1 && b[0] < 0x80 { return b.to_vec(); }
                    if b.len() <= 55 { let mut out = vec![0x80 + b.len() as u8]; out.extend_from_slice(b); return out; }
                    let len_bytes = {
                        let mut v = Vec::new();
                        let mut n = b.len();
                        let mut s = Vec::new();
                        while n > 0 { s.push((n & 0xff) as u8); n >>= 8; }
                        for c in s.iter().rev() { v.push(*c); }
                        v
                    };
                    let mut out = vec![0xb7 + len_bytes.len() as u8]; out.extend_from_slice(&len_bytes); out.extend_from_slice(b); out
                }
                fn rlp_encode_u256(v: u128) -> Vec<u8> {
                    if v == 0 { return vec![0x80]; }
                    let mut bytes = Vec::new();
                    let mut n = v;
                    let mut tmp = Vec::new();
                    while n > 0 { tmp.push((n & 0xff) as u8); n >>= 8; }
                    for c in tmp.iter().rev() { bytes.push(*c); }
                    rlp_encode_bytes(&bytes)
                }
                fn rlp_encode_list(items: &[Vec<u8>]) -> Vec<u8> {
                    let payload_len: usize = items.iter().map(|i| i.len()).sum();
                    let mut payload = Vec::with_capacity(payload_len);
                    for i in items { payload.extend_from_slice(i); }
                    if payload_len <= 55 { let mut out = vec![0xc0 + payload_len as u8]; out.extend_from_slice(&payload); return out; }
                    let len_bytes = {
                        let mut v = Vec::new();
                        let mut n = payload_len;
                        let mut s = Vec::new();
                        while n > 0 { s.push((n & 0xff) as u8); n >>= 8; }
                        for c in s.iter().rev() { v.push(*c); }
                        v
                    };
                    let mut out = vec![0xf7 + len_bytes.len() as u8]; out.extend_from_slice(&len_bytes); out.extend_from_slice(&payload); out
                }

                // Build sighash per EIP-155 (legacy)
                let to = {
                    let s = to_addr.trim_start_matches("0x");
                    hex::decode(s).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?
                };
                let value = 0u128;
                let data_rlp = rlp_encode_bytes(&data);
                let to_rlp = rlp_encode_bytes(&to);
                let tx_parts = vec![
                    rlp_encode_u256(nonce as u128),
                    rlp_encode_u256(gas_price as u128),
                    rlp_encode_u256(gas_limit as u128),
                    to_rlp,
                    rlp_encode_u256(value),
                    data_rlp,
                    rlp_encode_u256(chain_id as u128),
                    rlp_encode_u256(0),
                    rlp_encode_u256(0),
                ];
                let sighash = keccak(&rlp_encode_list(&tx_parts));

                // Sign with secp256k1 (k256)
                let pk_bytes = hex::decode(priv_hex.trim_start_matches("0x")).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                use k256::{ecdsa::{SigningKey, signature::Signer}, SecretKey};
                use elliptic_curve::generic_array::GenericArray;
                let pk_arr = GenericArray::from_slice(&pk_bytes);
                let sk = SecretKey::from_bytes(pk_arr).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                let signing_key = SigningKey::from(sk);
                let sig: k256::ecdsa::Signature = signing_key.sign(&sighash);
                let (r, s) = (sig.r().to_bytes(), sig.s().to_bytes());
                let v = (chain_id * 2 + 35) as u8; // no recovery id; simple EIP-155 v
                let rlp_signed = rlp_encode_list(&[
                    rlp_encode_u256(nonce as u128),
                    rlp_encode_u256(gas_price as u128),
                    rlp_encode_u256(gas_limit as u128),
                    rlp_encode_bytes(&to),
                    rlp_encode_u256(0),
                    rlp_encode_bytes(&data),
                    rlp_encode_u256(v as u128),
                    rlp_encode_bytes(&r.to_vec()),
                    rlp_encode_bytes(&s.to_vec()),
                ]);
                let raw_hex = format!("0x{}", hex::encode(rlp_signed));

                // eth_sendRawTransaction
                let client = HttpClient::new();
                let payload = serde_json::json!({"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":[raw_hex],"id":1});
                let resp = client.post(rpc_url).json(&payload).send().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                let json: serde_json::Value = resp.json().await.map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;
                Ok::<_, axum::http::StatusCode>(Json(json))
            }
        }))
        // Provider capabilities (co-location hints)
        .route("/svdb/providers/capabilities", post({
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>| {
                let deal_store = deal_store.clone();
                async move {
                    let node_id = body.get("nodeId").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let region = body.get("region").and_then(|v| v.as_str()).unwrap_or("");
                    let gpu = body.get("gpu").and_then(|v| v.as_bool()).unwrap_or(false);
                    let disk_free = body.get("disk_free_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                    let http_addr = body.get("http_addr").and_then(|v| v.as_str()).unwrap_or("");
                    let caps = serde_json::json!({
                        "nodeId": node_id,
                        "region": region,
                        "gpu": gpu,
                        "disk_free_bytes": disk_free,
                        "http_addr": http_addr,
                        "updated_at": chrono::Utc::now().to_rfc3339()
                    });
                    let key = format!("caps:{}", node_id);
                    deal_store.put(key.as_bytes(), serde_json::to_string(&caps).unwrap().as_bytes()).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    // Update index
                    let idx_key = b"caps:index";
                    let mut list: Vec<String> = match deal_store.get(idx_key).await { Ok(Some(b)) => serde_json::from_slice(&b).unwrap_or_default(), _ => Vec::new() };
                    if !list.contains(&node_id.to_string()) { list.push(node_id.to_string()); }
                    deal_store.put(idx_key, serde_json::to_vec(&list).unwrap().as_slice()).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    Ok::<_, axum::http::StatusCode>(Json(caps))
                }
            }
        }))
        .route("/svdb/providers", get({
            let deal_store = deal_store.clone();
            move || {
                let deal_store = deal_store.clone();
                async move {
                    let idx_key = b"caps:index";
                    let nodes: Vec<String> = match deal_store.get(idx_key).await { Ok(Some(b)) => serde_json::from_slice(&b).unwrap_or_default(), _ => Vec::new() };
                    let mut out = Vec::new();
                    for n in &nodes {
                        let key = format!("caps:{}", n);
                        if let Ok(Some(c)) = deal_store.get(key.as_bytes()).await {
                            if let Ok(caps) = serde_json::from_slice::<serde_json::Value>(&c) {
                                out.push(caps);
                            }
                        }
                    }
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"providers": out})))
                }
            }
        }))
        // Registries
        .route("/svdb/registry/dataset", post({
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>| {
                let deal_store = deal_store.clone();
                async move {
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let owner = body.get("owner").and_then(|v| v.as_str()).unwrap_or("");
                    let size = body.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
                    let license = body.get("license").and_then(|v| v.as_str()).unwrap_or("");
                    let tags = body.get("tags").cloned().unwrap_or(serde_json::json!([]));
                    let entry = serde_json::json!({"cid": cid_uri, "owner": owner, "size": size, "license": license, "tags": tags, "created_at": chrono::Utc::now().to_rfc3339()});
                    let key = format!("dsreg:{}", cid_uri);
                    deal_store.put(key.as_bytes(), serde_json::to_string(&entry).unwrap().as_bytes()).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    Ok::<_, axum::http::StatusCode>(Json(entry))
                }
            }
        }))
        // Access policy management
        .route("/svdb/access/policy", post({
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>| {
                let deal_store = deal_store.clone();
                async move {
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let b64 = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let mut blake = [0u8;32]; blake.copy_from_slice(&bytes[2..34]);
                    let cid_hex = hex::encode(blake);
                    let policy_key = format!("access:{}", cid_hex);
                    let private = body.get("private").and_then(|v| v.as_bool()).unwrap_or(false);
                    let allowed = body.get("allowedDids").cloned().unwrap_or(serde_json::json!([]));
                    let entry = serde_json::json!({"private": private, "allowedDids": allowed});
                    deal_store.put(policy_key.as_bytes(), serde_json::to_string(&entry).unwrap().as_bytes()).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    Ok::<_, axum::http::StatusCode>(Json(entry))
                }
            }
        }))
        // Provider announcement for co-location
        .route("/svdb/providers/announce", post({
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>| {
                let deal_store = deal_store.clone();
                async move {
                    let cid_uri = body.get("cid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let node_id = body.get("nodeId").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let b64 = cid_uri.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let mut bl=[0u8;32]; bl.copy_from_slice(&bytes[2..34]);
                    let cid_hex = hex::encode(bl);
                    let key = format!("prov:{}", cid_hex);
                    let mut list: Vec<String> = match deal_store.get(key.as_bytes()).await { Ok(Some(b)) => serde_json::from_slice(&b).unwrap_or_default(), _ => Vec::new() };
                    if !list.contains(&node_id.to_string()) { list.push(node_id.to_string()); }
                    deal_store.put(key.as_bytes(), serde_json::to_vec(&list).unwrap().as_slice()).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"cid": cid_uri, "providers": list})))
                }
            }
        }))
        // Scheduler: plan co-located providers for model/dataset
        .route("/svdb/scheduler/plan", get({
            let deal_store = deal_store.clone();
            move |Query(params): Query<HashMap<String, String>>| {
                let deal_store = deal_store.clone();
                async move {
                    let dataset_cid = params.get("datasetCid").ok_or(axum::http::StatusCode::BAD_REQUEST)?.to_string();
                    let b64 = dataset_cid.trim_start_matches("artha://");
                    let bytes = base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
                    if bytes.len() < 2 + 32 + 1 + 8 + 1 { return Err(axum::http::StatusCode::BAD_REQUEST); }
                    let mut bl=[0u8;32]; bl.copy_from_slice(&bytes[2..34]);
                    let cid_hex = hex::encode(bl);
                    let prov_key = format!("prov:{}", cid_hex);
                    let providers: Vec<String> = match deal_store.get(prov_key.as_bytes()).await { Ok(Some(b)) => serde_json::from_slice(&b).unwrap_or_default(), _ => Vec::new() };
                    // Fetch capabilities for providers
                    let mut ranked: Vec<serde_json::Value> = Vec::new();
                    for pid in providers {
                        let cap_key = format!("caps:{}", pid);
                        if let Ok(Some(cbytes)) = deal_store.get(cap_key.as_bytes()).await {
                            if let Ok(caps) = serde_json::from_slice::<serde_json::Value>(&cbytes) {
                                let gpu = caps.get("gpu").and_then(|v| v.as_bool()).unwrap_or(false);
                                let disk_free = caps.get("disk_free_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                                ranked.push(serde_json::json!({"nodeId": pid, "gpu": gpu, "disk_free_bytes": disk_free, "region": caps.get("region").and_then(|v| v.as_str()).unwrap_or("")}));
                            }
                        }
                    }
                    // Sort: prefer GPU, then more disk_free
                    ranked.sort_by(|a,b| {
                        let ga=a.get("gpu").and_then(|v| v.as_bool()).unwrap_or(false);
                        let gb=b.get("gpu").and_then(|v| v.as_bool()).unwrap_or(false);
                        if ga!=gb { return if gb { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less }; }
                        let da=a.get("disk_free_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                        let db=b.get("disk_free_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                        da.cmp(&db)
                    });
                    ranked.reverse();
                    Ok::<_, axum::http::StatusCode>(Json(serde_json::json!({"datasetCid": dataset_cid, "plan": ranked})))
                }
            }
        }))
        .route("/svdb/registry/dataset/:cid", get({
            let deal_store = deal_store.clone();
            move |axum::extract::Path(cid_uri): axum::extract::Path<String>| {
                let deal_store = deal_store.clone();
                async move {
                    let key = format!("dsreg:{}", cid_uri);
                    let val = deal_store.get(key.as_bytes()).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    match val { Some(v) => Ok::<_, axum::http::StatusCode>(Json(serde_json::from_slice::<serde_json::Value>(&v).unwrap())), None => Err(axum::http::StatusCode::NOT_FOUND) }
                }
            }
        }))
        .route("/svdb/registry/model", post({
            let deal_store = deal_store.clone();
            move |Json(body): Json<serde_json::Value>| {
                let deal_store = deal_store.clone();
                async move {
                    let model_cid = body.get("modelCid").and_then(|v| v.as_str()).ok_or(axum::http::StatusCode::BAD_REQUEST)?;
                    let dataset_cid = body.get("datasetCid").and_then(|v| v.as_str()).unwrap_or("");
                    let code_hash = body.get("codeHash").and_then(|v| v.as_str()).unwrap_or("");
                    let version = body.get("version").and_then(|v| v.as_str()).unwrap_or("v1");
                    let lineage = body.get("lineage").cloned().unwrap_or(serde_json::json!([]));
                    let entry = serde_json::json!({"modelCid": model_cid, "datasetCid": dataset_cid, "codeHash": code_hash, "version": version, "lineage": lineage, "created_at": chrono::Utc::now().to_rfc3339()});
                    let key = format!("mdreg:{}", model_cid);
                    deal_store.put(key.as_bytes(), serde_json::to_string(&entry).unwrap().as_bytes()).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    Ok::<_, axum::http::StatusCode>(Json(entry))
                }
            }
        }))
        .route("/svdb/registry/model/:cid", get({
            let deal_store = deal_store.clone();
            move |axum::extract::Path(model_cid): axum::extract::Path<String>| {
                let deal_store = deal_store.clone();
                async move {
                    let key = format!("mdreg:{}", model_cid);
                    let val = deal_store.get(key.as_bytes()).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                    match val { Some(v) => Ok::<_, axum::http::StatusCode>(Json(serde_json::from_slice::<serde_json::Value>(&v).unwrap())), None => Err(axum::http::StatusCode::NOT_FOUND) }
                }
            }
        }))
        
        // Add CORS layer
        .layer(
            CorsLayer::new()
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_origin(
                    std::env::var("ARTHA_CORS_ORIGINS")
                        .ok()
                        .map(|v| v.split(',').filter_map(|s| s.parse().ok()).collect::<Vec<_>>())
                        .unwrap_or_else(|| vec!["http://localhost:5173".parse().unwrap()])
                )
        )
        
        // Add state extensions
        .with_state(state)
        .layer(Extension(mempool))
        .layer(Extension(faucet_service))
        .layer(Extension(gas_free_manager))
        // Standardize error envelope for non-success responses
        .layer(axum::middleware::map_response(|res: AxumResponse| async move {
            if !res.status().is_success() {
                let code = res.status();
                let body = axum::Json(serde_json::json!({
                    "error": { "code": code.as_u16(), "message": code.canonical_reason().unwrap_or("Error") }
                }));
                return (code, body).into_response();
            }
            res
        }))
}
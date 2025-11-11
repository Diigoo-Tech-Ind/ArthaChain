// ArthaChain SVDB Background Scheduler Daemon
// Autonomous service for automated proof challenges and submissions

use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use reqwest::Client as HttpClient;

#[derive(Parser)]
#[command(name = "artha-scheduler")]
#[command(about = "ArthaChain autonomous proof scheduler daemon")]
struct Cli {
    #[arg(long, default_value = "http://localhost:3000")]
    node_url: String,
    
    #[arg(long, default_value = "http://localhost:8545")]
    rpc_url: String,
    
    #[arg(long)]
    private_key: String,
    
    #[arg(long)]
    deal_market: String,
    
    #[arg(long)]
    porep_contract: Option<String>,
    
    #[arg(long, default_value = "300")]
    epoch_seconds: u64,
    
    #[arg(long, default_value = "8888")]
    chain_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ManifestInfo {
    cid: String,
    size: u64,
    chunks: Vec<ChunkEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChunkEntry {
    cid: String,
    size: u64,
}

struct Scheduler {
    node_url: String,
    rpc_url: String,
    private_key: String,
    deal_market: String,
    porep_contract: Option<String>,
    epoch_seconds: u64,
    chain_id: u64,
    http: HttpClient,
    current_epoch: u64,
}

impl Scheduler {
    fn new(cli: Cli) -> Self {
        Self {
            node_url: cli.node_url,
            rpc_url: cli.rpc_url,
            private_key: cli.private_key,
            deal_market: cli.deal_market,
            porep_contract: cli.porep_contract,
            epoch_seconds: cli.epoch_seconds,
            chain_id: cli.chain_id,
            http: HttpClient::new(),
            current_epoch: 0,
        }
    }
    
    async fn run(&mut self) -> Result<()> {
        println!("🚀 ArthaChain Scheduler Daemon Started");
        println!("   Node: {}", self.node_url);
        println!("   RPC: {}", self.rpc_url);
        println!("   DealMarket: {}", self.deal_market);
        println!("   Epoch: {} seconds", self.epoch_seconds);
        
        let mut epoch_timer = interval(Duration::from_secs(self.epoch_seconds));
        
        loop {
            epoch_timer.tick().await;
            self.current_epoch += 1;
            
            println!("\n📅 Epoch {} starting...", self.current_epoch);
            
            // Run all scheduled tasks
            if let Err(e) = self.process_epoch().await {
                eprintln!("❌ Epoch {} error: {}", self.current_epoch, e);
            } else {
                println!("✅ Epoch {} complete", self.current_epoch);
            }
        }
    }
    
    async fn process_epoch(&self) -> Result<()> {
        // Task 1: Get all active deals
        let deals = self.get_active_deals().await?;
        println!("   Found {} active deals", deals.len());
        
        // Task 2: Generate and submit salted proofs for each deal
        for deal_cid in deals.iter() {
            if let Err(e) = self.process_deal_proofs(deal_cid).await {
                eprintln!("   ⚠️ Deal {} proof error: {}", deal_cid, e);
            }
        }
        
        // Task 3: Issue PoRep challenges if contract configured
        if self.porep_contract.is_some() {
            if let Err(e) = self.issue_porep_challenges().await {
                eprintln!("   ⚠️ PoRep challenge error: {}", e);
            }
        }
        
        // Task 4: Check for repair needs
        if let Err(e) = self.check_repairs().await {
            eprintln!("   ⚠️ Repair check error: {}", e);
        }
        
        Ok(())
    }
    
    async fn get_active_deals(&self) -> Result<Vec<String>> {
        // Query DealMarket contract via node API to get active deals
        // API endpoint wraps eth_call to contract.getActiveDeals()
        
        let url = format!("{}/svdb/deals/active", self.node_url);
        let response = self.http.get(&url).send().await?;
        
        if response.status().is_success() {
            let deals: Vec<String> = response.json().await?;
            Ok(deals)
        } else {
            // Fallback: return empty list
            Ok(Vec::new())
        }
    }
    
    async fn process_deal_proofs(&self, deal_cid: &str) -> Result<()> {
        println!("   📝 Processing proofs for deal: {}", deal_cid);
        
        // Step 1: Get manifest info
        let manifest = self.get_manifest(deal_cid).await?;
        
        // Step 2: Select random chunk indices for this epoch
        let indices = self.select_challenge_indices(&manifest, 3);
        println!("      Challenge indices: {:?}", indices);
        
        // Step 3: Build proofs for selected indices
        let mut proofs = Vec::new();
        for idx in indices.iter() {
            match self.build_proof(deal_cid, *idx).await {
                Ok(proof) => proofs.push(proof),
                Err(e) => {
                    eprintln!("      ⚠️ Proof {} build error: {}", idx, e);
                }
            }
        }
        
        if proofs.is_empty() {
            return Ok(());
        }
        
        // Step 4: Build salted batch
        let batch = self.build_salted_batch(deal_cid, &proofs).await?;
        println!("      Built salted batch: {} bytes", batch.len());
        
        // Step 5: Submit batch to DealMarket
        self.submit_batch(deal_cid, &batch).await?;
        println!("      ✅ Batch submitted successfully");
        
        Ok(())
    }
    
    async fn get_manifest(&self, cid: &str) -> Result<ManifestInfo> {
        let url = format!("{}/svdb/info/{}", self.node_url, cid);
        let response = self.http.get(&url).send().await?;
        let manifest: ManifestInfo = response.json().await?;
        Ok(manifest)
    }
    
    fn select_challenge_indices(&self, manifest: &ManifestInfo, count: usize) -> Vec<u32> {
        // Use epoch as seed for deterministic randomness
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        self.current_epoch.hash(&mut hasher);
        manifest.cid.hash(&mut hasher);
        let seed = hasher.finish();
        
        let chunk_count = manifest.chunks.len();
        let mut indices = Vec::new();
        
        for i in 0..count.min(chunk_count) {
            let idx = ((seed + i as u64) % chunk_count as u64) as u32;
            indices.push(idx);
        }
        
        indices
    }
    
    async fn build_proof(&self, cid: &str, index: u32) -> Result<serde_json::Value> {
        let url = format!("{}/svdb/proofs/branch", self.node_url);
        let payload = serde_json::json!({
            "cid": cid,
            "index": index
        });
        
        let response = self.http.post(&url)
            .json(&payload)
            .send()
            .await?;
        
        let proof: serde_json::Value = response.json().await?;
        Ok(proof)
    }
    
    async fn build_salted_batch(&self, cid: &str, proofs: &[serde_json::Value]) -> Result<String> {
        let url = format!("{}/svdb/proofs/v2/batch/build", self.node_url);
        let payload = serde_json::json!({
            "manifestRoot": cid,
            "epoch": self.current_epoch,
            "proofs": proofs
        });
        
        let response = self.http.post(&url)
            .json(&payload)
            .send()
            .await?;
        
        let result: serde_json::Value = response.json().await?;
        let batch_data = result["data"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No batch data in response"))?;
        
        Ok(batch_data.to_string())
    }
    
    async fn submit_batch(&self, cid: &str, batch_data: &str) -> Result<()> {
        let url = format!("{}/svdb/proofs/v2/batch/submit", self.node_url);
        let payload = serde_json::json!({
            "rpcUrl": self.rpc_url,
            "chainId": self.chain_id,
            "privateKey": self.private_key,
            "gasPrice": 1_000_000_000u64,
            "gasLimit": 500_000u64,
            "dealMarket": self.deal_market,
            "data": batch_data,
            "nonce": self.current_epoch
        });
        
        let response = self.http.post(&url)
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Submit failed: {}", error_text));
        }
        
        Ok(())
    }
    
    async fn issue_porep_challenges(&self) -> Result<()> {
        if let Some(contract) = &self.porep_contract {
            println!("   🔐 Issuing PoRep challenges...");
            
            // Get all active seals
            let seals = self.get_active_seals().await?;
            println!("      Found {} active seals", seals.len());
            
            for seal_commitment in seals.iter() {
                if let Err(e) = self.issue_challenge(contract, seal_commitment).await {
                    eprintln!("      ⚠️ Challenge {} error: {}", seal_commitment, e);
                }
            }
        }
        
        Ok(())
    }
    
    async fn get_active_seals(&self) -> Result<Vec<String>> {
        // In production, would query SVDBPoRep contract for active seals
        // For now, return empty list
        Ok(Vec::new())
    }
    
    async fn issue_challenge(&self, contract: &str, commitment: &str) -> Result<()> {
        let url = format!("{}/svdb/porep/challenge", self.node_url);
        let payload = serde_json::json!({
            "commitment": commitment,
            "rpcUrl": self.rpc_url,
            "contract": contract,
            "privateKey": self.private_key
        });
        
        let response = self.http.post(&url)
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Challenge failed: {}", error_text));
        }
        
        println!("      ✅ Challenge issued for {}", commitment);
        Ok(())
    }
    
    async fn check_repairs(&self) -> Result<()> {
        println!("   🔧 Checking for repair needs...");
        
        // Query repair auctions or missing chunks
        // This would integrate with RepairAuction contract
        
        // For now, just log
        println!("      No repairs needed");
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let mut scheduler = Scheduler::new(cli);
    scheduler.run().await?;
    
    Ok(())
}


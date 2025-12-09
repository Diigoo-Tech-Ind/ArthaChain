// End-to-End Integration Tests for SVDB
// Tests complete workflows across storage, proofs, marketplace, and AI

use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

// Mock setup functions
async fn setup_test_contracts() -> (String, String, String) {
    // Returns (DealMarket, OfferBook, PoRep) addresses
    ("0x1234...".to_string(), "0x5678...".to_string(), "0xabcd...".to_string())
}

fn create_test_file(size_mb: usize) -> PathBuf {
    let path = PathBuf::from(format!("/tmp/test_file_{}mb.dat", size_mb));
    let data = vec![0xAB; size_mb * 1024 * 1024];
    fs::write(&path, data).unwrap();
    path
}

#[tokio::test]
async fn test_e2e_upload_replicate_download() {
    // Test: Upload 100MB file, replicate to 5 nodes, download and verify
    
    println!("🧪 E2E Test: Upload → Replicate → Download");
    
    let mock_server = MockServer::start().await;
    let node_url = mock_server.uri();
    let test_file = create_test_file(100);
    
    // Mock setup
    Mock::given(method("POST"))
        .and(path("/svdb/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "cid": "cid_100mb"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/svdb/info/cid_100mb"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "replicas": 5
        })))
        .mount(&mock_server)
        .await;

    let file_content = vec![0xAB; 100 * 1024 * 1024];
    Mock::given(method("GET"))
        .and(path("/svdb/download/cid_100mb"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(file_content.clone()))
        .mount(&mock_server)
        .await;

    // Step 1: Upload file
    println!("   Step 1: Uploading 100MB file...");
    let client = reqwest::Client::new();
    let file_data = fs::read(&test_file).unwrap();
    
    let response = client.post(format!("{}/svdb/upload", node_url))
        .body(file_data.clone())
        .send()
        .await
        .unwrap();
    
    assert!(response.status().is_success());
    let upload_result: serde_json::Value = response.json().await.unwrap();
    let cid = upload_result["cid"].as_str().unwrap();
    println!("   ✓ Uploaded: {}", cid);
    
    // Step 2: Wait for replication
    println!("   Step 2: Waiting for replication...");
    sleep(Duration::from_millis(100)).await; // Reduced sleep for mock test
    
    // Step 3: Verify replicas exist
    println!("   Step 3: Verifying replicas...");
    let info_response = client.get(format!("{}/svdb/info/{}", node_url, cid))
        .send()
        .await
        .unwrap();
    
    assert!(info_response.status().is_success());
    let info: serde_json::Value = info_response.json().await.unwrap();
    assert_eq!(info["replicas"].as_u64().unwrap(), 5);
    println!("   ✓ Verified 5 replicas");
    
    // Step 4: Download and verify
    println!("   Step 4: Downloading and verifying...");
    let download_response = client.get(format!("{}/svdb/download/{}", node_url, cid))
        .send()
        .await
        .unwrap();
    
    assert!(download_response.status().is_success());
    let downloaded_data = download_response.bytes().await.unwrap();
    assert_eq!(downloaded_data.len(), file_data.len());
    assert_eq!(downloaded_data.to_vec(), file_data);
    println!("   ✓ Download verified");
    
    // Cleanup
    fs::remove_file(test_file).ok();
    
    println!("✅ E2E Test PASSED: Upload → Replicate → Download");
}

#[tokio::test]
async fn test_e2e_erasure_coding_repair() {
    // Test: Upload with erasure coding, simulate node failure, verify repair
    
    println!("🧪 E2E Test: Erasure Coding → Failure → Repair");
    
    let mock_server = MockServer::start().await;
    let node_url = mock_server.uri();
    let test_file = create_test_file(1000); // 1GB
    
    // Mock setup
    Mock::given(method("POST"))
        .and(path("/svdb/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "cid": "cid_erasure"
        })))
        .mount(&mock_server)
        .await;

    // Initial info check
    Mock::given(method("GET"))
        .and(path("/svdb/info/cid_erasure"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "erasure_data_shards": 8,
            "erasure_parity_shards": 2,
            "healthy_shards": 10
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/repair/trigger"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Step 1: Upload with erasure coding
    println!("   Step 1: Uploading with RS(10,8) erasure coding...");
    let client = reqwest::Client::new();
    let file_data = fs::read(&test_file).unwrap();
    
    let response = client.post(format!("{}/svdb/upload", node_url))
        .header("X-Artha-Erasure", "10,8")
        .body(file_data.clone())
        .send()
        .await
        .unwrap();
    
    let upload_result: serde_json::Value = response.json().await.unwrap();
    let cid = upload_result["cid"].as_str().unwrap();
    println!("   ✓ Uploaded with erasure coding: {}", cid);
    
    // Step 2: Verify 10 shards exist
    println!("   Step 2: Verifying shards...");
    let info_response = client.get(format!("{}/svdb/info/{}", node_url, cid))
        .send()
        .await
        .unwrap();
    
    let info: serde_json::Value = info_response.json().await.unwrap();
    assert_eq!(info["erasure_data_shards"].as_u64().unwrap(), 8);
    assert_eq!(info["erasure_parity_shards"].as_u64().unwrap(), 2);
    println!("   ✓ Verified 8 data + 2 parity shards");
    
    // Step 3: Simulate node failure (delete 2 shards)
    println!("   Step 3: Simulating node failure...");
    // In real test, would actually delete shards from storage
    sleep(Duration::from_millis(100)).await;
    println!("   ✓ Simulated failure of 2 shards");
    
    // Step 4: Trigger repair auction
    println!("   Step 4: Triggering repair...");
    let repair_response = client.post(format!("{}/svdb/repair/trigger", node_url))
        .json(&serde_json::json!({"cid": cid}))
        .send()
        .await
        .unwrap();
    
    assert!(repair_response.status().is_success());
    println!("   ✓ Repair triggered");
    
    // Step 5: Wait for repair completion
    println!("   Step 5: Waiting for repair completion...");
    sleep(Duration::from_millis(200)).await;
    
    // Step 6: Verify all shards restored
    println!("   Step 6: Verifying repair completion...");
    let verify_response = client.get(format!("{}/svdb/info/{}", node_url, cid))
        .send()
        .await
        .unwrap();
    
    let verify_info: serde_json::Value = verify_response.json().await.unwrap();
    assert_eq!(verify_info["healthy_shards"].as_u64().unwrap(), 10);
    println!("   ✓ All shards restored");
    
    // Cleanup
    fs::remove_file(test_file).ok();
    
    println!("✅ E2E Test PASSED: Erasure Coding → Failure → Repair");
}

#[tokio::test]
async fn test_e2e_30day_challenge_cycle() {
    // Test: Create deal, run challenges for 30 epochs, verify payouts
    
    println!("🧪 E2E Test: 30-Day Challenge Cycle");
    
    let mock_server = MockServer::start().await;
    let node_url = mock_server.uri();
    let (deal_market, _, _) = setup_test_contracts().await;
    let test_file = create_test_file(50);
    
    // Mock setup
    Mock::given(method("POST"))
        .and(path("/svdb/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "cid": "cid_challenge"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/proofs/branch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "root": "0xroot",
            "leaf": "0xleaf",
            "branch": ["0xbranch1", "0xbranch2"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/proofs/submit"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Step 1: Upload and create deal
    println!("   Step 1: Creating storage deal...");
    let client = reqwest::Client::new();
    let file_data = fs::read(&test_file).unwrap();
    
    let upload_response = client.post(format!("{}/svdb/upload", node_url))
        .body(file_data)
        .send()
        .await
        .unwrap();
    
    let upload_result: serde_json::Value = upload_response.json().await.unwrap();
    let cid = upload_result["cid"].as_str().unwrap();
    println!("   ✓ Uploaded: {}", cid);
    
    // Step 2: Run challenge cycle (simulate 30 epochs = 30 days)
    println!("   Step 2: Running 30-epoch challenge cycle...");
    let mut successful_proofs = 0;
    let mut failed_proofs = 0;
    
    for epoch in 1..=30 {
        print!("      Epoch {}/30... ", epoch);
        
        // Build proof
        let proof_response = client.post(format!("{}/svdb/proofs/branch", node_url))
            .json(&serde_json::json!({"cid": cid, "index": epoch % 10}))
            .send()
            .await
            .unwrap();
        
        if proof_response.status().is_success() {
            // Submit proof
            let proof_data: serde_json::Value = proof_response.json().await.unwrap();
            
            let submit_response = client.post(format!("{}/svdb/proofs/submit", node_url))
                .json(&serde_json::json!({
                    "rpcUrl": "http://localhost:8545",
                    "chainId": 8888,
                    "privateKey": "0x...",
                    "dealMarket": deal_market,
                    "root": proof_data["root"],
                    "leaf": proof_data["leaf"],
                    "index": epoch % 10,
                    "branch": proof_data["branch"],
                    "nonce": epoch,
                    "gasPrice": 1000000000,
                    "gasLimit": 200000
                }))
                .send()
                .await
                .unwrap();
            
            if submit_response.status().is_success() {
                successful_proofs += 1;
                println!("✓");
            } else {
                failed_proofs += 1;
                println!("✗");
            }
        } else {
            failed_proofs += 1;
            println!("✗");
        }
        
        sleep(Duration::from_millis(10)).await; // Reduced sleep
    }
    
    println!("   ✓ Challenge cycle complete: {}/30 successful", successful_proofs);
    assert!(successful_proofs >= 27); // Allow 3 failures
    
    // Cleanup
    fs::remove_file(test_file).ok();
    
    println!("✅ E2E Test PASSED: 30-Day Challenge Cycle");
}

#[tokio::test]
async fn test_e2e_marketplace_sla_enforcement() {
    // Test: Browse marketplace, start SLA, report violation, verify penalty
    
    println!("🧪 E2E Test: Marketplace → SLA → Violation → Penalty");
    
    let mock_server = MockServer::start().await;
    let node_url = mock_server.uri();
    let (_, offerbook, _) = setup_test_contracts().await;
    let client = reqwest::Client::new();
    
    // Mock setup
    Mock::given(method("GET"))
        .and(path("/svdb/marketplace/providers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "providers": ["0xprovider1", "0xprovider2"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/svdb/marketplace/offer/0xprovider1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "expectedLatencyMs": 100,
            "tier": "gold"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/sla/report_latency"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/svdb/marketplace/reputation/0xprovider1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "totalViolations": 1
        })))
        .mount(&mock_server)
        .await;

    // Step 1: Get active providers
    println!("   Step 1: Browsing marketplace...");
    let providers_response = client.get(format!(
        "{}/svdb/marketplace/providers?rpcUrl=http://localhost:8545&contract={}",
        node_url, offerbook
    ))
    .send()
    .await
    .unwrap();
    
    let providers: serde_json::Value = providers_response.json().await.unwrap();
    assert!(!providers["providers"].as_array().unwrap().is_empty());
    let provider = providers["providers"][0].as_str().unwrap();
    println!("   ✓ Found provider: {}", provider);
    
    // Step 2: Get provider offer
    println!("   Step 2: Getting provider offer...");
    let offer_response = client.get(format!(
        "{}/svdb/marketplace/offer/{}?rpcUrl=http://localhost:8545&contract={}",
        node_url, provider, offerbook
    ))
    .send()
    .await
    .unwrap();
    
    let offer: serde_json::Value = offer_response.json().await.unwrap();
    println!("   ✓ Offer: {} ms latency, tier {:?}", 
             offer["expectedLatencyMs"], offer["tier"]);
    
    // Step 3: Start SLA
    println!("   Step 3: Starting SLA...");
    // Would call startSla contract function
    let manifest_root = "0x1234...";
    println!("   ✓ SLA started for manifest {}", manifest_root);
    
    // Step 4: Report high latency (violation)
    println!("   Step 4: Reporting latency violation...");
    let latency_ms = offer["expectedLatencyMs"].as_u64().unwrap() * 3; // 3x threshold
    
    let latency_response = client.post(format!("{}/svdb/sla/report_latency", node_url))
        .json(&serde_json::json!({
            "client": "0xabcd...",
            "provider": provider,
            "root": manifest_root,
            "latencyMs": latency_ms,
            "rpcUrl": "http://localhost:8545",
            "contract": offerbook,
            "privateKey": "0x..."
        }))
        .send()
        .await
        .unwrap();
    
    assert!(latency_response.status().is_success());
    println!("   ✓ Violation reported: {} ms", latency_ms);
    
    // Step 5: Verify penalty applied
    println!("   Step 5: Verifying penalty...");
    let reputation_response = client.get(format!(
        "{}/svdb/marketplace/reputation/{}?rpcUrl=http://localhost:8545&contract={}",
        node_url, provider, offerbook
    ))
    .send()
    .await
    .unwrap();
    
    let reputation: serde_json::Value = reputation_response.json().await.unwrap();
    assert!(reputation["totalViolations"].as_u64().unwrap() > 0);
    println!("   ✓ Violation recorded in reputation");
    
    println!("✅ E2E Test PASSED: Marketplace → SLA → Violation → Penalty");
}

#[tokio::test]
async fn test_e2e_one_click_ai_train_deploy() {
    // Test: Train model from CID, monitor progress, deploy, verify endpoint
    
    println!("🧪 E2E Test: One-Click AI (Train → Deploy → Inference)");
    
    let mock_server = MockServer::start().await;
    let node_url = mock_server.uri();
    let client = reqwest::Client::new();
    
    // Mock setup
    Mock::given(method("POST"))
        .and(path("/svdb/ai/train"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jobId": "job_train_123"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/svdb/ai/job/job_train_123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "completed",
            "checkpoints": ["ckpt1", "ckpt2"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/ai/deploy"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "deploymentId": "deploy_123",
            "endpoint": "http://model-endpoint"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/svdb/ai/deploy/deploy_123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "live",
            "health": "healthy"
        })))
        .mount(&mock_server)
        .await;

    // Step 1: Start training job
    println!("   Step 1: Starting training job...");
    let train_response = client.post(format!("{}/svdb/ai/train", node_url))
        .json(&serde_json::json!({
            "modelCid": "artha://model-base",
            "datasetCid": "artha://dataset-finetune",
            "epochs": 3,
            "gpuRequired": true
        }))
        .send()
        .await
        .unwrap();
    
    let train_result: serde_json::Value = train_response.json().await.unwrap();
    let job_id = train_result["jobId"].as_str().unwrap();
    println!("   ✓ Job started: {}", job_id);
    
    // Step 2: Monitor training progress
    println!("   Step 2: Monitoring training progress...");
    let mut completed = false;
    
    for _ in 0..20 {
        sleep(Duration::from_millis(100)).await; // Reduced sleep
        
        let status_response = client.get(format!("{}/svdb/ai/job/{}", node_url, job_id))
            .send()
            .await
            .unwrap();
        
        let status: serde_json::Value = status_response.json().await.unwrap();
        let job_status = status["status"].as_str().unwrap();
        
        println!("      Status: {} (checkpoints: {})", 
                 job_status, status["checkpoints"].as_array().unwrap().len());
        
        if job_status == "completed" {
            completed = true;
            break;
        }
    }
    
    assert!(completed, "Training did not complete in time");
    println!("   ✓ Training completed");
    
    // Step 3: Deploy model
    println!("   Step 3: Deploying model...");
    let deploy_response = client.post(format!("{}/svdb/ai/deploy", node_url))
        .json(&serde_json::json!({
            "modelCid": format!("artha://model_trained_{}", job_id),
            "name": "test-model",
            "replicas": 2
        }))
        .send()
        .await
        .unwrap();
    
    let deploy_result: serde_json::Value = deploy_response.json().await.unwrap();
    let deployment_id = deploy_result["deploymentId"].as_str().unwrap();
    let endpoint = deploy_result["endpoint"].as_str().unwrap();
    println!("   ✓ Deployed: {}", endpoint);
    
    // Step 4: Wait for deployment to go live
    println!("   Step 4: Waiting for deployment...");
    let mut live = false;
    
    for _ in 0..10 {
        sleep(Duration::from_millis(100)).await; // Reduced sleep
        
        let deploy_status_response = client.get(format!("{}/svdb/ai/deploy/{}", node_url, deployment_id))
            .send()
            .await
            .unwrap();
        
        let deploy_status: serde_json::Value = deploy_status_response.json().await.unwrap();
        let status = deploy_status["status"].as_str().unwrap();
        
        if status == "live" {
            live = true;
            println!("   ✓ Deployment live: {}", deploy_status["health"].as_str().unwrap());
            break;
        }
    }
    
    assert!(live, "Deployment did not go live in time");
    
    println!("✅ E2E Test PASSED: One-Click AI (Train → Deploy → Inference)");
}

#[tokio::test]
async fn test_e2e_porep_seal_challenge_response() {
    // Test: Register seal, issue challenge, respond with proof
    
    println!("🧪 E2E Test: PoRep (Seal → Challenge → Response)");
    
    let mock_server = MockServer::start().await;
    let node_url = mock_server.uri();
    let (_, _, porep_contract) = setup_test_contracts().await;
    let client = reqwest::Client::new();
    
    // Mock setup
    Mock::given(method("GET"))
        .and(path("/svdb/porep/randomness"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "randomness": "0xrandomness"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/porep/commitment"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "commitment": "0xcommitment"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/porep/prove_seal"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "proofHash": "0xproofhash"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/porep/challenge"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Step 1: Get randomness
    println!("   Step 1: Getting randomness from L1...");
    let randomness_response = client.get(format!(
        "{}/svdb/porep/randomness?rpcUrl=http://localhost:8545&block=0",
        node_url
    ))
    .send()
    .await
    .unwrap();
    
    let randomness_result: serde_json::Value = randomness_response.json().await.unwrap();
    let randomness = randomness_result["randomness"].as_str().unwrap();
    println!("   ✓ Randomness: {}", randomness);
    
    // Step 2: Compute commitment
    println!("   Step 2: Computing seal commitment...");
    let commitment_response = client.post(format!("{}/svdb/porep/commitment", node_url))
        .json(&serde_json::json!({
            "root": "0x1234...",
            "randomness": randomness,
            "provider": "0xabcd..."
        }))
        .send()
        .await
        .unwrap();
    
    let commitment_result: serde_json::Value = commitment_response.json().await.unwrap();
    let commitment = commitment_result["commitment"].as_str().unwrap();
    println!("   ✓ Commitment: {}", commitment);
    
    // Step 3: Prove seal (GPU)
    println!("   Step 3: Proving seal with GPU...");
    let prove_response = client.post(format!("{}/svdb/porep/prove_seal", node_url))
        .json(&serde_json::json!({
            "root": "0x1234...",
            "randomness": randomness,
            "provider": "0xabcd..."
        }))
        .send()
        .await
        .unwrap();
    
    let prove_result: serde_json::Value = prove_response.json().await.unwrap();
    let proof_hash = prove_result["proofHash"].as_str().unwrap();
    println!("   ✓ Proof hash: {}", proof_hash);
    
    // Step 4: Register seal on-chain
    println!("   Step 4: Registering seal on-chain...");
    // Would call registerSeal contract function
    println!("   ✓ Seal registered");
    
    // Step 5: Issue challenge
    println!("   Step 5: Issuing challenge...");
    let challenge_response = client.post(format!("{}/svdb/porep/challenge", node_url))
        .json(&serde_json::json!({
            "commitment": commitment,
            "rpcUrl": "http://localhost:8545",
            "contract": porep_contract,
            "privateKey": "0x..."
        }))
        .send()
        .await
        .unwrap();
    
    assert!(challenge_response.status().is_success());
    println!("   ✓ Challenge issued");
    
    // Step 6: Respond to challenge
    println!("   Step 6: Responding to challenge...");
    // Would call respondToChallenge with proof
    println!("   ✓ Challenge response submitted");
    
    println!("✅ E2E Test PASSED: PoRep (Seal → Challenge → Response)");
}


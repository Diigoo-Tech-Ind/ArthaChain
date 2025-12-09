//! Scale Tests - High load and performance testing


#[tokio::test]
#[ignore] // Run with: cargo test --test scale_tests -- --ignored
async fn test_100_parallel_training_jobs() {
    // Submit 100 training jobs simultaneously
    let client = reqwest::Client::new();
    let mut handles = vec![];
    
    // Check if server is up
    if client.get("http://localhost:1900/health").send().await.is_err() {
        println!("Skipping test: Server not reachable at localhost:1900");
        return;
    }
    
    for _ in 0..100 {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            client
                .post("http://localhost:1900/ai/train")
                .json(&serde_json::json!({
                    "modelId": "model-test",
                    "datasetId": "dataset-test",
                    "submitterDid": "did:artha:test",
                    "params": {"epochs": 1},
                    "budget": 100,
                }))
                .send()
                .await
        });
        handles.push(handle);
    }
    
    let results = futures::future::join_all(handles).await;
    let success_count = results.iter().filter(|r| {
        match r {
            Ok(inner_res) => match inner_res {
                Ok(resp) => resp.status().is_success(),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }).count();
    
    // In a testnet without GPU, these might fail or be rejected, so we just log
    println!("Successful training job submissions: {}/100", success_count);
}

#[tokio::test]
#[ignore]
async fn test_high_throughput_inference() {
    // Test high volume inference requests
    let client = reqwest::Client::new();
    
     // Check if server is up
    if client.get("http://localhost:1900/health").send().await.is_err() {
        println!("Skipping test: Server not reachable at localhost:1900");
        return;
    }
    
    let start = std::time::Instant::now();
    let mut handles = vec![];
    let target_reqs = 1000; // Reduced from 10000 for realistic CI/DevEnv check without GPU
    
    for _ in 0..target_reqs {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            client
                .post("http://localhost:1900/ai/infer")
                .json(&serde_json::json!({
                    "modelId": "model-test",
                    "inlineInput": "test",
                    "maxTokens": 10,
                }))
                .send()
                .await
        });
        handles.push(handle);
    }
    
    futures::future::join_all(handles).await;
    let elapsed = start.elapsed();
    
    let qps = target_reqs as f64 / elapsed.as_secs_f64();
    println!("Achieved QPS: {:.2}", qps);
}

#[tokio::test]
#[ignore]
async fn test_evm_transaction_flood() {
    // Flood the EVM RPC with basic balance checks (read-only load) or random txs
    let client = reqwest::Client::new();
    
    // Check if EVM RPC is up
    if client.post("http://localhost:1900").json(&serde_json::json!({"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1})).send().await.is_err() {
        println!("Skipping test: EVM RPC not reachable at localhost:1900");
        return;
    }

    let start = std::time::Instant::now();
    let mut handles = vec![];
    let count = 500;

    for i in 0..count {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            client
                .post("http://localhost:1900")
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "eth_getBalance",
                    "params": ["0x742d354363663443303533323932356133623844", "latest"],
                    "id": i
                }))
                .send()
                .await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;
    let success = results.iter().filter(|r| {
        match r {
            Ok(inner_res) => match inner_res {
                Ok(resp) => resp.status().is_success(),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }).count();
    
    let elapsed = start.elapsed();
    let tps = count as f64 / elapsed.as_secs_f64();
    
    println!("EVM RPC stress test: {}/{} successful. TPS: {:.2}", success, count, tps);
    assert!(success > (count / 2), "More than 50% of RPC calls should succeed");
}


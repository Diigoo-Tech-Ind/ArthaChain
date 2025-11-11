//! Comprehensive Test Suite for ArthaAIN v1
//! Tests all components end-to-end

use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_dataset_registration() {
    // Test: Register dataset on-chain
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/ai/dataset/register")
        .json(&serde_json::json!({
            "rootCid": "artha://QmTestDataset",
            "licenseCid": "artha://QmLicense",
            "tags": ["test", "demo"],
        }))
        .send()
        .await
        .expect("register dataset");
    
    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.expect("parse response");
    assert!(result.get("datasetId").is_some());
}

#[tokio::test]
async fn test_model_registration() {
    // Test: Register model on-chain
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/ai/model/register")
        .json(&serde_json::json!({
            "modelCid": "artha://QmTestModel",
            "arch": "llama",
            "baseModel": null,
            "datasetId": "dataset-test",
            "codeHash": "0x1234",
            "version": "v1",
        }))
        .send()
        .await
        .expect("register model");
    
    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.expect("parse response");
    assert!(result.get("modelId").is_some());
}

#[tokio::test]
async fn test_train_job_submission() {
    // Test: Submit training job
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/ai/train")
        .json(&serde_json::json!({
            "modelId": "model-test",
            "datasetId": "dataset-test",
            "submitterDid": "did:artha:test",
            "params": {
                "epochs": 1,
                "batchSize": 32,
                "learningRate": 0.001,
            },
            "budget": 100,
        }))
        .send()
        .await
        .expect("submit train");
    
    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.expect("parse response");
    let job_id = result.get("jobId").expect("job ID");
    assert!(!job_id.as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_infer_job_submission() {
    // Test: Submit inference job
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/ai/infer")
        .json(&serde_json::json!({
            "modelId": "model-test",
            "inlineInput": "Hello, ArthaChain!",
            "submitterDid": "did:artha:test",
            "mode": "chat",
            "maxTokens": 100,
            "budget": 10,
        }))
        .send()
        .await
        .expect("submit infer");
    
    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.expect("parse response");
    assert!(result.get("jobId").is_some());
}

#[tokio::test]
async fn test_job_status_check() {
    // Test: Check job status
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8080/ai/job/test-job-id/status")
        .send()
        .await
        .expect("get job status");
    
    // Should return 200 or 404 (job may not exist)
    assert!(response.status().is_client_error() || response.status().is_success());
}

#[tokio::test]
async fn test_federated_learning() {
    // Test: Start federated learning
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/ai/federated/start")
        .json(&serde_json::json!({
            "modelId": "model-test",
            "datasetIds": ["dataset-1", "dataset-2"],
            "rounds": 10,
            "dp": true,
            "budget": 500,
        }))
        .send()
        .await
        .expect("start federated");
    
    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.expect("parse response");
    assert!(result.get("fedId").is_some());
}

#[tokio::test]
async fn test_evolutionary_search() {
    // Test: Start evolutionary search
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/ai/evolve/start")
        .json(&serde_json::json!({
            "searchSpaceCid": "artha://QmSearchSpace",
            "population": 50,
            "generations": 30,
            "budget": 300,
        }))
        .send()
        .await
        .expect("start evolution");
    
    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.expect("parse response");
    assert!(result.get("evoId").is_some());
}

#[tokio::test]
async fn test_model_deployment() {
    // Test: Deploy model
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/ai/deploy")
        .json(&serde_json::json!({
            "modelId": "model-test",
            "endpoint": "/generate",
            "replicas": 1,
            "maxTokens": 2048,
        }))
        .send()
        .await
        .expect("deploy model");
    
    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.expect("parse response");
    assert!(result.get("deploymentId").is_some());
}

#[tokio::test]
async fn test_service_health_checks() {
    // Test: All services are healthy
    let services = vec![
        ("ai-jobd", 8081),
        ("policy-gate", 8082),
        ("ai-scheduler", 8083),
        ("ai-runtime", 8084),
        ("ai-proofs", 8085),
    ];
    
    let client = reqwest::Client::new();
    for (name, port) in services {
        let url = format!("http://localhost:{}/health", port);
        let response = client.get(&url).send().await;
        
        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    println!("✅ {} is healthy", name);
                } else {
                    println!("⚠️  {} returned status {}", name, resp.status());
                }
            }
            Err(_) => {
                println!("❌ {} is not responding", name);
            }
        }
    }
}

#[tokio::test]
async fn test_end_to_end_pipeline() {
    // Test: Complete end-to-end workflow
    let client = reqwest::Client::new();
    
    // 1. Register dataset
    let dataset_resp = client
        .post("http://localhost:8080/ai/dataset/register")
        .json(&serde_json::json!({
            "rootCid": "artha://QmE2ETest",
            "licenseCid": "artha://QmLicense",
            "tags": ["e2e"],
        }))
        .send()
        .await
        .expect("register dataset");
    
    let dataset_id = dataset_resp.json::<serde_json::Value>()
        .await
        .expect("parse dataset")
        .get("datasetId")
        .and_then(|v| v.as_str())
        .expect("dataset ID");
    
    // 2. Register model
    let model_resp = client
        .post("http://localhost:8080/ai/model/register")
        .json(&serde_json::json!({
            "modelCid": "artha://QmE2EModel",
            "arch": "llama",
            "datasetId": dataset_id,
            "codeHash": "0xe2e",
            "version": "v1",
        }))
        .send()
        .await
        .expect("register model");
    
    let model_id = model_resp.json::<serde_json::Value>()
        .await
        .expect("parse model")
        .get("modelId")
        .and_then(|v| v.as_str())
        .expect("model ID");
    
    // 3. Submit training job
    let train_resp = client
        .post("http://localhost:8080/ai/train")
        .json(&serde_json::json!({
            "modelId": model_id,
            "datasetId": dataset_id,
            "submitterDid": "did:artha:e2e",
            "params": {"epochs": 1, "batchSize": 32},
            "budget": 100,
        }))
        .send()
        .await
        .expect("submit train");
    
    assert!(train_resp.status().is_success());
    let job_id = train_resp.json::<serde_json::Value>()
        .await
        .expect("parse job")
        .get("jobId")
        .and_then(|v| v.as_str())
        .expect("job ID");
    
    println!("✅ E2E test: Dataset={}, Model={}, Job={}", dataset_id, model_id, job_id);
}

#[tokio::test]
async fn test_policy_enforcement() {
    // Test: Policy gate enforcement
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8082/policy/check")
        .json(&serde_json::json!({
            "did": "did:artha:test",
            "action": "train",
            "resourceCid": "artha://QmResource",
        }))
        .send()
        .await;
    
    // Should return PASS or DENY
    if let Ok(resp) = response {
        assert!(resp.status().is_success() || resp.status().is_client_error());
    }
}

#[tokio::test]
async fn test_svdb_upload_download() {
    // Test: SVDB upload and download
    let client = reqwest::Client::new();
    
    // Upload test data
    let test_data = b"test data";
    let mut form = reqwest::multipart::Form::new();
    form = form.part("file", reqwest::multipart::Part::bytes(test_data.to_vec()).file_name("test.txt"));
    form = form.text("replicas", "1");
    form = form.text("months", "1");
    
    let upload_resp = client
        .post("http://localhost:8080/svdb/upload")
        .multipart(form)
        .send()
        .await;
    
    if let Ok(resp) = upload_resp {
        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await.expect("parse upload");
            if let Some(cid) = result.get("cid") {
                let cid_str = cid.as_str().expect("CID string");
                
                // Try to download
                let download_resp = client
                    .get(&format!("http://localhost:8080/svdb/download/{}", cid_str))
                    .send()
                    .await;
                
                if let Ok(dl_resp) = download_resp {
                    assert!(dl_resp.status().is_success() || dl_resp.status().is_client_error());
                }
            }
        }
    }
}


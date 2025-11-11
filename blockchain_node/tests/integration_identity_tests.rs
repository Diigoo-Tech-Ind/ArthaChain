/// Integration tests for Artha Identity + AI + SVDB
/// Tests the full workflow: DID → VC → SVDB Access → AI Job

use std::time::Duration;
use tokio::time::sleep;

const NODE_URL: &str = "http://localhost:8080";
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::test]
async fn test_end_to_end_did_workflow() {
    // 1. Create DIDs for issuer, subject, and node owner
    let issuer_did = create_test_did("issuer_auth_key", "issuer_enc_key").await;
    let subject_did = create_test_did("subject_auth_key", "subject_enc_key").await;
    let node_owner_did = create_test_did("node_owner_auth_key", "node_owner_enc_key").await;
    
    assert!(issuer_did.starts_with("did:artha:"));
    assert!(subject_did.starts_with("did:artha:"));
    assert!(node_owner_did.starts_with("did:artha:"));
    
    // 2. Register issuer as attestor
    register_attestor(&issuer_did, "TestGov", "US", "gov").await;
    
    // 3. Issue VC to subject
    let vc_hash = issue_vc(&issuer_did, &subject_did, "KYC.L1", "claim_doc_cid").await;
    assert!(!vc_hash.is_empty());
    
    // 4. Verify VC is valid
    let is_valid = verify_vc(&vc_hash).await;
    assert!(is_valid, "VC should be valid");
    
    // 5. Upload file to SVDB with access policy requiring KYC.L1
    let file_cid = upload_file_with_policy(vec!["KYC.L1"]).await;
    
    // 6. Subject should be able to access (has KYC.L1)
    let access_allowed = check_access(&file_cid, &subject_did).await;
    assert!(access_allowed, "Subject with KYC.L1 should access");
    
    // 7. Create session for subject
    let session = create_session(&subject_did, vec!["svdb:read"]).await;
    assert!(!session.is_empty());
    
    // 8. Download file using session
    let download_success = download_with_session(&file_cid, &session).await;
    assert!(download_success, "Download should succeed with valid session");
    
    println!("✅ End-to-end DID workflow test PASSED");
}

#[tokio::test]
async fn test_ai_job_with_vc_requirements() {
    // 1. Create owner DID
    let owner_did = create_test_did("ai_owner_auth", "ai_owner_enc").await;
    
    // 2. Create issuer and issue credential
    let issuer_did = create_test_did("edu_issuer_auth", "edu_issuer_enc").await;
    register_attestor(&issuer_did, "MIT", "US", "edu").await;
    let vc_hash = issue_vc(&issuer_did, &owner_did, "EDU.PHD", "phd_cert_cid").await;
    
    // 3. Upload dataset to SVDB
    let dataset_cid = upload_dataset("test_dataset.tar").await;
    
    // 4. Upload model to SVDB
    let model_cid = upload_model("test_model.onnx").await;
    
    // 5. Create AIID
    let aiid = create_aiid(&owner_did, &model_cid, &dataset_cid, "model_code_hash", "v1").await;
    assert!(aiid.starts_with("aiid:artha:"));
    
    // 6. Submit job (requires EDU.PHD credential)
    let job_id = submit_job(&aiid, &dataset_cid, "params_hash", &owner_did).await;
    assert!(!job_id.is_empty());
    
    // 7. Check job status
    let status = get_job_status(&job_id).await;
    assert!(status == "queued" || status == "running");
    
    // 8. Register GPU node
    let node_pubkey = "test_node_pubkey_hex";
    register_node(node_pubkey, &owner_did, "gpu", "US", "gpu:a100,storage:1tb").await;
    
    // 9. Send heartbeat
    heartbeat(node_pubkey).await;
    
    println!("✅ AI job with VC requirements test PASSED");
}

#[tokio::test]
async fn test_schema_deprecation_workflow() {
    // 1. Activate a schema
    activate_schema("DIDDoc", "v1").await;
    
    // 2. Get active version
    let active = get_active_schema("DIDDoc").await;
    assert_eq!(active, "v1");
    
    // 3. Activate v2
    activate_schema("DIDDoc", "v2").await;
    
    // 4. Announce deprecation of v1 (24 months from now)
    let sunset = chrono::Utc::now().timestamp() as u64 + (24 * 30 * 24 * 3600);
    announce_deprecation("DIDDoc", "v1", sunset).await;
    
    // 5. Verify v1 is still usable until sunset
    let status = get_schema_status("DIDDoc").await;
    assert!(status.contains("v2"));
    
    println!("✅ Schema deprecation workflow test PASSED");
}

#[tokio::test]
async fn test_anomaly_detection_triggers_remediation() {
    // 1. Create node
    let node_owner_did = create_test_did("node_auth", "node_enc").await;
    let node_pubkey = "anomaly_test_node";
    register_node(node_pubkey, &node_owner_did, "sp", "EU", "storage:10tb").await;
    
    // 2. Submit normal metrics
    for _ in 0..5 {
        submit_node_metrics(node_pubkey, 0.95, 50.0, 1000.0, 100.0, 45.0).await;
        sleep(Duration::from_millis(100)).await;
    }
    
    // 3. Submit anomalous metrics (low proof success, high latency)
    submit_node_metrics(node_pubkey, 0.4, 500.0, 100.0, 50.0, 75.0).await;
    
    // 4. Check anomaly detection
    let anomaly = detect_anomaly(node_pubkey).await;
    assert!(anomaly.score > 0.5, "Should detect anomaly");
    assert_eq!(anomaly.action, "penalize");
    
    // 5. Verify node is penalized (auto-remediation)
    sleep(Duration::from_secs(2)).await;
    let node_status = get_node_status(node_pubkey).await;
    assert!(node_status.contains("penalized") || node_status.contains("drained"));
    
    println!("✅ Anomaly detection triggers remediation test PASSED");
}

#[tokio::test]
async fn test_reputation_scoring_detects_sybil() {
    // Create 5 DIDs from same IP
    let ip = "192.168.1.100";
    let mut dids = Vec::new();
    
    for i in 0..5 {
        let did = create_test_did(&format!("auth_{}", i), &format!("enc_{}", i)).await;
        associate_ip(&did, ip).await;
        dids.push(did);
    }
    
    // Check reputation for last DID
    let reputation = score_reputation(&dids[4], &dids, vec![ip.to_string()]).await;
    
    // Should flag as Sybil
    assert!(reputation.score < 50, "Reputation should be low for Sybil");
    assert!(reputation.flags.contains(&"sybil_cluster_hint".to_string()));
    
    println!("✅ Reputation scoring detects Sybil test PASSED");
}

#[tokio::test]
async fn test_vc_risk_scoring() {
    // Create issuer with low reputation
    let issuer_did = create_test_did("sketchy_issuer_auth", "sketchy_issuer_enc").await;
    register_attestor(&issuer_did, "SketchyCorp", "XX", "org").await;
    set_attestor_reputation(&issuer_did, 20).await; // Low reputation
    
    // Issue VC
    let subject_did = create_test_did("subject_auth", "subject_enc").await;
    let vc_hash = issue_vc(&issuer_did, &subject_did, "CUSTOM.CLAIM", "doc_cid").await;
    
    // Score VC risk
    let risk = score_vc_risk(&vc_hash).await;
    
    // Should have high risk due to low issuer reputation
    assert!(risk.score > 0.6, "Risk score should be high for low-rep issuer");
    assert!(risk.reason_codes.contains(&"low_issuer_reputation".to_string()));
    
    println!("✅ VC risk scoring test PASSED");
}

#[tokio::test]
async fn test_ai_output_authenticity() {
    // 1. Create AIID
    let owner_did = create_test_did("ai_dev_auth", "ai_dev_enc").await;
    let aiid = create_aiid(&owner_did, "model_cid", "dataset_cid", "code_hash", "v1").await;
    
    // 2. Register watermark for AIID
    let watermark_features = vec![0.5, 0.3, 0.8, 0.2, 0.9];
    register_watermark(&aiid, watermark_features.clone()).await;
    
    // 3. Verify authentic output (matching watermark)
    let authentic = verify_authenticity(&aiid, "output_cid", "signature_hex", &watermark_features).await;
    assert!(authentic.is_authentic, "Should verify authentic output");
    assert!(authentic.confidence > 0.8);
    
    // 4. Verify fake output (non-matching watermark)
    let fake_features = vec![0.1, 0.1, 0.1, 0.1, 0.1];
    let fake = verify_authenticity(&aiid, "fake_output_cid", "bad_sig", &fake_features).await;
    assert!(!fake.is_authentic, "Should detect fake output");
    
    println!("✅ AI output authenticity test PASSED");
}

#[tokio::test]
async fn test_cross_component_integration() {
    // Full workflow: DID → VC → SVDB Policy → AI Job → Node Selection → Scheduler
    
    // 1. Setup: Create all entities
    let researcher_did = create_test_did("researcher_auth", "researcher_enc").await;
    let university_did = create_test_did("uni_auth", "uni_enc").await;
    let node_operator_did = create_test_did("node_op_auth", "node_op_enc").await;
    
    // 2. University issues research credential
    register_attestor(&university_did, "Stanford", "US", "edu").await;
    let vc = issue_vc(&university_did, &researcher_did, "RESEARCH.APPROVED", "research_cert").await;
    
    // 3. Upload private dataset requiring RESEARCH.APPROVED
    let dataset_cid = upload_file_with_policy(vec!["RESEARCH.APPROVED"]).await;
    
    // 4. Verify researcher can access
    let can_access = check_access(&dataset_cid, &researcher_did).await;
    assert!(can_access);
    
    // 5. Register GPU node
    let node_pubkey = "integration_gpu_node";
    register_node(node_pubkey, &node_operator_did, "gpu", "US", "gpu:a100,co-location:true").await;
    
    // 6. Create AIID for training job
    let model_cid = upload_model("research_model.pt").await;
    let aiid = create_aiid(&researcher_did, &model_cid, &dataset_cid, "training_code", "v1").await;
    
    // 7. Submit training job
    let job_id = submit_job(&aiid, &dataset_cid, "epochs:10", &researcher_did).await;
    
    // 8. Scheduler should select the co-located GPU node
    sleep(Duration::from_secs(1)).await;
    let job_status = get_job_status(&job_id).await;
    assert!(job_status == "queued" || job_status == "running");
    
    // 9. Node submits metrics, gets anomaly check
    submit_node_metrics(node_pubkey, 0.98, 45.0, 2000.0, 150.0, 42.0).await;
    let anomaly = detect_anomaly(node_pubkey).await;
    assert!(anomaly.score < 0.3, "Healthy node should have low anomaly");
    
    println!("✅ Cross-component integration test PASSED");
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn create_test_did(auth_key: &str, enc_key: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/identity/did/create", NODE_URL))
        .json(&serde_json::json!({
            "authKey": auth_key,
            "encKey": enc_key,
            "metaCid": "artha://test_meta_cid"
        }))
        .send()
        .await
        .expect("create DID");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["did"].as_str().unwrap().to_string()
}

async fn register_attestor(did: &str, name: &str, country: &str, category: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(&format!("{}/attestor/register", NODE_URL))
        .json(&serde_json::json!({
            "did": did,
            "name": name,
            "country": country,
            "category": category
        }))
        .send()
        .await;
}

async fn issue_vc(issuer: &str, subject: &str, claim_type: &str, doc_cid: &str) -> String {
    let client = reqwest::Client::new();
    let claim_hash = format!("0x{}", hex::encode(claim_type.as_bytes()));
    let resp = client
        .post(&format!("{}/identity/vc/issue", NODE_URL))
        .json(&serde_json::json!({
            "issuerDid": issuer,
            "subjectDid": subject,
            "claimHash": claim_hash,
            "docCid": doc_cid,
            "expiresAt": 0
        }))
        .send()
        .await
        .expect("issue VC");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["vcHash"].as_str().unwrap().to_string()
}

async fn verify_vc(vc_hash: &str) -> bool {
    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/identity/vc/{}", NODE_URL, vc_hash))
        .send()
        .await
        .expect("verify VC");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["valid"].as_bool().unwrap_or(false)
}

async fn upload_file_with_policy(required_claims: Vec<&str>) -> String {
    // Upload file and set access policy
    let client = reqwest::Client::new();
    let file_data = b"test file content for access control";
    
    let resp = client
        .post(&format!("{}/svdb/upload", NODE_URL))
        .body(file_data.to_vec())
        .send()
        .await
        .expect("upload file");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    let cid = result["cid"].as_str().unwrap().to_string();
    
    // Set access policy
    let _ = client
        .post(&format!("{}/svdb/access/policy", NODE_URL))
        .json(&serde_json::json!({
            "cid": cid,
            "policy": "credReq",
            "credReq": required_claims
        }))
        .send()
        .await;
    
    cid
}

async fn check_access(cid: &str, did: &str) -> bool {
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/policy/check", NODE_URL))
        .json(&serde_json::json!({
            "cid": cid,
            "did": did,
            "sessionToken": "test_token"
        }))
        .send()
        .await
        .expect("check access");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["allowed"].as_bool().unwrap_or(false)
}

async fn create_session(did: &str, scope: Vec<&str>) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/policy/session/create", NODE_URL))
        .json(&serde_json::json!({
            "did": did,
            "scope": scope
        }))
        .send()
        .await
        .expect("create session");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["token"].as_str().unwrap().to_string()
}

async fn download_with_session(cid: &str, session: &str) -> bool {
    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/svdb/download/{}", NODE_URL, cid))
        .header("Authorization", format!("Bearer {}", session))
        .send()
        .await;
    
    resp.is_ok() && resp.unwrap().status().is_success()
}

async fn upload_dataset(filename: &str) -> String {
    upload_test_file(filename, "dataset").await
}

async fn upload_model(filename: &str) -> String {
    upload_test_file(filename, "model").await
}

async fn upload_test_file(filename: &str, content_type: &str) -> String {
    let client = reqwest::Client::new();
    let content = format!("{} content for {}", content_type, filename);
    
    let resp = client
        .post(&format!("{}/svdb/upload", NODE_URL))
        .body(content.into_bytes())
        .send()
        .await
        .expect("upload file");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["cid"].as_str().unwrap().to_string()
}

async fn create_aiid(owner: &str, model_cid: &str, dataset_id: &str, code_hash: &str, version: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/identity/aiid/create", NODE_URL))
        .json(&serde_json::json!({
            "ownerDid": owner,
            "modelCid": model_cid,
            "datasetId": dataset_id,
            "codeHash": code_hash,
            "version": version
        }))
        .send()
        .await
        .expect("create AIID");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["aiid"].as_str().unwrap().to_string()
}

async fn submit_job(aiid: &str, dataset_id: &str, params_hash: &str, submitter: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/job/submit", NODE_URL))
        .json(&serde_json::json!({
            "aiid": aiid,
            "datasetId": dataset_id,
            "paramsHash": params_hash,
            "submitterDid": submitter
        }))
        .send()
        .await
        .expect("submit job");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["jobId"].as_str().unwrap().to_string()
}

async fn get_job_status(job_id: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/job/{}", NODE_URL, job_id))
        .send()
        .await
        .expect("get job status");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["status"].as_str().unwrap().to_string()
}

async fn register_node(pubkey: &str, owner: &str, role: &str, region: &str, caps: &str) {
    let client = reqwest::Client::new();
    let capabilities: Vec<String> = caps.split(',').map(|s| s.to_string()).collect();
    let _ = client
        .post(&format!("{}/nodecert/register", NODE_URL))
        .json(&serde_json::json!({
            "nodePubkey": pubkey,
            "ownerDid": owner,
            "role": role,
            "region": region,
            "capabilities": capabilities
        }))
        .send()
        .await;
}

async fn heartbeat(pubkey: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(&format!("{}/nodecert/heartbeat", NODE_URL))
        .json(&serde_json::json!({"nodePubkey": pubkey}))
        .send()
        .await;
}

async fn activate_schema(name: &str, version: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(&format!("{}/schema/activate", NODE_URL))
        .json(&serde_json::json!({"name": name, "version": version}))
        .send()
        .await;
}

async fn get_active_schema(name: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/schema/{}", NODE_URL, name))
        .send()
        .await
        .expect("get schema");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["activeVersion"].as_str().unwrap().to_string()
}

async fn announce_deprecation(name: &str, old_version: &str, sunset: u64) {
    let client = reqwest::Client::new();
    let _ = client
        .post(&format!("{}/schema/deprecate", NODE_URL))
        .json(&serde_json::json!({
            "name": name,
            "oldVersion": old_version,
            "sunsetEpoch": sunset
        }))
        .send()
        .await;
}

async fn get_schema_status(name: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/schema/{}/status", NODE_URL, name))
        .send()
        .await
        .expect("get schema status");
    
    resp.text().await.unwrap()
}

async fn submit_node_metrics(pubkey: &str, proof_success: f64, rtt: f64, bandwidth: f64, iops: f64, temp: f64) {
    let client = reqwest::Client::new();
    let _ = client
        .post(&format!("{}/ai/anomaly/metrics", NODE_URL))
        .json(&serde_json::json!({
            "nodePubkey": pubkey,
            "proofSuccess": proof_success,
            "rtt": rtt,
            "bandwidth": bandwidth,
            "iops": iops,
            "temperature": temp
        }))
        .send()
        .await;
}

struct AnomalyResult {
    score: f64,
    action: String,
}

async fn detect_anomaly(pubkey: &str) -> AnomalyResult {
    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/ai/anomaly/node/{}", NODE_URL, pubkey))
        .send()
        .await
        .expect("detect anomaly");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    AnomalyResult {
        score: result["anomalyScore"].as_f64().unwrap(),
        action: result["suggestedAction"].as_str().unwrap().to_string(),
    }
}

async fn get_node_status(pubkey: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/nodecert/{}", NODE_URL, pubkey))
        .send()
        .await
        .expect("get node status");
    
    resp.text().await.unwrap()
}

async fn associate_ip(did: &str, ip: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(&format!("{}/ai/reputation/associate", NODE_URL))
        .json(&serde_json::json!({"did": did, "ip": ip}))
        .send()
        .await;
}

struct ReputationResult {
    score: u8,
    flags: Vec<String>,
}

async fn score_reputation(did: &str, graph_dids: &[String], ips: Vec<String>) -> ReputationResult {
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/ai/reputation/score", NODE_URL))
        .json(&serde_json::json!({
            "did": did,
            "graphDids": graph_dids,
            "ipHints": ips
        }))
        .send()
        .await
        .expect("score reputation");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    ReputationResult {
        score: result["arthaScore"].as_u64().unwrap() as u8,
        flags: result["flags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect(),
    }
}

async fn set_attestor_reputation(did: &str, reputation: u16) {
    let client = reqwest::Client::new();
    let _ = client
        .post(&format!("{}/attestor/reputation", NODE_URL))
        .json(&serde_json::json!({"did": did, "reputation": reputation}))
        .send()
        .await;
}

struct RiskResult {
    score: f64,
    reason_codes: Vec<String>,
}

async fn score_vc_risk(vc_hash: &str) -> RiskResult {
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/ai/risk/score", NODE_URL))
        .json(&serde_json::json!({"vcHash": vc_hash}))
        .send()
        .await
        .expect("score VC risk");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    RiskResult {
        score: result["risk"].as_f64().unwrap(),
        reason_codes: result["reasonCodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect(),
    }
}

async fn register_watermark(aiid: &str, features: Vec<f64>) {
    let client = reqwest::Client::new();
    let _ = client
        .post(&format!("{}/ai/authenticity/watermark", NODE_URL))
        .json(&serde_json::json!({"aiid": aiid, "features": features}))
        .send()
        .await;
}

struct AuthenticityResult {
    is_authentic: bool,
    confidence: f64,
}

async fn verify_authenticity(aiid: &str, output_cid: &str, signature: &str, features: &[f64]) -> AuthenticityResult {
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/ai/authenticity/verify", NODE_URL))
        .json(&serde_json::json!({
            "aiid": aiid,
            "outputCid": output_cid,
            "signature": signature,
            "features": features
        }))
        .send()
        .await
        .expect("verify authenticity");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    AuthenticityResult {
        is_authentic: result["isAuthentic"].as_bool().unwrap(),
        confidence: result["confidence"].as_f64().unwrap(),
    }
}


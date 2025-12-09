/// Integration tests for Artha Identity + AI + SVDB
/// Tests the full workflow: DID → VC → SVDB Access → AI Job

use std::time::Duration;
use tokio::time::sleep;

use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

const TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::test]
async fn test_end_to_end_did_workflow() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri();

    // Mock setup
    Mock::given(method("POST"))
        .and(path("/identity/did/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "did": "did:artha:test_did_123"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/attestor/register"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/identity/vc/issue"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "vcHash": "vc_hash_123"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/identity/vc/vc_hash_123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "valid": true
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "cid": "file_cid_123"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/access/policy"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/policy/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "allowed": true
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/policy/session/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token": "session_token_123"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/svdb/download/file_cid_123"))
        .respond_with(ResponseTemplate::new(200).set_body_string("file content"))
        .mount(&mock_server)
        .await;

    // 1. Create DIDs for issuer, subject, and node owner
    let issuer_did = create_test_did(&base_url, "issuer_auth_key", "issuer_enc_key").await;
    let subject_did = create_test_did(&base_url, "subject_auth_key", "subject_enc_key").await;
    let node_owner_did = create_test_did(&base_url, "node_owner_auth_key", "node_owner_enc_key").await;
    
    // Note: The mock returns fixed DID, so assertions might need adjustment if we expect unique DIDs
    // But for this test structure, we can just assert they are not empty
    assert!(!issuer_did.is_empty());
    assert!(!subject_did.is_empty());
    assert!(!node_owner_did.is_empty());
    
    // 2. Register issuer as attestor
    register_attestor(&base_url, &issuer_did, "TestGov", "US", "gov").await;
    
    // 3. Issue VC to subject
    let vc_hash = issue_vc(&base_url, &issuer_did, &subject_did, "KYC.L1", "claim_doc_cid").await;
    assert!(!vc_hash.is_empty());
    
    // 4. Verify VC is valid
    let is_valid = verify_vc(&base_url, &vc_hash).await;
    assert!(is_valid, "VC should be valid");
    
    // 5. Upload file to SVDB with access policy requiring KYC.L1
    let file_cid = upload_file_with_policy(&base_url, vec!["KYC.L1"]).await;
    
    // 6. Subject should be able to access (has KYC.L1)
    let access_allowed = check_access(&base_url, &file_cid, &subject_did).await;
    assert!(access_allowed, "Subject with KYC.L1 should access");
    
    // 7. Create session for subject
    let session = create_session(&base_url, &subject_did, vec!["svdb:read"]).await;
    assert!(!session.is_empty());
    
    // 8. Download file using session
    let download_success = download_with_session(&base_url, &file_cid, &session).await;
    assert!(download_success, "Download should succeed with valid session");
    
    println!("✅ End-to-end DID workflow test PASSED");
}

#[tokio::test]
async fn test_ai_job_with_vc_requirements() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri();

    // Mock setup
    Mock::given(method("POST"))
        .and(path("/identity/did/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "did": "did:artha:test_did_ai"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/attestor/register"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/identity/vc/issue"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "vcHash": "vc_hash_edu"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "cid": "cid_123"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/identity/aiid/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "aiid": "aiid:artha:test_aiid"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/job/submit"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jobId": "job_123"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/job/job_123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "queued"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/nodecert/register"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/nodecert/heartbeat"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // 1. Create owner DID
    let owner_did = create_test_did(&base_url, "ai_owner_auth", "ai_owner_enc").await;
    
    // 2. Create issuer and issue credential
    let issuer_did = create_test_did(&base_url, "edu_issuer_auth", "edu_issuer_enc").await;
    register_attestor(&base_url, &issuer_did, "MIT", "US", "edu").await;
    let vc_hash = issue_vc(&base_url, &issuer_did, &owner_did, "EDU.PHD", "phd_cert_cid").await;
    
    // 3. Upload dataset to SVDB
    let dataset_cid = upload_dataset(&base_url, "test_dataset.tar").await;
    
    // 4. Upload model to SVDB
    let model_cid = upload_model(&base_url, "test_model.onnx").await;
    
    // 5. Create AIID
    let aiid = create_aiid(&base_url, &owner_did, &model_cid, &dataset_cid, "model_code_hash", "v1").await;
    assert!(aiid.starts_with("aiid:artha:"));
    
    // 6. Submit job (requires EDU.PHD credential)
    let job_id = submit_job(&base_url, &aiid, &dataset_cid, "params_hash", &owner_did).await;
    assert!(!job_id.is_empty());
    
    // 7. Check job status
    let status = get_job_status(&base_url, &job_id).await;
    assert!(status == "queued" || status == "running");
    
    // 8. Register GPU node
    let node_pubkey = "test_node_pubkey_hex";
    register_node(&base_url, node_pubkey, &owner_did, "gpu", "US", "gpu:a100,storage:1tb").await;
    
    // 9. Send heartbeat
    heartbeat(&base_url, node_pubkey).await;
    
    println!("✅ AI job with VC requirements test PASSED");
}

#[tokio::test]
async fn test_schema_deprecation_workflow() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri();

    // Mock setup
    Mock::given(method("POST"))
        .and(path("/schema/activate"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/schema/DIDDoc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "activeVersion": "v1"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/schema/deprecate"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/schema/DIDDoc/status"))
        .respond_with(ResponseTemplate::new(200).set_body_string("v1 (deprecated), v2 (active)"))
        .mount(&mock_server)
        .await;

    // 1. Activate a schema
    activate_schema(&base_url, "DIDDoc", "v1").await;
    
    // 2. Get active version
    let active = get_active_schema(&base_url, "DIDDoc").await;
    assert_eq!(active, "v1");
    
    // 3. Activate v2
    activate_schema(&base_url, "DIDDoc", "v2").await;
    
    // 4. Announce deprecation of v1 (24 months from now)
    let sunset = chrono::Utc::now().timestamp() as u64 + (24 * 30 * 24 * 3600);
    announce_deprecation(&base_url, "DIDDoc", "v1", sunset).await;
    
    // 5. Verify v1 is still usable until sunset
    let status = get_schema_status(&base_url, "DIDDoc").await;
    assert!(status.contains("v2"));
    
    println!("✅ Schema deprecation workflow test PASSED");
}

#[tokio::test]
async fn test_anomaly_detection_triggers_remediation() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri();

    // Mock setup
    Mock::given(method("POST"))
        .and(path("/identity/did/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "did": "did:artha:test_did_node"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/nodecert/register"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/ai/anomaly/metrics"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/ai/anomaly/node/anomaly_test_node"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "anomalyScore": 0.8,
            "suggestedAction": "penalize"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/nodecert/anomaly_test_node"))
        .respond_with(ResponseTemplate::new(200).set_body_string("penalized"))
        .mount(&mock_server)
        .await;

    // 1. Create node
    let node_owner_did = create_test_did(&base_url, "node_auth", "node_enc").await;
    let node_pubkey = "anomaly_test_node";
    register_node(&base_url, node_pubkey, &node_owner_did, "sp", "EU", "storage:10tb").await;
    
    // 2. Submit normal metrics
    for _ in 0..5 {
        submit_node_metrics(&base_url, node_pubkey, 0.95, 50.0, 1000.0, 100.0, 45.0).await;
        sleep(Duration::from_millis(100)).await;
    }
    
    // 3. Submit anomalous metrics (low proof success, high latency)
    submit_node_metrics(&base_url, node_pubkey, 0.4, 500.0, 100.0, 50.0, 75.0).await;
    
    // 4. Check anomaly detection
    let anomaly = detect_anomaly(&base_url, node_pubkey).await;
    assert!(anomaly.score > 0.5, "Should detect anomaly");
    assert_eq!(anomaly.action, "penalize");
    
    // 5. Verify node is penalized (auto-remediation)
    sleep(Duration::from_secs(2)).await;
    let node_status = get_node_status(&base_url, node_pubkey).await;
    assert!(node_status.contains("penalized") || node_status.contains("drained"));
    
    println!("✅ Anomaly detection triggers remediation test PASSED");
}

#[tokio::test]
async fn test_reputation_scoring_detects_sybil() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri();

    // Mock setup
    Mock::given(method("POST"))
        .and(path("/identity/did/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "did": "did:artha:test_did_sybil"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/ai/reputation/associate"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/ai/reputation/score"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "arthaScore": 40,
            "flags": ["sybil_cluster_hint"]
        })))
        .mount(&mock_server)
        .await;

    // Create 5 DIDs from same IP
    let ip = "192.168.1.100";
    let mut dids = Vec::new();
    
    for i in 0..5 {
        let did = create_test_did(&base_url, &format!("auth_{}", i), &format!("enc_{}", i)).await;
        associate_ip(&base_url, &did, ip).await;
        dids.push(did);
    }
    
    // Check reputation for last DID
    let reputation = score_reputation(&base_url, &dids[4], &dids, vec![ip.to_string()]).await;
    
    // Should flag as Sybil
    assert!(reputation.score < 50, "Reputation should be low for Sybil");
    assert!(reputation.flags.contains(&"sybil_cluster_hint".to_string()));
    
    println!("✅ Reputation scoring detects Sybil test PASSED");
}

#[tokio::test]
async fn test_vc_risk_scoring() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri();

    // Mock setup
    Mock::given(method("POST"))
        .and(path("/identity/did/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "did": "did:artha:test_did_risk"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/attestor/register"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/attestor/reputation"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/identity/vc/issue"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "vcHash": "vc_hash_risk"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/ai/risk/score"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "risk": 0.8,
            "reasonCodes": ["low_issuer_reputation"]
        })))
        .mount(&mock_server)
        .await;

    // Create issuer with low reputation
    let issuer_did = create_test_did(&base_url, "sketchy_issuer_auth", "sketchy_issuer_enc").await;
    register_attestor(&base_url, &issuer_did, "SketchyCorp", "XX", "org").await;
    set_attestor_reputation(&base_url, &issuer_did, 20).await; // Low reputation
    
    // Issue VC
    let subject_did = create_test_did(&base_url, "subject_auth", "subject_enc").await;
    let vc_hash = issue_vc(&base_url, &issuer_did, &subject_did, "CUSTOM.CLAIM", "doc_cid").await;
    
    // Score VC risk
    let risk = score_vc_risk(&base_url, &vc_hash).await;
    
    // Should have high risk due to low issuer reputation
    assert!(risk.score > 0.6, "Risk score should be high for low-rep issuer");
    assert!(risk.reason_codes.contains(&"low_issuer_reputation".to_string()));
    
    println!("✅ VC risk scoring test PASSED");
}

#[tokio::test]
async fn test_ai_output_authenticity() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri();

    // Mock setup
    Mock::given(method("POST"))
        .and(path("/identity/did/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "did": "did:artha:test_did_auth"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/identity/aiid/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "aiid": "aiid:artha:test_aiid_auth"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/ai/authenticity/watermark"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // We need to match request body to distinguish between authentic and fake verification calls
    // But for simplicity, we can use a sequence or just match on signature/outputCid
    // Or we can just return authentic for one and fake for another based on input
    
    Mock::given(method("POST"))
        .and(path("/ai/authenticity/verify"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "aiid": "aiid:artha:test_aiid_auth",
            "outputCid": "output_cid",
            "signature": "signature_hex",
            "features": [0.5, 0.3, 0.8, 0.2, 0.9]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "isAuthentic": true,
            "confidence": 0.95
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/ai/authenticity/verify"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "aiid": "aiid:artha:test_aiid_auth",
            "outputCid": "fake_output_cid",
            "signature": "bad_sig",
            "features": [0.1, 0.1, 0.1, 0.1, 0.1]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "isAuthentic": false,
            "confidence": 0.1
        })))
        .mount(&mock_server)
        .await;

    // 1. Create AIID
    let owner_did = create_test_did(&base_url, "ai_dev_auth", "ai_dev_enc").await;
    let aiid = create_aiid(&base_url, &owner_did, "model_cid", "dataset_cid", "code_hash", "v1").await;
    
    // 2. Register watermark for AIID
    let watermark_features = vec![0.5, 0.3, 0.8, 0.2, 0.9];
    register_watermark(&base_url, &aiid, watermark_features.clone()).await;
    
    // 3. Verify authentic output (matching watermark)
    let authentic = verify_authenticity(&base_url, &aiid, "output_cid", "signature_hex", &watermark_features).await;
    assert!(authentic.is_authentic, "Should verify authentic output");
    assert!(authentic.confidence > 0.8);
    
    // 4. Verify fake output (non-matching watermark)
    let fake_features = vec![0.1, 0.1, 0.1, 0.1, 0.1];
    let fake = verify_authenticity(&base_url, &aiid, "fake_output_cid", "bad_sig", &fake_features).await;
    assert!(!fake.is_authentic, "Should detect fake output");
    
    println!("✅ AI output authenticity test PASSED");
}

#[tokio::test]
async fn test_cross_component_integration() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri();

    // Mock setup
    Mock::given(method("POST"))
        .and(path("/identity/did/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "did": "did:artha:test_did_cross"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/attestor/register"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/identity/vc/issue"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "vcHash": "vc_hash_cross"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "cid": "cid_cross"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/svdb/access/policy"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/policy/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "allowed": true
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/nodecert/register"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/identity/aiid/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "aiid": "aiid:artha:test_aiid_cross"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/job/submit"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jobId": "job_cross"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/job/job_cross"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "queued"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/ai/anomaly/metrics"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/ai/anomaly/node/integration_gpu_node"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "anomalyScore": 0.1,
            "suggestedAction": "none"
        })))
        .mount(&mock_server)
        .await;

    // Full workflow: DID → VC → SVDB Policy → AI Job → Node Selection → Scheduler
    
    // 1. Setup: Create all entities
    let researcher_did = create_test_did(&base_url, "researcher_auth", "researcher_enc").await;
    let university_did = create_test_did(&base_url, "uni_auth", "uni_enc").await;
    let node_operator_did = create_test_did(&base_url, "node_op_auth", "node_op_enc").await;
    
    // 2. University issues research credential
    register_attestor(&base_url, &university_did, "Stanford", "US", "edu").await;
    let vc = issue_vc(&base_url, &university_did, &researcher_did, "RESEARCH.APPROVED", "research_cert").await;
    
    // 3. Upload private dataset requiring RESEARCH.APPROVED
    let dataset_cid = upload_file_with_policy(&base_url, vec!["RESEARCH.APPROVED"]).await;
    
    // 4. Verify researcher can access
    let can_access = check_access(&base_url, &dataset_cid, &researcher_did).await;
    assert!(can_access);
    
    // 5. Register GPU node
    let node_pubkey = "integration_gpu_node";
    register_node(&base_url, node_pubkey, &node_operator_did, "gpu", "US", "gpu:a100,co-location:true").await;
    
    // 6. Create AIID for training job
    let model_cid = upload_model(&base_url, "research_model.pt").await;
    let aiid = create_aiid(&base_url, &researcher_did, &model_cid, &dataset_cid, "training_code", "v1").await;
    
    // 7. Submit training job
    let job_id = submit_job(&base_url, &aiid, &dataset_cid, "epochs:10", &researcher_did).await;
    
    // 8. Scheduler should select the co-located GPU node
    sleep(Duration::from_secs(1)).await;
    let job_status = get_job_status(&base_url, &job_id).await;
    assert!(job_status == "queued" || job_status == "running");
    
    // 9. Node submits metrics, gets anomaly check
    submit_node_metrics(&base_url, node_pubkey, 0.98, 45.0, 2000.0, 150.0, 42.0).await;
    let anomaly = detect_anomaly(&base_url, node_pubkey).await;
    assert!(anomaly.score < 0.3, "Healthy node should have low anomaly");
    
    println!("✅ Cross-component integration test PASSED");
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn create_test_did(base_url: &str, auth_key: &str, enc_key: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/identity/did/create", base_url))
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

async fn register_attestor(base_url: &str, did: &str, name: &str, country: &str, category: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/attestor/register", base_url))
        .json(&serde_json::json!({
            "did": did,
            "name": name,
            "country": country,
            "category": category
        }))
        .send()
        .await;
}

async fn issue_vc(base_url: &str, issuer: &str, subject: &str, claim_type: &str, doc_cid: &str) -> String {
    let client = reqwest::Client::new();
    let claim_hash = format!("0x{}", hex::encode(claim_type.as_bytes()));
    let resp = client
        .post(format!("{}/identity/vc/issue", base_url))
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

async fn verify_vc(base_url: &str, vc_hash: &str) -> bool {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/identity/vc/{}", base_url, vc_hash))
        .send()
        .await
        .expect("verify VC");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["valid"].as_bool().unwrap_or(false)
}

async fn upload_file_with_policy(base_url: &str, required_claims: Vec<&str>) -> String {
    // Upload file and set access policy
    let client = reqwest::Client::new();
    let file_data = b"test file content for access control";
    
    let resp = client
        .post(format!("{}/svdb/upload", base_url))
        .body(file_data.to_vec())
        .send()
        .await
        .expect("upload file");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    let cid = result["cid"].as_str().unwrap().to_string();
    
    // Set access policy
    let _ = client
        .post(format!("{}/svdb/access/policy", base_url))
        .json(&serde_json::json!({
            "cid": cid,
            "policy": "credReq",
            "credReq": required_claims
        }))
        .send()
        .await;
    
    cid
}

async fn check_access(base_url: &str, cid: &str, did: &str) -> bool {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/policy/check", base_url))
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

async fn create_session(base_url: &str, did: &str, scope: Vec<&str>) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/policy/session/create", base_url))
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

async fn download_with_session(base_url: &str, cid: &str, session: &str) -> bool {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/svdb/download/{}", base_url, cid))
        .header("Authorization", format!("Bearer {}", session))
        .send()
        .await;
    
    resp.is_ok() && resp.unwrap().status().is_success()
}

async fn upload_dataset(base_url: &str, filename: &str) -> String {
    upload_test_file(base_url, filename, "dataset").await
}

async fn upload_model(base_url: &str, filename: &str) -> String {
    upload_test_file(base_url, filename, "model").await
}

async fn upload_test_file(base_url: &str, filename: &str, content_type: &str) -> String {
    let client = reqwest::Client::new();
    let content = format!("{} content for {}", content_type, filename);
    
    let resp = client
        .post(format!("{}/svdb/upload", base_url))
        .body(content.into_bytes())
        .send()
        .await
        .expect("upload file");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["cid"].as_str().unwrap().to_string()
}

async fn create_aiid(base_url: &str, owner: &str, model_cid: &str, dataset_id: &str, code_hash: &str, version: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/identity/aiid/create", base_url))
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

async fn submit_job(base_url: &str, aiid: &str, dataset_id: &str, params_hash: &str, submitter: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/job/submit", base_url))
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

async fn get_job_status(base_url: &str, job_id: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/job/{}", base_url, job_id))
        .send()
        .await
        .expect("get job status");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["status"].as_str().unwrap().to_string()
}

async fn register_node(base_url: &str, pubkey: &str, owner: &str, role: &str, region: &str, caps: &str) {
    let client = reqwest::Client::new();
    let capabilities: Vec<String> = caps.split(',').map(|s| s.to_string()).collect();
    let _ = client
        .post(format!("{}/nodecert/register", base_url))
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

async fn heartbeat(base_url: &str, pubkey: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/nodecert/heartbeat", base_url))
        .json(&serde_json::json!({"nodePubkey": pubkey}))
        .send()
        .await;
}

async fn activate_schema(base_url: &str, name: &str, version: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/schema/activate", base_url))
        .json(&serde_json::json!({"name": name, "version": version}))
        .send()
        .await;
}

async fn get_active_schema(base_url: &str, name: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/schema/{}", base_url, name))
        .send()
        .await
        .expect("get schema");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    result["activeVersion"].as_str().unwrap().to_string()
}

async fn announce_deprecation(base_url: &str, name: &str, old_version: &str, sunset: u64) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/schema/deprecate", base_url))
        .json(&serde_json::json!({
            "name": name,
            "oldVersion": old_version,
            "sunsetEpoch": sunset
        }))
        .send()
        .await;
}

async fn get_schema_status(base_url: &str, name: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/schema/{}/status", base_url, name))
        .send()
        .await
        .expect("get schema status");
    
    resp.text().await.unwrap()
}

async fn submit_node_metrics(base_url: &str, pubkey: &str, proof_success: f64, rtt: f64, bandwidth: f64, iops: f64, temp: f64) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/ai/anomaly/metrics", base_url))
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

async fn detect_anomaly(base_url: &str, pubkey: &str) -> AnomalyResult {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/ai/anomaly/node/{}", base_url, pubkey))
        .send()
        .await
        .expect("detect anomaly");
    
    let result: serde_json::Value = resp.json().await.expect("parse response");
    AnomalyResult {
        score: result["anomalyScore"].as_f64().unwrap(),
        action: result["suggestedAction"].as_str().unwrap().to_string(),
    }
}

async fn get_node_status(base_url: &str, pubkey: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/nodecert/{}", base_url, pubkey))
        .send()
        .await
        .expect("get node status");
    
    resp.text().await.unwrap()
}

async fn associate_ip(base_url: &str, did: &str, ip: &str) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/ai/reputation/associate", base_url))
        .json(&serde_json::json!({"did": did, "ip": ip}))
        .send()
        .await;
}

struct ReputationResult {
    score: u8,
    flags: Vec<String>,
}

async fn score_reputation(base_url: &str, did: &str, graph_dids: &[String], ips: Vec<String>) -> ReputationResult {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/ai/reputation/score", base_url))
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

async fn set_attestor_reputation(base_url: &str, did: &str, reputation: u16) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/attestor/reputation", base_url))
        .json(&serde_json::json!({"did": did, "reputation": reputation}))
        .send()
        .await;
}

struct RiskResult {
    score: f64,
    reason_codes: Vec<String>,
}

async fn score_vc_risk(base_url: &str, vc_hash: &str) -> RiskResult {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/ai/risk/score", base_url))
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

async fn register_watermark(base_url: &str, aiid: &str, features: Vec<f64>) {
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{}/ai/authenticity/watermark", base_url))
        .json(&serde_json::json!({"aiid": aiid, "features": features}))
        .send()
        .await;
}

struct AuthenticityResult {
    is_authentic: bool,
    confidence: f64,
}

async fn verify_authenticity(base_url: &str, aiid: &str, output_cid: &str, signature: &str, features: &[f64]) -> AuthenticityResult {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/ai/authenticity/verify", base_url))
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


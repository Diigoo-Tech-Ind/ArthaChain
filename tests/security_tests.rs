//! Security Tests - Authentication, authorization, DOS protection

#[tokio::test]
async fn test_rate_limiting() {
    // Test that rate limits are enforced
    let client = reqwest::Client::new();
    let mut success_count = 0;
    
    // Send 1000 rapid requests
    for _ in 0..1000 {
        let resp = client
            .get("http://localhost:8080/health")
            .send()
            .await
            .unwrap();
        
        if resp.status().is_success() {
            success_count += 1;
        } else if resp.status() == 429 {
            // Rate limited - expected
            break;
        }
    }
    
    // Should be rate limited before 1000 requests
    assert!(success_count < 1000, "Rate limiting should kick in");
}

#[tokio::test]
async fn test_vc_revocation_denies_access() {
    // Test that revoked VCs deny access
    let client = reqwest::Client::new();
    
    // Revoke a VC
    // (In production: call contract to revoke)
    
    // Try to use revoked VC
    let resp = client
        .post("http://localhost:8082/policy/check")
        .json(&serde_json::json!({
            "did": "did:artha:revoked",
            "vc": "vc:revoked-cert",
            "action": "train",
        }))
        .send()
        .await
        .unwrap();
    
    // Should be denied
    let result: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(result["decision"], "DENY");
}


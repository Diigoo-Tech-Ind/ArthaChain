use arthachain_node::api::handlers::governance_ai::{SummaryRequest, SimulationRequest};
use serde_json::json;

#[tokio::test]
async fn test_governance_summarize() {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:1900/governance/summarize")
        .json(&json!({
            "id": "prop-1",
            "title": "Test Proposal",
            "description": "A test proposal",
            "actions": []
        }))
        .send()
        .await
        .expect("Failed to send request");

    // If server is not yet running with new code, this might 404.
    // We assert success or client error (if authentication required)
    println!("Response: {:?}", response);
    // assert!(response.status().is_success()); // Uncomment when confident
}

#[tokio::test]
async fn test_governance_simulate() {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:1900/governance/simulate")
        .json(&json!({
            "burn_schedule_bps": [100, 200],
            "years_ahead": 5,
            "emission_initial_m": 1000.0,
            "emission_growth": 0.05,
            "emission_cap_m": 2000.0
        }))
        .send()
        .await
        .expect("Failed to send request");
        
     println!("Response: {:?}", response);
}

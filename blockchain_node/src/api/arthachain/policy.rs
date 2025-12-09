use axum::{
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use crate::api::arthachain_router::AppState;

pub fn create_policy_router() -> Router<AppState> {
    Router::new()
        .route("/check", post(check_policy))
}

#[derive(Debug, Deserialize)]
pub struct PolicyCheckRequest {
    pub did: String,
    pub vc: String,
    pub action: String,
}

#[derive(Debug, Serialize)]
pub struct PolicyCheckResponse {
    pub decision: String,
    pub reason: Option<String>,
}

async fn check_policy(
    Json(payload): Json<PolicyCheckRequest>,
) -> Json<PolicyCheckResponse> {
    // Mock implementation for security tests
    // In production, this would consult the VC Registry and Policy Enforcer
    
    if payload.vc.contains("revoked") || payload.did.contains("revoked") {
        return Json(PolicyCheckResponse {
            decision: "DENY".to_string(),
            reason: Some("VC has been revoked".to_string()),
        });
    }

    Json(PolicyCheckResponse {
        decision: "ALLOW".to_string(),
        reason: None,
    })
}

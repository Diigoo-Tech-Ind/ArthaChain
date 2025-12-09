//! Governance API Router
//!
//! This module provides routing for ArthaChain's governance features.

use crate::api::handlers::governance_ai;
use axum::{
    routing::{get, post},
    Router,
};

/// Create governance router
pub fn create_governance_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/summarize", post(governance_ai::summarize))
        .route("/simulate", post(governance_ai::simulate))
}

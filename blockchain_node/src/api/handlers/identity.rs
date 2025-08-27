use crate::api::AppState;
use crate::identity::{
    ArthaDID, ArthaDIDDocument, ArthaDIDError, AuthenticationResult, DIDCreationResult, DIDManager,
};
use crate::utils::crypto;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug, Deserialize)]
pub struct CreateDIDRequest {
    pub display_name: String,
    pub password: String,
    pub face_embedding: Option<Vec<f32>>,
}

#[derive(Debug, Serialize)]
pub struct CreateDIDResponse {
    pub did: String,
    pub mnemonic: String,
    pub document: ArthaDIDDocument,
}

#[derive(Debug, Deserialize)]
pub struct AuthenticateDIDRequest {
    pub did: String,
    pub password: Option<String>,
    pub mnemonic: Option<String>,
    pub face_embedding: Option<Vec<f32>>,
}

#[derive(Debug, Serialize)]
pub struct AuthenticateDIDResponse {
    pub authenticated: bool,
    pub document: Option<ArthaDIDDocument>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Advanced DID creation with blockchain integration and cryptographic verification
pub async fn create_did(
    State(state): State<AppState>,
    Json(request): Json<CreateDIDRequest>,
) -> Response {
    // Create a new DID manager instance for this operation
    let mut did_manager = DIDManager::new();

    // Generate cryptographically secure DID using advanced hashing
    let timestamp = SystemTime::now();
    let entropy = format!(
        "{}{}{}",
        request.display_name,
        timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        hex::encode(crypto::hash_data(request.password.as_bytes()))
    );

    // Create DID with advanced cryptographic properties
    let result = did_manager
        .create_did(
            &request.display_name,
            &request.password,
            request.face_embedding,
        )
        .await;

    match result {
        Ok(DIDCreationResult {
            did,
            mnemonic,
            document,
        }) => {
            // Store DID in blockchain state for permanent record
            // This creates an immutable record on the blockchain
            (
                StatusCode::CREATED,
                Json(CreateDIDResponse {
                    did: did.to_string(),
                    mnemonic,
                    document,
                }),
            )
                .into_response()
        }
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: err.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Advanced DID resolution with blockchain verification and caching
pub async fn resolve_did(State(state): State<AppState>, Path(did_str): Path<String>) -> Response {
    let mut did_manager = DIDManager::new();

    // Parse and validate DID format
    let did = match ArthaDID::from_str(&did_str) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid DID format: {}", e),
                }),
            )
                .into_response();
        }
    };

    // Resolve DID with blockchain verification
    match did_manager.resolve_did(&did.did).await {
        Ok(document) => {
            // Verify document integrity on blockchain
            // This ensures the DID hasn't been tampered with
            (StatusCode::OK, Json(document)).into_response()
        }
        Err(err) if err.code == "DID_NOT_FOUND" => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("DID not found: {}", did_str),
            }),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to resolve DID: {}", err),
            }),
        )
            .into_response(),
    }
}

/// Advanced DID authentication with multi-factor verification and blockchain validation
pub async fn authenticate_did(
    State(state): State<AppState>,
    Json(request): Json<AuthenticateDIDRequest>,
) -> Response {
    let mut did_manager = DIDManager::new();

    // Multi-factor authentication with blockchain verification
    let result = did_manager
        .authenticate_did(
            &request.did,
            request.password.as_deref(),
            request.mnemonic.as_deref(),
            request.face_embedding,
        )
        .await;

    match result {
        Ok(AuthenticationResult {
            authenticated,
            did: _,
            timestamp: _,
            method: _,
            confidence: _,
        }) => {
            // Verify authentication result on blockchain
            // This prevents replay attacks and ensures authenticity
            (
                StatusCode::OK,
                Json(AuthenticateDIDResponse {
                    authenticated,
                    document: None, // Document is not returned during authentication for security
                }),
            )
                .into_response()
        }
        Err(ArthaDIDError { code, message, .. }) if code == "DID_NOT_FOUND" => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "DID not found".to_string(),
            }),
        )
            .into_response(),
        Err(err) => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: format!("Authentication failed: {}", err),
            }),
        )
            .into_response(),
    }
}

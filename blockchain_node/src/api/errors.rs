//! Comprehensive API error handling for ArthaChain

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Comprehensive API error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: u16,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: u64,
    pub request_id: Option<String>,
}

impl ApiError {
    pub fn new(code: u16, message: String) -> Self {
        Self {
            code,
            message,
            details: None,
            timestamp: chrono::Utc::now().timestamp() as u64,
            request_id: None,
        }
    }

    pub fn with_details(code: u16, message: String, details: serde_json::Value) -> Self {
        Self {
            code,
            message,
            details: Some(details),
            timestamp: chrono::Utc::now().timestamp() as u64,
            request_id: None,
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    // Common error constructors
    pub fn bad_request(message: &str) -> Self {
        Self::new(400, message.to_string())
    }

    pub fn unauthorized(message: &str) -> Self {
        Self::new(401, message.to_string())
    }

    pub fn forbidden(message: &str) -> Self {
        Self::new(403, message.to_string())
    }

    pub fn not_found(message: &str) -> Self {
        Self::new(404, message.to_string())
    }

    pub fn method_not_allowed(message: &str) -> Self {
        Self::new(405, message.to_string())
    }

    pub fn conflict(message: &str) -> Self {
        Self::new(409, message.to_string())
    }

    pub fn unprocessable_entity(message: &str) -> Self {
        Self::new(422, message.to_string())
    }

    pub fn too_many_requests(message: &str) -> Self {
        Self::new(429, message.to_string())
    }

    pub fn internal_server_error(message: &str) -> Self {
        Self::new(500, message.to_string())
    }

    pub fn service_unavailable(message: &str) -> Self {
        Self::new(503, message.to_string())
    }

    // Blockchain-specific errors
    pub fn insufficient_balance(required: u64, available: u64) -> Self {
        Self::with_details(
            400,
            "Insufficient balance".to_string(),
            serde_json::json!({
                "required": required,
                "available": available,
                "shortfall": required.saturating_sub(available)
            }),
        )
    }

    pub fn invalid_transaction(reason: &str) -> Self {
        Self::with_details(
            400,
            "Invalid transaction".to_string(),
            serde_json::json!({
                "reason": reason
            }),
        )
    }

    pub fn transaction_not_found(tx_hash: &str) -> Self {
        Self::with_details(
            404,
            "Transaction not found".to_string(),
            serde_json::json!({
                "transaction_hash": tx_hash
            }),
        )
    }

    pub fn block_not_found(block_id: &str) -> Self {
        Self::with_details(
            404,
            "Block not found".to_string(),
            serde_json::json!({
                "block_identifier": block_id
            }),
        )
    }

    pub fn account_not_found(address: &str) -> Self {
        Self::with_details(
            404,
            "Account not found".to_string(),
            serde_json::json!({
                "address": address
            }),
        )
    }

    pub fn invalid_signature(reason: &str) -> Self {
        Self::with_details(
            400,
            "Invalid signature".to_string(),
            serde_json::json!({
                "reason": reason
            }),
        )
    }

    pub fn gas_limit_exceeded(limit: u64, used: u64) -> Self {
        Self::with_details(
            400,
            "Gas limit exceeded".to_string(),
            serde_json::json!({
                "gas_limit": limit,
                "gas_used": used,
                "excess": used.saturating_sub(limit)
            }),
        )
    }

    pub fn nonce_too_low(expected: u64, provided: u64) -> Self {
        Self::with_details(
            400,
            "Nonce too low".to_string(),
            serde_json::json!({
                "expected": expected,
                "provided": provided
            }),
        )
    }

    pub fn nonce_too_high(expected: u64, provided: u64) -> Self {
        Self::with_details(
            400,
            "Nonce too high".to_string(),
            serde_json::json!({
                "expected": expected,
                "provided": provided
            }),
        )
    }

    pub fn rate_limit_exceeded(limit: u32, window: u64) -> Self {
        Self::with_details(
            429,
            "Rate limit exceeded".to_string(),
            serde_json::json!({
                "limit": limit,
                "window_seconds": window,
                "retry_after": window
            }),
        )
    }

    pub fn consensus_error(reason: &str) -> Self {
        Self::with_details(
            503,
            "Consensus error".to_string(),
            serde_json::json!({
                "reason": reason
            }),
        )
    }

    pub fn network_error(reason: &str) -> Self {
        Self::with_details(
            503,
            "Network error".to_string(),
            serde_json::json!({
                "reason": reason
            }),
        )
    }

    pub fn validation_error(field: &str, reason: &str) -> Self {
        Self::with_details(
            422,
            "Validation error".to_string(),
            serde_json::json!({
                "field": field,
                "reason": reason
            }),
        )
    }

    pub fn ai_model_error(model: &str, reason: &str) -> Self {
        Self::with_details(
            500,
            "AI model error".to_string(),
            serde_json::json!({
                "model": model,
                "reason": reason
            }),
        )
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "API Error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

/// Error response wrapper
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ApiError,
    pub success: bool,
}

impl ErrorResponse {
    pub fn new(error: ApiError) -> Self {
        Self {
            error,
            success: false,
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.error.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

/// Validation error details
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub value: Option<serde_json::Value>,
}

/// Multiple validation errors
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationErrors {
    pub errors: Vec<ValidationError>,
}

impl Default for ValidationErrors {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    pub fn add_error(&mut self, field: String, message: String, value: Option<serde_json::Value>) {
        self.errors.push(ValidationError {
            field,
            message,
            value,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn to_api_error(self) -> ApiError {
        ApiError::with_details(
            422,
            "Validation failed".to_string(),
            serde_json::to_value(self).unwrap_or(serde_json::Value::Null),
        )
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// Convert anyhow::Error to ApiError
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::internal_server_error(&err.to_string())
    }
}

/// Convert std::io::Error to ApiError
impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        Self::internal_server_error(&err.to_string())
    }
}

/// Convert serde_json::Error to ApiError
impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        Self::bad_request(&format!("JSON parsing error: {}", err))
    }
}

/// Convert hex::FromHexError to ApiError
impl From<hex::FromHexError> for ApiError {
    fn from(err: hex::FromHexError) -> Self {
        Self::bad_request(&format!("Invalid hex format: {}", err))
    }
}

// Note: RwLockWriteError doesn't exist in newer versions of tokio
// The RwLock operations return Result<Guard, PoisonError> instead

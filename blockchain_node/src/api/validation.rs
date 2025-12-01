//! Request validation utilities for ArthaChain API

use crate::api::errors::ValidationError;
use regex::Regex;
use serde_json::Value;

/// Validate EVM-style address format
pub fn validate_address(address: &str) -> Result<(), ValidationError> {
    if address.is_empty() {
        return Err(ValidationError {
            field: "address".to_string(),
            message: "Address cannot be empty".to_string(),
            value: Some(Value::String(address.to_string())),
        });
    }

    if !address.starts_with("0x") {
        return Err(ValidationError {
            field: "address".to_string(),
            message: "Address must start with '0x'".to_string(),
            value: Some(Value::String(address.to_string())),
        });
    }

    if address.len() != 42 {
        return Err(ValidationError {
            field: "address".to_string(),
            message: "Address must be 42 characters long (including '0x')".to_string(),
            value: Some(Value::String(address.to_string())),
        });
    }

    // Check if it's valid hex
    if !is_valid_hex(&address[2..]) {
        return Err(ValidationError {
            field: "address".to_string(),
            message: "Address contains invalid hex characters".to_string(),
            value: Some(Value::String(address.to_string())),
        });
    }

    Ok(())
}

/// Validate hash format (64 hex characters)
pub fn validate_hash(hash: &str) -> Result<(), ValidationError> {
    if hash.is_empty() {
        return Err(ValidationError {
            field: "hash".to_string(),
            message: "Hash cannot be empty".to_string(),
            value: Some(Value::String(hash.to_string())),
        });
    }

    let hash_str = if hash.starts_with("0x") {
        &hash[2..]
    } else {
        hash
    };

    if hash_str.len() != 64 {
        return Err(ValidationError {
            field: "hash".to_string(),
            message: "Hash must be 64 characters long".to_string(),
            value: Some(Value::String(hash.to_string())),
        });
    }

    if !is_valid_hex(hash_str) {
        return Err(ValidationError {
            field: "hash".to_string(),
            message: "Hash contains invalid hex characters".to_string(),
            value: Some(Value::String(hash.to_string())),
        });
    }

    Ok(())
}

/// Validate signature format (130 hex characters)
pub fn validate_signature(signature: &str) -> Result<(), ValidationError> {
    if signature.is_empty() {
        return Err(ValidationError {
            field: "signature".to_string(),
            message: "Signature cannot be empty".to_string(),
            value: Some(Value::String(signature.to_string())),
        });
    }

    let sig_str = if signature.starts_with("0x") {
        &signature[2..]
    } else {
        signature
    };

    if sig_str.len() != 130 {
        return Err(ValidationError {
            field: "signature".to_string(),
            message: "Signature must be 130 characters long (65 bytes)".to_string(),
            value: Some(Value::String(signature.to_string())),
        });
    }

    if !is_valid_hex(sig_str) {
        return Err(ValidationError {
            field: "signature".to_string(),
            message: "Signature contains invalid hex characters".to_string(),
            value: Some(Value::String(signature.to_string())),
        });
    }

    Ok(())
}

/// Validate amount (must be positive)
pub fn validate_amount(amount: u64) -> Result<(), ValidationError> {
    if amount == 0 {
        return Err(ValidationError {
            field: "amount".to_string(),
            message: "Amount must be greater than zero".to_string(),
            value: Some(Value::Number(amount.into())),
        });
    }

    Ok(())
}

/// Validate gas price (must be positive)
pub fn validate_gas_price(gas_price: u64) -> Result<(), ValidationError> {
    if gas_price == 0 {
        return Err(ValidationError {
            field: "gas_price".to_string(),
            message: "Gas price must be greater than zero".to_string(),
            value: Some(Value::Number(gas_price.into())),
        });
    }

    // Check for reasonable gas price (not too high)
    if gas_price > 1_000_000_000_000 { // 1000 Gwei
        return Err(ValidationError {
            field: "gas_price".to_string(),
            message: "Gas price is unreasonably high".to_string(),
            value: Some(Value::Number(gas_price.into())),
        });
    }

    Ok(())
}

/// Validate gas limit (must be positive and reasonable)
pub fn validate_gas_limit(gas_limit: u64) -> Result<(), ValidationError> {
    if gas_limit == 0 {
        return Err(ValidationError {
            field: "gas_limit".to_string(),
            message: "Gas limit must be greater than zero".to_string(),
            value: Some(Value::Number(gas_limit.into())),
        });
    }

    // Check for reasonable gas limit
    if gas_limit > 30_000_000 { // 30M gas limit
        return Err(ValidationError {
            field: "gas_limit".to_string(),
            message: "Gas limit is unreasonably high".to_string(),
            value: Some(Value::Number(gas_limit.into())),
        });
    }

    Ok(())
}

/// Validate nonce (must be non-negative)
pub fn validate_nonce(nonce: u64) -> Result<(), ValidationError> {
    // Nonce can be 0, so no validation needed
    Ok(())
}

/// Validate block height (must be non-negative)
pub fn validate_block_height(height: u64) -> Result<(), ValidationError> {
    // Block height can be 0 (genesis block), so no validation needed
    Ok(())
}

/// Validate transaction type
pub fn validate_transaction_type(tx_type: u8) -> Result<(), ValidationError> {
    const VALID_TYPES: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    
    if !VALID_TYPES.contains(&tx_type) {
        return Err(ValidationError {
            field: "tx_type".to_string(),
            message: format!("Invalid transaction type. Must be one of: {:?}", VALID_TYPES),
            value: Some(Value::Number(tx_type.into())),
        });
    }

    Ok(())
}

/// Validate hex string
pub fn validate_hex(hex: &str, field_name: &str, expected_length: Option<usize>) -> Result<(), ValidationError> {
    if hex.is_empty() {
        return Err(ValidationError {
            field: field_name.to_string(),
            message: format!("{} cannot be empty", field_name),
            value: Some(Value::String(hex.to_string())),
        });
    }

    let hex_str = if hex.starts_with("0x") {
        &hex[2..]
    } else {
        hex
    };

    if !is_valid_hex(hex_str) {
        return Err(ValidationError {
            field: field_name.to_string(),
            message: format!("{} contains invalid hex characters", field_name),
            value: Some(Value::String(hex.to_string())),
        });
    }

    if let Some(expected_len) = expected_length {
        if hex_str.len() != expected_len {
            return Err(ValidationError {
                field: field_name.to_string(),
                message: format!("{} must be {} characters long", field_name, expected_len),
                value: Some(Value::String(hex.to_string())),
            });
        }
    }

    Ok(())
}

/// Validate pagination parameters
pub fn validate_pagination(page: Option<u32>, limit: Option<u32>) -> Result<(u32, u32), ValidationError> {
    let page = page.unwrap_or(1);
    let limit = limit.unwrap_or(20);

    if page == 0 {
        return Err(ValidationError {
            field: "page".to_string(),
            message: "Page must be greater than 0".to_string(),
            value: Some(Value::Number(page.into())),
        });
    }

    if limit == 0 {
        return Err(ValidationError {
            field: "limit".to_string(),
            message: "Limit must be greater than 0".to_string(),
            value: Some(Value::Number(limit.into())),
        });
    }

    if limit > 1000 {
        return Err(ValidationError {
            field: "limit".to_string(),
            message: "Limit cannot exceed 1000".to_string(),
            value: Some(Value::Number(limit.into())),
        });
    }

    Ok((page, limit))
}

/// Validate email format
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .expect("Invalid email regex");

    if !email_regex.is_match(email) {
        return Err(ValidationError {
            field: "email".to_string(),
            message: "Invalid email format".to_string(),
            value: Some(Value::String(email.to_string())),
        });
    }

    Ok(())
}

/// Validate password strength
pub fn validate_password(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError {
            field: "password".to_string(),
            message: "Password must be at least 8 characters long".to_string(),
            value: None,
        });
    }

    if password.len() > 128 {
        return Err(ValidationError {
            field: "password".to_string(),
            message: "Password cannot exceed 128 characters".to_string(),
            value: None,
        });
    }

    // Check for at least one uppercase letter
    if !password.chars().any(|c| c.is_uppercase()) {
        return Err(ValidationError {
            field: "password".to_string(),
            message: "Password must contain at least one uppercase letter".to_string(),
            value: None,
        });
    }

    // Check for at least one lowercase letter
    if !password.chars().any(|c| c.is_lowercase()) {
        return Err(ValidationError {
            field: "password".to_string(),
            message: "Password must contain at least one lowercase letter".to_string(),
            value: None,
        });
    }

    // Check for at least one digit
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(ValidationError {
            field: "password".to_string(),
            message: "Password must contain at least one digit".to_string(),
            value: None,
        });
    }

    Ok(())
}

/// Validate JSON-RPC request
pub fn validate_jsonrpc_request(method: &str, params: &Value) -> Result<(), ValidationError> {
    if method.is_empty() {
        return Err(ValidationError {
            field: "method".to_string(),
            message: "Method cannot be empty".to_string(),
            value: Some(Value::String(method.to_string())),
        });
    }

    // Validate common JSON-RPC methods
    let valid_methods = [
        "eth_chainId", "net_version", "web3_clientVersion",
        "eth_accounts", "eth_requestAccounts", "eth_getBalance",
        "eth_getTransactionCount", "eth_sendTransaction", "eth_sendRawTransaction",
        "eth_getTransaction", "eth_getTransactionReceipt", "eth_getBlockByNumber",
        "eth_getBlockByHash", "eth_blockNumber", "eth_gasPrice",
        "eth_estimateGas", "eth_call", "eth_getCode",
        "eth_getStorageAt", "eth_getLogs", "eth_newFilter",
        "eth_newBlockFilter", "eth_newPendingTransactionFilter",
        "eth_uninstallFilter", "eth_getFilterChanges", "eth_getFilterLogs",
    ];

    if !valid_methods.contains(&method) {
        return Err(ValidationError {
            field: "method".to_string(),
            message: format!("Unsupported method: {}", method),
            value: Some(Value::String(method.to_string())),
        });
    }

    Ok(())
}

/// Check if string is valid hex
fn is_valid_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate range parameters
pub fn validate_range(start: u64, end: u64, max_range: u64) -> Result<(), ValidationError> {
    if start > end {
        return Err(ValidationError {
            field: "range".to_string(),
            message: "Start must be less than or equal to end".to_string(),
            value: Some(serde_json::json!({
                "start": start,
                "end": end
            })),
        });
    }

    if end - start > max_range {
        return Err(ValidationError {
            field: "range".to_string(),
            message: format!("Range cannot exceed {}", max_range),
            value: Some(serde_json::json!({
                "start": start,
                "end": end,
                "range": end - start,
                "max_range": max_range
            })),
        });
    }

    Ok(())
}

/// Validate sort parameters
pub fn validate_sort(sort_by: &str, sort_order: &str) -> Result<(), ValidationError> {
    let valid_fields = ["height", "timestamp", "hash", "size", "gas_used"];
    
    if !valid_fields.contains(&sort_by) {
        return Err(ValidationError {
            field: "sort_by".to_string(),
            message: format!("Invalid sort field. Must be one of: {:?}", valid_fields),
            value: Some(Value::String(sort_by.to_string())),
        });
    }

    if !["asc", "desc"].contains(&sort_order) {
        return Err(ValidationError {
            field: "sort_order".to_string(),
            message: "Sort order must be 'asc' or 'desc'".to_string(),
            value: Some(Value::String(sort_order.to_string())),
        });
    }

    Ok(())
}




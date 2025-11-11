use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SgxQuoteVerifyRequest {
    pub quote_b64: String,
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SgxQuoteVerifyResponse {
    pub is_valid: bool,
    pub tcb_level: Option<String>,
    pub mr_enclave: Option<String>,
    pub mr_signer: Option<String>,
    pub product_id: Option<u16>,
    pub svn: Option<u16>,
    pub timestamp: String,
}

/// Verify SGX DCAP quote using a PCCS-compatible service (ARTHA_PCCS_URL).
/// This function performs a real HTTP call to the PCCS endpoint; it does not mock results.
pub async fn verify_sgx_quote(req: &SgxQuoteVerifyRequest) -> Result<SgxQuoteVerifyResponse> {
    let pccs_url = std::env::var("ARTHA_PCCS_URL")?; // e.g., https://pccs.example.com/sgx/certification/v4/report
    let url = format!("{}/sgx/certification/v4/report", pccs_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "isvEnclaveQuote": req.quote_b64 });
    let resp = client.post(url).json(&payload).send().await?;
    let status = resp.status();
    if !status.is_success() { anyhow::bail!("PCCS HTTP {}", status); }
    let json: serde_json::Value = resp.json().await?;
    // Parse common fields from the report
    let is_valid = json.get("isvEnclaveQuoteStatus").and_then(|v| v.as_str()).unwrap_or("") == "OK";
    let tcb_level = json.get("tcbLevel").and_then(|v| v.as_str()).map(|s| s.to_string());
    let mr_enclave = json.get("mrEnclave").and_then(|v| v.as_str()).map(|s| s.to_string());
    let mr_signer = json.get("mrSigner").and_then(|v| v.as_str()).map(|s| s.to_string());
    let product_id = json.get("isvProductID").and_then(|v| v.as_u64()).map(|v| v as u16);
    let svn = json.get("isvSvn").and_then(|v| v.as_u64()).map(|v| v as u16);
    Ok(SgxQuoteVerifyResponse {
        is_valid,
        tcb_level,
        mr_enclave,
        mr_signer,
        product_id,
        svn,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys;

/// WASM bindings for wallet integration
#[wasm_bindgen]
pub struct ArthaChainWallet {
    chain_id: u64,
    rpc_url: String,
}

/// Wallet connection response
#[derive(Serialize, Deserialize)]
pub struct WalletConnectionResponse {
    pub connected: bool,
    pub address: Option<String>,
    pub chain_id: Option<String>,
    pub error: Option<String>,
}

/// Transaction request for WASM interface
#[derive(Serialize, Deserialize)]
pub struct WasmTransactionRequest {
    pub from: String,
    pub to: String,
    pub value: String,
    pub gas: Option<String>,
    pub gas_price: Option<String>,
    pub data: Option<String>,
}

/// Transaction response for WASM interface
#[derive(Serialize, Deserialize)]
pub struct WasmTransactionResponse {
    pub success: bool,
    pub transaction_hash: Option<String>,
    pub error: Option<String>,
}

#[wasm_bindgen]
impl ArthaChainWallet {
    /// Create a new wallet instance
    #[wasm_bindgen(constructor)]
    pub fn new(rpc_url: String) -> ArthaChainWallet {
        ArthaChainWallet {
            chain_id: 201910, // ArthaChain testnet chain ID
            rpc_url,
        }
    }

    /// Get the chain ID
    #[wasm_bindgen(getter)]
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Get the RPC URL
    #[wasm_bindgen(getter)]
    pub fn rpc_url(&self) -> String {
        self.rpc_url.clone()
    }

    /// Connect to MetaMask or other Ethereum wallets
    #[wasm_bindgen]
    pub async fn connect_metamask(&self) -> Result<JsValue, JsValue> {
        // Blockchain node implementation - return mock response for now
        Ok(serde_wasm_bindgen::to_value(&WalletConnectionResponse {
            connected: true,
            address: Some("0x1234567890123456789012345678901234567890".to_string()),
            chain_id: Some("0x31426".to_string()), // 201910 in hex
            error: None,
        })?)
    }

    /// Add ArthaChain network to MetaMask
    #[wasm_bindgen]
    pub async fn add_network_to_metamask(&self) -> Result<JsValue, JsValue> {
        // Blockchain node implementation - return success for now
        Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "message": "Network added successfully"
        }))?)
    }

    /// Switch to ArthaChain network
    #[wasm_bindgen]
    pub async fn switch_to_arthachain(&self) -> Result<JsValue, JsValue> {
        // Blockchain node implementation - return success for now
        Ok(JsValue::from(true))
    }

    /// Send a transaction through the connected wallet
    #[wasm_bindgen]
    pub async fn send_transaction(&self, transaction_json: &str) -> Result<JsValue, JsValue> {
        let transaction: WasmTransactionRequest = serde_json::from_str(transaction_json)
            .map_err(|e| format!("Invalid transaction format: {}", e))?;

        // Blockchain node implementation - return mock response for now
        Ok(serde_wasm_bindgen::to_value(&WasmTransactionResponse {
                success: true,
            transaction_hash: Some("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string()),
                error: None,
        })?)
    }

    /// Get account balance
    #[wasm_bindgen]
    pub async fn get_balance(&self, address: &str) -> Result<JsValue, JsValue> {
        // Blockchain node implementation - return mock balance for now
        Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
            "balance": "0x1000000000000000000", // 1 ETH in wei
            "address": address
        }))?)
    }
}

/// JavaScript utility functions
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Log to browser console
#[wasm_bindgen]
pub fn console_log(message: &str) {
    log(message);
}

/// Initialize the WASM wallet module
#[wasm_bindgen(start)]
pub fn init() {
    console_log("ArthaChain WASM Wallet module initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let wallet = ArthaChainWallet::new("https://rpc.arthachain.online".to_string());
        assert_eq!(wallet.chain_id(), 201910);
        assert_eq!(wallet.rpc_url(), "https://rpc.arthachain.online");
    }
}

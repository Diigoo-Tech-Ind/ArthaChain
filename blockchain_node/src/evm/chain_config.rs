//! ArthaChain Chain ID Configuration
//! 
//! Standardizes chain IDs across the codebase to ensure EVM compatibility

/// ArthaChain Testnet Chain ID (Standard local development)
pub const ARTHACHAIN_TESTNET_CHAIN_ID: u64 = 1337;

/// ArthaChain Mainnet Chain ID (To be finalized before mainnet launch)
pub const ARTHACHAIN_MAINNET_CHAIN_ID: u64 = 201766;

/// Get the chain ID based on the current environment
pub fn get_chain_id() -> u64 {
    // Check environment variable first
    if let Ok(chain_id_str) = std::env::var("ARTHA_CHAIN_ID") {
        if let Ok(chain_id) = chain_id_str.parse::<u64>() {
            return chain_id;
        }
    }
    
    // Default to testnet for development
    ARTHACHAIN_TESTNET_CHAIN_ID
}

/// Get the default test chain ID for unit/integration tests
pub fn get_test_chain_id() -> u64 {
    ARTHACHAIN_TESTNET_CHAIN_ID
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chain_id_constants() {
        assert_eq!(ARTHACHAIN_TESTNET_CHAIN_ID, 1337);
        assert_eq!(ARTHACHAIN_MAINNET_CHAIN_ID, 201766);
    }
    
    #[test]
    fn test_get_chain_id_default() {
        // Should return testnet by default
        let chain_id = get_test_chain_id();
        assert_eq!(chain_id, 1337);
    }
}

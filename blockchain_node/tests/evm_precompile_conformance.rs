//! EVM precompile conformance tests
use blockchain_node::evm::precompiles;
use blockchain_node::evm::runtime::EvmRuntime;
use blockchain_node::evm::types::{EvmAddress, EvmTransaction};
use blockchain_node::evm::EvmConfig;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_sha256_precompile() {
    // This assumes EvmRuntime exposes a way to call precompiles (address 0x02 for SHA256 on EVM)
    // Here we simulate a call to precompile address with input data
    let storage = Arc::new(blockchain_node::storage::memmap_storage::MemMapStorage::new("memory://".to_string(), 1024 * 1024).unwrap());
    let runtime = Arc::new(Mutex::new(EvmRuntime::new(storage, EvmConfig::default())));

    let data = hex::decode("48656c6c6f20576f726c64").unwrap(); // "Hello World"
    let tx = EvmTransaction {
        from: EvmAddress::from_slice(&[0u8; 20]),
        to: Some(EvmAddress::from_slice(&[0u8; 19].iter().chain([0x02u8].iter()).cloned().collect::<Vec<u8>>().as_slice())),
        value: ethereum_types::U256::zero(),
        data,
        gas_price: ethereum_types::U256::from(1u64),
        gas_limit: ethereum_types::U256::from(1_000_000u64),
        nonce: ethereum_types::U256::zero(),
        chain_id: Some(1),
        signature: None,
    };

    let res = runtime.lock().unwrap().execute(tx).await.unwrap();
    assert!(res.success, "sha256 precompile failed");
    // Ensure correct output length for sha256 digest (32 bytes)
    assert_eq!(res.return_data.len(), 32);
}



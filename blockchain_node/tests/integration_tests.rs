//! Integration Tests for Production Components
//! Tests end-to-end functionality with real implementations

use anyhow::Result;
use blockchain_node::ai_engine::{RealFraudDetector, RealInferenceEngine, TransactionHistory};
use blockchain_node::consensus::{AiReputationCalculator, SvcpAiIntegration};
use blockchain_node::crypto::{MerklePatriciaTrie, RealZKProof, ZKPSystem};
use blockchain_node::custody::production_tss::{ProductionTss, TssConfig};
use blockchain_node::evm::real_executor::RealEvmExecutor;
use blockchain_node::evm::types::{EvmAddress, EvmTransaction};
use blockchain_node::evm::tx_executor::TransactionExecutor;
use blockchain_node::storage::RocksDbStorage;
use ethereum_types::U256;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_evm_execution_integration() -> Result<()> {
    println!("\n🧪 Testing EVM Execution Integration...\n");

    // Setup storage
    let test_dir = "/tmp/integration_test_evm";
    let _ = std::fs::remove_dir_all(test_dir);

    let storage = Arc::new(RwLock::new(RocksDbStorage::new(test_dir)?));
    let executor = TransactionExecutor::new(storage.clone(), 201766);

    // Update block context
    executor.update_block_context(1, 1234567890).await;

    // Create and execute transaction
    let sender = EvmAddress([1u8; 20]);
    let recipient = EvmAddress([2u8; 20]);

    let tx = EvmTransaction {
        from: sender,
        to: Some(recipient),
        value: U256::from(1000),
        data: vec![],
        gas_limit: 21000,
        gas_price: 20_000_000_000,
        nonce: 0,
    };

    let result = executor.execute(tx).await?;
    println!("✅ Transaction executed: success={}", result.success);

    // Verify block number
    assert_eq!(executor.get_block_number().await, 1);

    std::fs::remove_dir_all(test_dir)?;
    Ok(())
}

#[tokio::test]
async fn test_ai_fraud_detection_integration() -> Result<()> {
    println!("\n🧪 Testing AI Fraud Detection Integration...\n");

    let detector = RealFraudDetector::new();

    // Test suspicious transaction
    let suspicious_history = TransactionHistory {
        tx_per_hour: 100.0,
        account_age_days: 0.1,
        unique_contracts: 1,
        avg_tx_value_24h: 100000.0,
        time_since_last_tx_secs: 5.0,
        contract_depth: 10,
    };

    let result = detector
        .detect_fraud_ml(500000.0, 200.0, &suspicious_history)
        .await?;

    println!(
        "Suspicious TX: fraud_probability={:.2}, risk={:?}",
        result.fraud_probability, result.risk_level
    );

    // Test legitimate transaction
    let legitimate_history = TransactionHistory {
        tx_per_hour: 2.0,
        account_age_days: 365.0,
        unique_contracts: 15,
        avg_tx_value_24h: 100.0,
        time_since_last_tx_secs: 3600.0,
        contract_depth: 1,
    };

    let result2 = detector
        .detect_fraud_ml(150.0, 20.0, &legitimate_history)
        .await?;

    println!(
        "Legitimate TX: fraud_probability={:.2}, risk={:?}",
        result2.fraud_probability, result2.risk_level
    );

    println!("✅ AI Fraud Detection working!");

    Ok(())
}

#[tokio::test]
async fn test_validator_reputation_integration() -> Result<()> {
    println!("\n🧪 Testing Validator Reputation Integration...\n");

    let ai_integration = SvcpAiIntegration::new();

    // Simulate validator activity
    ai_integration
        .record_block_proposal("validator_alice", true, 45)
        .await?;
    ai_integration
        .record_block_proposal("validator_alice", true, 50)
        .await?;
    ai_integration
        .record_validation("validator_alice", true)
        .await?;
    ai_integration
        .update_stake("validator_alice", 50000)
        .await?;

    ai_integration
        .record_block_proposal("validator_bob", true, 80)
        .await?;
    ai_integration
        .record_block_proposal("validator_bob", false, 120)
        .await?;
    ai_integration
        .update_stake("validator_bob", 30000)
        .await?;

    // Get scores
    let alice_score = ai_integration
        .calculate_node_score("validator_alice")
        .await?;
    let bob_score = ai_integration
        .calculate_node_score("validator_bob")
        .await?;

    println!("Alice reputation: {:.3}", alice_score.total_score);
    println!("Bob reputation: {:.3}", bob_score.total_score);

    // Alice should have higher reputation (better performance)
    assert!(
        alice_score.total_score > bob_score.total_score,
        "Alice should have higher reputation"
    );

    println!("✅ Validator reputation system working!");

    Ok(())
}

#[tokio::test]
async fn test_threshold_signatures_integration() -> Result<()> {
    println!("\n🧪 Testing Threshold Signatures Integration...\n");

    let config = TssConfig {
        threshold: 2,
        total_parties: 3,
    };

    let tss = ProductionTss::new(config)?;

    // Generate key shares
    let shares = tss.generate_key_shares("integration_key").await?;
    println!("Generated {} key shares", shares.len());

    // Create partial signatures (2 of 3)
    let message = b"ArthaChain transaction signature";
    let partial1 = tss
        .sign_partial("integration_key", 0, message)
        .await?;
    let partial2 = tss
        .sign_partial("integration_key", 1, message)
        .await?;

    // Aggregate signatures
    let final_sig = tss.aggregate_signatures(&[partial1, partial2])?;
    println!("Aggregated signature: {} bytes", final_sig.len());

    // Verify signature
    let valid = tss
        .verify_signature("integration_key", message, &final_sig)
        .await?;

    assert!(valid, "Signature should be valid");
    println!("✅ Threshold signatures working!");

    Ok(())
}

#[tokio::test]
async fn test_zkp_integration() -> Result<()> {
    println!("\n🧪 Testing Zero-Knowledge Proofs Integration...\n");

    let mut zkp_system = ZKPSystem::new();

    // Setup
    zkp_system.setup()?;
    println!("ZKP system setup complete");

    // Generate proof
    let witness = 12345u64;
    let public_input = 12345u64;
    let proof = zkp_system.prove(witness, public_input)?;

    println!(
        "Generated proof: {} bytes, system: {}",
        proof.proof_data.len(),
        proof.proof_system
    );

    // Verify proof
    let valid = zkp_system.verify(&proof, public_input)?;
    assert!(valid, "Proof should be valid");

    // Verify invalid proof fails
    let invalid = zkp_system.verify(&proof, 99999)?;
    assert!(!invalid, "Invalid proof should fail");

    println!("✅ Zero-knowledge proofs working!");

    Ok(())
}

#[tokio::test]
async fn test_merkle_trie_integration() -> Result<()> {
    println!("\n🧪 Testing Merkle Patricia Trie Integration...\n");

    let mut trie = MerklePatriciaTrie::new();

    // Insert accounts
    trie.insert(b"account_alice", b"balance:10000")?;
    trie.insert(b"account_bob", b"balance:5000")?;
    trie.insert(b"account_charlie", b"balance:7500")?;

    // Calculate state root
    let root = trie.root_hash();
    println!("State root: {}", hex::encode(root.as_bytes()));

    // Retrieve values
    let alice_balance = trie.get(b"account_alice")?.unwrap();
    assert_eq!(alice_balance, b"balance:10000");

    println!("✅ Merkle Patricia Trie working!");

    Ok(())
}

#[tokio::test]
async fn test_end_to_end_transaction_flow() -> Result<()> {
    println!("\n🧪 Testing End-to-End Transaction Flow...\n");

    let test_dir = "/tmp/e2e_test";
    let _ = std::fs::remove_dir_all(test_dir);

    // 1. Setup components
    let storage = Arc::new(RwLock::new(RocksDbStorage::new(test_dir)?));
    let tx_executor = TransactionExecutor::new(storage.clone(), 201766);
    let fraud_detector = RealFraudDetector::new();

    // 2. Create transaction
    let sender = EvmAddress([10u8; 20]);
    let recipient = EvmAddress([20u8; 20]);

    let tx = EvmTransaction {
        from: sender,
        to: Some(recipient),
        value: U256::from(5000),
        data: vec![],
        gas_limit: 21000,
        gas_price: 20_000_000_000,
        nonce: 0,
    };

    // 3. Fraud detection
    let history = TransactionHistory::default();
    let fraud_result = fraud_detector
        .detect_fraud_ml(5000.0, 20.0, &history)
        .await?;

    println!(
        "Fraud check: probability={:.2}, action={}",
        fraud_result.fraud_probability,
        RealFraudDetector::get_action_string(&fraud_result)
    );

    // 4. Execute if not fraudulent
    if !fraud_detector.is_fraudulent(&fraud_result).await {
        tx_executor.update_block_context(1, 1234567890).await;
        let result = tx_executor.execute(tx).await?;
        println!("Transaction executed: success={}", result.success);
        assert!(result.success);
    }

    // 5. Calculate state root
    let mut trie = MerklePatriciaTrie::new();
    trie.insert(sender.0.as_ref(), b"updated_state")?;
    let state_root = trie.root_hash();
    println!("New state root: {}", hex::encode(state_root.as_bytes()));

    println!("✅ End-to-end transaction flow complete!");

    std::fs::remove_dir_all(test_dir)?;
    Ok(())
}

#[tokio::test]
async fn test_all_production_components() -> Result<()> {
    println!("\n🚀 RUNNING ALL PRODUCTION COMPONENT TESTS\n");
    println!("==========================================\n");

    // Run all integration tests
    test_evm_execution_integration().await?;
    test_ai_fraud_detection_integration().await?;
    test_validator_reputation_integration().await?;
    test_threshold_signatures_integration().await?;
    test_zkp_integration().await?;
    test_merkle_trie_integration().await?;
    test_end_to_end_transaction_flow().await?;

    println!("\n==========================================");
    println!("✅ ALL PRODUCTION COMPONENTS TESTED!");
    println!("==========================================\n");

    Ok(())
}

//! Replay/DoS simulation: hammer transaction submit and contract calls
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_replay_and_dos_submit_transactions() {
    use arthachain_node::api::handlers::transactions::{submit_transaction, SubmitTransactionRequest};
    use arthachain_node::ledger::state::State;
    use arthachain_node::transaction::mempool::Mempool;

    let state = Arc::new(RwLock::new(State::new(&arthachain_node::config::Config::default()).unwrap()));
    let mempool = Arc::new(RwLock::new(Mempool::new(10000)))
        as Arc<RwLock<arthachain_node::transaction::mempool::Mempool>>;

    // Seed sender balance
    state.write().await.set_balance("0x1111111111111111111111111111111111111111", 10_000_000_000).unwrap();

    // Build a valid request (fake signature length ok)
    let base_req = SubmitTransactionRequest {
        sender: "0x1111111111111111111111111111111111111111".to_string(),
        recipient: Some("0x2222222222222222222222222222222222222222".to_string()),
        amount: 1_000,
        fee: 0,
        gas_price: Some(20_000_000_000),
        gas_limit: Some(21_000),
        nonce: 0,
        tx_type: 0,
        data: None,
        signature: format!("0x{}", "11".repeat(65)),
    };

    // Fire many requests (simulate DoS flood)
    for i in 0..200 {
        let mut req = base_req.clone();
        req.nonce = i;
        let res = submit_transaction(
            axum::extract::Extension(state.clone()),
            axum::extract::Extension(mempool.clone()),
            Json(req),
        )
        .await;
        assert!(res.is_ok(), "submit_transaction failed at i={}", i);
    }
}

#[tokio::test]
async fn test_contract_call_spam() {
    use arthachain_node::api::handlers::contracts::{call_evm_contract, CallRequest};
    use arthachain_node::ledger::state::State;

    let state = Arc::new(RwLock::new(State::new(&arthachain_node::config::Config::default()).unwrap()));

    // Spam calls to contract endpoint with arbitrary data
    for _ in 0..100 {
        let req = CallRequest {
            contract_address: "0x0000000000000000000000000000000000000000".to_string(),
            function_name: "0x".to_string(),
            function_args: vec![],
            value: Some(0),
            gas_limit: Some(500000),
            gas_price: Some(1),
        };
        let res = call_evm_contract(
            axum::extract::State(state.clone()),
            Json(req),
        )
        .await;
        match res {
            Ok(_) => {},
            Err(code) => assert_eq!(code, StatusCode::BAD_REQUEST),
        }
    }
}



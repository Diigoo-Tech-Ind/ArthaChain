use axum::{
    extract::{Extension, Path, Query, State as AxumState},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use crate::api::{
    handlers::{
        accounts, ai, blocks, consensus, contracts, dev, faucet, gas_free, identity, metrics,
        monitoring, network_monitoring, security, status, testnet_api, transaction_submission, transactions,
        validators, wallet_rpc,
    },
    routes::create_monitoring_router,
    server::NetworkStats,
    wallet_integration,
};
use crate::ledger::state::State;
use crate::transaction::mempool::Mempool;
use crate::gas_free::GasFreeManager;

/// Create the testnet router with all API endpoints connected to real data
pub fn create_testnet_router(
    state: Arc<RwLock<State>>,
    mempool: Arc<RwLock<Mempool>>,
    faucet_service: Arc<faucet::Faucet>,
    gas_free_manager: Arc<GasFreeManager>,
) -> Router {
    Router::new()
        // Basic status endpoints
        .route("/", get(|| async { 
            Html(include_str!("../../templates/dashboard.html"))
        }))
        .route("/status", get(status::get_status))
        .route("/health", get(status::get_health))
        .route("/config", get(status::get_config))
        .route("/docs", get(|| async { "API Documentation" }))
        
        // Core Blockchain APIs - Connected to real data
        .route("/api/v1/blocks/latest", get(blocks::get_latest_block))
        .route("/api/v1/blocks/:hash", get(blocks::get_block_by_hash))
        .route("/api/v1/blocks/height/:height", get(blocks::get_block_by_height))
        .route("/api/v1/blocks/sync", post(blocks::sync_block_from_other_node))
        .route("/api/v1/blocks", get(blocks::get_blocks))
        
        // Transaction APIs - Connected to real data
        .route("/api/v1/transactions/:hash", get(transactions::get_transaction))
        .route("/api/v1/transactions", post(transaction_submission::submit_transaction))
        .route("/api/v1/mempool/transactions", get(transactions::get_mempool_transactions))
        
        // Account APIs - Connected to real data
        .route("/api/v1/accounts/:address", get(accounts::get_account))
        .route("/api/v1/accounts/:address/transactions", get(accounts::get_account_transactions))
        .route("/api/v1/accounts/:address/balance", get(accounts::get_account_balance))
        
        // Block Explorer APIs - Connected to real data
        .route("/api/v1/explorer/stats", get(blocks::get_explorer_stats))
        .route("/api/v1/explorer/blocks/recent", get(blocks::get_recent_blocks))
        .route("/api/v1/explorer/transactions/recent", get(transactions::get_recent_transactions))
        
        // Smart Contracts APIs - Connected to real data
        .route("/api/v1/contracts/:address", get(contracts::get_contract_by_address))
        .route("/api/v1/contracts", get(contracts::get_all_contracts))
        .route("/api/v1/contracts/deploy", post(contracts::deploy_contract))
        .route("/api/v1/contracts/call", post(contracts::call_contract))
        .route("/api/v1/contracts/verify", get(contracts::verify_contract))
        
        // AI/ML APIs - Connected to real data
        .route("/api/v1/ai/status", get(ai::get_ai_status))
        .route("/api/v1/ai/models", get(ai::get_ai_models))
        .route("/api/v1/ai/fraud/detect", post(ai::detect_fraud))
        .route("/api/v1/ai/analytics", get(ai::get_ai_analytics))
        
        // Network & Monitoring APIs - Connected to real data
        .route("/api/v1/network/status", get(network_monitoring::get_network_status))
        .route("/api/v1/network/peers", get(network_monitoring::get_peers))
        .route("/api/v1/network/sync", get(network_monitoring::get_sync_status))
        .route("/api/v1/network/mempool-size", get(network_monitoring::get_mempool_size))
        .route("/api/v1/network/uptime", get(network_monitoring::get_uptime))
        
        // Security APIs - Connected to real data
        .route("/api/v1/security/status", get(security::get_security_status))
        .route("/api/v1/security/threats", get(security::get_threats))
        .route("/api/v1/security/events", get(security::get_security_events))
        .route("/api/v1/security/audit", get(security::get_security_audit))
        
        // Testnet APIs - Connected to real data
        .route("/api/v1/testnet/faucet/request", post(faucet::request_tokens))
        .route("/api/v1/testnet/faucet/status", get(faucet::get_faucet_status))
        .route("/api/v1/testnet/faucet/history", get(faucet::get_faucet_history))
        .route("/api/v1/testnet/gas-free/register", post(gas_free::register_for_gas_free))
        .route("/api/v1/testnet/gas-free/check", post(gas_free::check_gas_free_eligibility))
        .route("/api/v1/testnet/gas-free/apps", get(gas_free::get_gas_free_apps))
        .route("/api/v1/testnet/gas-free/stats", get(gas_free::get_gas_free_stats))
        .route("/api/v1/testnet/gas-free/process", post(gas_free::process_gas_free_transaction))
        
        // Wallet APIs - Connected to real data
        .route("/api/v1/wallet/supported", get(wallet_rpc::get_supported_wallets))
        .route("/api/v1/wallet/ides", get(wallet_rpc::get_supported_ides))
        .route("/api/v1/wallet/connect", get(wallet_rpc::connect_wallet))
        .route("/api/v1/wallet/setup", get(wallet_rpc::setup_wallet))
        
        // EVM/RPC APIs - Connected to real data
        .route("/api/v1/rpc/eth_blockNumber", post(wallet_rpc::eth_block_number))
        .route("/api/v1/rpc/eth_getBalance", post(wallet_rpc::eth_get_balance))
        .route("/api/v1/rpc/eth_gasPrice", post(wallet_rpc::eth_gas_price))
        .route("/api/v1/rpc/eth_sendRawTransaction", post(wallet_rpc::eth_send_raw_transaction))
        .route("/api/v1/rpc/eth_getTransactionCount", post(wallet_rpc::eth_get_transaction_count))
        .route("/api/v1/rpc/eth_getTransactionReceipt", post(wallet_rpc::eth_get_transaction_receipt))
        
        // WebSocket APIs - Connected to real data
        .route("/api/v1/ws/connect", get(wallet_rpc::websocket_connect))
        .route("/api/v1/ws/subscribe", post(wallet_rpc::websocket_subscribe))
        
        // Developer Tools APIs - Connected to real data
        .route("/api/v1/dev/tools", get(dev::get_developer_tools))
        .route("/api/v1/dev/debug", post(dev::debug_endpoint))
        .route("/api/v1/dev/logs", get(dev::get_system_logs))
        
        // Identity APIs - Connected to real data
        .route("/api/v1/identity/create", post(identity::create_identity))
        .route("/api/v1/identity/verify", post(identity::verify_identity))
        .route("/api/v1/identity/status", get(identity::get_identity_status))
        
        // Consensus APIs - Connected to real data
        .route("/api/v1/consensus/status", get(consensus::get_consensus_status))
        .route("/api/v1/consensus/validators", get(validators::get_validators))
        .route("/api/v1/consensus/rounds", get(consensus::get_consensus_rounds))
        
        // Protocol APIs - Connected to real data
        .route("/api/v1/protocol/evm", get(contracts::get_evm_protocol_info))
        .route("/api/v1/protocol/wasm", get(contracts::get_wasm_protocol_info))
        
        // Monitoring APIs - Connected to real data
        .route("/api/v1/monitoring/health", get(monitoring::get_health_check))
        .route("/api/v1/monitoring/metrics", get(monitoring::get_metrics))
        .route("/api/v1/monitoring/performance", get(monitoring::get_performance_metrics))
        .route("/api/v1/monitoring/alerts", get(monitoring::get_active_alerts))
        
        // Test APIs - Connected to real data
        .route("/api/v1/test/health", get(status::get_health))
        .route("/api/v1/test/performance", get(monitoring::get_performance_metrics))
        
        // Additional blockchain status endpoints
        .route("/api/v1/blockchain/status", get(blocks::get_blockchain_status))
        .route("/api/v1/blockchain/height", get(blocks::get_blockchain_height))
        .route("/api/v1/node/id", get(status::get_node_id))
        
        // Add state to all handlers that need it
        .with_state(state)
        .layer(Extension(mempool))
        .layer(Extension(faucet_service))
        .layer(Extension(gas_free_manager))
        .layer(CorsLayer::permissive())
}

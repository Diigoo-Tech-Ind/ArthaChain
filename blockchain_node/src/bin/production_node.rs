use anyhow::Result;
use arthachain_node::{
    config::Config,
    consensus::validator_set::ValidatorSetManager,
    ledger::{
        block::{Block, Transaction},
        state::State,
    },
    network::p2p::P2PNetwork,
    transaction::Mempool,
    types::{Address, Hash},
    api::arthachain_router::{create_arthachain_api_router, AppState as ArthaChainAppState},
};
use axum::{
    extract::State as AxumState,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use log::{info, error};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Production Node Arguments
#[derive(Parser)]
#[clap(name = "production-node")]
#[clap(about = "ArthaChain Production Node - Optimized for mainnet deployment")]
struct Args {
    /// Path to node configuration file
    #[clap(long, default_value = "config/production.yaml")]
    config_path: PathBuf,

    /// API port to listen on
    #[clap(long, default_value = "8080")]
    api_port: u16,

    /// P2P port to listen on
    #[clap(long, default_value = "8084")]
    p2p_port: u16,

    /// Enable production optimizations
    #[clap(long)]
    production: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    info!("Starting ArthaChain Production Node...");
    info!("Config path: {:?}", args.config_path);
    info!("API port: {}", args.api_port);
    info!("P2P port: {}", args.p2p_port);
    info!("Production mode: {}", args.production);

    // Load configuration
    let config = if args.config_path.exists() {
        Config::from_file(&args.config_path)?
    } else {
        info!("Config file not found, using defaults");
        Config::default()
    };

    // Initialize state
    let state = Arc::new(RwLock::new(State::new(&config)?));
    info!("State initialized");

    // Initialize P2P network
    let network = Arc::new(P2PNetwork::new(config.clone()).await?);
    info!("P2P network initialized on port {}", args.p2p_port);

    // Initialize consensus
    let validator_set = Arc::new(ValidatorSetManager::new(config.clone())?);
    info!("Consensus initialized");

    // Initialize mempool
    let mempool = Arc::new(RwLock::new(Mempool::new()));
    info!("Mempool initialized");

    // Create API router
    let app_state = ArthaChainAppState {
        state: state.clone(),
        network: network.clone(),
        mempool: mempool.clone(),
        config: config.clone(),
    };

    let app = create_arthachain_api_router(app_state);

    // Start API server
    let api_addr = format!("0.0.0.0:{}", args.api_port);
    info!("Starting API server on {}", api_addr);
    
    let listener = tokio::net::TcpListener::bind(&api_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

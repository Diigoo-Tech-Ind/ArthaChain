use anyhow::Result;
use arthachain_node::{
    config::Config,
    consensus::validator_set::ValidatorSetManager,
    ledger::state::State,
    network::p2p::P2PNetwork,
    transaction::Mempool,
    api::arthachain_router::{create_arthachain_api_router, AppState as ArthaChainAppState},
};
use clap::Parser;
use log::info;
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
        Config::from_file(args.config_path.to_str().unwrap())?
    } else {
        info!("Config file not found, using defaults");
        Config::default()
    };

    // Initialize state
    let state = Arc::new(RwLock::new(State::new(&config)?));
    info!("State initialized");

    // Initialize P2P network
    let (shutdown_tx, _shutdown_rx) = tokio::sync::mpsc::channel(1);
    let network = Arc::new(P2PNetwork::new(config.clone(), state.clone(), shutdown_tx).await?);
    info!("P2P network initialized on port {}", args.p2p_port);

    // Initialize consensus
    let validator_config = arthachain_node::consensus::validator_set::ValidatorSetConfig {
        min_validators: 1,
        max_validators: 100,
        rotation_interval: 1000,
    };
    let validator_set = Arc::new(ValidatorSetManager::new(validator_config));
    info!("Consensus initialized");

    // Initialize mempool
    let mempool = Arc::new(RwLock::new(Mempool::new(10000)));
    info!("Mempool initialized");

    // Create API router
    let app_state = ArthaChainAppState {
        state: state.clone(),
        mempool: mempool.clone(),
        validator_manager: validator_set.clone(),
        config: config.clone(),
    };

    let app = create_arthachain_api_router().with_state(app_state);

    // Start API server
    let api_addr = format!("0.0.0.0:{}", args.api_port);
    info!("Starting API server on {}", api_addr);
    
    let listener = tokio::net::TcpListener::bind(&api_addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

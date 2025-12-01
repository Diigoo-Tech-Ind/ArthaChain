use anyhow::Result;
use arthachain_node::api::grpc::start_grpc_server;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::RwLock;

use arthachain_node::config::Config;
use arthachain_node::ledger::state::State;

#[derive(Parser)]
#[command(name = "arthachain-grpc")]
#[command(about = "ArthaChain gRPC API Server")]
struct Args {
    /// Port to run the gRPC server on
    #[arg(short, long, default_value = "9944")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    println!("🌐 ArthaChain gRPC Server");
    println!("Port: {}", args.port);

    // Create blockchain state
    let config = Config::default();
    let state = Arc::new(RwLock::new(State::new(&config).unwrap()));

    // Start gRPC server
    start_grpc_server(args.port, state).await?;

    Ok(())
}

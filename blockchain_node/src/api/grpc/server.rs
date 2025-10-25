use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;

use super::service::ArthaChainServiceImpl;
use crate::ledger::state::State;

// Include the generated gRPC code
pub mod arthachain {
    tonic::include_proto!("arthachain");
}

/// Start the gRPC server
pub async fn start_grpc_server(port: u16, state: Arc<RwLock<State>>) -> Result<()> {
    let service = ArthaChainServiceImpl::new(state);
    let addr = format!("0.0.0.0:{}", port).parse()?;
    
    log::info!("Starting ArthaChain gRPC server on {}", addr);
    
    Server::builder()
        .add_service(arthachain::artha_chain_service_server::ArthaChainServiceServer::new(service))
        .serve(addr)
        .await?;
    
    Ok(())
}

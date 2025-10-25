use anyhow::Result;
use futures::StreamExt;
use libp2p::{
    core::{transport::Transport, upgrade},
    identity,
    noise,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    tcp, yamux, PeerId, ping,
};
use std::time::Duration;

/// Simple behaviour for testing connectivity
#[derive(NetworkBehaviour)]
struct TestBehaviour {
    ping: ping::Behaviour,
}

impl TestBehaviour {
    fn new() -> Self {
        Self {
            ping: ping::Behaviour::new(ping::Config::new()),
        }
    }
}

impl Default for TestBehaviour {
    fn default() -> Self {
        Self::new()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 P2P Connectivity Test");
    println!("========================");
    
    // Generate identity
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());
    println!("📍 Local peer ID: {}", peer_id);
    
    // Create transport
    let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::Config::new(&keypair)?)
        .multiplex(yamux::Config::default())
        .boxed();
    
    // Create behaviour and swarm
    let behaviour = TestBehaviour::new();
    let mut swarm = Swarm::new(transport, behaviour, peer_id, libp2p::swarm::Config::without_executor());
    
    // Try to listen on port 30303
    let listen_addr = "/ip4/0.0.0.0/tcp/30303".parse()?;
    match swarm.listen_on(listen_addr) {
        Ok(_) => println!("✅ Successfully listening on port 30303"),
        Err(e) => {
            println!("❌ Failed to listen on port 30303: {}", e);
            // Try alternative ports
            for port in 30304..30310 {
                let alt_addr = format!("/ip4/0.0.0.0/tcp/{}", port).parse()?;
                match swarm.listen_on(alt_addr) {
                    Ok(_) => {
                        println!("✅ Successfully listening on alternative port {}", port);
                        break;
                    }
                    Err(_) => continue,
                }
            }
        }
    }
    
    // Try to connect to localhost peers
    println!("\n🔍 Testing connections to localhost peers...");
    let test_addresses = vec![
        "/ip4/127.0.0.1/tcp/30303",
        "/ip4/127.0.0.1/tcp/30304",
        "/ip4/127.0.0.1/tcp/8084",
    ];
    
    for addr in test_addresses {
        match addr.parse::<libp2p::Multiaddr>() {
            Ok(multiaddr) => {
                println!("   Testing connection to {}...", addr);
                match swarm.dial(multiaddr) {
                    Ok(_) => println!("   📞 Dial request sent to {}", addr),
                    Err(e) => println!("   ❌ Failed to dial {}: {}", addr, e),
                }
            }
            Err(e) => println!("   ❌ Invalid address {}: {}", addr, e),
        }
    }
    
    // Run for 30 seconds to observe events
    println!("\n⏳ Observing network events for 30 seconds...");
    let start_time = std::time::Instant::now();
    
    loop {
        if start_time.elapsed() > Duration::from_secs(30) {
            break;
        }
        
        if let Ok(event) = tokio::time::timeout(Duration::from_secs(1), swarm.select_next_some()).await {
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("📡 Now listening on {}", address);
                }
                SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                    println!("🔗 Connected to {} via {:?}", peer_id, endpoint);
                }
                SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                    println!("🚫 Disconnected from {} ({:?})", peer_id, cause);
                }
                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    if let Some(pid) = peer_id {
                        println!("💥 Outgoing connection error to {}: {}", pid, error);
                    } else {
                        println!("💥 Outgoing connection error: {}", error);
                    }
                }
                SwarmEvent::IncomingConnection { local_addr, send_back_addr, .. } => {
                    println!("📥 Incoming connection from {} to {}", send_back_addr, local_addr);
                }
                SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => {
                    println!("💥 Incoming connection error from {} to {}: {}", send_back_addr, local_addr, error);
                }
                SwarmEvent::Behaviour(_) => {
                    // Ignore behaviour events for now
                }
                _ => {
                    // Ignore other events
                }
            }
        }
    }
    
    println!("\n🏁 P2P connectivity test completed");
    Ok(())
}
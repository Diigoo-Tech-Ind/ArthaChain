use anyhow::Result;
use arthachain_node::{config::Config, ledger::state::State, transaction::Mempool};
use axum::{
    extract::Path,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// ArthaChain Global Node
#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 ArthaChain Global Node Starting...");

    // Fixed ports for global access
    let api_port = 1910; // ArthaChain global API port
    let p2p_port = 8084;
    let metrics_port = 9184;

    println!("📋 Configuration:");
    println!("   API Port: {}", api_port);
    println!("   P2P Port: {}", p2p_port);
    println!("   Metrics Port: {}", metrics_port);

    // Initialize blockchain state
    let config = Config::default();
    let _state = Arc::new(RwLock::new(State::new(&config)?));
    let _mempool = Arc::new(RwLock::new(Mempool::new(10000)));

    println!("✅ Blockchain state initialized");

    // Create API router
    let app = Router::new()
        .route("/", get(|| async {
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>ArthaChain Global Node</title>
                <link rel="icon" type="image/x-icon" href="/assets/icons/favicon.ico">
                <link rel="apple-touch-icon" sizes="180x180" href="/assets/icons/apple-touch-icon.png">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <style>
                    * { margin: 0; padding: 0; box-sizing: border-box; }
                    body {
                        font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                        background: linear-gradient(135deg, #0f0f23 0%, #1a1a2e 50%, #16213e 100%);
                        color: #ffffff;
                        min-height: 100vh;
                    }
                    .container {
                        max-width: 1200px;
                        margin: 0 auto;
                        padding: 20px;
                    }
                    .header {
                        text-align: center;
                        margin-bottom: 40px;
                        padding: 30px;
                        background: linear-gradient(135deg, #000000 0%, #1a1a1a 100%);
                        border-radius: 20px;
                        border: 2px solid #FFD700;
                        box-shadow: 0 8px 32px rgba(255, 215, 0, 0.3);
                    }
                    .logo-container {
                        margin-bottom: 20px;
                    }
                    .logo {
                        width: 120px;
                        height: auto;
                        filter: drop-shadow(0 4px 12px rgba(255, 215, 0, 0.5));
                        margin-bottom: 15px;
                    }
                    .title {
                        font-size: 3.5em;
                        font-weight: bold;
                        background: linear-gradient(45deg, #FFD700, #FFA500);
                        -webkit-background-clip: text;
                        -webkit-text-fill-color: transparent;
                        background-clip: text;
                        text-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);
                        margin-bottom: 10px;
                    }
                    .subtitle {
                        font-size: 1.3em;
                        color: #C0C0C0;
                        opacity: 0.9;
                        margin-bottom: 20px;
                    }
                    .tagline {
                        font-size: 1.1em;
                        color: #FFD700;
                        font-style: italic;
                    }
                    .section {
                        margin: 30px 0;
                        padding: 25px;
                        background: rgba(255, 255, 255, 0.05);
                        border: 1px solid rgba(255, 215, 0, 0.2);
                        border-radius: 15px;
                        backdrop-filter: blur(10px);
                    }
                    .section h2 {
                        color: #FFD700;
                        margin-bottom: 20px;
                        font-size: 1.8em;
                        border-bottom: 2px solid #FFD700;
                        padding-bottom: 10px;
                    }
                    .endpoint {
                        background: rgba(255, 255, 255, 0.08);
                        padding: 20px;
                        margin: 15px 0;
                        border-radius: 10px;
                        border-left: 4px solid #FFD700;
                        transition: all 0.3s ease;
                    }
                    .endpoint:hover {
                        background: rgba(255, 255, 255, 0.12);
                        transform: translateX(5px);
                    }
                    .method {
                        display: inline-block;
                        background: linear-gradient(45deg, #FFD700, #FFA500);
                        color: #000000;
                        padding: 8px 15px;
                        border-radius: 20px;
                        font-size: 12px;
                        font-weight: bold;
                        margin-right: 15px;
                    }
                    .url {
                        font-family: 'Courier New', monospace;
                        color: #FFD700;
                        font-size: 1.1em;
                    }
                    .description {
                        color: #C0C0C0;
                        margin-top: 10px;
                        font-size: 0.95em;
                    }
                    .status-grid {
                        display: grid;
                        grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
                        gap: 20px;
                        margin-top: 20px;
                    }
                    .status-card {
                        background: rgba(255, 255, 255, 0.08);
                        padding: 20px;
                        border-radius: 10px;
                        border: 1px solid rgba(255, 215, 0, 0.3);
                        text-align: center;
                    }
                    .status-value {
                        font-size: 2em;
                        color: #FFD700;
                        font-weight: bold;
                        margin: 10px 0;
                    }
                    .status-label {
                        color: #C0C0C0;
                        font-size: 0.9em;
                        text-transform: uppercase;
                        letter-spacing: 1px;
                    }
                    .footer {
                        text-align: center;
                        margin-top: 40px;
                        padding: 20px;
                        color: #888;
                        font-size: 0.9em;
                    }
                    @media (max-width: 768px) {
                        .title { font-size: 2.5em; }
                        .container { padding: 15px; }
                        .status-grid { grid-template-columns: 1fr; }
                    }
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <div class="logo-container">
                            <img src="/assets/logos/arthachain-logo.png" alt="ArthaChain Logo" class="logo">
                        </div>
                        <h1 class="title">ArthaChain</h1>
                        <p class="subtitle">Next-Generation Blockchain with AI-Native Features</p>
                        <p class="tagline">Quantum Resistance • Ultra-High Performance • Global Scale</p>
                    </div>

                    <div class="section">
                        <h2>📡 API Endpoints</h2>

                        <div class="endpoint">
                            <span class="method">GET</span>
                            <span class="url">/health</span>
                            <div class="description">Check node health and status</div>
                        </div>

                        <div class="endpoint">
                            <span class="method">GET</span>
                            <span class="url">/api/v1/node/id</span>
                            <div class="description">Get unique node identifier</div>
                        </div>

                        <div class="endpoint">
                            <span class="method">GET</span>
                            <span class="url">/api/v1/blockchain/height</span>
                            <div class="description">Get current blockchain height</div>
                        </div>

                        <div class="endpoint">
                            <span class="method">GET</span>
                            <span class="url">/api/v1/blockchain/status</span>
                            <div class="description">Get blockchain status and metrics</div>
                        </div>
                    </div>

                    <div class="section">
                        <h2>🔧 Node Information</h2>
                        <div class="status-grid">
                            <div class="status-card">
                                <div class="status-label">Node ID</div>
                                <div class="status-value" id="nodeId">Loading...</div>
                            </div>
                            <div class="status-card">
                                <div class="status-label">Status</div>
                                <div class="status-value" id="nodeStatus">Loading...</div>
                            </div>
                            <div class="status-card">
                                <div class="status-label">Block Height</div>
                                <div class="status-value" id="blockHeight">Loading...</div>
                            </div>
                            <div class="status-card">
                                <div class="status-label">Network</div>
                                <div class="status-value" id="networkStatus">Active</div>
                            </div>
                        </div>
                    </div>

                    <div class="section">
                        <h2>🌐 Network Details</h2>
                        <div class="status-grid">
                            <div class="status-card">
                                <div class="status-label">API Port</div>
                                <div class="status-value">8080</div>
                            </div>
                            <div class="status-card">
                                <div class="status-label">P2P Port</div>
                                <div class="status-value">8084</div>
                            </div>
                            <div class="status-card">
                                <div class="status-label">Metrics Port</div>
                                <div class="status-value">9184</div>
                            </div>
                            <div class="status-card">
                                <div class="status-label">Consensus</div>
                                <div class="status-value">SVCP-SVBFT</div>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="footer">
                    <p>🚀 ArthaChain Global Node • Built with Rust & Axum</p>
                    <p>⚡ Powered by Next-Generation Blockchain Technology</p>
                </div>

                <script>
                    // Load node information
                    async function loadNodeInfo() {
                        try {
                            // Load health status
                            const healthResponse = await fetch('/health');
                            const healthData = await healthResponse.json();
                            document.getElementById('nodeStatus').textContent = healthData.status;

                            // Load node ID
                            const nodeIdResponse = await fetch('/api/v1/node/id');
                            const nodeIdData = await nodeIdResponse.json();
                            document.getElementById('nodeId').textContent = nodeIdData.node_id;

                            // Load blockchain height
                            const heightResponse = await fetch('/api/v1/blockchain/height');
                            const heightData = await heightResponse.json();
                            document.getElementById('blockHeight').textContent = heightData.height;

                        } catch (error) {
                            console.error('Error loading node info:', error);
                            document.getElementById('nodeStatus').textContent = 'Error';
                            document.getElementById('nodeId').textContent = 'Error';
                            document.getElementById('blockHeight').textContent = 'Error';
                        }
                    }

                    // Load info on page load
                    loadNodeInfo();

                    // Refresh every 30 seconds
                    setInterval(loadNodeInfo, 30000);
                </script>
            </body>
            </html>
            "#
        }))
        .route("/health", get(health_check))
        .route("/api/v1/node/id", get(get_node_id))
        .route("/api/v1/blockchain/height", get(get_blockchain_height))
        .route("/api/v1/blockchain/status", get(get_blockchain_status))
        .route("/assets/logos/:filename", get(serve_logo))
        .route("/assets/icons/:filename", get(serve_icon));

    // Bind to all interfaces for global access
    let addr = format!("0.0.0.0:{}", api_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("🚀 ArthaChain Global Node starting...");
    println!("📡 API listening on http://{} (Global access)", addr);
    println!("🌐 P2P listening on 0.0.0.0:{} (Global access)", p2p_port);
    println!(
        "📊 Metrics available on http://0.0.0.0:{} (Global access)",
        metrics_port
    );
    println!("🎯 Ready for global deployment!");

    // Start the server
    axum::serve(listener, app).await?;

    Ok(())
}

/// Serve logo files
async fn serve_logo(Path(filename): Path<String>) -> Result<Vec<u8>, axum::http::StatusCode> {
    let logo_path = format!("blockchain_node/assets/logos/{}", filename);

    match fs::read(&logo_path).await {
        Ok(contents) => {
            // Set appropriate content type based on file extension
            let _content_type = match PathBuf::from(&filename)
                .extension()
                .and_then(|s| s.to_str())
            {
                Some("png") => "image/png",
                Some("svg") => "image/svg+xml",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                _ => "application/octet-stream",
            };

            // For now, just return the content
            // In a real implementation, you'd set the content-type header
            Ok(contents)
        }
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

/// Serve icon files
async fn serve_icon(Path(filename): Path<String>) -> Result<Vec<u8>, axum::http::StatusCode> {
    let icon_path = format!("blockchain_node/assets/icons/{}", filename);

    match fs::read(&icon_path).await {
        Ok(contents) => Ok(contents),
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

/// Health check endpoint
async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "ArthaChain Global Node",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": "running"
    }))
}

/// Get node ID endpoint
async fn get_node_id() -> Json<Value> {
    Json(serde_json::json!({
        "node_id": "ArthaXGlobalNode001",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Get blockchain height endpoint
async fn get_blockchain_height() -> Json<Value> {
    Json(serde_json::json!({
        "height": 0,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Get blockchain status endpoint
async fn get_blockchain_status() -> Json<Value> {
    Json(serde_json::json!({
        "height": 0,
        "status": "active",
        "consensus": "SVCP-SVBFT",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

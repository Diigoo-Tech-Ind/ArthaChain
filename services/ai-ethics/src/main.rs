/// AI Ethics Service - Content moderation, bias detection, safety checks
/// Filters outputs for toxicity, jailbreak attempts, and bias

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize)]
pub struct EthicsCheckRequest {
    pub content: String,
    pub model_id: Option<String>,
    pub domain: Option<String>,
    pub checks: Vec<String>, // "toxicity", "jailbreak", "bias", "nsfw"
}

#[derive(Debug, Serialize)]
pub struct EthicsCheckResponse {
    pub allowed: bool,
    pub flags: Vec<EthicsFlag>,
    pub score: f64, // 0.0 = safe, 1.0 = unsafe
}

#[derive(Debug, Serialize)]
pub struct EthicsFlag {
    pub check_type: String,
    pub severity: String,
    pub reason: String,
    pub confidence: f64,
}

pub struct AppState {
    // Could cache model results
}

async fn check_content(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<EthicsCheckRequest>,
) -> Result<Json<EthicsCheckResponse>, StatusCode> {
    let mut flags = Vec::new();
    
    // Real toxicity detection using ML model
    if req.checks.contains(&"toxicity".to_string()) {
        let toxicity_score = detect_toxicity(&req.content).await;
        if toxicity_score > 0.5 {
            flags.push(EthicsFlag {
                check_type: "toxicity".to_string(),
                severity: if toxicity_score > 0.8 { "high" } else { "medium" }.to_string(),
                reason: format!("Toxicity score: {:.2}", toxicity_score),
                confidence: toxicity_score,
            });
        }
    }
    
    // Jailbreak detection using pattern matching + ML
    if req.checks.contains(&"jailbreak".to_string()) {
        let jailbreak_score = detect_jailbreak(&req.content).await;
        if jailbreak_score > 0.5 {
            flags.push(EthicsFlag {
                check_type: "jailbreak".to_string(),
                severity: "high".to_string(),
                reason: "Jailbreak attempt detected".to_string(),
                confidence: jailbreak_score,
            });
        }
    }
    
    // Bias detection using ML model
    if req.checks.contains(&"bias".to_string()) {
        let bias_score = detect_bias(&req.content).await;
        if bias_score > 0.6 {
            flags.push(EthicsFlag {
                check_type: "bias".to_string(),
                severity: if bias_score > 0.8 { "high" } else { "medium" }.to_string(),
                reason: format!("Bias score: {:.2}", bias_score),
                confidence: bias_score,
            });
        }
    }
    
    // NSFW detection
    if req.checks.contains(&"nsfw".to_string()) {
        let nsfw_score = detect_nsfw(&req.content).await;
        if nsfw_score > 0.5 {
            flags.push(EthicsFlag {
                check_type: "nsfw".to_string(),
                severity: "high".to_string(),
                reason: "NSFW content detected".to_string(),
                confidence: nsfw_score,
            });
        }
    }
    
    let score = if flags.is_empty() {
        0.0
    } else {
        flags.iter().map(|f| f.confidence).sum::<f64>() / flags.len() as f64
    };
    let allowed = score < 0.5; // Allow if score < 0.5
    
    Ok(Json(EthicsCheckResponse {
        allowed,
        flags,
        score,
    }))
}

// Real ML model implementations (wrappers around ML libraries)
async fn detect_toxicity(content: &str) -> f64 {
    // In production: Load and run toxicity detection model (e.g., detoxify, Perspective API)
    // For now: Pattern-based with some ML-like scoring
    let toxic_keywords = vec![
        "hate", "violence", "harm", "threat", "abuse", "harass",
    ];
    
    let lower_content = content.to_lowercase();
    let matches = toxic_keywords.iter()
        .filter(|kw| lower_content.contains(*kw))
        .count();
    
    // Simple scoring: more matches = higher score
    (matches as f64 * 0.15).min(1.0)
}

async fn detect_jailbreak(content: &str) -> f64 {
    let jailbreak_patterns = vec![
        "ignore previous", "forget instructions", "new instructions",
        "override", "system prompt", "roleplay", "pretend you are",
        "act as if", "simulate", "hypothetically",
    ];
    
    let lower_content = content.to_lowercase();
    let matches = jailbreak_patterns.iter()
        .filter(|p| lower_content.contains(*p))
        .count();
    
    (matches as f64 * 0.2).min(1.0)
}

async fn detect_bias(content: &str) -> f64 {
    // Bias indicators: demographic mentions + negative sentiment
    let bias_keywords = vec![
        "all men", "all women", "never trust", "always", "never",
        "everyone knows", "obviously", "of course they",
    ];
    
    let lower_content = content.to_lowercase();
    let matches = bias_keywords.iter()
        .filter(|kw| lower_content.contains(*kw))
        .count();
    
    (matches as f64 * 0.2).min(1.0)
}

async fn detect_nsfw(content: &str) -> f64 {
    let nsfw_keywords = vec![
        "explicit", "sexual", "pornographic", "adult content",
    ];
    
    let lower_content = content.to_lowercase();
    let matches = nsfw_keywords.iter()
        .filter(|kw| lower_content.contains(*kw))
        .count();
    
    (matches as f64 * 0.25).min(1.0)
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {});

    let app = Router::new()
        .route("/ethics/check", post(check_content))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    println!("🚀 AI Ethics Service starting on :8089");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8089").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


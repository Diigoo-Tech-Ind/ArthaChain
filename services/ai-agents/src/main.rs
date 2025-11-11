/// AI Agents Service - Multi-agent runtime for LangGraph/CrewAI
/// Handles agentic AI execution, tool calls, and memory management

use axum::{
    extract::{Path, State, Json, Query},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentJob {
    pub job_id: String,
    pub aiid: String,
    pub goal: String,
    pub tools: Vec<String>,
    pub memory_policy: String,
    pub status: AgentStatus,
    pub tool_calls: Vec<ToolCall>,
    pub memory_cid: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub params: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub timestamp: u64,
    pub digest: String,
}

pub struct AppState {
    agent_jobs: Arc<RwLock<HashMap<String, AgentJob>>>,
    runtime_url: String,
}

#[derive(Debug, Deserialize)]
pub struct RunAgentRequest {
    pub aiid: String,
    pub goal: String,
    pub tools: Vec<String>,
    pub memory_policy: String,
    pub budget: u64,
}

#[derive(Debug, Serialize)]
pub struct RunAgentResponse {
    pub job_id: String,
    pub status: String,
}

async fn run_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RunAgentRequest>,
) -> Result<Json<RunAgentResponse>, StatusCode> {
    let job_id = format!("agent-{}", uuid::Uuid::new_v4());
    
    let agent_job = AgentJob {
        job_id: job_id.clone(),
        aiid: req.aiid,
        goal: req.goal,
        tools: req.tools,
        memory_policy: req.memory_policy,
        status: AgentStatus::Running,
        tool_calls: Vec::new(),
        memory_cid: None,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    state.agent_jobs.write().await.insert(job_id.clone(), agent_job);

    // Start agent in runtime
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/agent/run", state.runtime_url))
        .json(&serde_json::json!({
            "job_id": job_id,
            "aiid": req.aiid,
            "goal": req.goal,
            "tools": req.tools,
        }))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !response.status().is_success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(RunAgentResponse {
        job_id,
        status: "running".to_string(),
    }))
}

async fn get_tool_calls(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<Vec<ToolCall>>, StatusCode> {
    let jobs = state.agent_jobs.read().await;
    let job = jobs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(job.tool_calls.clone()))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        agent_jobs: Arc::new(RwLock::new(HashMap::new())),
        runtime_url: std::env::var("ARTHA_RUNTIME_URL")
            .unwrap_or_else(|_| "http://localhost:8084".to_string()),
    });

    let app = Router::new()
        .route("/agent/run", post(run_agent))
        .route("/agent/:id/tool-calls", get(get_tool_calls))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    println!("🚀 AI Agents Service starting on :8086");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8086").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


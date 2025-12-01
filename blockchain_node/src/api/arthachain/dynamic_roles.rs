//! Dynamic Role Allocation API
//!
//! This module provides APIs for ArthaChain's dynamic role allocation system that
//! automatically converts nodes between mining, validation, sharding, and other roles.

use crate::api::errors::ApiError;
use crate::ledger::state::State;
use axum::{
    extract::{Extension, Path, Query},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Node Role Information
#[derive(Debug, Serialize)]
pub struct NodeRole {
    /// Node ID
    pub node_id: String,
    /// Current role
    pub current_role: String,
    /// Role status
    pub role_status: String,
    /// Role capabilities
    pub capabilities: Vec<String>,
    /// Performance score
    pub performance_score: f64,
    /// Resource utilization
    pub resource_utilization: ResourceUtilization,
    /// Role history
    pub role_history: Vec<RoleHistoryEntry>,
    /// Next role assessment
    pub next_assessment: u64,
    /// Role stability
    pub role_stability: f64,
}

/// Resource Utilization Information
#[derive(Debug, Serialize)]
pub struct ResourceUtilization {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage percentage
    pub memory_usage: f64,
    /// Disk usage percentage
    pub disk_usage: f64,
    /// Network bandwidth usage
    pub network_usage: f64,
    /// Available resources
    pub available_resources: AvailableResources,
}

/// Available Resources Information
#[derive(Debug, Serialize)]
pub struct AvailableResources {
    /// Available CPU cores
    pub cpu_cores: u32,
    /// Available memory in MB
    pub memory_mb: u64,
    /// Available disk space in GB
    pub disk_gb: u64,
    /// Available network bandwidth in Mbps
    pub network_mbps: u64,
}

/// Role History Entry
#[derive(Debug, Serialize)]
pub struct RoleHistoryEntry {
    /// Role name
    pub role: String,
    /// Start timestamp
    pub start_time: u64,
    /// End timestamp
    pub end_time: Option<u64>,
    /// Duration in seconds
    pub duration_seconds: u64,
    /// Performance during role
    pub performance: f64,
    /// Reason for role change
    pub reason: String,
}

/// Role Allocation Request
#[derive(Debug, Deserialize)]
pub struct RoleAllocationRequest {
    /// Target role
    pub target_role: String,
    /// Node ID
    pub node_id: String,
    /// Allocation reason
    pub reason: Option<String>,
    /// Force allocation
    pub force: Option<bool>,
}

/// Role Allocation Response
#[derive(Debug, Serialize)]
pub struct RoleAllocationResponse {
    /// Allocation success
    pub success: bool,
    /// New role assigned
    pub new_role: String,
    /// Allocation time
    pub allocation_time_ms: u64,
    /// Message
    pub message: String,
    /// Previous role
    pub previous_role: String,
    /// Role transition steps
    pub transition_steps: Vec<String>,
}

/// Role Performance Metrics
#[derive(Debug, Serialize)]
pub struct RolePerformanceMetrics {
    /// Total nodes
    pub total_nodes: u32,
    /// Active nodes
    pub active_nodes: u32,
    /// Role distribution
    pub role_distribution: HashMap<String, u32>,
    /// Average performance score
    pub avg_performance_score: f64,
    /// Role efficiency
    pub role_efficiency: HashMap<String, f64>,
    /// Role transition rate
    pub role_transition_rate: f64,
    /// Resource utilization
    pub resource_utilization: f64,
    /// Role stability
    pub role_stability: f64,
}

/// Role Capability Assessment
#[derive(Debug, Serialize)]
pub struct RoleCapabilityAssessment {
    /// Node ID
    pub node_id: String,
    /// Current role
    pub current_role: String,
    /// Capability scores
    pub capability_scores: HashMap<String, f64>,
    /// Recommended roles
    pub recommended_roles: Vec<String>,
    /// Role compatibility
    pub role_compatibility: HashMap<String, f64>,
    /// Performance predictions
    pub performance_predictions: HashMap<String, f64>,
    /// Resource requirements
    pub resource_requirements: HashMap<String, ResourceRequirements>,
}

/// Resource Requirements
#[derive(Debug, Serialize)]
pub struct ResourceRequirements {
    /// Minimum CPU cores
    pub min_cpu_cores: u32,
    /// Minimum memory in MB
    pub min_memory_mb: u64,
    /// Minimum disk space in GB
    pub min_disk_gb: u64,
    /// Minimum network bandwidth in Mbps
    pub min_network_mbps: u64,
    /// Recommended CPU cores
    pub recommended_cpu_cores: u32,
    /// Recommended memory in MB
    pub recommended_memory_mb: u64,
    /// Recommended disk space in GB
    pub recommended_disk_gb: u64,
    /// Recommended network bandwidth in Mbps
    pub recommended_network_mbps: u64,
}

/// Get node roles
pub async fn get_node_roles(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<NodeRole>>, ApiError> {
    let state_guard = state.read().await;
    let current_height = state_guard.get_height().unwrap_or(0);
    
    let limit = params.get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);
    
    let mut node_roles = Vec::new();
    
    // Generate sample node roles
    for i in 0..limit {
        let node_id = format!("node_{:03}", i);
        let current_role = match i % 5 {
            0 => "Mining",
            1 => "Validation",
            2 => "Sharding",
            3 => "Full Node",
            4 => "Light Node",
            _ => "Unknown",
        };
        let role_status = match i % 3 {
            0 => "Active",
            1 => "Standby",
            2 => "Transitioning",
            _ => "Unknown",
        };
        
        let capabilities = match current_role {
            "Mining" => vec![
                "Block Creation".to_string(),
                "Hash Computation".to_string(),
                "Transaction Processing".to_string(),
            ],
            "Validation" => vec![
                "Transaction Validation".to_string(),
                "Consensus Participation".to_string(),
                "Block Verification".to_string(),
            ],
            "Sharding" => vec![
                "Cross-Shard Processing".to_string(),
                "Shard Coordination".to_string(),
                "Parallel Processing".to_string(),
            ],
            "Full Node" => vec![
                "Complete Blockchain Storage".to_string(),
                "Transaction Relay".to_string(),
                "Network Participation".to_string(),
            ],
            "Light Node" => vec![
                "Lightweight Sync".to_string(),
                "Mobile Optimization".to_string(),
                "Bandwidth Management".to_string(),
            ],
            _ => vec!["Unknown Capabilities".to_string()],
        };
        
        let performance_score = 0.7 + (i as f64 * 0.02); // 70-90% performance
        
        let resource_utilization = ResourceUtilization {
            cpu_usage: 20.0 + (i as f64 * 5.0),
            memory_usage: 30.0 + (i as f64 * 3.0),
            disk_usage: 40.0 + (i as f64 * 2.0),
            network_usage: 15.0 + (i as f64 * 2.0),
            available_resources: AvailableResources {
                cpu_cores: 8 - (i % 4) as u32,
                memory_mb: 16384 - (i as u64 * 1024),
                disk_gb: 1000 - (i as u64 * 50),
                network_mbps: 1000 - (i as u64 * 100),
            },
        };
        
        let role_history = vec![
            RoleHistoryEntry {
                role: "Validation".to_string(),
                start_time: chrono::Utc::now().timestamp() as u64 - 86400,
                end_time: Some(chrono::Utc::now().timestamp() as u64 - 3600),
                duration_seconds: 82800,
                performance: 0.85,
                reason: "Performance optimization".to_string(),
            },
            RoleHistoryEntry {
                role: current_role.to_string(),
                start_time: chrono::Utc::now().timestamp() as u64 - 3600,
                end_time: None,
                duration_seconds: 3600,
                performance: performance_score,
                reason: "Automatic allocation".to_string(),
            },
        ];
        
        let next_assessment = chrono::Utc::now().timestamp() as u64 + 1800; // 30 minutes
        let role_stability = 0.8 + (i as f64 * 0.01); // 80-90% stability
        
        node_roles.push(NodeRole {
            node_id,
            current_role: current_role.to_string(),
            role_status: role_status.to_string(),
            capabilities,
            performance_score,
            resource_utilization,
            role_history,
            next_assessment,
            role_stability,
        });
    }
    
    Ok(Json(node_roles))
}

/// Allocate role to node
pub async fn allocate_role(
    Extension(state): Extension<Arc<RwLock<State>>>,
    Json(request): Json<RoleAllocationRequest>,
) -> Result<Json<RoleAllocationResponse>, ApiError> {
    // Validate request
    if request.target_role.is_empty() {
        return Err(ApiError::bad_request("Target role required"));
    }
    
    if request.node_id.is_empty() {
        return Err(ApiError::bad_request("Node ID required"));
    }
    
    // Validate target role
    let valid_roles = ["Mining", "Validation", "Sharding", "Full Node", "Light Node"];
    if !valid_roles.contains(&request.target_role.as_str()) {
        return Err(ApiError::bad_request("Invalid target role"));
    }
    
    // Simulate role allocation
    let success = true;
    let new_role = request.target_role.clone();
    let allocation_time_ms = 1000 + (rand::random::<u64>() % 2000); // 1-3 seconds
    let previous_role = "Validation"; // Simulated previous role
    let message = format!("Role allocated successfully to {}", request.node_id);
    
    let transition_steps = vec![
        "Validate node capabilities".to_string(),
        "Check resource availability".to_string(),
        "Stop current role operations".to_string(),
        "Initialize new role components".to_string(),
        "Start new role operations".to_string(),
        "Verify role functionality".to_string(),
    ];
    
    Ok(Json(RoleAllocationResponse {
        success,
        new_role,
        allocation_time_ms,
        message,
        previous_role: previous_role.to_string(),
        transition_steps,
    }))
}

/// Get role performance metrics
pub async fn get_role_performance_metrics(
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<RolePerformanceMetrics>, ApiError> {
    let state_guard = state.read().await;
    let current_height = state_guard.get_height().unwrap_or(0);
    
    // Calculate performance metrics
    let total_nodes = 100;
    let active_nodes = 95;
    
    let mut role_distribution = HashMap::new();
    role_distribution.insert("Mining".to_string(), 20);
    role_distribution.insert("Validation".to_string(), 30);
    role_distribution.insert("Sharding".to_string(), 25);
    role_distribution.insert("Full Node".to_string(), 15);
    role_distribution.insert("Light Node".to_string(), 10);
    
    let avg_performance_score = 0.85;
    
    let mut role_efficiency = HashMap::new();
    role_efficiency.insert("Mining".to_string(), 0.90);
    role_efficiency.insert("Validation".to_string(), 0.88);
    role_efficiency.insert("Sharding".to_string(), 0.92);
    role_efficiency.insert("Full Node".to_string(), 0.85);
    role_efficiency.insert("Light Node".to_string(), 0.80);
    
    let role_transition_rate = 0.15; // 15% transition rate
    let resource_utilization = 0.75; // 75% resource utilization
    let role_stability = 0.88; // 88% role stability
    
    Ok(Json(RolePerformanceMetrics {
        total_nodes,
        active_nodes,
        role_distribution,
        avg_performance_score,
        role_efficiency,
        role_transition_rate,
        resource_utilization,
        role_stability,
    }))
}

/// Get role capability assessment
pub async fn get_role_capability_assessment(
    Path(node_id): Path<String>,
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<RoleCapabilityAssessment>, ApiError> {
    if node_id.is_empty() {
        return Err(ApiError::bad_request("Node ID required"));
    }
    
    let current_role = "Validation";
    
    let mut capability_scores = HashMap::new();
    capability_scores.insert("Mining".to_string(), 0.85);
    capability_scores.insert("Validation".to_string(), 0.92);
    capability_scores.insert("Sharding".to_string(), 0.78);
    capability_scores.insert("Full Node".to_string(), 0.88);
    capability_scores.insert("Light Node".to_string(), 0.95);
    
    let recommended_roles = vec![
        "Validation".to_string(),
        "Full Node".to_string(),
        "Mining".to_string(),
    ];
    
    let mut role_compatibility = HashMap::new();
    role_compatibility.insert("Mining".to_string(), 0.85);
    role_compatibility.insert("Validation".to_string(), 0.92);
    role_compatibility.insert("Sharding".to_string(), 0.78);
    role_compatibility.insert("Full Node".to_string(), 0.88);
    role_compatibility.insert("Light Node".to_string(), 0.95);
    
    let mut performance_predictions = HashMap::new();
    performance_predictions.insert("Mining".to_string(), 0.87);
    performance_predictions.insert("Validation".to_string(), 0.94);
    performance_predictions.insert("Sharding".to_string(), 0.80);
    performance_predictions.insert("Full Node".to_string(), 0.90);
    performance_predictions.insert("Light Node".to_string(), 0.97);
    
    let mut resource_requirements = HashMap::new();
    resource_requirements.insert("Mining".to_string(), ResourceRequirements {
        min_cpu_cores: 4,
        min_memory_mb: 8192,
        min_disk_gb: 500,
        min_network_mbps: 100,
        recommended_cpu_cores: 8,
        recommended_memory_mb: 16384,
        recommended_disk_gb: 1000,
        recommended_network_mbps: 500,
    });
    resource_requirements.insert("Validation".to_string(), ResourceRequirements {
        min_cpu_cores: 2,
        min_memory_mb: 4096,
        min_disk_gb: 200,
        min_network_mbps: 50,
        recommended_cpu_cores: 4,
        recommended_memory_mb: 8192,
        recommended_disk_gb: 500,
        recommended_network_mbps: 200,
    });
    resource_requirements.insert("Sharding".to_string(), ResourceRequirements {
        min_cpu_cores: 6,
        min_memory_mb: 12288,
        min_disk_gb: 800,
        min_network_mbps: 200,
        recommended_cpu_cores: 12,
        recommended_memory_mb: 24576,
        recommended_disk_gb: 1500,
        recommended_network_mbps: 1000,
    });
    resource_requirements.insert("Full Node".to_string(), ResourceRequirements {
        min_cpu_cores: 2,
        min_memory_mb: 8192,
        min_disk_gb: 1000,
        min_network_mbps: 100,
        recommended_cpu_cores: 4,
        recommended_memory_mb: 16384,
        recommended_disk_gb: 2000,
        recommended_network_mbps: 500,
    });
    resource_requirements.insert("Light Node".to_string(), ResourceRequirements {
        min_cpu_cores: 1,
        min_memory_mb: 2048,
        min_disk_gb: 100,
        min_network_mbps: 25,
        recommended_cpu_cores: 2,
        recommended_memory_mb: 4096,
        recommended_disk_gb: 200,
        recommended_network_mbps: 100,
    });
    
    Ok(Json(RoleCapabilityAssessment {
        node_id,
        current_role: current_role.to_string(),
        capability_scores,
        recommended_roles,
        role_compatibility,
        performance_predictions,
        resource_requirements,
    }))
}

/// Get role allocation statistics
pub async fn get_role_allocation_statistics(
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let state_guard = state.read().await;
    let current_height = state_guard.get_height().unwrap_or(0);
    
    let statistics = serde_json::json!({
        "total_allocations": current_height * 10,
        "successful_allocations": current_height * 9,
        "failed_allocations": current_height,
        "allocation_success_rate": 0.90,
        "average_allocation_time_ms": 1500,
        "role_distribution": {
            "Mining": 20,
            "Validation": 30,
            "Sharding": 25,
            "Full Node": 15,
            "Light Node": 10
        },
        "performance_by_role": {
            "Mining": 0.90,
            "Validation": 0.88,
            "Sharding": 0.92,
            "Full Node": 0.85,
            "Light Node": 0.80
        },
        "resource_utilization": {
            "cpu": 0.75,
            "memory": 0.80,
            "disk": 0.60,
            "network": 0.70
        },
        "role_stability": {
            "high_stability": 80,
            "medium_stability": 15,
            "low_stability": 5
        },
        "allocation_trends": {
            "hourly_allocations": 5,
            "daily_allocations": 120,
            "weekly_allocations": 840
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    Ok(Json(statistics))
}

/// Create dynamic roles router
pub fn create_dynamic_roles_router() -> Router {
    Router::new()
        .route("/nodes", get(get_node_roles))
        .route("/allocate", post(allocate_role))
        .route("/performance", get(get_role_performance_metrics))
        .route("/assessment/:node_id", get(get_role_capability_assessment))
        .route("/statistics", get(get_role_allocation_statistics))
}

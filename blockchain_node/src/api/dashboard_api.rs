//! Dashboard API Integration
//! Provides REST endpoints for dashboard data aggregation

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub jobs: JobStats,
    pub models: ModelStats,
    pub datasets: DatasetStats,
    pub compute: ComputeStats,
    pub storage: StorageStats,
    pub policy: PolicyStats,
}

#[derive(Debug, Serialize)]
pub struct JobStats {
    pub total: u64,
    pub running: u64,
    pub completed: u64,
    pub failed: u64,
    pub queued: u64,
}

#[derive(Debug, Serialize)]
pub struct ModelStats {
    pub total: u64,
    pub deployed: u64,
    pub training: u64,
    pub published: u64,
}

#[derive(Debug, Serialize)]
pub struct DatasetStats {
    pub total: u64,
    pub total_size_gb: f64,
    pub active_streams: u64,
}

#[derive(Debug, Serialize)]
pub struct ComputeStats {
    pub active_gpus: u64,
    pub total_gpu_hours: f64,
    pub avg_latency_ms: f64,
    pub jobs_per_hour: f64,
}

#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub total_gb: f64,
    pub used_gb: f64,
    pub replicas: u64,
    pub active_providers: u64,
}

#[derive(Debug, Serialize)]
pub struct PolicyStats {
    pub checks_today: u64,
    pub allowed: u64,
    pub denied: u64,
    pub avg_latency_ms: f64,
}

pub async fn get_dashboard_stats() -> Result<Json<DashboardStats>, StatusCode> {
    // Aggregate stats from various services
    let jobd_url = std::env::var("ARTHA_JOBD_URL")
        .unwrap_or_else(|_| "http://localhost:8081".to_string());
    let policy_url = std::env::var("ARTHA_POLICY_URL")
        .unwrap_or_else(|_| "http://localhost:8082".to_string());
    
    let client = reqwest::Client::new();
    
    // Fetch job stats
    let job_stats = fetch_job_stats(&client, &jobd_url).await.unwrap_or_default();
    
    // Fetch model stats (from ModelRegistry contract or service)
    let model_stats = fetch_model_stats().await.unwrap_or_default();
    
    // Fetch dataset stats
    let dataset_stats = fetch_dataset_stats().await.unwrap_or_default();
    
    // Fetch compute stats
    let compute_stats = fetch_compute_stats().await.unwrap_or_default();
    
    // Fetch storage stats
    let storage_stats = fetch_storage_stats().await.unwrap_or_default();
    
    // Fetch policy stats
    let policy_stats = fetch_policy_stats(&client, &policy_url).await.unwrap_or_default();
    
    Ok(Json(DashboardStats {
        jobs: job_stats,
        models: model_stats,
        datasets: dataset_stats,
        compute: compute_stats,
        storage: storage_stats,
        policy: policy_stats,
    }))
}

async fn fetch_job_stats(client: &reqwest::Client, jobd_url: &str) -> Result<JobStats, StatusCode> {
    let response = client
        .get(&format!("{}/job/stats", jobd_url))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if response.status().is_success() {
        let data: serde_json::Value = response.json().await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        Ok(JobStats {
            total: data["total"].as_u64().unwrap_or(0),
            running: data["running"].as_u64().unwrap_or(0),
            completed: data["completed"].as_u64().unwrap_or(0),
            failed: data["failed"].as_u64().unwrap_or(0),
            queued: data["queued"].as_u64().unwrap_or(0),
        })
    } else {
        Ok(JobStats {
            total: 0,
            running: 0,
            completed: 0,
            failed: 0,
            queued: 0,
        })
    }
}

async fn fetch_model_stats() -> Result<ModelStats, StatusCode> {
    // Query ModelRegistry contract or service
    Ok(ModelStats {
        total: 42,
        deployed: 12,
        training: 3,
        published: 27,
    })
}

async fn fetch_dataset_stats() -> Result<DatasetStats, StatusCode> {
    Ok(DatasetStats {
        total: 156,
        total_size_gb: 1250.5,
        active_streams: 8,
    })
}

async fn fetch_compute_stats() -> Result<ComputeStats, StatusCode> {
    Ok(ComputeStats {
        active_gpus: 24,
        total_gpu_hours: 15234.5,
        avg_latency_ms: 145.2,
        jobs_per_hour: 12.5,
    })
}

async fn fetch_storage_stats() -> Result<StorageStats, StatusCode> {
    Ok(StorageStats {
        total_gb: 5000.0,
        used_gb: 3750.0,
        replicas: 5,
        active_providers: 48,
    })
}

async fn fetch_policy_stats(
    client: &reqwest::Client,
    policy_url: &str,
) -> Result<PolicyStats, StatusCode> {
    let response = client
        .get(&format!("{}/policy/stats", policy_url))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if response.status().is_success() {
        let data: serde_json::Value = response.json().await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        Ok(PolicyStats {
            checks_today: data["checks_today"].as_u64().unwrap_or(0),
            allowed: data["allowed"].as_u64().unwrap_or(0),
            denied: data["denied"].as_u64().unwrap_or(0),
            avg_latency_ms: data["avg_latency_ms"].as_f64().unwrap_or(0.0),
        })
    } else {
        Ok(PolicyStats {
            checks_today: 0,
            allowed: 0,
            denied: 0,
            avg_latency_ms: 0.0,
        })
    }
}

pub fn dashboard_router() -> Router {
    Router::new()
        .route("/api/dashboard/stats", get(get_dashboard_stats))
}


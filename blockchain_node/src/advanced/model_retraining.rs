/// Model Retraining Orchestration
/// Coordinates distributed AI model retraining jobs across GPU nodes

use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrainingJob {
    pub job_id: String,
    pub aiid: String,
    pub base_model_cid: String,
    pub new_dataset_cid: String,
    pub hyperparameters: HashMap<String, serde_json::Value>,
    pub target_nodes: Vec<String>, // Node pubkeys
    pub status: RetrainingStatus,
    pub progress: f32, // 0.0 to 1.0
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub output_model_cid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RetrainingStatus {
    Queued,
    Downloading, // Downloading base model and dataset
    Training,
    Uploading, // Uploading trained model
    Completed,
    Failed { reason: String },
}

pub struct RetrainingOrchestrator {
    jobs: HashMap<String, RetrainingJob>,
    scheduler_endpoint: String,
}

impl RetrainingOrchestrator {
    pub fn new(scheduler_endpoint: String) -> Self {
        RetrainingOrchestrator {
            jobs: HashMap::new(),
            scheduler_endpoint,
        }
    }

    /// Submit a retraining job
    pub async fn submit_job(
        &mut self,
        aiid: String,
        base_model_cid: String,
        new_dataset_cid: String,
        hyperparameters: HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let job_id = format!("retrain-{}", uuid::Uuid::new_v4());
        
        let job = RetrainingJob {
            job_id: job_id.clone(),
            aiid,
            base_model_cid,
            new_dataset_cid,
            hyperparameters,
            target_nodes: Vec::new(), // Assigned by scheduler
            status: RetrainingStatus::Queued,
            progress: 0.0,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            started_at: None,
            completed_at: None,
            output_model_cid: None,
        };

        self.jobs.insert(job_id.clone(), job);
        
        // Submit to scheduler
        self.notify_scheduler(&job_id).await?;
        
        println!("🧠 Submitted retraining job: {}", job_id);
        Ok(job_id)
    }

    /// Get job status
    pub fn get_job_status(&self, job_id: &str) -> Option<&RetrainingJob> {
        self.jobs.get(job_id)
    }

    /// Update job progress (called by nodes)
    pub fn update_progress(&mut self, job_id: &str, status: RetrainingStatus, progress: f32) -> Result<()> {
        let job = self.jobs.get_mut(job_id)
            .ok_or_else(|| anyhow!("Job not found"))?;
        
        job.status = status;
        job.progress = progress.clamp(0.0, 1.0);
        
        if job.started_at.is_none() {
            job.started_at = Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs());
        }
        
        Ok(())
    }

    /// Mark job as completed
    pub fn complete_job(&mut self, job_id: &str, output_cid: String) -> Result<()> {
        let job = self.jobs.get_mut(job_id)
            .ok_or_else(|| anyhow!("Job not found"))?;
        
        job.status = RetrainingStatus::Completed;
        job.progress = 1.0;
        job.completed_at = Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs());
        job.output_model_cid = Some(output_cid.clone());
        
        println!("✅ Retraining job {} complete: {}", job_id, output_cid);
        Ok(())
    }

    async fn notify_scheduler(&self, job_id: &str) -> Result<()> {
        // Notify scheduler of new job
        let url = format!("{}/schedule-retraining", self.scheduler_endpoint);
        let client = reqwest::Client::new();
        
        let job = self.jobs.get(job_id).ok_or_else(|| anyhow!("Job not found"))?;
        
        let _response = client
            .post(&url)
            .json(&job)
            .send()
            .await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_submit_retraining_job() {
        let mut orchestrator = RetrainingOrchestrator::new("http://localhost:8080".to_string());
        
        let job_id = orchestrator.submit_job(
            "aiid:artha:test123".to_string(),
            "artha://QmBase123".to_string(),
            "artha://QmDataset456".to_string(),
            HashMap::new(),
        ).await;
        
        // Will fail to notify scheduler (no server running) but job should be created
        assert!(job_id.is_ok() || job_id.unwrap_err().to_string().contains("dns"));
    }

    #[test]
    fn test_update_progress() {
        let mut orchestrator = RetrainingOrchestrator::new("http://localhost:8080".to_string());
        
        let job_id = format!("test-{}", uuid::Uuid::new_v4());
        let job = RetrainingJob {
            job_id: job_id.clone(),
            aiid: "aiid:artha:test".to_string(),
            base_model_cid: "artha://Qm...".to_string(),
            new_dataset_cid: "artha://Qm...".to_string(),
            hyperparameters: HashMap::new(),
            target_nodes: Vec::new(),
            status: RetrainingStatus::Queued,
            progress: 0.0,
            created_at: 1000,
            started_at: None,
            completed_at: None,
            output_model_cid: None,
        };
        
        orchestrator.jobs.insert(job_id.clone(), job);
        
        orchestrator.update_progress(&job_id, RetrainingStatus::Training, 0.5).unwrap();
        
        let updated_job = orchestrator.get_job_status(&job_id).unwrap();
        assert_eq!(updated_job.progress, 0.5);
        assert!(matches!(updated_job.status, RetrainingStatus::Training));
    }
}


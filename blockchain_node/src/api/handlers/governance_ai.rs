use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::governance::ai_assistant::{self, Proposal as AiProposal, ProposalAction, SimulationInput};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryRequest { pub id: String, pub title: String, pub description: String, pub actions: Vec<ProposalAction> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationRequest { pub burn_schedule_bps: Vec<u32>, pub years_ahead: u32, pub emission_initial_m: f64, pub emission_growth: f64, pub emission_cap_m: f64 }

pub async fn summarize(Json(req): Json<SummaryRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let p = AiProposal { id: req.id, title: req.title, description: req.description, actions: req.actions };
    let s = ai_assistant::summarize(&p);
    Ok(Json(serde_json::to_value(s).unwrap()))
}

pub async fn simulate(Json(req): Json<SimulationRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let si = SimulationInput { burn_schedule_bps: req.burn_schedule_bps, years_ahead: req.years_ahead, emission_initial_m: req.emission_initial_m, emission_growth: req.emission_growth, emission_cap_m: req.emission_cap_m };
    let out = ai_assistant::simulate(&si);
    Ok(Json(serde_json::to_value(out).unwrap()))
}



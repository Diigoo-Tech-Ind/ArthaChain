//! AI Governance Assistant: proposal summaries and parameter simulations
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub actions: Vec<ProposalAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProposalAction {
    UpdateBurnRate { new_schedule_bps: Vec<u32> },
    SetManager { manager: String, address: String },
    UpdatePoolAddress { pool: String, address: String },
    EmergencyBurnOverride { bps: u32, active: bool },
    ParamChange { key: String, value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub key_points: Vec<String>,
    pub risk_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationInput {
    pub burn_schedule_bps: Vec<u32>,
    pub years_ahead: u32,
    pub emission_initial_m: f64,
    pub emission_growth: f64,
    pub emission_cap_m: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub projected_burn_bps: Vec<(u32, u32)>,
    pub emissions_m: Vec<(u32, f64)>,
}

pub fn summarize(proposal: &Proposal) -> Summary {
    let mut key_points = Vec::new();
    let mut risk_notes = Vec::new();
    for a in &proposal.actions {
        match a {
            ProposalAction::UpdateBurnRate { new_schedule_bps } => {
                key_points.push(format!("Updates burn schedule to {:?} bps", new_schedule_bps));
                if !is_non_decreasing(new_schedule_bps) { risk_notes.push("Burn schedule must be non-decreasing".into()); }
                if new_schedule_bps.iter().any(|&r| r>10000) { risk_notes.push("Burn bps must be ≤10000".into()); }
            }
            ProposalAction::SetManager { manager, address } => key_points.push(format!("Sets {} to {}", manager, address)),
            ProposalAction::UpdatePoolAddress { pool, address } => key_points.push(format!("Updates pool {} to {}", pool, address)),
            ProposalAction::EmergencyBurnOverride { bps, active } => {
                key_points.push(format!("Emergency burn override {} at {} bps", if *active {"ENABLED"} else {"DISABLED"}, bps));
                if *bps > 10000 { risk_notes.push("Emergency burn bps must be ≤10000".into()); }
            }
            ProposalAction::ParamChange { key, value } => key_points.push(format!("Param {} -> {}", key, value)),
        }
    }
    Summary { id: proposal.id.clone(), title: proposal.title.clone(), key_points, risk_notes }
}

fn is_non_decreasing(v: &Vec<u32>) -> bool { v.windows(2).all(|w| w[1] >= w[0]) }

pub fn simulate(input: &SimulationInput) -> SimulationResult {
    let mut projected = Vec::new();
    for y in 0..input.years_ahead { projected.push((y, burn_for_year(input.burn_schedule_bps.clone(), y))); }
    let mut emissions = Vec::new();
    for c in 0..=((input.years_ahead/3) as u32 + 1) {
        let em = (input.emission_initial_m * input.emission_growth.powi(c as i32)).min(input.emission_cap_m);
        emissions.push((c*3, round2(em)));
    }
    SimulationResult { projected_burn_bps: projected, emissions_m: emissions }
}

fn burn_for_year(schedule: Vec<u32>, year: u32) -> u32 {
    let idx = (year/2) as usize; if idx >= schedule.len() { *schedule.last().unwrap_or(&schedule[0]) } else { schedule[idx] }
}

fn round2(x: f64) -> f64 { (x * 100.0).round()/100.0 }



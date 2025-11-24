//! Governance Module
//! Handles on-chain governance, proposals, and AI-assisted decision making

pub mod ai_assistant;

// Re-export main types
pub use ai_assistant::{Proposal, ProposalAction, SimulationInput, SimulationResult};

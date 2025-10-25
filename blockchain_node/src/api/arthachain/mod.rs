//! ArthaChain-Specific API Module
//!
//! This module contains APIs that showcase ArthaChain's unique features:
//! - SVCP-SVBFT Consensus
//! - DAG-based Parallel Processing
//! - AI-Native Integration
//! - Quantum Resistance
//! - Self-Healing Systems
//! - Dynamic Role Allocation
//! - Cross-Chain Bridges
//! - Mobile Optimization

pub mod consensus;
pub mod dag;
pub mod ai_native;
pub mod quantum_resistance;
pub mod self_healing;
pub mod dynamic_roles;
pub mod cross_chain;
pub mod mobile;
pub mod enterprise;

pub use consensus::*;
pub use dag::*;
pub use ai_native::*;
pub use quantum_resistance::*;
pub use self_healing::*;
pub use dynamic_roles::*;
pub use cross_chain::*;
pub use mobile::*;
pub use enterprise::*;

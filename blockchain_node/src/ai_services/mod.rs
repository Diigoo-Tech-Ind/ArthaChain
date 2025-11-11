// AI/ML microservices for self-learning system optimization
// All services are stateless with frozen REST/OpenAPI contracts

pub mod risk_scoring;
pub mod anomaly_detection;
pub mod reputation_scoring;
pub mod authenticity_verification;

pub use risk_scoring::*;
pub use anomaly_detection::*;
pub use reputation_scoring::*;
pub use authenticity_verification::*;


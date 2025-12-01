use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyConfig {
    pub default_mode: String, // "mpc", "tee", "hybrid"
    pub mpc_threshold: u8,
    pub mpc_parties: u8,
    pub tee_attestation_required: bool,
    pub key_rotation_days: u32,
    pub backup_enabled: bool,
}

impl Default for CustodyConfig {
    fn default() -> Self {
        CustodyConfig {
            default_mode: "hybrid".to_string(),
            mpc_threshold: 2,
            mpc_parties: 3,
            tee_attestation_required: true,
            key_rotation_days: 90,
            backup_enabled: true,
        }
    }
}

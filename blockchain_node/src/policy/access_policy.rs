use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AccessPolicy {
    Public,
    TokenGated,
    Allowlist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestAccessPolicy {
    pub policy: AccessPolicy,
    pub allow: Option<Vec<String>>,  // DIDs or org IDs
    pub cred_req: Option<Vec<String>>,  // Required VC types: "KYC.L1", "UNI.STUDENT", etc.
}

impl Default for ManifestAccessPolicy {
    fn default() -> Self {
        Self {
            policy: AccessPolicy::Public,
            allow: None,
            cred_req: None,
        }
    }
}

impl ManifestAccessPolicy {
    pub fn is_public(&self) -> bool {
        self.policy == AccessPolicy::Public
    }

    pub fn requires_token(&self) -> bool {
        self.policy == AccessPolicy::TokenGated || self.policy == AccessPolicy::Allowlist
    }

    pub fn is_allowed(&self, did: &str) -> bool {
        match &self.allow {
            None => true, // No allowlist means everyone allowed (if token valid)
            Some(list) => list.iter().any(|allowed_did| allowed_did == did),
        }
    }

    pub fn get_required_claims(&self) -> Vec<String> {
        self.cred_req.clone().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEnforcementResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub required_claims: Vec<String>,
}

impl PolicyEnforcementResult {
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reason: None,
            required_claims: vec![],
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
            required_claims: vec![],
        }
    }

    pub fn needs_claims(claims: Vec<String>) -> Self {
        Self {
            allowed: false,
            reason: Some("Missing required credentials".to_string()),
            required_claims: claims,
        }
    }
}

pub struct PolicyEnforcer {
    attestor_registry_addr: String,
    vc_registry_addr: String,
    did_registry_addr: String,
}

impl PolicyEnforcer {
    pub fn new(
        did_registry_addr: String,
        vc_registry_addr: String,
        attestor_registry_addr: String,
    ) -> Self {
        Self {
            attestor_registry_addr,
            vc_registry_addr,
            did_registry_addr,
        }
    }

    pub async fn enforce(
        &self,
        policy: &ManifestAccessPolicy,
        requestor_did: Option<&str>,
        session_token: Option<&str>,
    ) -> PolicyEnforcementResult {
        // Public access - allow immediately
        if policy.is_public() {
            return PolicyEnforcementResult::allow();
        }

        // Token-gated or allowlist requires authentication
        if policy.requires_token() {
            if session_token.is_none() || requestor_did.is_none() {
                return PolicyEnforcementResult::deny("Authentication required");
            }

            let did = requestor_did.unwrap();

            // Check allowlist
            if policy.policy == AccessPolicy::Allowlist && !policy.is_allowed(did) {
                return PolicyEnforcementResult::deny("Not in allowlist");
            }

            // Check required credentials via VCRegistry
            let required_claims = policy.get_required_claims();
            if !required_claims.is_empty() {
                // Real implementation: Query VCRegistry.hasClaimType for each required claim
                let mut vc_checker = super::VCChecker::new(
                    self.vc_registry_addr.clone(),
                    self.attestor_registry_addr.clone(),
                    "http://localhost:8545".to_string(),  // Should come from config
                );
                
                match vc_checker.check_required_claims(did, &required_claims).await {
                    Ok(missing) if !missing.is_empty() => {
                        return PolicyEnforcementResult::needs_claims(missing);
                    }
                    Err(e) => {
                        return PolicyEnforcementResult::deny(format!("VC check failed: {}", e));
                    }
                    _ => {}
                }
            }
        }

        PolicyEnforcementResult::allow()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_policy() {
        let policy = ManifestAccessPolicy::default();
        assert!(policy.is_public());
        assert!(!policy.requires_token());
    }

    #[test]
    fn test_token_gated_policy() {
        let policy = ManifestAccessPolicy {
            policy: AccessPolicy::TokenGated,
            allow: None,
            cred_req: Some(vec!["KYC.L1".to_string()]),
        };
        assert!(!policy.is_public());
        assert!(policy.requires_token());
        assert_eq!(policy.get_required_claims(), vec!["KYC.L1".to_string()]);
    }

    #[test]
    fn test_allowlist_policy() {
        let policy = ManifestAccessPolicy {
            policy: AccessPolicy::Allowlist,
            allow: Some(vec!["did:artha:abc123".to_string()]),
            cred_req: None,
        };
        assert!(policy.is_allowed("did:artha:abc123"));
        assert!(!policy.is_allowed("did:artha:xyz789"));
    }
}


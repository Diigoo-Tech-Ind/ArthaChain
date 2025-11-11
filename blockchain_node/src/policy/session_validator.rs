use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArthaSession {
    pub session_id: String,  // ArthaSessionID
    pub did: String,  // Artha-DID
    pub issued_at: u64,
    pub expires_at: u64,
    pub scope: Vec<String>,  // Permissions: ["read", "write", "execute"]
    pub signature: String,  // JWT/macaroon signature
}

pub struct SessionValidator {
    sessions: HashMap<String, ArthaSession>,
    session_ttl: u64,  // Seconds
}

impl SessionValidator {
    pub fn new(session_ttl: u64) -> Self {
        Self {
            sessions: HashMap::new(),
            session_ttl,
        }
    }

    pub fn create_session(
        &mut self,
        did: String,
        scope: Vec<String>,
    ) -> ArthaSession {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let session_id = format!(
            "session_{}_{}", 
            &did[10..20], 
            now
        );

        let session = ArthaSession {
            session_id: session_id.clone(),
            did,
            issued_at: now,
            expires_at: now + self.session_ttl,
            scope,
            signature: format!("sig_{}", &session_id[8..20]),
        };

        self.sessions.insert(session_id, session.clone());
        session
    }

    pub fn validate_session(&self, session_id: &str) -> Result<&ArthaSession, String> {
        let session = self.sessions.get(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now > session.expires_at {
            return Err("Session expired".to_string());
        }

        Ok(session)
    }

    pub fn has_scope(&self, session_id: &str, required_scope: &str) -> Result<bool, String> {
        let session = self.validate_session(session_id)?;
        Ok(session.scope.iter().any(|s| s == required_scope))
    }

    pub fn revoke_session(&mut self, session_id: &str) -> Result<(), String> {
        self.sessions.remove(session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        Ok(())
    }

    pub fn cleanup_expired(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.sessions.retain(|_, session| session.expires_at > now);
    }

    pub fn get_session_count(&self) -> usize {
        self.sessions.len()
    }
}

pub fn parse_bearer_token(header: Option<&str>) -> Option<String> {
    header?
        .strip_prefix("Bearer ")?
        .to_string()
        .into()
}

pub fn extract_did_from_session(session: &ArthaSession) -> &str {
    &session.did
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let mut validator = SessionValidator::new(3600);
        let session = validator.create_session(
            "did:artha:test123".to_string(),
            vec!["read".to_string(), "write".to_string()],
        );

        assert_eq!(session.did, "did:artha:test123");
        assert!(session.scope.contains(&"read".to_string()));
    }

    #[test]
    fn test_validate_session() {
        let mut validator = SessionValidator::new(3600);
        let session = validator.create_session(
            "did:artha:test123".to_string(),
            vec!["read".to_string()],
        );

        let result = validator.validate_session(&session.session_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_expired_session() {
        let mut validator = SessionValidator::new(0); // Expire immediately
        let session = validator.create_session(
            "did:artha:test123".to_string(),
            vec!["read".to_string()],
        );

        std::thread::sleep(std::time::Duration::from_secs(1));

        let result = validator.validate_session(&session.session_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_has_scope() {
        let mut validator = SessionValidator::new(3600);
        let session = validator.create_session(
            "did:artha:test123".to_string(),
            vec!["read".to_string(), "write".to_string()],
        );

        assert!(validator.has_scope(&session.session_id, "read").unwrap());
        assert!(validator.has_scope(&session.session_id, "write").unwrap());
        assert!(!validator.has_scope(&session.session_id, "execute").unwrap());
    }

    #[test]
    fn test_revoke_session() {
        let mut validator = SessionValidator::new(3600);
        let session = validator.create_session(
            "did:artha:test123".to_string(),
            vec!["read".to_string()],
        );

        let result = validator.revoke_session(&session.session_id);
        assert!(result.is_ok());

        let validate_result = validator.validate_session(&session.session_id);
        assert!(validate_result.is_err());
    }

    #[test]
    fn test_parse_bearer_token() {
        let header = Some("Bearer session_abc123");
        let token = parse_bearer_token(header);
        assert_eq!(token, Some("session_abc123".to_string()));

        let invalid_header = Some("Invalid format");
        let no_token = parse_bearer_token(invalid_header);
        assert_eq!(no_token, None);
    }
}


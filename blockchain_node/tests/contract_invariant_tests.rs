/// Property-based and invariant tests for Identity contracts
/// Tests DID/VC/AIID contracts for critical invariants

use proptest::prelude::*;
use std::collections::HashSet;

// Mock contract state for testing
#[derive(Debug, Clone)]
struct DIDRegistryState {
    dids: Vec<DIDDocument>,
    owner_to_dids: Vec<(String, Vec<usize>)>, // owner -> did indices
}

#[derive(Debug, Clone)]
struct DIDDocument {
    did_hash: String,
    owner: String,
    auth_key: String,
    enc_key: String,
    created_at: u64,
    updated_at: u64,
    revoked: bool,
}

#[derive(Debug, Clone)]
struct VCRegistryState {
    vcs: Vec<VerifiableCredential>,
    subject_to_vcs: Vec<(String, Vec<usize>)>, // subject -> vc indices
}

#[derive(Debug, Clone)]
struct VerifiableCredential {
    vc_hash: String,
    issuer_did: String,
    subject_did: String,
    claim_hash: String,
    issued_at: u64,
    expires_at: u64,
    revoked: bool,
}

// Invariant 1: DID uniqueness
// A DID hash can only exist once in the registry
fn invariant_did_uniqueness(state: &DIDRegistryState) -> bool {
    let mut seen = HashSet::new();
    for did in &state.dids {
        if !seen.insert(&did.did_hash) {
            return false; // Duplicate found
        }
    }
    true
}

// Invariant 2: DID ownership consistency
// Every DID must have exactly one owner, and owner_to_dids mapping must be consistent
fn invariant_did_ownership_consistency(state: &DIDRegistryState) -> bool {
    // Build reverse mapping from DID -> owner
    let mut did_to_owner = std::collections::HashMap::new();
    for did in &state.dids {
        did_to_owner.insert(&did.did_hash, &did.owner);
    }
    
    // Check owner_to_dids consistency
    for (owner, did_indices) in &state.owner_to_dids {
        for &idx in did_indices {
            if idx >= state.dids.len() {
                return false; // Invalid index
            }
            let did = &state.dids[idx];
            if did.owner != *owner {
                return false; // Ownership mismatch
            }
        }
    }
    
    true
}

// Invariant 3: DID temporal consistency
// created_at <= updated_at for all DIDs
fn invariant_did_temporal_consistency(state: &DIDRegistryState) -> bool {
    state.dids.iter().all(|did| did.created_at <= did.updated_at)
}

// Invariant 4: Revoked DIDs cannot be updated (except revocation itself)
// Once revoked, updated_at should not change further
fn invariant_revoked_did_immutability(old_state: &DIDRegistryState, new_state: &DIDRegistryState) -> bool {
    for (old_did, new_did) in old_state.dids.iter().zip(new_state.dids.iter()) {
        if old_did.revoked && old_did.did_hash == new_did.did_hash {
            // If DID was revoked in old state, keys should not change
            if old_did.auth_key != new_did.auth_key || old_did.enc_key != new_did.enc_key {
                return false;
            }
        }
    }
    true
}

// Invariant 5: VC uniqueness
// A VC hash can only exist once
fn invariant_vc_uniqueness(state: &VCRegistryState) -> bool {
    let mut seen = HashSet::new();
    for vc in &state.vcs {
        if !seen.insert(&vc.vc_hash) {
            return false;
        }
    }
    true
}

// Invariant 6: VC temporal validity
// issued_at <= expires_at (if expires_at != 0)
fn invariant_vc_temporal_validity(state: &VCRegistryState) -> bool {
    state.vcs.iter().all(|vc| {
        vc.expires_at == 0 || vc.issued_at <= vc.expires_at
    })
}

// Invariant 7: VC subject mapping consistency
fn invariant_vc_subject_consistency(state: &VCRegistryState) -> bool {
    for (subject_did, vc_indices) in &state.subject_to_vcs {
        for &idx in vc_indices {
            if idx >= state.vcs.len() {
                return false;
            }
            let vc = &state.vcs[idx];
            if vc.subject_did != *subject_did {
                return false;
            }
        }
    }
    true
}

// Invariant 8: Revoked VCs remain revoked
// Once a VC is revoked, it cannot be un-revoked
fn invariant_vc_revocation_permanence(old_state: &VCRegistryState, new_state: &VCRegistryState) -> bool {
    for (old_vc, new_vc) in old_state.vcs.iter().zip(new_state.vcs.iter()) {
        if old_vc.revoked && old_vc.vc_hash == new_vc.vc_hash {
            if !new_vc.revoked {
                return false; // Revocation reversed - invalid!
            }
        }
    }
    true
}

// Invariant 9: VC expiration monotonicity
// Expiration time cannot be extended after issuance
fn invariant_vc_expiration_monotonicity(old_state: &VCRegistryState, new_state: &VCRegistryState) -> bool {
    for (old_vc, new_vc) in old_state.vcs.iter().zip(new_state.vcs.iter()) {
        if old_vc.vc_hash == new_vc.vc_hash {
            if old_vc.expires_at != 0 && new_vc.expires_at > old_vc.expires_at {
                return false; // Expiration extended - invalid!
            }
        }
    }
    true
}

// Invariant 10: Cross-registry DID reference integrity
// All issuer_did and subject_did in VCs must reference valid DIDs
fn invariant_cross_registry_integrity(did_state: &DIDRegistryState, vc_state: &VCRegistryState) -> bool {
    let valid_dids: HashSet<_> = did_state.dids.iter().map(|d| &d.did_hash).collect();
    
    for vc in &vc_state.vcs {
        if !valid_dids.contains(&vc.issuer_did) || !valid_dids.contains(&vc.subject_did) {
            return false; // VC references non-existent DID
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_uniqueness_invariant() {
        let state = DIDRegistryState {
            dids: vec![
                DIDDocument {
                    did_hash: "0xabc123".to_string(),
                    owner: "0x1".to_string(),
                    auth_key: "0xkey1".to_string(),
                    enc_key: "0xenc1".to_string(),
                    created_at: 1000,
                    updated_at: 1000,
                    revoked: false,
                },
                DIDDocument {
                    did_hash: "0xdef456".to_string(),
                    owner: "0x2".to_string(),
                    auth_key: "0xkey2".to_string(),
                    enc_key: "0xenc2".to_string(),
                    created_at: 2000,
                    updated_at: 2000,
                    revoked: false,
                },
            ],
            owner_to_dids: vec![],
        };
        
        assert!(invariant_did_uniqueness(&state));
    }

    #[test]
    fn test_did_uniqueness_violation() {
        let state = DIDRegistryState {
            dids: vec![
                DIDDocument {
                    did_hash: "0xabc123".to_string(),
                    owner: "0x1".to_string(),
                    auth_key: "0xkey1".to_string(),
                    enc_key: "0xenc1".to_string(),
                    created_at: 1000,
                    updated_at: 1000,
                    revoked: false,
                },
                DIDDocument {
                    did_hash: "0xabc123".to_string(), // Duplicate!
                    owner: "0x2".to_string(),
                    auth_key: "0xkey2".to_string(),
                    enc_key: "0xenc2".to_string(),
                    created_at: 2000,
                    updated_at: 2000,
                    revoked: false,
                },
            ],
            owner_to_dids: vec![],
        };
        
        assert!(!invariant_did_uniqueness(&state));
    }

    #[test]
    fn test_did_ownership_consistency() {
        let state = DIDRegistryState {
            dids: vec![
                DIDDocument {
                    did_hash: "0xabc123".to_string(),
                    owner: "0x1".to_string(),
                    auth_key: "0xkey1".to_string(),
                    enc_key: "0xenc1".to_string(),
                    created_at: 1000,
                    updated_at: 1000,
                    revoked: false,
                },
                DIDDocument {
                    did_hash: "0xdef456".to_string(),
                    owner: "0x1".to_string(),
                    auth_key: "0xkey2".to_string(),
                    enc_key: "0xenc2".to_string(),
                    created_at: 2000,
                    updated_at: 2000,
                    revoked: false,
                },
            ],
            owner_to_dids: vec![("0x1".to_string(), vec![0, 1])],
        };
        
        assert!(invariant_did_ownership_consistency(&state));
    }

    #[test]
    fn test_did_temporal_consistency() {
        let valid_state = DIDRegistryState {
            dids: vec![
                DIDDocument {
                    did_hash: "0xabc123".to_string(),
                    owner: "0x1".to_string(),
                    auth_key: "0xkey1".to_string(),
                    enc_key: "0xenc1".to_string(),
                    created_at: 1000,
                    updated_at: 2000, // updated_at > created_at (valid)
                    revoked: false,
                },
            ],
            owner_to_dids: vec![],
        };
        
        assert!(invariant_did_temporal_consistency(&valid_state));
        
        let invalid_state = DIDRegistryState {
            dids: vec![
                DIDDocument {
                    did_hash: "0xabc123".to_string(),
                    owner: "0x1".to_string(),
                    auth_key: "0xkey1".to_string(),
                    enc_key: "0xenc1".to_string(),
                    created_at: 2000,
                    updated_at: 1000, // updated_at < created_at (invalid!)
                    revoked: false,
                },
            ],
            owner_to_dids: vec![],
        };
        
        assert!(!invariant_did_temporal_consistency(&invalid_state));
    }

    #[test]
    fn test_revoked_did_immutability() {
        let old_state = DIDRegistryState {
            dids: vec![
                DIDDocument {
                    did_hash: "0xabc123".to_string(),
                    owner: "0x1".to_string(),
                    auth_key: "0xkey1".to_string(),
                    enc_key: "0xenc1".to_string(),
                    created_at: 1000,
                    updated_at: 1500,
                    revoked: true, // Already revoked
                },
            ],
            owner_to_dids: vec![],
        };
        
        let valid_new_state = DIDRegistryState {
            dids: vec![
                DIDDocument {
                    did_hash: "0xabc123".to_string(),
                    owner: "0x1".to_string(),
                    auth_key: "0xkey1".to_string(), // Keys unchanged (valid)
                    enc_key: "0xenc1".to_string(),
                    created_at: 1000,
                    updated_at: 1500,
                    revoked: true,
                },
            ],
            owner_to_dids: vec![],
        };
        
        assert!(invariant_revoked_did_immutability(&old_state, &valid_new_state));
        
        let invalid_new_state = DIDRegistryState {
            dids: vec![
                DIDDocument {
                    did_hash: "0xabc123".to_string(),
                    owner: "0x1".to_string(),
                    auth_key: "0xNEWKEY".to_string(), // Key rotated after revocation (invalid!)
                    enc_key: "0xenc1".to_string(),
                    created_at: 1000,
                    updated_at: 2000,
                    revoked: true,
                },
            ],
            owner_to_dids: vec![],
        };
        
        assert!(!invariant_revoked_did_immutability(&old_state, &invalid_new_state));
    }

    #[test]
    fn test_vc_uniqueness() {
        let state = VCRegistryState {
            vcs: vec![
                VerifiableCredential {
                    vc_hash: "0xvc1".to_string(),
                    issuer_did: "0xissuer1".to_string(),
                    subject_did: "0xsubject1".to_string(),
                    claim_hash: "0xclaim1".to_string(),
                    issued_at: 1000,
                    expires_at: 5000,
                    revoked: false,
                },
                VerifiableCredential {
                    vc_hash: "0xvc2".to_string(),
                    issuer_did: "0xissuer1".to_string(),
                    subject_did: "0xsubject2".to_string(),
                    claim_hash: "0xclaim2".to_string(),
                    issued_at: 2000,
                    expires_at: 6000,
                    revoked: false,
                },
            ],
            subject_to_vcs: vec![],
        };
        
        assert!(invariant_vc_uniqueness(&state));
    }

    #[test]
    fn test_vc_temporal_validity() {
        let valid_state = VCRegistryState {
            vcs: vec![
                VerifiableCredential {
                    vc_hash: "0xvc1".to_string(),
                    issuer_did: "0xissuer1".to_string(),
                    subject_did: "0xsubject1".to_string(),
                    claim_hash: "0xclaim1".to_string(),
                    issued_at: 1000,
                    expires_at: 5000, // issued_at < expires_at (valid)
                    revoked: false,
                },
            ],
            subject_to_vcs: vec![],
        };
        
        assert!(invariant_vc_temporal_validity(&valid_state));
        
        let invalid_state = VCRegistryState {
            vcs: vec![
                VerifiableCredential {
                    vc_hash: "0xvc1".to_string(),
                    issuer_did: "0xissuer1".to_string(),
                    subject_did: "0xsubject1".to_string(),
                    claim_hash: "0xclaim1".to_string(),
                    issued_at: 5000,
                    expires_at: 1000, // issued_at > expires_at (invalid!)
                    revoked: false,
                },
            ],
            subject_to_vcs: vec![],
        };
        
        assert!(!invariant_vc_temporal_validity(&invalid_state));
    }

    #[test]
    fn test_vc_revocation_permanence() {
        let old_state = VCRegistryState {
            vcs: vec![
                VerifiableCredential {
                    vc_hash: "0xvc1".to_string(),
                    issuer_did: "0xissuer1".to_string(),
                    subject_did: "0xsubject1".to_string(),
                    claim_hash: "0xclaim1".to_string(),
                    issued_at: 1000,
                    expires_at: 5000,
                    revoked: true, // Already revoked
                },
            ],
            subject_to_vcs: vec![],
        };
        
        let valid_new_state = VCRegistryState {
            vcs: vec![
                VerifiableCredential {
                    vc_hash: "0xvc1".to_string(),
                    issuer_did: "0xissuer1".to_string(),
                    subject_did: "0xsubject1".to_string(),
                    claim_hash: "0xclaim1".to_string(),
                    issued_at: 1000,
                    expires_at: 5000,
                    revoked: true, // Still revoked (valid)
                },
            ],
            subject_to_vcs: vec![],
        };
        
        assert!(invariant_vc_revocation_permanence(&old_state, &valid_new_state));
        
        let invalid_new_state = VCRegistryState {
            vcs: vec![
                VerifiableCredential {
                    vc_hash: "0xvc1".to_string(),
                    issuer_did: "0xissuer1".to_string(),
                    subject_did: "0xsubject1".to_string(),
                    claim_hash: "0xclaim1".to_string(),
                    issued_at: 1000,
                    expires_at: 5000,
                    revoked: false, // Un-revoked! (invalid!)
                },
            ],
            subject_to_vcs: vec![],
        };
        
        assert!(!invariant_vc_revocation_permanence(&old_state, &invalid_new_state));
    }

    #[test]
    fn test_cross_registry_integrity() {
        let did_state = DIDRegistryState {
            dids: vec![
                DIDDocument {
                    did_hash: "0xdid1".to_string(),
                    owner: "0x1".to_string(),
                    auth_key: "0xkey1".to_string(),
                    enc_key: "0xenc1".to_string(),
                    created_at: 1000,
                    updated_at: 1000,
                    revoked: false,
                },
                DIDDocument {
                    did_hash: "0xdid2".to_string(),
                    owner: "0x2".to_string(),
                    auth_key: "0xkey2".to_string(),
                    enc_key: "0xenc2".to_string(),
                    created_at: 2000,
                    updated_at: 2000,
                    revoked: false,
                },
            ],
            owner_to_dids: vec![],
        };
        
        let valid_vc_state = VCRegistryState {
            vcs: vec![
                VerifiableCredential {
                    vc_hash: "0xvc1".to_string(),
                    issuer_did: "0xdid1".to_string(), // Valid DID
                    subject_did: "0xdid2".to_string(), // Valid DID
                    claim_hash: "0xclaim1".to_string(),
                    issued_at: 3000,
                    expires_at: 8000,
                    revoked: false,
                },
            ],
            subject_to_vcs: vec![],
        };
        
        assert!(invariant_cross_registry_integrity(&did_state, &valid_vc_state));
        
        let invalid_vc_state = VCRegistryState {
            vcs: vec![
                VerifiableCredential {
                    vc_hash: "0xvc1".to_string(),
                    issuer_did: "0xdid1".to_string(),
                    subject_did: "0xNONEXISTENT".to_string(), // Invalid DID!
                    claim_hash: "0xclaim1".to_string(),
                    issued_at: 3000,
                    expires_at: 8000,
                    revoked: false,
                },
            ],
            subject_to_vcs: vec![],
        };
        
        assert!(!invariant_cross_registry_integrity(&did_state, &invalid_vc_state));
    }
}

// Property-based tests using proptest
proptest! {
    #[test]
    fn prop_did_hashes_are_unique(dids in prop::collection::vec(
        (any::<String>(), any::<String>(), any::<u64>()),
        0..10
    )) {
        let did_docs: Vec<DIDDocument> = dids.iter().enumerate().map(|(i, (owner, key, ts))| {
            DIDDocument {
                did_hash: format!("0x{:x}", i), // Unique hash
                owner: owner.clone(),
                auth_key: key.clone(),
                enc_key: format!("0xenc{}", i),
                created_at: *ts,
                updated_at: *ts,
                revoked: false,
            }
        }).collect();
        
        let state = DIDRegistryState {
            dids: did_docs,
            owner_to_dids: vec![],
        };
        
        prop_assert!(invariant_did_uniqueness(&state));
    }

    #[test]
    fn prop_temporal_ordering_holds(created_at in 0u64..1000000u64, delta in 0u64..1000000u64) {
        let did = DIDDocument {
            did_hash: "0xtest".to_string(),
            owner: "0xowner".to_string(),
            auth_key: "0xauth".to_string(),
            enc_key: "0xenc".to_string(),
            created_at,
            updated_at: created_at + delta, // Always >= created_at
            revoked: false,
        };
        
        let state = DIDRegistryState {
            dids: vec![did],
            owner_to_dids: vec![],
        };
        
        prop_assert!(invariant_did_temporal_consistency(&state));
    }

    #[test]
    fn prop_vc_temporal_validity_holds(issued_at in 0u64..1000000u64, duration in 0u64..1000000u64) {
        let vc = VerifiableCredential {
            vc_hash: "0xvc".to_string(),
            issuer_did: "0xissuer".to_string(),
            subject_did: "0xsubject".to_string(),
            claim_hash: "0xclaim".to_string(),
            issued_at,
            expires_at: issued_at + duration, // Always >= issued_at
            revoked: false,
        };
        
        let state = VCRegistryState {
            vcs: vec![vc],
            subject_to_vcs: vec![],
        };
        
        prop_assert!(invariant_vc_temporal_validity(&state));
    }
}


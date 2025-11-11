# Artha Identity + AI + SVDB — 10-Year LTS Policy

**Version:** 1.0  
**Effective Date:** November 2, 2025  
**Sunset Date:** November 2, 2035  
**Status:** ✅ ACTIVE

---

## Executive Summary

ArthaChain commits to **10 years of Long-Term Support (LTS)** for the Identity + AI + SVDB integrated platform. This policy guarantees:

- **Frozen ABIs** for all v1 registry contracts
- **Stable URI namespaces** (did:artha, aiid:artha, etc.)
- **Backward-compatible schema evolution**
- **24-month minimum deprecation window**
- **Semantic SDK versioning**

This document defines the policies, processes, and technical commitments that enable applications built on Artha to run unchanged for a decade.

---

## 1. Frozen Elements (No Breaking Changes Until 2035)

### 1.1 URI Namespaces

| Namespace | Format | Registry | Frozen Since |
|-----------|--------|----------|--------------|
| `did:artha:` | `did:artha:<hash>` | ArthaDIDRegistry | Nov 2, 2025 |
| `aiid:artha:` | `aiid:artha:<hash>` | ArthaAIIDRegistry | Nov 2, 2025 |
| `ArthaNodeCert` | `bytes32 pubkey` | NodeCertRegistry | Nov 2, 2025 |
| `ArthaDatasetID` | `bytes32` | DatasetRegistry | Nov 2, 2025 |
| `ArthaJobID` | `bytes32` | JobRegistry | Nov 2, 2025 |
| `ArthaProofID` | `bytes32` | ProofRegistry | Nov 2, 2025 |
| `ArthaOrgID` | `did:artha:<hash>` | ArthaDIDRegistry | Nov 2, 2025 |
| `ArthaAppID` | `did:artha:<hash>` | ArthaDIDRegistry | Nov 2, 2025 |
| `ArthaLicenseID` | `bytes32` | DatasetRegistry | Nov 2, 2025 |
| `ArthaSLAID` | `bytes32` | NodeCertRegistry | Nov 2, 2025 |
| `ArthaSessionID` | `string (JWT)` | SessionValidator | Nov 2, 2025 |

**Commitment:** These namespaces will NOT change format or semantics for 10 years. Applications can hardcode parsing logic safely.

### 1.2 Contract ABIs (v1)

All v1 function signatures are frozen. No parameter changes, no removal of functions.

#### ArthaDIDRegistry v1
```solidity
function createDID(bytes32 authKey, bytes32 encKey, bytes32 metaCid) external returns (bytes32);
function getDID(bytes32 did) external view returns (DIDDocument memory);
function rotateKeys(bytes32 did, bytes32 newAuth, bytes32 newEnc) external;
function revokeDID(bytes32 did) external;
function verifySignature(bytes32 did, bytes32 messageHash, bytes32 signature) external view returns (bool);
```

#### ArthaAIIDRegistry v1
```solidity
function createAIID(bytes32 ownerDid, bytes32 modelCid, bytes32 datasetId, bytes32 codeHash, string calldata version) external returns (bytes32);
function getAIID(bytes32 aiid) external view returns (AIIdentity memory);
function rotateAIID(bytes32 aiid, bytes32 newModelCid, string calldata newVersion) external;
function linkOwner(bytes32 aiid, bytes32 ownerDid) external;
function revokeAIID(bytes32 aiid) external;
```

#### VCRegistry v1
```solidity
function issueVC(bytes32 issuerDid, bytes32 subjectDid, bytes32 claimHash, bytes32 docCid, uint64 expiresAt) external returns (bytes32);
function revokeVC(bytes32 vcHash) external;
function isValid(bytes32 vcHash) external view returns (bool);
```

*[Full contract ABIs in `/contracts/`]*

**Commitment:** These function signatures will NOT change for 10 years. New features will be added as NEW functions only.

### 1.3 Event Signatures

All v1 event signatures are frozen for indexing compatibility.

```solidity
event DIDCreated(bytes32 indexed did, address indexed owner);
event VCIssued(bytes32 indexed vcHash, bytes32 indexed issuerDid, bytes32 indexed subjectDid);
event AIIDCreated(bytes32 indexed aiid, bytes32 indexed ownerDid);
event SchemaActivated(string indexed name, string version);
```

**Commitment:** Indexers built against these event signatures will work for 10 years. Event topic0 hashes will not change.

---

## 2. Schema Versioning Policy

### 2.1 Schema Identifier Format

All schemas use the format: `artha://schema/<SchemaName>@<version>`

Example:
- `artha://schema/DIDDoc@v1`
- `artha://schema/AIIDDoc@v1`
- `artha://schema/VC@v1`

### 2.2 Version Compatibility Rules

| Change Type | Version Increment | Backward Compatible | Deprecation Required |
|-------------|-------------------|---------------------|----------------------|
| Add optional field | Minor (v1.1 → v1.2) | ✅ Yes | ❌ No |
| Add new schema | N/A (new v1) | ✅ Yes | ❌ No |
| Rename field | Major (v1 → v2) | ❌ No | ✅ Yes (24 months) |
| Remove field | Major (v1 → v2) | ❌ No | ✅ Yes (24 months) |
| Change field type | Major (v1 → v2) | ❌ No | ✅ Yes (24 months) |

### 2.3 Schema Evolution Example

**v1 (Nov 2025):**
```json
{
  "@schema": "artha://schema/DIDDoc@v1",
  "id": "did:artha:...",
  "publicKey": [...]
}
```

**v1.1 (May 2026) — Additive, compatible:**
```json
{
  "@schema": "artha://schema/DIDDoc@v1.1",
  "id": "did:artha:...",
  "publicKey": [...],
  "recoveryKey": "..." // NEW optional field
}
```

**v2 (Nov 2027) — Breaking change:**
```json
{
  "@schema": "artha://schema/DIDDoc@v2",
  "id": "did:artha:...",
  "keys": {  // RENAMED from publicKey
    "auth": "...",
    "enc": "..."
  }
}
```

**Timeline:**
- Nov 2025: v1 launched
- May 2026: v1.1 released (backward compatible, both work)
- Nov 2027: v2 announced, v1 deprecated
- Nov 2029: v1 sunset (24 months later)

### 2.4 VersionRegistry Contract

All schema versions are tracked on-chain:

```solidity
function setActiveSchema(string name, string version) external onlyGovernance;
function announceDeprecation(string name, string oldVersion, uint64 sunsetEpoch) external onlyGovernance;
function getActiveSchema(string name) external view returns (string);
```

**Commitment:** Schema deprecations will always provide ≥ 24 months notice via blockchain events.

---

## 3. Deprecation Process

### 3.1 Deprecation Timeline

```
┌──────────────┬──────────────┬──────────────┬──────────────┐
│   Active     │  Deprecated  │   Sunset     │  Archived    │
│   (Years)    │  (24 months) │  (Final 6mo) │  (Reference) │
└──────────────┴──────────────┴──────────────┴──────────────┘
     ^              ^               ^               ^
     │              │               │               │
   Launch     Deprecation      Migration        End of
              Announced       Deadline         Support
```

### 3.2 Deprecation Phases

#### Phase 1: Active (Years 1-N)
- Full support
- Bug fixes
- Security patches
- No breaking changes

#### Phase 2: Deprecated (24 months)
- Deprecation warning in docs
- On-chain `SchemaDeprecationAnnounced` event
- SDK emits warnings
- New version available and recommended
- Old version still fully functional

#### Phase 3: Sunset (6 months)
- Final migration window
- Automated migration tools provided
- Support ends after sunset date
- Old version still functional but unsupported

#### Phase 4: Archived (Post-sunset)
- No support
- Documentation moved to archive
- Old version still functional (blockchain immutable)
- No SDK updates

### 3.3 Migration Support

ArthaChain provides:
- **Automated migration scripts** for common patterns
- **Side-by-side SDK support** during deprecation window
- **Migration guides** with code examples
- **Test suites** for migration validation

Example SDK shim (TypeScript):
```typescript
// OLD v1 API (deprecated but still works)
await arthaID.createDID(authKey, encKey, metaCid);

// NEW v2 API (recommended)
await arthaID.v2.createDID({ keys: { auth: authKey, enc: encKey }, meta: metaCid });

// SDK automatically calls correct contract version
```

---

## 4. SDK Semantic Versioning

### 4.1 Version Format

**Format:** `MAJOR.MINOR.PATCH`

Example: `arthajs@1.3.2`

### 4.2 Version Increment Rules

| Change | Version | Example | Backward Compatible |
|--------|---------|---------|---------------------|
| Bug fix | PATCH | 1.3.2 → 1.3.3 | ✅ Yes |
| New feature (additive) | MINOR | 1.3.3 → 1.4.0 | ✅ Yes |
| Breaking API change | MAJOR | 1.4.0 → 2.0.0 | ❌ No |

### 4.3 SDK Compatibility Matrix

| SDK Version | Contract Version | Schema Version | Support Status |
|-------------|------------------|----------------|----------------|
| arthajs@1.x | v1 | DIDDoc@v1, VC@v1 | ✅ Active (Until 2035) |
| arthajs@2.x | v1, v2 | DIDDoc@v2, VC@v1 | 🔄 In Development |
| arthapy@1.x | v1 | DIDDoc@v1, VC@v1 | ✅ Active (Until 2035) |
| arthai CLI 1.x | v1 | All v1 schemas | ✅ Active (Until 2035) |

### 4.4 Long-Term Support Windows

- **Major versions:** 5 years active support
- **Minor versions:** 2 years active support
- **Security patches:** Backported to all active majors

Example:
- `arthajs@1.0.0` released Nov 2025
- `arthajs@2.0.0` released Nov 2028
- `arthajs@1.x` supported until Nov 2030 (security patches only after v2 release)

---

## 5. Public Schema Registry

### 5.1 Web Interface

**URL:** https://schemas.arthachain.online

Features:
- Browse all schemas
- View deprecation status
- Download JSON schemas
- See migration paths
- Check compatibility

### 5.2 REST API

```bash
# Get active schema version
curl https://schemas.arthachain.online/api/v1/schema/DIDDoc

# Get all versions
curl https://schemas.arthachain.online/api/v1/schema/DIDDoc/versions

# Check if deprecated
curl https://schemas.arthachain.online/api/v1/schema/DIDDoc@v1/status
```

### 5.3 On-Chain Registry

All schema metadata is also stored on-chain in the `VersionRegistry` contract:

```solidity
// Deployed at: 0x... (mainnet address TBD)
contract VersionRegistry {
    mapping(string => SchemaVersion) public schemas;
    
    struct SchemaVersion {
        string activeVersion;
        uint64 deprecationSunsetEpoch;
        uint64 lastUpdated;
    }
}
```

---

## 6. Governance & Updates

### 6.1 Who Can Deprecate Schemas?

Schema deprecations require **DAO approval** via on-chain governance:

1. Proposal submitted to DAO
2. 7-day discussion period
3. 72-hour voting window
4. 60% supermajority required
5. 48-hour timelock after approval
6. Execution by governance multisig

### 6.2 Emergency Procedures

**Security-critical updates** can be fast-tracked:

1. Emergency council (5-of-9 multisig) can pause affected contracts
2. 24-hour window for community review
3. Automatic unpause after 7 days if no fix deployed
4. Post-incident transparency report required

**Example:** Critical vulnerability in signature verification

- Emergency council pauses `ArthaDIDRegistry.verifySignature`
- Fix deployed and audited within 24 hours
- Council approves fix deployment
- Post-mortem published publicly

### 6.3 Transparency Commitments

All governance actions are:
- ✅ Publicly logged on-chain
- ✅ Announced 48 hours in advance (except emergencies)
- ✅ Documented in changelog
- ✅ Explained in blog posts

---

## 7. Testing & Validation

### 7.1 Compatibility Test Suite

ArthaChain maintains a comprehensive test suite ensuring:
- Old SDKs work with new contracts
- Old schemas are readable by new parsers
- Deprecated functions still work until sunset

**Test Coverage:**
- 1,000+ compatibility test cases
- Automated regression testing
- Cross-version integration tests
- Performance benchmarks

### 7.2 Deprecation Dry Runs

Before announcing any deprecation:
1. Internal dry run with simulated sunset
2. Public beta period (3 months)
3. Community feedback period
4. Final deprecation announcement

### 7.3 Continuous Monitoring

Automated systems monitor:
- Usage of deprecated APIs (SDK telemetry, opt-in)
- Migration progress
- Compatibility issues reported by users
- Performance regressions

---

## 8. Developer Resources

### 8.1 Migration Guides

Location: `docs/migrations/`

Example files:
- `DIDDoc_v1_to_v2.md` — Step-by-step migration
- `VC_v1_to_v1.1.md` — Additive changes guide
- `AIID_v1_deprecation.md` — Timeline and alternatives

### 8.2 SDK Documentation

- **arthajs:** https://docs.arthachain.online/sdk/js
- **arthapy:** https://docs.arthachain.online/sdk/py
- **arthai CLI:** https://docs.arthachain.online/cli

Each SDK version has frozen documentation:
- `arthajs@1.x` docs will remain accessible until 2035
- Historical versions available at: `docs.arthachain.online/sdk/js/v1.x`

### 8.3 Community Support

- **Discord:** https://discord.gg/arthachain
- **Forum:** https://forum.arthachain.online
- **GitHub Discussions:** https://github.com/arthachain/arthachain/discussions

---

## 9. Compliance & Audits

### 9.1 Annual LTS Reviews

Every year, ArthaChain publishes a **LTS Compliance Report**:
- ✅ Confirmation that no breaking changes were introduced
- ✅ List of new features added (backward compatible)
- ✅ Deprecation announcements and status
- ✅ Security audit summaries

### 9.2 Third-Party Audits

- **Contract audits:** Every 12 months (Trail of Bits, ConsenSys Diligence)
- **SDK audits:** Every 18 months (NCC Group, Cure53)
- **Infrastructure audits:** Annually (SOC 2 Type II)

### 9.3 Backwards Compatibility Certification

Applications can request **LTS Certification**:
- ArthaChain tests your app against future SDK versions
- Guaranteed compatibility badge
- Priority support during migration windows

---

## 10. Contact & Support

### 10.1 LTS Policy Questions

- **Email:** lts@arthachain.online
- **Discord:** #lts-policy channel
- **Office Hours:** Every Thursday, 2 PM UTC (Zoom)

### 10.2 Report Compatibility Issues

- **GitHub:** https://github.com/arthachain/arthachain/issues/new?template=compatibility
- **Severity SLA:** Critical issues responded to within 4 hours

### 10.3 Request Schema Deprecation

- **Governance Forum:** https://forum.arthachain.online/c/proposals
- **Template:** Use "Schema Deprecation Proposal" template
- **Timeline:** Allow 60 days for review + 24 months for deprecation

---

## Appendix A: Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | Nov 2, 2025 | Initial 10-year LTS policy published |

---

## Appendix B: Legal Commitments

This policy represents a **technical commitment** by ArthaChain Foundation. While we cannot legally guarantee code behavior for 10 years, we commit to:

1. **Best effort maintenance** of backward compatibility
2. **Public transparency** for all breaking changes
3. **Community governance** for deprecation decisions
4. **Open-source codebase** allowing forks if governance changes direction

**Effective Date:** November 2, 2025  
**Signed:** ArthaChain Foundation Technical Committee

---

*This document is stored on SVDB at `artha://lts-policy-v1` and on-chain in the VersionRegistry contract.*


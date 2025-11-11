## Bridges & Oracle Hardening

### Threat Model
- Replay across chains, reorgs, gas griefing, oracle price manipulation, governance key compromise.

### Circuit Breakers
- Rate caps per address and per window; bounded queue sizes.
- Pause guardians with separate keys; multi-sig; timelock for resume.
- Proof verification strictness; minimum confirmations; reorg depth handling.

### Keys & On-call
- Separate guardian keys; cold storage for governance; HSM-backed signers.
- On-call roster with escalation matrix; 24/7 rotation.

### Transparency
- Public incident log; change logs for parameter updates; publish audit trails of bridge/oracle changes.



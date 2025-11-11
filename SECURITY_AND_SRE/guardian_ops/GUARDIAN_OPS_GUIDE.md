# Guardian Operations Guide

## Roles
- Guardians: manage pause/resume and parameter updates.
- Upgrader: manages UUPS upgrades.

## Keys
- Use separate keys per environment; HSM-backed where possible.
- Multisig for sensitive actions; document signers and thresholds.

## Procedures
- Pause Bridge/Oracle: propose → review → execute → log in transparency log.
- Resume: verify conditions → execute → monitor → log.
- Parameter updates: propose change, verify bounds, execute via governance, log.
- Key rotation: announce window, update configs, verify, log.

## On-call
- Escalation matrix with response times; 24/7 rotation.
- Pager integration and runbook references.

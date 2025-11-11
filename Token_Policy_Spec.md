# ArthaCoin Token Policy Specification

## Scope
- Contracts: `ArthaCoin`, `CycleManager`, `BurnManager`, `AntiWhaleManager`, `PriceOracle`.

## Invariants
- Emission splits sum to 100%: 45% validators, 20% rewards pool, 10% ecosystem, 10% marketing, 5% developers, 5% DAO, 5% treasury.
- Burn rate is non-decreasing over time; emergency override must be <= 10000 bps.
- Anti-whale limits: holding ≤ 1.5% supply, transfer ≤ 0.5% supply, unless overrides active.

## Governance Controls
- `GOVERNANCE_ROLE` may:
  - Update manager addresses (`setCycleManager`, `setBurnManager`, `setAntiWhaleManager`).
  - Update pool addresses (non-zero only).
  - Set burn exemptions for system pools.
  - Adjust anti-whale overrides and grace periods.
  - Set emergency burn rate override (≤ 10000 bps).
  - Update burn schedule (non-decreasing, ≤ 10000 bps each step).
- `UPGRADER_ROLE` authorizes UUPS upgrades.

## Non‑Changeable
- Emission split percentages at the token level (must remain as specified; only pool destination addresses changeable).
- Decimals (18) and symbol (`ARTHA`).

## Emergency Procedures
- Emergency burn override: temporarily set via governance, must be logged and timeboxed, and later disabled.
- Anti-whale emergency disable: set to 100% caps to allow recovery; must be logged and later restored.

## Transparency
- Emit events for all manager and pool updates; maintain public changelog.



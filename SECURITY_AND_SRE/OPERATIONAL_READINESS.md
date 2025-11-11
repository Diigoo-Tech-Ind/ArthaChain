## Operational Readiness (SRE)

### SLOs
- RPC p95 latency: < 200ms
- Block finality time: < 2s
- Faucet uptime: 99.9%
- Indexer lag: < 2 blocks

### Telemetry
- Prometheus: existing dashboards `grafana-*.json` and `prometheus.yml` under `blockchain_node/`.
- OpenTelemetry: enable tracing export; tag spans by shard, role, endpoint.
- On-chain health probes: `/api/v1/monitoring/*`, `/api/v1/test/health`.

### Dashboards
- Consensus, network performance, security monitoring (Grafana JSONs included in repo).
- Slash/rotation: extend to validator rotation/slashing metrics exposed by staking subsystem.

### Runbooks
- Incident triage: collect logs, metrics, traces; identify scope (consensus, RPC, storage, faucet, bridge).
- Key rotation: rotate governance and guardian keys via DAO proposals and HSM-backed custodians.
- Chain halt & resume: coordinated stop, finalize state, resume with checkpoint.
- UUPS rollback: freeze upgrades, deploy prior implementation, execute via `UPGRADER_ROLE` with timelock.
- Bridge pause: guardians invoke pause; queues drained safely; reorg handling enforced.
- Oracle failure: revert to floor/ceiling bounds; switch DAO to safe base price.



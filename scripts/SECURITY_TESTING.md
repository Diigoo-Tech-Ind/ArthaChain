### ArthaChain Security Testing Suite

Scope: ArthaCoin, CycleManager, BurnManager, AntiWhaleManager, DealMarket, ModelRegistry, DatasetRegistry, DAO module, EVM runtime glue, bridges, PriceOracle, faucet.

1) Foundry (fuzz + invariants)
- Config: `contracts/foundry.toml`
- Run fuzz/invariants:
```
cd contracts
forge test -vvv
forge test --ffi --runs 256 --mc ArthaCoinInvariants -vvv
```

2) Echidna (property-based fuzzing)
- Config: `contracts/echidna.yaml`
- Target: `contracts/test/ArthaCoinEchidna.sol`
```
echidna-test contracts/test/ArthaCoinEchidna.sol --config contracts/echidna.yaml
```

3) EVM Conformance
- Rust tests: `blockchain_node/tests/evm_precompile_conformance.rs`
- Run:
```
cargo test --package blockchain_node evm_precompile_conformance -- --nocapture
```

4) Replay/DoS Simulations
- Add load generation against `/api/v1/transactions/submit` and contract calls.
- Use `blockchain_node/tests/` scenarios and `scripts/security_checks.sh` in CI.

5) Storage Proof Correctness
- Validate `DealMarket` payout proofs and SVDB endpoints with SDKs (`arthajs`, `arthapy`).
- Add batch verification tests ensuring inclusion/ordering constraints.

6) Bridges/Oracle
- Bridges: `blockchain_node/src/bridges/ethereum.rs`, `blockchain_node/src/bridges/cross_chain.rs`.
- Oracle: `contracts/PriceOracle.sol` governance control and bounds.

CI: integrate `forge test`, `echidna-test` (nightly), and `cargo test` in pipeline.

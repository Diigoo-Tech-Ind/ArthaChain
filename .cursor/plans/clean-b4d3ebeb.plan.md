<!-- b4d3ebeb-1da9-4aee-8a7f-a14e2801ea75 4f8b0c26-0676-496c-ba49-fff3349114ca -->
# Clean Build, Tests Passing, Zero Warnings

#### Scope

- Focus first on `blockchain_node` crate (lib + tests), then expand to full workspace.
- Prioritize compilation errors, then run unit tests, then eliminate warnings.

#### Steps

1) Snapshot current failures

- Run `cargo test --workspace` to capture all compile errors/warnings (already observed). Categorize into: model mismatches, missing methods, handler signatures, type mismatches, trait bounds, and unused items.

2) Align `types::Transaction` with API usage

- Edit `blockchain_node/src/types.rs` to include EVM-friendly optional fields used by API handlers: `from`, `to`, `nonce`, `gas_price`, `gas_limit`, `data: Option<Vec<u8>>`, `signature: Option<Signature>`, `hash`, and alias/replace `value` vs `amount` consistently.
- Update constructors and serde derives accordingly.

3) Fix API handlers expecting those fields

- `blockchain_node/src/api/handlers/transaction_submission.rs`: handle `Option<Vec<u8>>` safely (no `.is_empty()` on `Option`; use `as_ref().map(...)`).
- `blockchain_node/src/api/handlers/transactions.rs`: resolve type mismatches (e.g., `Address` vs `String`) and remove fields not present or add them per step 2.

4) Implement missing methods used by routes

- Cross-shard manager (`blockchain_node/src/consensus/cross_shard/integration.rs` or its module): add `get_shard_stats(shard_id)`, `get_connected_shards()` returning real or placeholder values wired to existing state.
- Gas-free manager: add `get_whitelisted_companies()` in `blockchain_node/src/contracts/gas_free.rs`.
- P2P network: add `get_peer_info(&PeerId)` in `blockchain_node/src/network/p2p.rs` or adapt caller to existing API.
- Wallet RPC/state access: stop calling `state.get_transaction(...)` directly; expose methods on `State` or route via service layer used elsewhere.

5) Fix Axum handler signatures

- `blockchain_node/src/api/testnet_router.rs` and related handlers: ensure function signatures implement `axum::handler::Handler` for axum 0.7; add `#[axum::debug_handler]` where helpful and correct arguments/return types.

6) Resolve trait/derive issues

- `blockchain_node/src/wasm/host_functions.rs`: remove `#[derive(Debug)]` or change trait object to `dyn Storage + Debug` with a manual `Debug` impl to satisfy `HostContext`.
- `blockchain_node/src/api/grpc/service.rs`: stop calling non-existent `Block::size()`; compute from serialized length or add a method.

7) Fix numeric/type mismatches

- `blockchain_node/src/api/server.rs`: cast/convert `u32` to `u64` for `connected_peers`, `active_validators`.
- `blockchain_node/src/performance/parallel_processor.rs`: accept `&Address` instead of `&String` (or convert appropriately) in `hash_address` calls.

8) Make unreachable/unused code clean

- Remove or refactor unreachable blocks (e.g., in `smart_contract_engine.rs`).
- Remove unused imports and variables, drop unnecessary `mut`, or prefix intentionally-unused with `_`.

9) Iterate builds and tests

- Re-run `cargo test --workspace` until compile succeeds; then execute unit tests and fix any failing assertions.

10) Warnings to zero

- Run `cargo clippy --workspace -- -Dwarnings`; fix remaining lints (unused, needless borrows, etc.).

11) Finalize

- Ensure `cargo test` passes and `cargo clippy` emits zero warnings. Document major interface changes in `ARTHACHAIN_ARCHITECTURE.md` if needed.

### To-dos

- [ ] Align `types::Transaction` with API/EVM needs
- [ ] Fix API handlers to match updated `Transaction` fields and types
- [ ] Implement `get_shard_stats` and `get_connected_shards`
- [ ] Add gas-free `get_whitelisted_companies`, P2P `get_peer_info`, and wallet RPC state accessors
- [ ] Correct axum handler signatures and add debug_handler where needed
- [ ] Fix WASM Debug derive and replace `Block::size()` usage
- [ ] Resolve u32→u64 and Address/String mismatches
- [ ] Remove unreachable code, unused imports/vars, redundant mut
- [ ] Build workspace and run unit tests to green
- [ ] Run clippy with -Dwarnings and fix all lints
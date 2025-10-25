# SVDB Phase 1 Test Plan (Correctness, Performance, Economics, Security)

## Correctness
- RS decode: 10k stripes, drop ≤2 shards → 0 reconstruction errors (automated in rs_erasure_tests.rs)
- Merkle/Poseidon: Upload 100 random files, recompute roots client-side; /svdb/info roots must match
- Salted proofs V2: For N manifests, derive salts from L1 and verify pre-check via verifySalted; ensure payouts only on valid proofs
- Policy: private/allowlist DIDs must be enforced; unauthorized requests rejected

## Performance
- Upload 1 GB and 10 GB (X concurrency): measure p50/p95 E2E time; ensure < targets
- Download 1 GB cross-region from 3 providers: p95 < 60s; verify resume after mid-stream kill

## Economics
- Deals: replicas=3, months=3, verify endowment lock; 30 epochs pass, 2 misses; provider receives 28/30 payouts
- Retrieval settle: quotes vs totalWei settle within ±1% of price

## Security
- Announce spoof: unsigned/stale announces rejected; only signed (ArthaID) accepted
- Voucher replay: reused nonce rejected
- Tampered chunk: proof should fail and payout blocked

## Scripts
- acceptance.sh: Drives upload→deal→download→pin, runs subset perf checks
- perf_upload.sh: Upload size matrix and aggregate timings
- econ_payouts.sh: Simulate epochs and validate payouts vs misses
- security_checks.sh: Negative scenarios (replay, spoof, tamper)

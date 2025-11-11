#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"

echo "[1/4] Running Rust tests (blockchain_node) in Docker..."
docker run --rm -v "$ROOT_DIR":/work -w /work rust:1.90 \
  bash -lc 'apt-get update && apt-get install -y pkg-config libssl-dev && cargo test -p blockchain_node'

echo "[2/4] Running Foundry tests (contracts) in Docker..."
docker run --rm -v "$ROOT_DIR":/work -w /work/contracts ghcr.io/foundry-rs/foundry:latest forge test -vvv
docker run --rm -v "$ROOT_DIR":/work -w /work/contracts ghcr.io/foundry-rs/foundry:latest \
  forge test --ffi --runs 256 --mc ArthaCoinInvariants -vvv

echo "[3/4] Running Echidna property tests (contracts) in Docker..."
docker run --rm -v "$ROOT_DIR":/work -w /work/contracts trailofbits/echidna \
  echidna-test contracts/test/ArthaCoinEchidna.sol --config contracts/echidna.yaml

echo "[4/4] Running economic simulations (Python) locally..."
python3 "$ROOT_DIR/scripts/econ/sim_emissions.py"
python3 "$ROOT_DIR/scripts/econ/sim_burn_curve.py"
python3 "$ROOT_DIR/scripts/econ/sim_storage_supply.py"
python3 "$ROOT_DIR/scripts/econ/sim_deal_market.py"

echo "All tests and simulations completed. CSV outputs under data/econ/."



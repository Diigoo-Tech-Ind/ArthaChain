#!/usr/bin/env bash
set -euo pipefail

# Drives Phase 1 epoch-proof flow for N epochs using short epoch seconds

NODE_URL=${ARTHA_NODE:-http://127.0.0.1:8080}
EPOCHS=${ARTHA_EPOCHS:-30}
WAIT=${ARTHA_EPOCH_SECONDS:-60}

CID=${ARTHA_CID:?set ARTHA_CID to manifest CID (artha://...)}

echo "Running $EPOCHS epochs at $WAIT seconds per epoch for CID=$CID"
for ((i=1;i<=EPOCHS;i++)); do
  echo "Epoch $i/$(($EPOCHS))";
  sleep "$WAIT";
  curl -sS "$NODE_URL/svdb/retrievals/$CID" >/dev/null || true
done

echo "Done. Review on-chain payout events and retrieval stats."



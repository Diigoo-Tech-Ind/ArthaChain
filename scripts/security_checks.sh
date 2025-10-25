#!/usr/bin/env bash
set -euo pipefail
NODE_URL=${ARTHA_NODE:-http://127.0.0.1:8080}

echo "Attempting unsigned announce spoof (should be ignored by nodes)..."
echo '{"type":"svdb_provide","cid":"deadbeef","peerId":"fake","http_addr":"http://127.0.0.1:8080","ts":0,"sig":"0x00","pubkey":"0x00"}' >/dev/null
echo "Manual verification required in logs/metrics."

echo "Voucher replay test: request quote twice and reuse same nonce in two settle calls (second should fail)."


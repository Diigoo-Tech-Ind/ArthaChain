#!/usr/bin/env bash
set -euo pipefail

NODE_URL=${ARTHA_NODE:-http://127.0.0.1:8080}
# Optional additional storage providers (comma-separated)
PROVIDER_URLS=${ARTHA_PROVIDERS:-}
# DealMarket config for payouts (optional)
DEAL_MARKET=${ARTHA_DEALMARKET:-}
RPC_URL=${ARTHA_RPC_URL:-}
CHAIN_ID=${ARTHA_CHAIN_ID:-}
PRIVATE_KEY=${ARTHA_PRIVATE_KEY:-}
TMP_DIR=$(mktemp -d)
DATA_FILE="$TMP_DIR/data.bin"
OUT_FILE="$TMP_DIR/out.bin"

# Default: 256MB; override via ARTHA_SIZE_MB
SIZE_MB=${ARTHA_SIZE_MB:-256}
dd if=/dev/urandom of="$DATA_FILE" bs=1M count="$SIZE_MB" status=none

CID=$(ARTHA_NODE="$NODE_URL" node "$(dirname "$0")/../sdk/arthajs/index.ts" upload "$DATA_FILE" || true)
if [[ -z "$CID" ]]; then
  # fallback via curl
  BOUNDARY=----arthajs-$RANDOM
  BODY=$(
    {
      printf -- "--$BOUNDARY\r\n";
      printf -- "Content-Disposition: form-data; name=\"file\"; filename=\"data.bin\"\r\n";
      printf -- "Content-Type: application/octet-stream\r\n\r\n";
      cat "$DATA_FILE";
      printf -- "\r\n--$BOUNDARY--\r\n";
    } | curl -sS -X POST "$NODE_URL/svdb/upload" -H "Content-Type: multipart/form-data; boundary=$BOUNDARY" --data-binary @-)
  CID=$(echo "$BODY" | jq -r .cid)
fi

echo "CID=$CID"

ARTHA_NODE="$NODE_URL" node "$(dirname "$0")/../sdk/arthajs/index.ts" info "$CID" >/dev/null || true

# Optionally create a deal (requires env vars)
if [[ -n "$DEAL_MARKET" && -n "$RPC_URL" && -n "$CHAIN_ID" && -n "$PRIVATE_KEY" ]]; then
  SIZE_BYTES=$(stat -f%z "$DATA_FILE" 2>/dev/null || stat -c%s "$DATA_FILE")
  PAYLOAD=$(cat <<JSON
{
  "cid": "$CID",
  "size": $SIZE_BYTES,
  "replicas": ${ARTHA_REPLICAS:-5},
  "months": ${ARTHA_MONTHS:-1},
  "maxPrice": ${ARTHA_MAX_PRICE:-1000},
  "rpcUrl": "$RPC_URL",
  "chainId": ${CHAIN_ID},
  "privateKey": "$PRIVATE_KEY",
  "dealMarket": "$DEAL_MARKET"
}
JSON
)
  echo "$PAYLOAD" | curl -sS -X POST "$NODE_URL/svdb/deals" -H 'Content-Type: application/json' --data-binary @- >/dev/null
fi

# Optionally replicate by fetching from additional providers (pre-warm caches)
if [[ -n "$PROVIDER_URLS" ]]; then
  IFS=',' read -r -a providers <<< "$PROVIDER_URLS"
  CID_B64=${CID#artha://}
  for p in "${providers[@]}"; do
    curl -sS "$p/svdb/download/$CID_B64" -o /dev/null || true
  done
fi

ARTHA_NODE="$NODE_URL" node "$(dirname "$0")/../sdk/arthajs/index.ts" download "$CID" "$OUT_FILE" || curl -sS "$NODE_URL/svdb/download/${CID#artha://}" -o "$OUT_FILE"

cmp -s "$DATA_FILE" "$OUT_FILE"
echo "OK: upload/download integrity verified"

# Optional: wait epochs and check retrieval stats
if [[ -n "$RPC_URL" ]]; then
  sleep ${ARTHA_WAIT_SECS:-65}
  curl -sS "$NODE_URL/svdb/retrievals/$CID" | jq . || true
fi



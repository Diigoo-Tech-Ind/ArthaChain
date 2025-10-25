#!/usr/bin/env bash
set -euo pipefail
NODE_URL=${ARTHA_NODE:-http://127.0.0.1:8080}
RPC_URL=${ARTHA_RPC_URL:?set}
DEAL_MARKET=${ARTHA_DEALMARKET:?set}
CHAIN_ID=${ARTHA_CHAIN_ID:?set}
PRIV=${ARTHA_PRIVATE_KEY:?set}

# Upload tiny file
TMP=$(mktemp -d); FILE="$TMP/data.bin"; dd if=/dev/urandom of="$FILE" bs=1M count=4 status=none
CID=$(ARTHA_NODE="$NODE_URL" node "$(dirname "$0")/../sdk/arthajs/index.ts" upload "$FILE")
SIZE=$(stat -f%z "$FILE" 2>/dev/null || stat -c%s "$FILE")

# Create deal
curl -sS -X POST "$NODE_URL/svdb/deals" -H 'Content-Type: application/json' --data-binary @- <<JSON >/dev/null
{"cid":"$CID","size":$SIZE,"replicas":1,"months":1,"maxPrice":1000,"rpcUrl":"$RPC_URL","chainId":$CHAIN_ID,"privateKey":"$PRIV","dealMarket":"$DEAL_MARKET"}
JSON

echo "Waiting epochs to observe payouts..."
sleep ${ARTHA_WAIT_EPOCHS_SECS:-180}
echo "Done. Check chain explorer for payout events."


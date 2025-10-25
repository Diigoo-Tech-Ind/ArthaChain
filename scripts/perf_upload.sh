#!/usr/bin/env bash
set -euo pipefail
NODE_URL=${ARTHA_NODE:-http://127.0.0.1:8080}
SIZES_MB=(1024 10240)
for sz in "${SIZES_MB[@]}"; do
  TMP=$(mktemp -d)
  FILE="$TMP/data.bin"
  dd if=/dev/urandom of="$FILE" bs=1M count="$sz" status=none
  t0=$(date +%s)
  CID=$(ARTHA_NODE="$NODE_URL" node "$(dirname "$0")/../sdk/arthajs/index.ts" upload "$FILE")
  t1=$(date +%s)
  echo "upload_mb=${sz} cid=${CID} duration_s=$((t1-t0))"
  OUT="$TMP/out.bin"
  t2=$(date +%s)
  ARTHA_NODE="$NODE_URL" node "$(dirname "$0")/../sdk/arthajs/index.ts" download "$CID" "$OUT"
  t3=$(date +%s)
  echo "download_mb=${sz} duration_s=$((t3-t2))"
  rm -rf "$TMP"
done


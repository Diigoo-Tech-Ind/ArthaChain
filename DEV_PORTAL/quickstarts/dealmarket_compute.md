## Quickstart: Compute task via DealMarket

1) Create deal (API):
```
curl -X POST "$API/svdb/deals" \
  -H 'Content-Type: application/json' \
  -d '{"cid":"artha://bafy...","size":1073741824,"replicas":1,"months":1,"maxPrice":1000000000000000}'
```

2) Settle retrievals (arthajs):
```
await sdk.settle({ rpcUrl, chainId, privateKey, dealMarket, manifestRoot, bytesServed, provider, totalWei })
```

## Quickstart: Dataset pin + SVDB proof

1) Upload file (arthajs):
```
const sdk = new ArthaJS(process.env.ARTHA_NODE)
const cid = await sdk.uploadFile('./data.bin')
```

2) Build Merkle branch and submit payout (arthajs):
```
const { root, leaf, branch, index } = await sdk.buildMerkleBranch(cid, 0)
await sdk.submitPayout({ rpcUrl, chainId, privateKey, nonce, gasPrice, gasLimit, dealMarket, root, leaf, index, branch })
```



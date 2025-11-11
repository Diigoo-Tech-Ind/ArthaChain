# Reference App (TypeScript)

- Uses `sdk/arthajs` to:
  - Upload dataset and pin
  - Build proofs and submit payouts
  - Create DealMarket deals and settle retrievals
  - Query SVDB info

## Setup
```
node -v
npm i
export ARTHA_NODE=http://127.0.0.1:8080
```

## Run
```
node index.ts upload ./data.bin
```

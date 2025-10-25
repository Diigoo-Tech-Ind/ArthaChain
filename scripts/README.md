# SVDB Scripts

Run with appropriate env vars:

```
ARTHA_NODE=http://127.0.0.1:8080 ./scripts/perf_upload.sh
ARTHA_NODE=http://127.0.0.1:8080 ARTHA_RPC_URL=... ARTHA_DEALMARKET=0x... ARTHA_CHAIN_ID=... ARTHA_PRIVATE_KEY=0x... ./scripts/econ_payouts.sh
./scripts/security_checks.sh

All API errors return a consistent JSON envelope:

```
{
  "error": { "code": 400, "message": "Bad request", "details": { /* optional */ } }
}
```
```



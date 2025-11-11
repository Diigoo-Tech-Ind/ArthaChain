# Marketplace UX (Model/Dataset)

## Search
- Filter by owner, version, datasetCidRoot, modelCidRoot, codeHash, updatedAt.

## Pricing & Revenue Splits
- Model owners specify splits (e.g., 80% owner, 20% platform). Store off-chain metadata keyed by modelCidRoot, on-chain payout via DealMarket retrievals.

## Withdrawal Proofs
- Use Merkle proofs for aggregated retrieval vouchers; settle via `DealMarket` functions.

## Pages
- Model detail: owner, dataset link, lineage, version history.
- Dataset detail: size, replicas, months, deals.
- Revenue dashboard: payouts, pending settlements, proof builder.

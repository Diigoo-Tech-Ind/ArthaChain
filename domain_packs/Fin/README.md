# ArthaAIN.Fin - Finance Domain Pack

## Overview
Financial services AI templates with regulatory compliance.

## Components

### Baseline Models
- Fraud detection (transaction monitoring)
- Credit scoring
- Risk assessment
- Algorithmic trading strategies

### Dataset Schemas
- Transaction schema (PCI-DSS compliant)
- Credit report schema
- Market data schema (OHLCV)

### Compliance Flags
- PCI-DSS Level 1
- KYC/AML requirements
- MiFID II (EU)
- SEC regulations (US)

### Sample Datasets
- `artha://QmFinDemo1` - Transaction history (synthetic)
- `artha://QmFinDemo2` - Credit reports (synthetic)
- `artha://QmFinDemo3` - Market data (public)

## Usage

```bash
# Register financial dataset (requires KYC VC)
arthai dataset register artha://QmFinDemo1 --tags finance,fraud --compliance kyc,aml

# Train fraud detection model
arthai train --model model-fin-fraud --data dataset-fin-transactions \
    --epochs 20 --compliance kyc,pci-dss

# Deploy with token gating
arthai deploy --model model-fin-fraud --endpoint /detect \
    --access token --token fin-api-token-2024
```

## Compliance Requirements
- VC required: `vc:kyc-level-1`, `vc:aml-certification`
- Data residency: EU/US regulations
- Model explainability: SHAP/LIME required
- Audit trail: 7-year retention


# ArthaAIN.Health - Healthcare Domain Pack

## Overview
Templates, baseline models, and compliance configurations for healthcare AI applications.

## Components

### Baseline Models
- Medical image classification (chest X-ray, retina, skin lesions)
- Clinical note NLP (ICD-10 coding, extraction)
- Drug interaction prediction
- Patient risk scoring

### Dataset Schemas
- DICOM image schema
- HL7 FHIR patient data schema
- Clinical note schema
- Lab result schema

### Compliance Flags
- HIPAA compliance required
- FDA 510(k) premarket notification (if applicable)
- GDPR data protection
- SOC 2 Type 2 audit trail

### Sample Datasets (SVDB CIDs)
- `artha://QmHealthDemo1` - Chest X-ray dataset (synthetic)
- `artha://QmHealthDemo2` - Clinical notes (de-identified)
- `artha://QmHealthDemo3` - Lab results (synthetic)

## Usage

```bash
# Register health dataset
arthai dataset register artha://QmHealthDemo1 --tags health,radiology --compliance hipaa

# Train medical image model
arthai train --model model-health-xray --data dataset-health-chest --epochs 10 \
    --compliance hipaa,gdpr

# Deploy with access controls
arthai deploy --model model-health-xray --endpoint /diagnose \
    --access allowlist --dids did:artha:doctor1,did:artha:doctor2
```

## Compliance Requirements
- All models must have `health` tag
- VCs required: `vc:medical-license`, `vc:hipaa-training`
- Data encryption: XChaCha20 mandatory
- Audit logging: All inference requests logged


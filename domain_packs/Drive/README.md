# ArthaAIN.Drive - Autonomous Driving Domain Pack

## Overview
Self-driving car AI templates with safety validation.

## Components

### Baseline Models
- Object detection (pedestrians, vehicles, signs)
- Path planning
- Behavior prediction
- Sensor fusion

### Compliance Flags
- ISO 26262 (functional safety)
- NHTSA guidelines (US)
- EU type approval

## Usage

```bash
arthai dataset register artha://QmDriveDemo1 --tags autonomous,driving --compliance iso26262
arthai train --model model-drive-objects --data dataset-drive-images --compliance iso26262
```


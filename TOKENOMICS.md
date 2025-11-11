# ArthaChain Tokenomics

## Overview

ArthaChain employs a dual-tokenomics model that combines deflationary mechanics with adaptive reward distribution. The system burns a portion of gas fees while keeping transfers intact, and distributes rewards to various network participants based on their contributions. This model ensures user-friendly transactions while maintaining deflationary pressure through progressive gas fee burns.

**ARTHA is the native utility and governance token of the ArthaChain ecosystem.**

## Token Utility

The ARTHA token serves multiple functions within the ArthaChain ecosystem:

### Transaction Fees
ARTHA is used to pay for all on-chain transactions, including transfers, smart contract executions, and cross-chain operations. Gas fees are paid in ARTHA and subject to progressive burn rates.

### Governance
Token holders can participate in on-chain governance through the DAO system. Voting power is proportional to token holdings, enabling decentralized decision-making for protocol upgrades, parameter adjustments, and treasury allocations.

### Storage Payments
Users pay for SVDB (Sovereign Verifiable Database) storage services using ARTHA. Storage providers receive payments in ARTHA for hosting and maintaining data with verifiable proofs.

### Compute Credits
ARTHA serves as payment for GPU compute resources. AI training jobs, inference tasks, and other compute-intensive operations are paid for using ARTHA tokens.

### Staking and Rewards
While ArthaChain does not require staking for validators, tokens can be allocated to various reward pools. Participants earn ARTHA rewards based on their network contributions, including block creation, validation, storage provision, and compute services.

### Network Security
Gas fees paid in ARTHA help secure the network by incentivizing validators and preventing spam attacks. The burn mechanism ensures long-term value appreciation while maintaining network security.

## Token Metrics and Supply

### Initial Supply
- **Genesis Emission**: 50,000,000 ARTHA
- **Initial Circulation**: Distributed at network launch according to the emission split

### Emission Schedule

ArthaChain uses a 3-year cycle emission model:

- **Base Emission**: 50,000,000 ARTHA per cycle
- **Growth Rate**: 5% increase per cycle
- **Maximum Emission Cap**: 129,093,000 ARTHA per cycle (reached after cycle 10, approximately year 30)
- **Cycle Duration**: 3 years

**Emission Formula:**
```
Cycle Emission = Base Emission × (1.05)^cycle
Capped at: 129,093,000 ARTHA per cycle
```

### Maximum Supply

ArthaChain does not have a hard-capped maximum supply. Instead, the supply is controlled through:

1. **Emission Caps**: Maximum 129,093,000 ARTHA per 3-year cycle
2. **Progressive Burns**: 40% to 96% of gas fees burned over time
3. **Adaptive Control**: Governance can adjust burn rates and emission schedules

**Governance Adjustment Authority:**
Adjustments to burn rates or emission schedules require DAO approval with a two-thirds (2/3) majority quorum. Proposals must be submitted through the on-chain governance system, undergo a minimum voting period, and receive support from token holders representing at least 2/3 of participating votes. Emergency adjustments may be proposed by the governance council but still require community ratification.

**Net Supply Formula:**
```
Net Supply Change = Emissions - Burns
(Units: ARTHA tokens per 3-year cycle)

Net Inflation Rate = (Emissions - Burns) / Current Supply × 100%
(Units: Percentage per cycle)
```

### Max Supply Projection

The following projection illustrates how emissions, burns, and net supply interact over time:

```
Supply Dynamics Over Time (Projected)

ARTHA (Millions)
│
200 │                                    ╱─────────────── (Emission Cap)
    │                                  ╱
150 │                                ╱
    │                              ╱
100 │                            ╱
    │                          ╱
 50 │                        ╱
    │                      ╱
  0 │────────────────────╱───────────────────────────────
    │                    │
    └────────────────────┴─────────────────────────────────→ Years
    0    5    10   15   20   25   30

Emission Curve:     ╱───────╲─────────────── (grows then capped)
Burn Rate Curve:    ╱───────────────╲─────── (increasing 40%→96%)
Net Supply:         ╱───────────╲─────────── (flattening/negative)
```

**Key Observations:**
- **Years 1-10**: Emissions grow from 50M to 129M per cycle, burn rate increases from 40% to 68%
- **Years 10-17**: Emissions capped at 129M, burn rate continues increasing to 96%
- **Years 17+**: With high network activity, burns can exceed emissions, leading to net deflation
- **Net Supply**: Flattens over time and potentially becomes negative (deflationary) in later years with sufficient transaction volume

### Emission vs Burn vs Supply Curve

A detailed view of how emissions, burns, and net supply interact:

```
Emission vs Burn vs Supply Dynamics

ARTHA (Millions per Cycle)
│
150 │
    │                    ╱─────────────── Emission Cap (129M)
    │                  ╱
100 │                ╱
    │              ╱
 50 │            ╱
    │          ╱
  0 │────────╱───────────────────────────────────────────────
    │        │
    └────────┴────────────────────────────────────────────────→ Years
    0        5        10       15       20       25       30

Emission Curve:     ╱───────╲───────────────────────────────
                    │       │
                    │       └─ Capped at 129M
                    │
                    └─ Grows 5% per cycle

Burn Rate (%):      ╱───────────────────────────────╲────────
                    │                               │
                    │                               └─ 96%
                    │
                    └─ 40% (Year 1)

Net Supply Change:  ╱───────────╲───────────────────────────
                    │           │
                    │           └─ Potentially negative
                    │               (deflation)
                    │
                    └─ Positive (inflation)

Legend:
  ─── Emission (growing then capped)
  ─── Burn Rate (increasing 40%→96%)
  ─── Net Supply (flattening/negative)
```

### Token Flow Cycle

The complete lifecycle of ARTHA tokens from creation to distribution:

```
┌─────────────────────────────────────────────────────────────────┐
│                    TOKEN FLOW CYCLE                             │
└─────────────────────────────────────────────────────────────────┘

                    ┌─────────────────┐
                    │   EMISSIONS     │
                    │  (3-Year Cycle) │
                    └────────┬────────┘
                             │
                             ▼
        ┌────────────────────────────────────┐
        │     DISTRIBUTION POOLS             │
        │  (Validators, Rewards, Ecosystem)  │
        └───────────────┬────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
   ┌─────────┐    ┌─────────┐    ┌─────────┐
   │Network  │    │Rewards  │    │Ecosystem│
   │Participants│ │Pool    │    │Pools    │
   └────┬────┘    └─────────┘    └─────────┘
        │
        ▼
┌───────────────────────────────────────────┐
│         NETWORK ACTIVITY                   │
│  (Transactions, Storage, Compute)         │
└───────────────┬────────────────────────────┘
                │
                ▼
        ┌───────────────┐
        │   GAS FEES    │
        └───────┬───────┘
                │
        ┌───────┼───────┐
        │       │       │
        ▼       ▼       ▼
   ┌────────┐ ┌────┐ ┌──────────┐
   │ BURNED │ │Treas│ │ Consensus│
   │        │ │ury  │ │  Pools   │
   └────────┘ └────┘ └─────┬────┘
                            │
                            ▼
                    ┌──────────────┐
                    │  REWARDS     │
                    │  DISTRIBUTED │
                    └──────┬───────┘
                           │
                           ▼
                    ┌──────────────┐
                    │  PARTICIPANTS│
                    │  (Earn ARTHA)│
                    └──────┬───────┘
                           │
                           └───► [Cycle Repeats]
```

**Cycle Phases:**
1. **Emission**: New tokens minted every 3 years
2. **Distribution**: Tokens allocated to pools
3. **Network Activity**: Tokens used for transactions, storage, compute
4. **Gas Fees**: Fees collected from transactions
5. **Burn & Distribution**: Portion burned, remainder distributed
6. **Rewards**: Participants earn tokens for contributions
7. **Repeat**: Cycle continues with new emissions

### Supply Projections

**Year 1-3 (Cycle 1):**
- Emissions: 50,000,000 ARTHA
- Estimated Burns: Variable based on network activity
- Net Supply: Depends on burn rate and transaction volume

**Year 4-6 (Cycle 2):**
- Emissions: 52,500,000 ARTHA (5% increase)
- Burn Rate: 47% (Years 3-4)
- Net Supply: Lower growth due to increased burn rate

**Year 10+ (Cycle 4+):**
- Emissions: Capped at 129,093,000 ARTHA per cycle
- Burn Rate: 68%+ (Years 9-10)
- Net Supply: Potentially deflationary if burns exceed emissions

## Core Principles

1. **No Burn on Transfers**: Users receive the full transfer amount - no tokens are burned from transfer amounts
2. **Burn on Gas Fees Only**: Deflationary pressure comes from burning a portion of gas fees (40% to 96% progressive over time)
3. **Adaptive Distribution**: Reward distribution automatically adapts based on network features (sharding and DAG processing)
4. **Multi-Role Rewards**: Different network participants (validators, storage providers, GPU providers, consensus participants) receive rewards based on their contributions

## Token Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    EMISSION (3-Year Cycles)                     │
│             50M ARTHA base, +5% per cycle, capped              │
└───────────────────────────┬───────────────────────────────────┘
                             │
                             ▼
        ┌────────────────────────────────────────┐
        │      DISTRIBUTION POOLS (100%)         │
        └────────────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
   ┌─────────┐         ┌──────────┐        ┌──────────┐
   │Validators│        │ Rewards  │        │ Ecosystem│
   │  Pool   │        │  Pool    │        │  Pools   │
   │  (45%)  │        │  (20%)   │        │  (35%)   │
   └────┬────┘        └──────────┘        └──────────┘
        │
        ├──► Validators (20% of 45% = 9%)
        ├──► Storage Providers (40% of 45% = 18%)
        └──► GPU Providers (40% of 45% = 18%)

┌─────────────────────────────────────────────────────────────────┐
│                    GAS FEES (Per Transaction)                   │
└───────────────────────────┬───────────────────────────────────┘
                             │
                             ▼
        ┌────────────────────────────────────────┐
        │         PROGRESSIVE BURN                │
        │   40% (Year 1) → 96% (Year 17+)        │
        └────────────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
   ┌─────────┐         ┌──────────┐        ┌──────────┐
   │  BURNED │         │ Treasury │        │ Consensus│
   │         │         │  (12%)   │        │  (88%)   │
   └─────────┘         └──────────┘        └─────┬────┘
                                                  │
        ┌─────────────────────────────────────────┼──────────────┐
        │                                         │              │
        ▼                                         ▼              ▼
   ┌─────────┐                              ┌──────────┐   ┌──────────┐
   │  SVCP  │                              │  SVBFT  │   │ Sharding │
   │  (55%) │                              │  (45%)  │   │   (18%)  │
   │  or    │                              │  or     │   │   or     │
   │  (30%) │                              │  (25%)  │   │   (15%)  │
   └─────────┘                              └──────────┘   └──────────┘
        │                                         │              │
        └─────────────────────────────────────────┴──────────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  DAG Pool (15%) │
                    │  (if active)    │
                    └─────────────────┘
```

**Legend:**
- Solid lines: Always active
- Dashed lines: Conditional (based on network state)
- Percentages: Of remaining amount after burn

## Token Distribution

### Initial Emission Split

When new tokens are minted (emissions), they are distributed as follows:

- **Validators Pool**: 45% (further split: 20% validators, 40% storage, 40% GPU)
- **Staking Rewards Pool**: 20%
- **Ecosystem Grants**: 10%
- **Marketing**: 10%
- **Developers**: 5%
- **DAO Governance**: 5%
- **Treasury Reserve**: 5%

### Validator Pool Sub-Distribution

The 45% allocated to the Validators Pool is further split:

- **Validators (Node Operators)**: 20% of the 45% = 9% of total emissions
- **Storage Providers (SVDB)**: 40% of the 45% = 18% of total emissions
- **GPU Providers (Compute)**: 40% of the 45% = 18% of total emissions

## Gas Fee Distribution

### Burn Mechanism

Gas fees are subject to a progressive burn rate:
- **Years 1-2**: 40% burned
- **Years 3-4**: 47% burned
- **Years 5-6**: 54% burned
- **Years 7-8**: 61% burned
- **Years 9-10**: 68% burned
- **Years 11-12**: 75% burned
- **Years 13-14**: 82% burned
- **Years 15-16**: 89% burned
- **Year 17+**: 96% burned

### Remaining Gas Fee Distribution

After the burn, the remaining gas fees are distributed as follows:

1. **Treasury**: 12% of remaining fees
2. **Consensus & Processing**: 88% of remaining fees (distributed adaptively)

### Adaptive Distribution Models

The system uses two distribution models depending on network state:

#### Model 1: Simple (Sharding/DAG Inactive)

When the network operates in a simple mode without sharding or parallel processing:

- **SVCP (Block Creation)**: 55% of remaining 88%
- **SVBFT (Block Confirmation)**: 45% of remaining 88%

#### Model 2: Comprehensive (Sharding/DAG Active)

When sharding or DAG processing is active:

- **SVCP (Block Creation)**: 30% of remaining 88%
- **SVBFT (Block Confirmation)**: 25% of remaining 88%
- **Sharding (Cross-Shard Coordination)**: 18% of remaining 88%
- **DAG (Parallel Processing)**: 15% of remaining 88%

## Example Scenarios

### Example 1: Simple Transfer (Year 1)

**Transaction Details:**
- Person A sends: 100 ARTHA to Person B
- Gas fee: 0.01 ARTHA
- Burn rate: 40% (Year 1)

**What Happens:**

1. **Transfer Amount**: Person B receives 100 ARTHA (full amount, no burn)

2. **Gas Fee Processing**:
   - Total gas fee: 0.01 ARTHA
   - Burned: 0.004 ARTHA (40%)
   - Remaining: 0.006 ARTHA (60%)

3. **Remaining Gas Fee Distribution** (assuming Simple Model):
   - Treasury: 0.00072 ARTHA (12% of 0.006)
   - SVCP Pool: 0.0033 ARTHA (55% of 0.00528)
   - SVBFT Pool: 0.002376 ARTHA (45% of 0.00528)

**Summary:**
- Person A pays: 100.01 ARTHA total
- Person B receives: 100 ARTHA
- Burned: 0.004 ARTHA
- Distributed to network: 0.006 ARTHA

### Example 2: Comprehensive Model (Year 1, Sharding Active)

**Transaction Details:**
- Person A sends: 100 ARTHA to Person B
- Gas fee: 0.01 ARTHA
- Burn rate: 40% (Year 1)
- Network state: Sharding active (4 shards)

**What Happens:**

1. **Transfer Amount**: Person B receives 100 ARTHA (full amount, no burn)

2. **Gas Fee Processing**:
   - Total gas fee: 0.01 ARTHA
   - Burned: 0.004 ARTHA (40%)
   - Remaining: 0.006 ARTHA (60%)

3. **Remaining Gas Fee Distribution** (Comprehensive Model):
   - Treasury: 0.00072 ARTHA (12% of 0.006)
   - SVCP Pool: 0.001584 ARTHA (30% of 0.00528)
   - SVBFT Pool: 0.00132 ARTHA (25% of 0.00528)
   - Sharding Pool: 0.0009504 ARTHA (18% of 0.00528)
   - DAG Pool: 0.000792 ARTHA (15% of 0.00528)

**Summary:**
- Person A pays: 100.01 ARTHA total
- Person B receives: 100 ARTHA
- Burned: 0.004 ARTHA
- Distributed to network: 0.006 ARTHA (split across 4 pools)

### Example 3: Emission Distribution

**Scenario:**
- Total emission: 100 ARTHA
- Cycle: Year 1

**Distribution:**

1. **Validators Pool (45 ARTHA)**:
   - Validators: 9 ARTHA (20% of 45)
   - Storage Providers: 18 ARTHA (40% of 45)
   - GPU Providers: 18 ARTHA (40% of 45)

2. **Other Pools**:
   - Staking Rewards: 20 ARTHA
   - Ecosystem Grants: 10 ARTHA
   - Marketing: 10 ARTHA
   - Developers: 5 ARTHA
   - DAO Governance: 5 ARTHA
   - Treasury Reserve: 5 ARTHA

**Total**: 100 ARTHA

### Example 4: High Burn Rate (Year 17+)

**Transaction Details:**
- Person A sends: 100 ARTHA to Person B
- Gas fee: 0.01 ARTHA
- Burn rate: 96% (Year 17+)

**What Happens:**

1. **Transfer Amount**: Person B receives 100 ARTHA (full amount, no burn)

2. **Gas Fee Processing**:
   - Total gas fee: 0.01 ARTHA
   - Burned: 0.0096 ARTHA (96%)
   - Remaining: 0.0004 ARTHA (4%)

3. **Remaining Gas Fee Distribution** (Simple Model):
   - Treasury: 0.000048 ARTHA (12% of 0.0004)
   - SVCP Pool: 0.00022 ARTHA (55% of 0.000352)
   - SVBFT Pool: 0.0001584 ARTHA (45% of 0.000352)

**Summary:**
- Person A pays: 100.01 ARTHA total
- Person B receives: 100 ARTHA
- Burned: 0.0096 ARTHA (96% of gas fee)
- Distributed to network: 0.0004 ARTHA (4% of gas fee)

## Key Features

### 1. User-Friendly Transfers

Unlike some deflationary models, ArthaChain does not burn tokens on transfers. Users always receive the full transfer amount, making the system more practical for everyday use.

### 2. Progressive Deflation

The burn rate increases over time, creating stronger deflationary pressure as the network matures. This helps control token supply while maintaining network security through gas fees.

### 3. Adaptive Rewards

The system automatically detects network features (sharding, DAG processing) and adjusts reward distribution accordingly. This ensures all active contributors are properly incentivized.

### 4. Multi-Role Participation

The tokenomics model recognizes and rewards multiple types of network participants:
- Block creators (SVCP)
- Block validators (SVBFT)
- Cross-shard coordinators (Sharding)
- Parallel processors (DAG)
- Storage providers (SVDB)
- Compute providers (GPU)
- Node operators (Validators)

### 5. Treasury Funding

A portion of gas fees (12%) goes to the treasury, providing funding for network development, maintenance, and governance activities.

## Economic Model

### Deflationary Pressure

The burn mechanism on gas fees creates deflationary pressure by reducing the total token supply over time. This can potentially increase token value if demand remains constant or grows.

**Deflationary Formula:**
```
Deflation Rate = (Burned Tokens / Total Supply) × 100%
(Units: Percentage per period)

Effective Deflation = Burn Rate × Gas Fee Volume
(Units: ARTHA tokens per period)
```

### Inflation Control

While emissions create new tokens, the progressive burn rate helps control overall inflation. As the burn rate increases over time, the net inflation rate decreases.

**Net Inflation Formula:**
```
Net Inflation = Emission Rate - Burn Rate
(Units: ARTHA tokens per 3-year cycle)

Net Inflation Rate = (Emissions - Burns) / Current Supply × 100%
(Units: Percentage per cycle)
```

**Example Calculation (Year 1):**
- Emission Rate: 50M ARTHA per 3-year cycle
- Burn Rate: 40% of gas fees
- If gas fees = 10M ARTHA: Burns = 4M ARTHA
- Net Supply Change: 50M - 4M = 46M ARTHA (net inflation)

**Example Calculation (Year 10):**
- Emission Rate: 129M ARTHA per 3-year cycle (capped)
- Burn Rate: 68% of gas fees
- If gas fees = 50M ARTHA: Burns = 34M ARTHA
- Net Supply Change: 129M - 34M = 95M ARTHA (net inflation)

**Example Calculation (Year 17+, High Activity):**
- Emission Rate: 129M ARTHA per 3-year cycle (capped)
- Burn Rate: 96% of gas fees
- If gas fees = 150M ARTHA: Burns = 144M ARTHA
- Net Supply Change: 129M - 144M = -15M ARTHA (net deflation)

### Revenue Diversification

The model is designed to shift from gas fee revenue to service-based revenue (storage, compute) over time, reducing reliance on high burn rates for deflationary benefits.

**Revenue Formula:**
```
Total Revenue = Gas Fee Revenue + Storage Revenue + Compute Revenue
(Units: ARTHA tokens per period)

Gas Fee Revenue = Transaction Volume × Gas Price × (1 - Burn Rate)
(Units: ARTHA tokens per period)

Storage Revenue = Storage Deals × GB-Month Rate
(Units: ARTHA tokens per period)

Compute Revenue = GPU Seconds × Compute Rate
(Units: ARTHA tokens per period)
```

## Comparison with Other Models

### Similar to Ethereum (EIP-1559)

ArthaChain's gas fee burn mechanism is similar to Ethereum's EIP-1559, which burns base fees. However, ArthaChain uses a progressive burn rate (40% to 96%) compared to Ethereum's fixed percentage.

### Similar to BNB (BEP-95)

Like Binance Coin's BEP-95 upgrade, ArthaChain burns gas fees in real-time. The key difference is ArthaChain's adaptive distribution model that rewards multiple network roles.

### Unique Features

1. **No Transfer Burn**: Unlike some deflationary tokens, ArthaChain does not burn on transfers
2. **Adaptive Distribution**: Automatically adjusts based on network features
3. **Multi-Role Rewards**: Recognizes and rewards all network participants
4. **Progressive Burn**: Burn rate increases over time (40% to 96%)

## Key Economic Formulas

### Supply Dynamics

**Total Supply Calculation:**
```
Total Supply(t) = Initial Supply + Σ(Emissions) - Σ(Burns)
(Units: ARTHA tokens at time t)
```

**Circulating Supply:**
```
Circulating Supply = Total Supply - Locked Tokens - Treasury Reserves
(Units: ARTHA tokens)
```

### Burn Rate Calculation

**Progressive Burn Rate:**
```
Burn Rate(t) = f(years_since_launch)
Where:
  Years 1-2:  40% (4000 basis points)
  Years 3-4:  47% (4700 basis points)
  Years 5-6:  54% (5400 basis points)
  Years 7-8:  61% (6100 basis points)
  Years 9-10: 68% (6800 basis points)
  Years 11-12: 75% (7500 basis points)
  Years 13-14: 82% (8200 basis points)
  Years 15-16: 89% (8900 basis points)
  Year 17+:   96% (9600 basis points)
```

**Burn Amount:**
```
Burned Amount = Gas Fee × (Burn Rate / 100)
(Units: ARTHA tokens per transaction)
```

### Reward Distribution

**Gas Fee Distribution (Simple Model):**
```
Treasury = Remaining Fee × 0.12
SVCP Pool = Remaining Fee × 0.88 × 0.55
SVBFT Pool = Remaining Fee × 0.88 × 0.45
```

**Gas Fee Distribution (Comprehensive Model):**
```
Treasury = Remaining Fee × 0.12
SVCP Pool = Remaining Fee × 0.88 × 0.30
SVBFT Pool = Remaining Fee × 0.88 × 0.25
Sharding Pool = Remaining Fee × 0.88 × 0.18
DAG Pool = Remaining Fee × 0.88 × 0.15
```

**Emission Distribution:**
```
Validator Pool = Total Emission × 0.45
  ├─ Validators = Validator Pool × 0.20
  ├─ Storage = Validator Pool × 0.40
  └─ GPU = Validator Pool × 0.40
```

### Network Economics

**Token Velocity:**
```
Velocity = (Transaction Volume × Average Transaction Value) / Circulating Supply
(Units: Dimensionless ratio, transactions per token)
```

**Network Value:**
```
Network Value = Circulating Supply × Token Price
(Units: USD or equivalent)

Network Value = (Storage Revenue + Compute Revenue + Gas Revenue) / Discount Rate
(Units: USD or equivalent, where Discount Rate is in percentage)
```

## Economic Transition Plan

ArthaChain's tokenomics model is designed to transition from emission-driven rewards to service-driven revenue over time. This transition ensures long-term sustainability and reduces reliance on token emissions.

**Phase 1: Early Network (Years 1-5)**
The network relies primarily on emission-based rewards to incentivize participation. Validators, storage providers, and GPU providers receive rewards from the 45% validator pool allocation. Gas fees provide additional income, with 40-54% burned to create deflationary pressure.

**Phase 2: Growth Phase (Years 5-10)**
As the network matures, service-based revenue begins to supplement emissions. Storage providers earn from SVDB deals (GB-month rates), GPU providers earn from compute jobs (GPU-seconds), and validators receive increasing portions of gas fees. The burn rate increases to 54-68%, reducing net inflation.

**Phase 3: Maturity (Years 10+)**
Service-based revenue becomes the primary income source. Storage and compute markets generate substantial revenue, reducing dependence on emissions. The burn rate reaches 68-96%, and with high network activity, the system can achieve net deflation. Emissions continue at the capped rate (129M per cycle) but are offset by increasing burns.

**Phase 4: Sustainable Equilibrium (Years 17+)**
The network operates primarily on service revenue. High burn rates (96%) combined with capped emissions create strong deflationary pressure. Network participants earn primarily from providing services (storage, compute, validation) rather than emissions, creating a self-sustaining economic model.

This transition ensures the network becomes economically sustainable without requiring perpetual token emissions, while maintaining strong incentives for all participants.

## Summary

ArthaChain's tokenomics model balances user-friendliness with deflationary mechanics. By burning a portion of gas fees while keeping transfers intact and using adaptive reward distribution, the system:

- Ensures users receive full transfer amounts
- Creates deflationary pressure through progressive gas fee burns
- Rewards all network participants appropriately based on contributions
- Adapts to network state automatically (simple vs. comprehensive models)
- Provides treasury funding for network development and governance

The model is designed to be sustainable long-term, with revenue shifting from gas fees to service-based income (storage and compute) as the network grows. The progressive burn rate ensures increasing deflationary pressure over time, potentially leading to net deflation in later years when network activity is high.

**Key Differentiators:**
- No burn on transfers (user-friendly)
- Progressive burn rate (40% to 96% over 17+ years)
- Adaptive distribution (responds to network features)
- Multi-role rewards (recognizes all contributors)
- Service-based revenue model (storage and compute)

---

## Export Version Notes

For PDF or website export, apply the following formatting:

### Typography
- **Formulas**: Use monospace font blocks (e.g., `Courier New` or `Consolas`)
- **Section Headings**: Apply blue highlight color (#4A90E2) to match ArthaChain branding
- **Code Blocks**: Maintain monospace formatting with syntax highlighting where applicable

### Diagrams
- **Token Flow Diagram**: Convert ASCII diagrams to vector graphics (SVG recommended)
- **Emission vs Burn vs Supply Curve**: Use line chart visualization with proper axis labels
- **Token Flow Cycle**: Create circular flow diagram showing the complete lifecycle

### Additional Visualizations
- **Token Flow Cycle**: Circular diagram showing emission → distribution → activity → fees → rewards → repeat
- **Emission vs Burn vs Supply Curve**: Multi-line chart with three curves (emission, burn rate, net supply) over time

### Color Scheme
- Primary: #4A90E2 (Blue - for headings and highlights)
- Secondary: Use grayscale for diagrams and charts
- Accent: Highlight important formulas and key metrics


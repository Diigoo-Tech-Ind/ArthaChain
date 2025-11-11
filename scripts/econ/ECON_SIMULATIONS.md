### ArthaChain Economic Simulation Pack

Includes agent-based and Monte Carlo simulations for:
- Emission timeline (CycleManager schedule)
- Burn curve (40% → 96%)
- Validator/staker APR
- GPU/storage supply elasticity
- Deal-market equilibria under load

Runbook:
```
python3 scripts/econ/sim_emissions.py
python3 scripts/econ/sim_burn_curve.py
python3 scripts/econ/sim_validator_apr.py
python3 scripts/econ/sim_storage_supply.py
python3 scripts/econ/sim_deal_market.py
```

Outputs CSV and charts under `data/econ/`.



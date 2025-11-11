import math
import csv
from pathlib import Path

# CycleManager schedule: cycle length 3 years, +5% per cycle until cap 129.093M
INITIAL = 50_000_000
CAP = 129_093_000
GROWTH = 1.05
MAX_CYCLE = 10

def emission_for_cycle(c: int) -> float:
    if c > MAX_CYCLE:
        return CAP
    return min(INITIAL * (GROWTH ** c), CAP)

def main():
    outdir = Path('data/econ')
    outdir.mkdir(parents=True, exist_ok=True)
    with open(outdir / 'emissions.csv', 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['cycle','years_since_deploy','emission_millions'])
        for c in range(0, 51):
            years = c * 3
            w.writerow([c, years, round(emission_for_cycle(c), 3)])

if __name__ == '__main__':
    main()



import csv
from pathlib import Path

# Equilibrium under load: simulate N clients with random sizes, M providers
import random

random.seed(42)

def simulate(rounds: int = 1000, providers: int = 50):
    results = []
    base_price = 50.0
    for r in range(rounds):
        size_gb = random.randint(1, 100)
        months = random.choice([1, 3, 6, 12])
        replicas = random.choice([1, 2, 3])
        price = base_price * (1.0 + 0.1 * (replicas - 1))
        matched = min(providers, replicas)
        spend = size_gb * months * price * matched
        results.append((r, size_gb, months, replicas, price, matched, spend))
    return results

def main():
    out = Path('data/econ')
    out.mkdir(parents=True, exist_ok=True)
    rows = simulate()
    with open(out / 'deal_market.csv', 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['round','size_gb','months','replicas','unit_price','matched','spend'])
        for row in rows:
            w.writerow(row)

if __name__ == '__main__':
    main()



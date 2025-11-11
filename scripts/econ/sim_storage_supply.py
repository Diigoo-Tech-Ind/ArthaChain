import csv
from pathlib import Path

# Simplified elasticity model: providers supply GB-month based on price from oracle

def supply_at_price(price: float) -> float:
    # upward sloping supply curve (arbitrary units)
    return max(0.0, 0.1 * (price - 1.0))

def demand_at_price(price: float) -> float:
    # downward sloping demand curve
    return max(0.0, 10.0 - 0.2 * price)

def main():
    out = Path('data/econ')
    out.mkdir(parents=True, exist_ok=True)
    with open(out / 'storage_supply.csv', 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['price_wei_per_gb_month','supply','demand'])
        for p in range(1, 201):
            price = float(p)
            w.writerow([price, round(supply_at_price(price), 6), round(demand_at_price(price), 6)])

if __name__ == '__main__':
    main()



import csv
from pathlib import Path

# BurnManager schedule in basis points for 2-year periods
SCHEDULE = [4000, 4700, 5400, 6100, 6800, 7500, 8200, 8900, 9600]

def burn_rate_for_year(year: int) -> int:
    period = year // 2
    if period >= len(SCHEDULE):
        return SCHEDULE[-1]
    return SCHEDULE[period]

def main():
    out = Path('data/econ')
    out.mkdir(parents=True, exist_ok=True)
    with open(out / 'burn_curve.csv', 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['year','burn_bps'])
        for y in range(0, 40):
            w.writerow([y, burn_rate_for_year(y)])

if __name__ == '__main__':
    main()



"""Offline joint access, capacity and clearing study for the published care schedules."""
import json
from collections import defaultdict, deque
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]


def share(claims, limit):
    """Proportional integer shares, highest remainder first, stable name tie-break."""
    total = sum(claims.values())
    if total <= limit:
        return claims.copy()
    portions = {key: value * limit // total for key, value in claims.items()}
    order = sorted(claims, key=lambda key: (-(claims[key] * limit % total), key))
    for key in order[:limit - sum(portions.values())]:
        portions[key] += 1
    return portions


def inputs(source):
    fixture = source['runtime_fixture']
    profiles = fixture['input']['population']['profiles']
    norms = {p['id']: p['estimate']['central'] for p in profiles['registry']['parameters']}
    need = [defaultdict(int) for _ in range(60)]
    for rule in profiles['rules']:
        if rule['item'] != 'care_time':
            continue
        cell = profiles['cells'][rule['cell']]
        rate = norms[rule['norm']['parameter']]
        assert rate['denominator'] == 1 and len(cell['person_days']) == 60
        for period, days in enumerate(cell['person_days']):
            need[period][cell['region'], cell['group']] += days * rate['numerator']
    capacity = {(row['period'], row['region']): int(row['delivered'])
                for row in fixture['service_evidence']['coverage'] if row['item'] == 'care_time'}
    for (t, region), available in capacity.items():
        assert sum(n for (r, _), n in need[t].items() if r == region) == available
    gap, = source['outage_counterexample']['gaps']
    assert gap['pool'] == 'north_care_staff' and int(gap['available']) == 0
    return need, capacity, gap['period']


def simulate(need, capacity, incident, income_loss, outage, delay, opening):
    treasury = opening
    pending = deque()
    totals = defaultdict(int)
    exceptions = []
    low = opening
    for t, claims in enumerate(need):
        eligible = {k: n // 2 if income_loss and t == incident and k[1] == 'adult' else n
                    for k, n in claims.items()}
        possible = {}
        for region in sorted({r for r, _ in claims}):
            available = 0 if outage and t == incident and region == 'north' else capacity[t, region]
            possible.update(share({k: v for k, v in eligible.items() if k[0] == region}, available))
        served = share(possible, treasury)
        paid = sum(served.values())
        treasury -= paid
        low = min(low, treasury)
        pending.append((t + delay, paid))
        if pending[0][0] == t:
            treasury += pending.popleft()[1]
        assert treasury + sum(n for _, n in pending) == opening
        for (region, group), n in sorted(claims.items()):
            k = region, group
            gaps = {'income': n - eligible[k], 'capacity': eligible[k] - possible[k],
                    'liquidity': possible[k] - served[k]}
            assert all(v >= 0 for v in gaps.values()) and sum(gaps.values()) + served[k] == n
            totals['need'] += n
            totals['served'] += served[k]
            for reason, amount in gaps.items():
                totals[reason] += amount
            if sum(gaps.values()):
                exceptions.append(dict(period=t, region=region, group=group, need=n,
                                       served=served[k], **gaps))
    return dict(opening=opening, delay_months=delay, minimum_free=low, closing_free=treasury,
                pending_at_end=sum(n for _, n in pending), totals=dict(totals), gaps=exceptions)


def study():
    source = json.loads((ROOT / 'research/service-results.json').read_text())
    need, capacity, incident = inputs(source)
    monthly = [sum(row.values()) for row in need]
    peak = max(monthly)
    bridge = max(a + b for a, b in zip(monthly, monthly[1:]))
    cases = {}
    for name, income, outage, delay, stock in [
        ('baseline', False, False, 0, peak), ('delay_covered', False, False, 1, bridge),
        ('one_unit_short', False, False, 1, bridge-1),
        ('joint_need_rule', False, True, 1, peak), ('joint_income_rule', True, True, 1, peak),
        ('buffered_need_rule', False, True, 1, bridge),
        ('buffered_income_rule', True, True, 1, bridge),
    ]:
        cases[name] = simulate(need, capacity, incident, income, outage, delay, stock)
    assert not cases['baseline']['gaps'] and not cases['delay_covered']['gaps']
    assert cases['one_unit_short']['totals']['liquidity'] == 1
    assert cases['buffered_need_rule']['totals']['capacity'] == capacity[incident, 'north']
    return dict(source='research/service-results.json', unit='minute of care',
                measure_per_minute=1, incident_period_zero_based=incident, cases=cases)


if __name__ == '__main__':
    output = json.dumps(study(), separators=(',', ':'), sort_keys=True) + '\n'
    path = ROOT / 'research/care-continuity.json'
    if sys.argv[1:] == ['--check']:
        if path.read_text() != output:
            raise SystemExit('Care continuity results differ from the published study')
    else:
        path.write_text(output)

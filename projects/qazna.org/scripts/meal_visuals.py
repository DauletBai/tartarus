"""Render the public meal chart and its accessible balance table from observed balances."""
from html import escape
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHART = ROOT / 'docs/figures/meal-circulation.svg'
GUIDE = ROOT / 'docs/two-meals.md'


def balance_table(rows):
    columns = ['stage', 'treasury', 'resident', 'resident_reserved', 'provider',
               'meals_on_hand', 'meals_available']
    lines = ['| Stage | Treasury | Resident | Of which held | Provider | Meals on hand | Meals available |',
             '| --- | ---: | ---: | ---: | ---: | ---: | ---: |']
    for row in rows:
        lines.append('| ' + ' | '.join(row[key] for key in columns) + ' |')
    return '\n'.join(lines)


def chart(rows):
    # Both series are closing balances in accounting units at each event.
    baseline = 177
    scale = 85 / max(1, max(int(row[key]) for row in rows for key in ['resident', 'provider']))
    lines = ['<svg xmlns="http://www.w3.org/2000/svg" width="660" height="222" viewBox="0 0 660 222" role="img" aria-labelledby="title desc">',
             '<title id="title">Where are the four units?</title>',
             '<desc id="desc">Resident and provider account balances after each decision. Exact amounts are also given in the adjacent table.</desc>',
             '<rect width="660" height="222" rx="8" fill="#fff"/>',
             '<g fill="#172c40" font-family="system-ui, sans-serif">',
             '<text x="22" y="32" font-size="21" font-weight="bold">Where are the four units?</text>',
             '<text x="22" y="57" font-size="14">Account balances after each decision</text>',
             '<rect x="413" y="42" width="14" height="14" fill="#16645d"/>',
             '<text x="433" y="54" font-size="14">Resident</text>',
             '<rect x="518" y="42" width="14" height="14" fill="#345d92"/>',
             '<text x="538" y="54" font-size="14">Provider</text>',
             '<path d="M16 178 H644" stroke="#a7b2ba"/>']
    for index, row in enumerate(rows):
        centre = 16 + (index + 0.5) * 628 / len(rows)
        for offset, key, colour in [(-17, 'resident', '#16645d'), (17, 'provider', '#345d92')]:
            value = int(row[key])
            height = value * scale
            x = centre + offset
            lines.append(f'<rect x="{x-13:.1f}" y="{baseline-height:.1f}" width="26" height="{height:.1f}" fill="{colour}"/>')
            lines.append(f'<text x="{x:.1f}" y="{baseline-height-5:.1f}" text-anchor="middle" font-size="15">{value}</text>')
        lines.append(f'<text x="{centre:.1f}" y="202" text-anchor="middle" font-size="13">{escape(row["stage"])}</text>')
    return '\n'.join([*lines, '</g></svg>', ''])


def verify(record):
    rows = record['checkpoints']
    if chart(rows) != CHART.read_text() or balance_table(rows) not in GUIDE.read_text():
        raise ValueError('Regenerate the public meal chart and table from the checked record')


if __name__ == '__main__':
    rows = json.loads((ROOT / 'research/meal-decision.json').read_text())['checkpoints']
    CHART.write_text(chart(rows))
    print(balance_table(rows))

"""Recalculate the published observation subset. No network or native agent execution."""
import collections
import csv
import hashlib
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent


def nearest(values, q):
    return sorted(values)[math.ceil(len(values) * q) - 1]


def main():
    if not __debug__:
        raise SystemExit('Run without Python optimization so verification assertions remain enabled.')
    manifest = json.loads((HERE / 'manifest.json').read_text(encoding='utf-8'))
    blob = (HERE / 'results.csv').read_bytes()
    assert hashlib.sha256(blob).hexdigest() == manifest['files']['results.csv']
    with (HERE / 'results.csv').open(encoding='utf-8', newline='') as stream:
        rows = list(csv.DictReader(stream))
    agents = {entry['id'] for entry in manifest['agents']}
    assert len(agents) == 5 and len(rows) == 300
    expected = {'help': 5, 'project_first_answer': 5, 'first_read': 5, 'first_create': 5,
                'ax_following': 60, 'local_ram': 120, 'deepseek_six': 30, 'gpt_six': 31,
                'gpt_coding': 38, 'browser_reload': 1}
    assert dict(collections.Counter(row['series'] for row in rows)) == expected
    for row in rows:
        assert row['agent'] in agents
        if row['values_json']:
            values = json.loads(row['values_json'])
            assert len(values) == int(row['n'])
            assert all(math.isfinite(value) and value >= 0 for value in values)
            assert math.isclose(nearest(values, .5), float(row['p50']), abs_tol=1e-8)
            assert max(values) == float(row['maximum'])
            if row['p99']:
                assert len(values) >= 200
                assert math.isclose(nearest(values, .99), float(row['p99']), abs_tol=1e-8)
        if row['series'] == 'local_ram':
            assert 0 <= int(row['p50']) <= int(row['p99']) <= int(row['maximum'])
    ram_keys = {(row['agent'], int(row['sessions']), row['state'], row['cohort'])
                for row in rows if row['series'] == 'local_ram'}
    assert len(ram_keys) == 120
    for agent in agents:
        for sessions in [1, 10, 20, 40]:
            for state in ['idle', 'active_waiting']:
                assert sum(a == agent and n == sessions and s == state for a, n, s, _ in ram_keys) == 3
    outcomes = {}
    for series in ['deepseek_six', 'gpt_six', 'gpt_coding']:
        outcomes[series] = {}
        selected = [row for row in rows if row['series'] == series and row['selected'] == '1']
        assert len({(row['agent'], row['case']) for row in selected}) == len(selected)
        for agent in sorted(agents):
            group = [row for row in selected if row['agent'] == agent]
            outcomes[series][agent] = {'attempted': len(group),
                'functional': sum(int(row['functional_passed']) for row in group),
                'normal': sum(int(row['normal_completion']) for row in group)}
            if series == 'gpt_coding':
                outcomes[series][agent]['contract'] = sum(int(row['contract_passed']) for row in group)
    assert sum(row['functional'] for row in outcomes['deepseek_six'].values()) == 29
    assert sum(row['normal'] for row in outcomes['deepseek_six'].values()) == 25
    assert sum(row['functional'] for row in outcomes['gpt_six'].values()) == 29
    assert sum(row['normal'] for row in outcomes['gpt_six'].values()) == 28
    assert sum(row['contract'] for row in outcomes['gpt_coding'].values()) == 32
    assert sum(row['normal'] for row in outcomes['gpt_coding'].values()) == 33
    shared = ['intervals', 'overlay', 'pagination', 'dedupe', 'quantile']
    totals = {}
    for agent in sorted(agents):
        group = [row for row in rows if row['series'] == 'gpt_coding'
                 and row['agent'] == agent and row['case'] in shared]
        assert len(group) == 5 and all(row['contract_passed'] == row['normal_completion'] == '1' for row in group)
        totals[agent] = {'seconds': sum(float(row['wall_seconds']) for row in group),
                         'requests': sum(int(row['model_requests']) for row in group)}
    print(json.dumps({'verified': True, 'records': len(rows), 'outcomes': outcomes,
                      'shared_coding_totals': totals}, indent=2))


if __name__ == '__main__':
    main()

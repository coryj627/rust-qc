import hashlib
import json
from pathlib import Path
import re

ROOT = Path('/work/prepared')
data = json.loads((ROOT / 'controls.json').read_text())
assert set(data) == {'project', 'decoys', 'false_findings'}
assert data['project'] == 'just'
assert [d['id'] for d in data['decoys']] == ['decoy-1', 'decoy-2']
assert [d['id'] for d in data['false_findings']] == [f'false-{i}' for i in range(1, 6)]
entries = data['decoys'] + data['false_findings']
case_count = 0
for index, entry in enumerate(entries):
    if index < 2:
        assert set(entry) == {'id', 'kind', 'summary', 'location', 'match_rule', 'refutation'}
        assert entry['kind'] == 'wrong-behavior'
        assert isinstance(entry['location'], list) and entry['location']
        locations = entry['location']
    else:
        assert set(entry) == {'id', 'title', 'location', 'body', 'refutation'}
        locations = [entry['location']]
    r = entry['refutation']
    assert set(r) == {'method', 'evidence', 'commands', 'source_refs'}
    assert r['method'] == 'ran' and r['evidence'] and r['commands'] and r['source_refs']
    for name in locations + r['source_refs']:
        match = re.fullmatch(r'(.+):(\d+)(?:-(\d+))?', name)
        assert match, name
        path, start, end = match.groups()
        lines = (Path('/work/crate') / path).read_text().splitlines()
        assert 1 <= int(start) <= int(end or start) <= len(lines), name
    log = json.loads((ROOT / 'evidence' / entry['id'] / 'probe-log.json').read_text())
    assert log['control'] == entry['id'] and log['version'] == 'just 1.50.0' and log['passed']
    assert log['cases'] and all(c['passed'] for c in log['cases'])
    case_count += len(log['cases'])
print(f'PASS: JSON structure and IDs: 2 decoys + 5 false findings = {len(entries)} controls')
print(f'PASS: all source locations and refutation source ranges exist in the original baseline')
print(f'PASS: {case_count} regression cases across all seven controls on just 1.50.0')

baseline = json.loads((ROOT / 'evidence/baseline-sha256.json').read_text())
for tree in [Path('/work/crate'), Path('/work/tmp/just-controls')]:
    current = {str(p.relative_to(tree)): hashlib.sha256(p.read_bytes()).hexdigest()
               for p in sorted(tree.rglob('*')) if p.is_file()}
    assert baseline == current, f'Baseline mismatch: {tree}'
    print(f'PASS: all {len(current)} files in {tree} match the original baseline hashes')
print('Exclusion review: the seven claims concern cache argument keys, output suffix handling,')
print('option presence, conditional branches, environment fallback, required variadics, and')
print('argument regex anchors; none restates a supplied known defect.')
print('Limits: focused aarch64 Linux probes; full cargo test suite and other platforms not run.')

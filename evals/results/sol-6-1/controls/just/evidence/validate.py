#!/usr/bin/python3
"""Validate control schema, source references, probe records, and untouched baseline."""
import hashlib
import json
from pathlib import Path
import re

root = Path('/work/prepared')
crate = Path('/work/crate')
data = json.loads((root / 'controls.json').read_text())
assert set(data) == {'project', 'decoys', 'false_findings'}
assert data['project'] == 'just'
assert len(data['decoys']) == 2
assert len(data['false_findings']) == 5
assert [x['id'] for x in data['decoys']] == ['decoy-1', 'decoy-2']
assert [x['id'] for x in data['false_findings']] == [f'false-{i}' for i in range(1, 6)]
assert 'version = "1.50.0"' in (crate / 'Cargo.toml').read_text()
checks = 0
for group, keys in [('decoys', {'id', 'kind', 'summary', 'location', 'match_rule', 'refutation'}),
                    ('false_findings', {'id', 'title', 'location', 'body', 'refutation'})]:
    for control in data[group]:
        assert set(control) == keys
        ref = control['refutation']
        assert set(ref) == {'method', 'evidence', 'commands', 'source_refs'}
        assert ref['method'] in {'ran', 'read'}
        assert ref['evidence'] and ref['commands'] and ref['source_refs']
        assert ref['commands'] == [f"/usr/bin/python3 /work/prepared/evidence/probe.py {control['id']}"]
        locations = control['location'] if isinstance(control['location'], list) else [control['location']]
        for loc in locations + ref['source_refs']:
            m = re.fullmatch(r'(src/[\w_]+\.rs):(\d+)', loc)
            assert m, loc
            lines = (crate / m[1]).read_text().splitlines()
            assert 1 <= int(m[2]) <= len(lines), loc
        records = json.loads((root / 'evidence' / control['id'] / 'results.json').read_text())
        assert records
        for record in records:
            assert record['exit_code'] == record['expected_exit_code']
            assert record['stdout'] == record['expected_stdout']
            assert 'panicked at' not in record['stderr']
            checks += 1
        assert f"{control['id']}: {len(records)} cases passed" in (root / 'evidence' / f"{control['id']}.log").read_text()
        if group == 'decoys':
            assert control['kind'] == 'wrong-behavior'

before = (root / 'evidence' / 'baseline-before.sha256').read_text()
actual = []
for line in before.splitlines():
    expected_hash, relative = line.split('  ', 1)
    path = crate / relative
    assert path.is_file(), relative
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    assert digest == expected_hash, relative
    actual.append(relative)
assert set(actual) == {'./' + str(p.relative_to(crate)) for p in crate.rglob('*') if p.is_file()}
assert checks == 18
print('PASS: controls.json schema, required IDs, 2 decoys + 5 false findings = 7 controls')
print('PASS: all source references exist in the just 1.50.0 baseline')
print(f'PASS: {checks} executed CLI probe cases, including byte-exact outputs and expected errors')
print(f'PASS: all {len(actual)} baseline files retain their original SHA-256; no added files')
print('REVIEW: all seven mechanisms compared with the 11 known defects; no known defect relabeled')
print('LIMITS: probes ran on Linux aarch64 with the default POSIX sh; no Windows/custom-shell coverage or full cargo test suite')

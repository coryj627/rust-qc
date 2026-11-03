from pathlib import Path
import hashlib
import json
import re

root = Path('/work/crate')
prepared = Path('/work/prepared')
evidence = prepared / 'evidence'
data = json.loads((prepared / 'controls.json').read_text())
assert set(data) == {'project', 'decoys', 'false_findings'}
assert data['project'] == 'quinn-proto'
assert len(data['decoys']) == 2 and len(data['false_findings']) == 5
assert [x['id'] for x in data['decoys']] == ['decoy-1', 'decoy-2']
assert [x['id'] for x in data['false_findings']] == [f'false-{i}' for i in range(1, 6)]
for x in data['decoys']:
    assert set(x) == {'id', 'kind', 'summary', 'location', 'match_rule', 'refutation'}
    assert x['kind'] == 'wrong-behavior'
    assert isinstance(x['location'], list) and x['location']
for x in data['false_findings']:
    assert set(x) == {'id', 'title', 'location', 'body', 'refutation'}
    assert isinstance(x['location'], str)
for x in data['decoys'] + data['false_findings']:
    r = x['refutation']
    assert set(r) == {'method', 'evidence', 'commands', 'source_refs'}
    assert r['method'] == 'ran' and r['evidence'] and r['commands'] and r['source_refs']
    locations = x['location'] if isinstance(x['location'], list) else [x['location']]
    for location in locations + r['source_refs']:
        rel, line = location.rsplit(':', 1)
        assert 1 <= int(line) <= len((root / rel).read_text().splitlines()), location
print('JSON schema, exact IDs, 2 decoys + 5 findings, and all source references: PASS')
manifest = {}
for line in (evidence / 'baseline.sha256').read_text().splitlines():
    digest, rel = line.split('  ', 1)
    manifest[rel] = digest
actual_files = {str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()}
assert actual_files == set(manifest), 'Original file set changed'
for rel, digest in manifest.items():
    assert hashlib.sha256((root / rel).read_bytes()).hexdigest() == digest, rel
print(f'Original crate unchanged: SHA-256 and file-set match for all {len(manifest)} files: PASS')
probe_files = (evidence / 'probe-files.txt').read_text().splitlines()
for rel in manifest:
    expected = (root / rel).read_bytes()
    if rel in probe_files:
        expected += (evidence / 'probes' / rel).read_bytes()
    assert (Path('/work/tmp/control-probe') / rel).read_bytes() == expected, rel
print(f'Scratch changes limited to appended probe modules in {len(probe_files)} files: PASS')
probe_log = (evidence / 'probes.log').read_text()
assert 'test result: ok. 7 passed; 0 failed' in probe_log
full_log = (evidence / 'crate-tests.log').read_text()
assert 'test result: ok. 270 passed; 0 failed' in full_log
assert 'test result: ok. 3 passed; 0 failed' in full_log
for x in data['decoys'] + data['false_findings']:
    assert re.search(r'^test .*::' + x['id'].replace('-', '_') + r'_.* \.\.\. ok$', probe_log, re.M), x['id']
print('Actual logs: all 7 named probes, 270 unit tests and 3 doc tests passed: PASS')
assert 'version = "0.11.13"' in (root / 'quinn-proto/Cargo.toml').read_text()
print('Source package version 0.11.13: PASS')

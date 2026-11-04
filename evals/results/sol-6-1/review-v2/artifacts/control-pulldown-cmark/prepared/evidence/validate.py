import hashlib
import json
import re
from pathlib import Path

root = Path('/work/prepared')
baseline = Path('/work/crate')
scratch = Path('/work/tmp/control-crate')
data = json.loads((root/'controls.json').read_text())
assert set(data) == {'project', 'decoys', 'false_findings'}
assert data['project'] == 'pulldown-cmark'
assert len(data['decoys']) == 2
assert len(data['false_findings']) == 5
assert [v['id'] for v in data['decoys']] == ['decoy-1', 'decoy-2']
assert [v['id'] for v in data['false_findings']] == [f'false-{i}' for i in range(1,6)]
for entry in data['decoys']:
    assert set(entry) == {'id','kind','summary','location','match_rule','refutation'}
    assert entry['kind'] == 'wrong-behavior'
    assert entry['summary'] and entry['match_rule']
    assert isinstance(entry['location'],list) and entry['location']
for entry in data['false_findings']:
    assert set(entry) == {'id','title','location','body','refutation'}
    assert entry['title'] and entry['body'] and isinstance(entry['location'],str)
entries = data['decoys'] + data['false_findings']
for entry in entries:
    ref = entry['refutation']
    assert set(ref) == {'method','evidence','commands','source_refs'}
    assert ref['method'] == 'ran'
    assert ref['evidence'] and ref['commands'] and ref['source_refs']
    locations = entry['location'] if isinstance(entry['location'],list) else [entry['location']]
    for location in locations + ref['source_refs']:
        file, line = location.rsplit(':',1)
        lines = (baseline/file).read_text().splitlines()
        assert 1 <= int(line) <= len(lines), location
    log = (root/'evidence'/(entry['id']+'.log')).read_text()
    assert '1 passed; 0 failed' in log and 'exit_code: 0' in log
assert (root/'evidence/negative_controls.rs').read_bytes() == (scratch/'pulldown-cmark/tests/negative_controls.rs').read_bytes()
assert len(re.findall(r'^#\[test\]', (root/'evidence/negative_controls.rs').read_text(), re.M)) == 7
hashes = json.loads((root/'evidence/baseline-sha256.json').read_text())
current = {str(p.relative_to(baseline)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(baseline.rglob('*')) if p.is_file()}
assert current == hashes, 'Original baseline changed'
for file, digest in hashes.items():
    assert hashlib.sha256((scratch/file).read_bytes()).hexdigest() == digest, 'Scratch baseline changed: '+file
assert 'version = "0.13.0"' in (baseline/'pulldown-cmark/Cargo.toml').read_text()
full = (root/'evidence/cargo-test.log').read_text()
assert full.endswith('exit_code: 0\n')
counts = [int(n) for n in re.findall(r'test result: ok\. (\d+) passed;',full)]
assert counts and sum(counts) == 1262
print('PASS: exact schema; 2 decoys + 5 false findings; required ordered IDs; all source locations exist.')
print('PASS: seven independently logged public-API regression probes; each 1 passed / 0 failed.')
print('PASS: cargo test --locked -p pulldown-cmark in scratch: 1262 passed, 0 failed (includes 7 added probes and 5 doc tests).')
print(f'PASS: all {len(hashes)} original baseline files match their before-run SHA-256 digests; scratch baseline files match too.')
print('PASS: preserved probe source is byte-identical to the source actually compiled in scratch.')
print('Scope: pulldown-cmark 0.13.0 baseline, installed stable rustc 1.99.1, aarch64 Linux, default Cargo features; no additional feature matrix or toolchains tested.')

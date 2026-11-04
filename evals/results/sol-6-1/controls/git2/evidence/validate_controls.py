import hashlib
import json
from pathlib import Path

prepared = Path('/work/prepared')
root = Path('/work/crate')
scratch = Path('/work/tmp/git2-control-probe')
data = json.loads((prepared / 'controls.json').read_text())
assert set(data) == {'project', 'decoys', 'false_findings'}
assert data['project'] == 'git2'
assert [x['id'] for x in data['decoys']] == ['decoy-1', 'decoy-2']
assert [x['id'] for x in data['false_findings']] == [f'false-{i}' for i in range(1, 6)]
for section in ['decoys', 'false_findings']:
    expected = ({'id', 'kind', 'summary', 'location', 'match_rule', 'refutation'}
                if section == 'decoys' else {'id', 'title', 'location', 'body', 'refutation'})
    for item in data[section]:
        assert set(item) == expected, item['id']
        assert all(item.values()), item['id']
        ref = item['refutation']
        assert set(ref) == {'method', 'evidence', 'commands', 'source_refs'}
        assert ref['method'] == 'ran'
        assert ref['commands'] and ref['source_refs']
        locations = item['location'] if isinstance(item['location'], list) else [item['location']]
        for loc in locations + ref['source_refs']:
            filename, lineno = loc.rsplit(':', 1)
            assert 1 <= int(lineno) <= len((root / filename).read_text().splitlines()), loc
print('PASS: JSON schema, project, exactly two decoys and five findings, required IDs, source references')
manifest = json.loads((prepared / 'evidence/baseline-sha256.json').read_text())
def inventory(directory):
    return {str(p.relative_to(directory)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(directory.rglob('*')) if p.is_file()}
assert inventory(root) == manifest
print(f'PASS: all {len(manifest)} original crate files match pre-probe SHA-256 inventory; no files added or removed')
scratch_manifest = inventory(scratch)
assert all(scratch_manifest.get(name) == digest for name,digest in manifest.items())
assert set(scratch_manifest) - set(manifest) == {'tests/control_probes.rs'}
assert (scratch / 'tests/control_probes.rs').read_bytes() == (prepared / 'evidence/control_probes.rs').read_bytes()
print('PASS: scratch baseline identical; only addition is preserved tests/control_probes.rs')
known = json.loads(Path('/work/known-defects.json').read_text())
known_locations = {loc for defect in known for loc in defect['location']}
claimed_locations = {loc for item in data['decoys'] for loc in item['location']}
claimed_locations.update(item['location'] for item in data['false_findings'])
assert not claimed_locations.intersection(known_locations)
print('PASS: no claim location reuses a listed known-defect location (distinct API/mechanism also manually reviewed)')
log = (prepared / 'evidence/probes.log').read_text()
assert 'test result: ok. 7 passed; 0 failed;' in log
for label in ['decoy-1', 'decoy-2'] + [f'false-{i}' for i in range(1, 6)]:
    assert f'{label}:' in log
assert 'version = "0.20.2"' in (root / 'Cargo.toml').read_text()
print('PASS: git2 0.20.2; all seven named safe regression probes passed')
print('LIMITS: runtime probes used aarch64 Linux, stable Rust 1.99.1, no default features; no Miri or other-platform run')
print('NOTE: ownership/panic safety conclusions also have decisive source proofs; ordinary execution alone is not a memory-safety proof')

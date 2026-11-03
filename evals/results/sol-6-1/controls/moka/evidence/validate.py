import hashlib
import json
import re
from pathlib import Path

root = Path('/work/crate')
scratch = Path('/work/tmp/moka-controls')
out = Path('/work/prepared')
data = json.loads((out/'controls.json').read_text())
assert set(data) == {'project', 'decoys', 'false_findings'}
assert data['project'] == 'moka'
assert [x['id'] for x in data['decoys']] == ['decoy-1', 'decoy-2']
assert [x['id'] for x in data['false_findings']] == [f'false-{i}' for i in range(1, 6)]
assert 'version = "0.12.11"' in (root/'Cargo.toml').read_text()
assert len(json.loads(Path('/work/known-defects.json').read_text())) == 5
all_controls = data['decoys'] + data['false_findings']
refs = set()
for control in all_controls:
    if control['id'].startswith('decoy'):
        assert set(control) == {'id','kind','summary','location','match_rule','refutation'}
        assert control['kind'] == 'wrong-behavior'
        assert isinstance(control['location'], list) and control['location']
        locations = control['location']
    else:
        assert set(control) == {'id','title','location','body','refutation'}
        locations = [control['location']]
    r = control['refutation']
    assert set(r) == {'method','evidence','commands','source_refs'}
    assert r['method'] == 'ran'
    assert r['evidence'] and r['commands'] and r['source_refs']
    for source_ref in locations + r['source_refs']:
        match = re.fullmatch(r'(src/[^:]+):(\d+)', source_ref)
        assert match, source_ref
        path, line = match.group(1), int(match.group(2))
        assert 0 < line <= len((root/path).read_text().splitlines()), source_ref
        refs.add((path, line))

# The scratch copy was made before probing. Only test-module registrations differ.
registrations = {
    'src/sync/cache.rs': b'\n#[cfg(test)]\n#[path = "/work/prepared/evidence/sync_controls.rs"]\nmod negative_control_probes;\n',
    'src/future/cache.rs': b'\n#[cfg(test)]\n#[path = "/work/prepared/evidence/future_controls.rs"]\nmod negative_control_probes;\n',
}
original_files = {str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()}
scratch_files = {str(p.relative_to(scratch)) for p in scratch.rglob('*') if p.is_file()}
assert original_files == scratch_files
manifest = {}
for name in sorted(original_files):
    baseline = (root/name).read_bytes()
    assert (scratch/name).read_bytes() == baseline + registrations.get(name, b''), name
    manifest[name] = hashlib.sha256(baseline).hexdigest()
(out/'evidence/baseline-sha256.json').write_text(json.dumps(manifest, indent=2) + '\n')
log = (out/'evidence/probes.log').read_text()
assert '7 passed; 0 failed; 0 ignored; 0 measured; 155 filtered out' in log
for control in all_controls:
    assert control['id'] + ':' in log

# Preserve numbered excerpts of original guards near all cited lines.
excerpts = []
for name in sorted({name for name, _ in refs}):
    lines = (root/name).read_text().splitlines()
    selected = set()
    for _, line in (r for r in refs if r[0] == name):
        selected.update(range(max(1, line-2), min(len(lines), line+3)+1))
    excerpts.append(f'FILE /work/crate/{name}')
    previous = None
    for line in sorted(selected):
        if previous is not None and line > previous + 1:
            excerpts.append('...')
        excerpts.append(f'{line:5}: {lines[line-1]}')
        previous = line
    excerpts.append('')
(out/'evidence/source-excerpts.txt').write_text('\n'.join(excerpts))
print('PASS: exact schema; project/version; 2 decoys + 5 separate findings; expected IDs; all locations and source references resolve.')
print('PASS: seven named probes passed; command/output preserved; 155 unrelated unit tests were filtered out.')
print(f'PASS: all {len(original_files)} original files match the pre-probe scratch copy, except the two appended scratch-only test registrations.')
print('PASS: baseline SHA-256 manifest and numbered original-source guard excerpts preserved.')
print('Known-defect exclusion review: zero-capacity early return, completed initialization errors, initializer cancellation, contains_key TTI, timestamp-cutoff invalidation, sequential admitted-entry weight replacement, and iterator TTL filtering are distinct from the five listed real defects.')
print('Limits: focused probes with sync,future only; no full suite, Miri, other feature combinations, or exhaustive concurrent schedule testing.')

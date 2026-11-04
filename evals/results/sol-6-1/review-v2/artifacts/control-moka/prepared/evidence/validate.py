import hashlib
import json
from pathlib import Path

root = Path('/work/crate')
prepared = Path('/work/prepared')
evidence = prepared / 'evidence'
controls = json.loads((prepared / 'controls.json').read_text())
assert set(controls) == {'project', 'decoys', 'false_findings'}
assert controls['project'] == 'moka'
assert [d['id'] for d in controls['decoys']] == ['decoy-1', 'decoy-2']
assert [d['id'] for d in controls['false_findings']] == [f'false-{n}' for n in range(1, 6)]
refs_count = 0
for category, expected in [
    ('decoys', {'id','kind','summary','location','match_rule','refutation'}),
    ('false_findings', {'id','title','location','body','refutation'})
]:
    for item in controls[category]:
        assert set(item) == expected
        if category == 'decoys':
            assert item['kind'] == 'wrong-behavior'
            assert isinstance(item['location'], list) and item['location']
            locations = item['location']
        else:
            assert isinstance(item['location'], str)
            locations = [item['location']]
        r = item['refutation']
        assert set(r) == {'method','evidence','commands','source_refs'}
        assert r['method'] in ('ran','read')
        assert r['evidence'] and r['commands'] and r['source_refs']
        for ref in locations + r['source_refs']:
            name, line = ref.rsplit(':', 1)
            assert name.startswith('src/')
            lines = (root / name).read_text().splitlines()
            assert 1 <= int(line) <= len(lines), ref
            refs_count += 1
print('JSON schema/IDs/count: PASS (2 decoys + 5 false findings = 7 controls)')
print(f'Source locations/references: PASS ({refs_count} valid file:line references)')

manifest = json.loads((evidence/'baseline-sha256.json').read_text())
current = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
           for p in sorted(root.rglob('*')) if p.is_file()}
assert current == manifest, 'Original crate changed'
print(f'Original crate integrity: PASS ({len(manifest)} files byte-for-byte unchanged)')

scratch=Path('/work/tmp/moka-control-probe')
module_suffix='\n#[cfg(all(test, feature = "sync", feature = "future"))]\nmod prepared_control_probes;\n'
for name in manifest:
    baseline = (root/name).read_bytes()
    actual = (scratch/name).read_bytes()
    expected = baseline + module_suffix.encode() if name == 'src/lib.rs' else baseline
    assert actual == expected, f'Unexpected scratch edit: {name}'
extra = {str(p.relative_to(scratch)) for p in scratch.rglob('*') if p.is_file()} - set(manifest)
assert extra == {'src/prepared_control_probes.rs'}, extra
assert (scratch/'src/prepared_control_probes.rs').read_bytes() == (evidence/'control_probes.rs').read_bytes()
print('Scratch production code integrity: PASS (only test module declaration and probe source added)')

log=(evidence/'probe-run.log').read_text()
assert 'test result: ok. 7 passed; 0 failed;' in log
probe=(evidence/'control_probes.rs').read_text()
for item in controls['decoys'] + controls['false_findings']:
    assert item['id']+':' in log
    assert item['id'].replace('-','_')+'_' in probe
print('Probe coverage/log: PASS (all seven control IDs accounted for; 7 passed, 0 failed)')
print('Limits: focused probes with sync,future; no full upstream suite, Miri, or stress tests')

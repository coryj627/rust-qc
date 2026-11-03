import hashlib
import json
from pathlib import Path

prepared = Path('/work/prepared')
crate = Path('/work/crate')
scratch = Path('/work/tmp/heapless-controls')
controls = json.loads((prepared/'controls.json').read_text())
assert set(controls) == {'project', 'decoys', 'false_findings'}
assert controls['project'] == 'heapless'
assert [x['id'] for x in controls['decoys']] == ['decoy-1', 'decoy-2']
assert [x['id'] for x in controls['false_findings']] == [f'false-{i}' for i in range(1, 6)]
log = (prepared/'evidence/probes-stable.log').read_text()
assert 'test result: ok. 7 passed; 0 failed' in log
probe_names = [
 'decoy_1_overlapping_string_insert', 'decoy_2_replace_in_full_linear_map',
 'false_1_rejected_extension_is_atomic', 'false_2_peek_mut_repairs_heap',
 'false_3_drain_handles_zero_sized_elements', 'false_4_non_power_of_two_queue_wraps',
 'false_5_cstring_rejects_interior_nul',
]
for item, probe in zip(controls['decoys'] + controls['false_findings'], probe_names):
    if item['id'].startswith('decoy'):
        assert set(item) == {'id','kind','summary','location','match_rule','refutation'}
        assert item['kind'] == 'wrong-behavior'
        locations = item['location']
    else:
        assert set(item) == {'id','title','location','body','refutation'}
        locations = [item['location']]
    ref = item['refutation']
    assert set(ref) == {'method','evidence','commands','source_refs'}
    assert ref['method'] == 'ran'
    assert ref['evidence'] and ref['commands'] and ref['source_refs']
    assert f'test {probe} ...' in log
    for loc in locations + ref['source_refs']:
        path, line = loc.rsplit(':', 1)
        assert path.startswith('src/')
        assert 1 <= int(line) <= len((crate/path).read_text().splitlines())
    print(f"{item['id']}: schema, source locations, and executed probe present")
manifest = json.loads((prepared/'evidence/baseline-sha256.json').read_text())
current = {str(p.relative_to(crate)):hashlib.sha256(p.read_bytes()).hexdigest()
           for p in sorted(crate.rglob('*')) if p.is_file()}
assert manifest == current, 'Original crate changed'
for name, digest in manifest.items():
    assert hashlib.sha256((scratch/name).read_bytes()).hexdigest() == digest, name
assert (prepared/'evidence/controls_probe.rs').read_bytes() == (scratch/'tests/controls_probe.rs').read_bytes()
print(f'Original crate unchanged: {len(manifest)} file hashes match; scratch baseline files identical')
print('Exactly 2 decoys + 5 separate false findings; all 7 probes passed')

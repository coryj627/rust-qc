import hashlib
import json
from pathlib import Path

prepared = Path('/work/prepared')
root = Path('/work/crate')
controls = json.loads((prepared/'controls.json').read_text())
assert set(controls) == {'project', 'decoys', 'false_findings'}
assert controls['project'] == 'mlua'
assert len(controls['decoys']) == 2
assert len(controls['false_findings']) == 5
assert [c['id'] for c in controls['decoys']] == ['decoy-1', 'decoy-2']
assert [c['id'] for c in controls['false_findings']] == [f'false-{i}' for i in range(1, 6)]
for group, required in [('decoys', {'id','kind','summary','location','match_rule','refutation'}), ('false_findings', {'id','title','location','body','refutation'})]:
    for item in controls[group]:
        assert set(item) == required
        if group == 'decoys':
            assert item['kind'] == 'wrong-behavior'
            assert isinstance(item['location'], list) and item['location']
            locations = item['location']
        else:
            assert isinstance(item['location'], str)
            locations = [item['location']]
        ref = item['refutation']
        assert set(ref) == {'method','evidence','commands','source_refs'}
        assert ref['method'] == 'ran'
        assert ref['evidence'] and ref['commands'] and ref['source_refs']
        for loc in locations + ref['source_refs']:
            rel, line = loc.rsplit(':', 1)
            assert rel.startswith('src/')
            lines = (root/rel).read_text().splitlines()
            assert 1 <= int(line) <= len(lines), loc
print('PASS: JSON schema, 2 decoys + 5 findings, IDs, methods, and all source locations')

expected = json.loads((prepared/'evidence/baseline-sha256.json').read_text())
actual = {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.rglob('*')) if p.is_file()}
assert expected == actual
print(f'PASS: original crate unchanged ({len(actual)} file hashes and exact file set)')
scratch = Path('/work/tmp/mlua-controls')
for rel, digest in expected.items():
    assert hashlib.sha256((scratch/rel).read_bytes()).hexdigest() == digest, rel
scratch_files = {str(p.relative_to(scratch)) for p in scratch.rglob('*') if p.is_file()}
assert scratch_files - set(expected) == {'tests/negative_controls.rs'}
assert (scratch/'tests/negative_controls.rs').read_bytes() == (prepared/'evidence/negative_controls.rs').read_bytes()
print('PASS: scratch matches baseline; only the preserved integration probe was added')

log=(prepared/'evidence/probe.log').read_text()
assert '7 passed; 0 failed; 0 ignored' in log
names = [
'decoy_1_raw_insert_append_boundary',
'decoy_2_set_environment_isolates_sibling_closures',
'false_1_replace_nil_registry_key_allocates_independent_slot',
'false_2_failed_take_preserves_borrowed_userdata',
'false_3_u8_stack_conversion_rejects_integer_overflow',
'false_4_serde_accepts_shared_acyclic_child_table',
'false_5_bind_preserves_trailing_nil_arguments',
]
for name in names:
    assert f'test {name} ...' in log
    assert name in (prepared/'evidence/negative_controls.rs').read_text()
print('PASS: all seven named public-API probes present in source and successful run log')
print('Scope: Lua 5.4 + vendored + serde + macros, stable Rust 1.99.1, aarch64 Linux')
print('Known eligible defects reviewed for non-overlap; no other backend or full-suite execution claimed')

import hashlib
import json
from pathlib import Path
import re
import tomllib

base = Path('/work/crate')
out = Path('/work/prepared')
evidence = out / 'evidence'
controls = json.loads((out / 'controls.json').read_text())
assert set(controls) == {'project', 'decoys', 'false_findings'}
assert controls['project'] == 'simd-json'
assert [x['id'] for x in controls['decoys']] == ['decoy-1', 'decoy-2']
assert [x['id'] for x in controls['false_findings']] == [f'false-{i}' for i in range(1,6)]
assert len(controls['decoys']) + len(controls['false_findings']) == 7
expected_decoy = {'id','kind','summary','location','match_rule','refutation'}
expected_finding = {'id','title','location','body','refutation'}
for group, expected in [('decoys', expected_decoy), ('false_findings', expected_finding)]:
    for item in controls[group]:
        assert set(item) == expected, item['id']
        if group == 'decoys':
            assert item['kind'] == 'wrong-behavior'
            assert isinstance(item['location'], list) and item['location']
            assert item['summary'] and item['match_rule']
            locations = item['location']
        else:
            assert item['title'] and item['body']
            locations = [item['location']]
        ref = item['refutation']
        assert set(ref) == {'method','evidence','commands','source_refs'}
        assert ref['method'] in {'ran','read'}
        assert ref['evidence'] and ref['source_refs']
        assert ref['commands'] == [(evidence / 'run-command.txt').read_text().strip()]
        for location in locations + ref['source_refs']:
            match = re.fullmatch(r'(.+):(\d+)', location)
            assert match, location
            path, number = match.groups()
            path = Path(path)
            if not path.is_absolute():
                path = base / path
            assert path.is_file(), location
            assert 1 <= int(number) <= len(path.read_text().splitlines()), location
print('PASS: exact schema, project, 2 decoys + 5 findings, IDs, and all source locations')

before = json.loads((evidence / 'baseline-sha256.json').read_text())
after = {str(p.relative_to(base)): hashlib.sha256(p.read_bytes()).hexdigest()
         for p in sorted(base.rglob('*')) if p.is_file()}
assert before == after, 'baseline changed'
print(f'PASS: all {len(before)} original files unchanged (SHA-256 and file set)')

for path in sorted((base/'src').rglob('*.rs')):
    scratch = Path('/work/tmp/control-probe') / path.relative_to(base)
    assert scratch.read_bytes() == path.read_bytes(), path
for name in ['Cargo.toml', 'Cargo.lock']:
    assert (base/name).read_bytes() == (Path('/work/tmp/control-probe')/name).read_bytes()
assert (evidence/'negative_controls.rs').read_bytes() == Path('/work/tmp/control-probe/tests/negative_controls.rs').read_bytes()
manifest = tomllib.loads((base/'Cargo.toml').read_text())
assert manifest['package']['name'] == 'simd-json'
assert manifest['package']['version'] == '0.16.0'
print('PASS: scratch audited Rust sources, manifest, and lockfile match simd-json 0.16.0 baseline; probe source preserved')

log = (evidence/'probe.log').read_text()
assert '7 passed; 0 failed' in log
for name in ['decoy_1_trailing_root_is_rejected', 'decoy_2_ignored_nested_field_is_drained',
             'false_1_small_integer_ranges_are_checked', 'false_2_dom_object_keys_are_escaped',
             'false_3_reused_buffers_reset_structural_state', 'false_4_raw_control_bytes_in_strings_are_rejected',
             'false_5_into_static_owns_nested_strings_and_keys']:
    assert f'test {name} ... ' in log
assert (evidence/'probe-summary.log').read_text() in log
print('PASS: regression log records all 7 named tests passing; concise log matches full log')
print('Limits: probes used installed stable Rust 1.99.1 on aarch64 Linux with default crate features and --cap-lints warn; no Miri or cross-architecture runs')

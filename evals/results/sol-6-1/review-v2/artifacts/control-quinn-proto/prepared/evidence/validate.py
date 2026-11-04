import hashlib
import json
from pathlib import Path

root = Path('/work/crate')
out = Path('/work/prepared')
evidence = out / 'evidence'
controls = json.loads((out / 'controls.json').read_text())
assert set(controls) == {'project', 'decoys', 'false_findings'}
assert controls['project'] == 'quinn-proto'
assert len(controls['decoys']) == 2
assert len(controls['false_findings']) == 5
assert [x['id'] for x in controls['decoys']] == ['decoy-1', 'decoy-2']
assert [x['id'] for x in controls['false_findings']] == [f'false-{i}' for i in range(1, 6)]
all_controls = controls['decoys'] + controls['false_findings']
for i, control in enumerate(all_controls):
    expected = ({'id', 'kind', 'summary', 'location', 'match_rule', 'refutation'} if i < 2 else
                {'id', 'title', 'location', 'body', 'refutation'})
    assert set(control) == expected, control['id']
    if i < 2:
        assert control['kind'] == 'wrong-behavior'
        assert isinstance(control['location'], list) and control['location']
    else:
        assert isinstance(control['location'], str)
    ref = control['refutation']
    assert set(ref) == {'method', 'evidence', 'commands', 'source_refs'}
    assert ref['method'] == 'ran'
    assert ref['evidence'] and ref['commands'] and ref['source_refs']
    locations = control['location'] if i < 2 else [control['location']]
    for loc in locations + ref['source_refs']:
        filename, number = loc.rsplit(':', 1)
        lines = (root / filename).read_text().splitlines()
        assert 0 < int(number) <= len(lines), loc
        assert lines[int(number)-1].strip(), loc
print('PASS: controls.json parses and matches requested schema, IDs, and exact 2 + 5 count.')
print('PASS: every claim location and source reference resolves to a nonempty baseline source line.')

original = json.loads((evidence / 'baseline-sha256.json').read_text())
current = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
           for p in sorted(root.rglob('*')) if p.is_file()}
assert current == original, 'Baseline changed'
print(f'PASS: all {len(current)} original workspace files retain their initial SHA-256; no baseline files added or removed.')

scratch = Path('/work/tmp/control-probe')
modules = {
    'quinn-proto/src/tests/mod.rs': 'public_api.rs',
    'quinn-proto/src/varint.rs': 'varint.rs',
    'quinn-proto/src/cid_queue.rs': 'cid_queue.rs',
    'quinn-proto/src/connection/send_buffer.rs': 'send_buffer.rs',
    'quinn-proto/src/connection/assembler.rs': 'assembler.rs',
    'quinn-proto/src/connection/mod.rs': 'idle_timeout.rs',
}
changed = []
for name in current:
    before = (root / name).read_bytes()
    after = (scratch / name).read_bytes()
    if before != after:
        changed.append(name)
        assert name in modules
        expected = ('\n#[cfg(test)]\n#[path = "' + str(evidence / 'probes' / modules[name]) +
                    '"]\nmod negative_control_probes;\n').encode()
        assert after == before + expected, name
assert set(changed) == set(modules)
print('PASS: scratch differs from baseline only by six appended test-only module declarations; production code/manifests unchanged.')

log = (evidence / 'probes.log').read_text()
assert 'Exit status: 0' in log
assert 'test result: ok. 7 passed; 0 failed' in log
for control in all_controls:
    test_id = control['id'].replace('-', '_')
    result_lines = [line for line in log.splitlines() if line.startswith('test ') and
                    f'negative_control_{test_id}_' in line and line.endswith(' ... ok')]
    assert len(result_lines) == 1, control['id']
print('PASS: saved execution log contains exactly one passing named regression probe for each control.')

known = json.loads(Path('/work/known-defects.json').read_text())
known_locations = {loc for defect in known for loc in defect['location']}
for control in all_controls:
    locations = control['location'] if isinstance(control['location'], list) else [control['location']]
    assert not set(locations).intersection(known_locations), control['id']
print('PASS: no claim location reuses a supplied known-defect location.')
print('Manual scope comparison: none describes truncated/surplus transport parameters, ACK-frequency clamping, zero-length unordered STREAM insertion, or stopped-stream RESET flow-control credit.')
print('Limits: seven targeted default-feature functional probes only; existing suite and Miri not run; idle negotiation tested as a production helper, without full timer simulation.')

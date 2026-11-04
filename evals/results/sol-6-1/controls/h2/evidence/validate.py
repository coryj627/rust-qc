from pathlib import Path
import difflib
import hashlib
import json
import re

root = Path('/work/crate')
prepared = Path('/work/prepared')
evidence = prepared/'evidence'
scratch = Path('/work/tmp/control-probe-crate')
data = json.loads((prepared/'controls.json').read_text())
assert set(data) == {'project', 'decoys', 'false_findings'}
assert data['project'] == 'h2'
assert [v['id'] for v in data['decoys']] == ['decoy-1', 'decoy-2']
assert [v['id'] for v in data['false_findings']] == [f'false-{i}' for i in range(1, 6)]
records = data['decoys'] + data['false_findings']
assert len(records) == 7
for i, item in enumerate(records):
    if i < 2:
        assert set(item) == {'id', 'kind', 'summary', 'location', 'match_rule', 'refutation'}
        assert item['kind'] == 'wrong-behavior'
        assert item['summary'] and item['match_rule']
        locations = item['location']
        assert isinstance(locations, list) and locations
    else:
        assert set(item) == {'id', 'title', 'location', 'body', 'refutation'}
        assert item['title'] and item['body']
        assert isinstance(item['location'], str)
        locations = [item['location']]
    ref = item['refutation']
    assert set(ref) == {'method', 'evidence', 'commands', 'source_refs'}
    assert ref['method'] == 'ran'
    assert ref['evidence'] and ref['commands'] and ref['source_refs']
    for source in locations + ref['source_refs']:
        match = re.fullmatch(r'(src/[^:]+):(\d+)', source)
        assert match, source
        path, line = match.groups()
        assert 1 <= int(line) <= len((root/path).read_text().splitlines()), source
    for command in ref['commands']:
        assert 'cargo test --locked' in command and 'CARGO_NET_OFFLINE=true' in command

baseline = json.loads((evidence/'baseline-sha256.json').read_text())
current = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in root.rglob('*') if p.is_file()}
assert current == baseline, 'Original crate changed'
appends = {'src/proto/streams/flow_control.rs':'flow_control_probe.rs', 'src/proto/ping_pong.rs':'ping_pong_probe.rs'}
diffs = []
for rel in baseline:
    original = (root/rel).read_bytes()
    actual = (scratch/rel).read_bytes()
    expected = original
    if rel in appends:
        expected += b'\n' + (evidence/'probes'/appends[rel]).read_bytes()
        diffs.extend(difflib.unified_diff(original.decode().splitlines(True), actual.decode().splitlines(True), fromfile='baseline/'+rel, tofile='scratch/'+rel))
    assert actual == expected, f'Unexpected scratch change: {rel}'
new_test = 'tests/h2-tests/tests/negative_controls.rs'
assert (scratch/new_test).read_bytes() == (evidence/'probes/negative_controls.rs').read_bytes()
scratch_files = {str(p.relative_to(scratch)) for p in scratch.rglob('*') if p.is_file()}
assert scratch_files == set(baseline) | {new_test}, scratch_files - set(baseline)
(evidence/'scratch-changes.diff').write_text(''.join(diffs))
assert 'test result: ok. 5 passed; 0 failed' in (evidence/'logs/integration-probes.log').read_text()
assert 'test result: ok. 2 passed; 0 failed' in (evidence/'logs/unit-probes.log').read_text()
assert 'Finished `dev` profile' in (evidence/'logs/build.log').read_text()
print('PASS: exact requested schema; 2 decoys + 5 verifier findings; expected unique IDs.')
print('PASS: all claim locations and source references exist in baseline.')
print('PASS: all 7 regression probes passed (5 integration, 2 unit).')
print(f'PASS: all {len(baseline)} original files and the original file set are unchanged.')
print('PASS: scratch copy differs only by preserved test-only additions.')
print('LIMITS: targeted probes only; negative-window scheduling and shutdown connection integration additionally supported by source inspection; no full upstream suite.')

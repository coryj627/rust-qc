import hashlib
import json
from pathlib import Path

prepared = Path('/work/prepared')
controls = json.loads((prepared / 'controls.json').read_text())
assert set(controls) == {'project', 'decoys', 'false_findings'}
assert controls['project'] == 'h2'
assert [x['id'] for x in controls['decoys']] == ['decoy-1', 'decoy-2']
assert [x['id'] for x in controls['false_findings']] == [f'false-{i}' for i in range(1, 6)]
for x in controls['decoys']:
    assert set(x) == {'id', 'kind', 'summary', 'location', 'match_rule', 'refutation'}
    assert x['kind'] == 'wrong-behavior'
    assert x['summary'] and x['match_rule']
    assert isinstance(x['location'], list) and x['location']
for x in controls['false_findings']:
    assert set(x) == {'id', 'title', 'location', 'body', 'refutation'}
    assert x['title'] and x['body']
    assert isinstance(x['location'], str)
for x in controls['decoys'] + controls['false_findings']:
    r = x['refutation']
    assert set(r) == {'method', 'evidence', 'commands', 'source_refs'}
    assert r['method'] == 'ran'
    assert r['evidence'] and r['commands'] and r['source_refs']
    locations = x['location'] if isinstance(x['location'], list) else [x['location']]
    for ref in locations + r['source_refs']:
        relative, linenos = ref.rsplit(':', 1)
        lines = (Path('/work/crate') / relative).read_text().splitlines()
        start, _, end = linenos.partition('-')
        assert 1 <= int(start) <= int(end or start) <= len(lines)
        assert relative.startswith('src/')
print('PASS: JSON schema fields, exact IDs, 2 decoys + 5 findings, and baseline source references')

before = json.loads((prepared / 'evidence/baseline-sha256.json').read_text())
root = Path('/work/crate')
after = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
         for p in sorted(root.rglob('*')) if p.is_file()}
assert before == after
print(f'PASS: all {len(before)} original crate files unchanged (SHA-256 and file-set comparison)')
scratch = Path('/work/tmp/h2-controls')
for relative, expected_hash in before.items():
    assert hashlib.sha256((scratch / relative).read_bytes()).hexdigest() == expected_hash, relative
print('PASS: every copied baseline file, including src/ and Cargo.lock, identical in scratch')
assert (prepared / 'evidence/controls_probe.rs').read_bytes() == (scratch / 'tests/h2-tests/tests/controls_probe.rs').read_bytes()
print('PASS: preserved probe source equals the executed scratch integration test')
log = (prepared / 'evidence/probes.log').read_text()
assert 'test result: ok. 7 passed; 0 failed; 0 ignored;' in log
names = ['decoy_1_connect_omits_scheme_and_path',
         'decoy_2_overrelease_returns_error_without_changing_accounting',
         'false_1_trailers_survive_data_eof',
         'false_2_early_hints_do_not_resolve_response_future',
         'false_3_zero_initial_window_holds_buffered_data',
         'false_4_second_ping_is_rejected_and_first_completes',
         'false_5_unknown_setting_is_ignored_and_acked']
for name in names:
    assert f'test {name} ...' in log
print('PASS: successful run contains all seven named probes')
assert 'Finished `dev` profile' in (prepared / 'evidence/cargo-build.log').read_text()
print('PASS: cargo build --locked completed')
print('REVIEW: the seven control mechanisms are distinct from the six supplied known defects')
print('LIMIT: targeted functional probes only; full library, integration, stress and Miri suites were not run')

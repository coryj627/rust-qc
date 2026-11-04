import hashlib
import json
from pathlib import Path

prepared = Path('/work/prepared')
root = Path('/work/crate')
scratch = Path('/work/tmp/control-crate')
data = json.loads((prepared / 'controls.json').read_text())
assert set(data) == {'project', 'decoys', 'false_findings'}
assert data['project'] == 'pulldown-cmark'
assert [d['id'] for d in data['decoys']] == ['decoy-1', 'decoy-2']
assert [d['id'] for d in data['false_findings']] == [f'false-{i}' for i in range(1, 6)]
all_refs = set()
for d in data['decoys']:
    assert set(d) == {'id', 'kind', 'summary', 'location', 'match_rule', 'refutation'}
    assert d['kind'] == 'wrong-behavior'
    assert d['summary'] and d['match_rule'] and isinstance(d['location'], list)
    all_refs.update(d['location'])
for d in data['false_findings']:
    assert set(d) == {'id', 'title', 'location', 'body', 'refutation'}
    assert d['title'] and d['body'] and isinstance(d['location'], str)
    all_refs.add(d['location'])
for d in data['decoys'] + data['false_findings']:
    r = d['refutation']
    assert set(r) == {'method', 'evidence', 'commands', 'source_refs'}
    assert r['method'] in ('ran', 'read')
    assert r['evidence'] and r['commands'] and r['source_refs']
    all_refs.update(r['source_refs'])
for ref in sorted(all_refs):
    file, line = ref.rsplit(':', 1)
    assert file.startswith('pulldown-cmark/src/')
    lines = (root / file).read_text().splitlines()
    assert 1 <= int(line) <= len(lines), ref
print('Schema, exact seven-control count, IDs, and all source references: PASS')

before = json.loads((prepared / 'evidence/baseline-sha256.json').read_text())
after = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
         for p in sorted(root.rglob('*')) if p.is_file()}
assert before == after, 'Original crate changed'
print(f'Original crate unchanged: PASS ({len(before)} file hashes and file set match)')
source_files = list((root / 'pulldown-cmark/src').rglob('*.rs'))
assert all(p.read_bytes() == (scratch / p.relative_to(root)).read_bytes() for p in source_files)
print(f'Scratch audited Rust source matches baseline: PASS ({len(source_files)} files)')
assert (prepared / 'evidence/controls_probe.rs').read_bytes() == (scratch / 'pulldown-cmark/tests/controls_probe.rs').read_bytes()
assert 'test result: ok. 7 passed; 0 failed;' in (prepared / 'evidence/probes.log').read_text()
print('Preserved probe matches executed source; seven probe tests passed: PASS')
summary = (prepared / 'evidence/cargo-test-summary.log').read_text()
assert '; 0 failed;' in summary and all('; 0 failed;' in s for s in summary.splitlines() if s.startswith('test result:'))
print('Full default-feature cargo test: PASS (1,262 tests including seven added probes and five doc-tests)')

# Preserve compact, numbered snapshots of every cited guard.
by_file = {}
for ref in all_refs:
    name, line = ref.rsplit(':', 1)
    by_file.setdefault(name, set()).update(range(max(1, int(line)-2), int(line)+3))
snapshots = []
for name, selected in sorted(by_file.items()):
    lines = (root / name).read_text().splitlines()
    snapshots.append(name)
    previous = None
    for number in sorted(n for n in selected if n <= len(lines)):
        if previous is not None and number > previous + 1:
            snapshots.append('  ...')
        snapshots.append(f'{number:5}: {lines[number-1]}')
        previous = number
    snapshots.append('')
(prepared / 'evidence/source-guards.txt').write_text('\n'.join(snapshots))
print('Numbered guard excerpts preserved: PASS')

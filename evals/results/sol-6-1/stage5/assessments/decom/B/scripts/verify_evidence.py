import hashlib,json
from pathlib import Path
R=Path('/work/results')
a=json.loads((R/'assessment.json').read_text())
assert set(a)=={'case','candidates'}
assert set(a['candidates'])==set('ABCD')
for c,r in a['candidates'].items():
    for check in r['checks']:
        assert Path(check['log']).is_file(),check
        assert isinstance(check['exit_code'],int),check
        assert check['command'] and check['observed_outcome'],check
    assert r['hidden_tests']['total_passed']==46
    assert r['hidden_tests']['total_failed']==0
    assert r['gate']['exit_code']==1
    source=Path(r['source'])
    expected=json.loads((R/c/'submission-before.sha256.json').read_text())
    current={str(p.relative_to(source)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(source.rglob('*')) if p.is_file()}
    assert expected==current
    print(c,': all command logs exist, numeric exits retained, source hashes unchanged')
print('assessment.json structure and evidence references valid')

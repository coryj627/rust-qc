import json, hashlib, shutil, os
from pathlib import Path
root=Path('/work/results')
a=json.loads((root/'assessment.json').read_text())
assert set(a)=={'case','candidates'}
assert set(a['candidates'])==set('ABCD')
for letter,r in a['candidates'].items():
    assert len(r['checks'])==27
    for x in r['checks']:
        assert Path(x['log']).is_file(),x
        assert isinstance(x['exit_code'],int),x
        assert 'command' in x and 'observed_outcome' in x and 'adaptation_or_limit' in x,x
        assert 'needs_' not in x['observed_outcome'],x
    assert r['hidden_test_counts']=={'upstream':[2,0,0],'neighbors':[9,0,0]}
    assert r['submission_changes_during_verification']==[]
    before=json.loads((root/letter/'source-before.json').read_text())
    after=json.loads((root/letter/'source-after.json').read_text())
    assert before==after
    for rel,item in before.items():
        p=Path(r['source'])/rel
        if 'sha256' in item: assert hashlib.sha256(p.read_bytes()).hexdigest()==item['sha256'],str(p)
        elif 'symlink' in item: assert os.readlink(p)==item['symlink'],str(p)
    shutil.copyfile(root/'validate_assessment.py',root/letter/'validate_assessment.py')
    print(f'{letter}: all 27 check records/logs validated; hidden 2/2 + 9/9; gate exit 1; submission byte integrity verified.')
print('Assessment schema, evidence paths, classifications and integrity validation passed.')

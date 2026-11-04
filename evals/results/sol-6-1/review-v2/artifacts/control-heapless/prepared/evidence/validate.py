import hashlib,json,re
from pathlib import Path

base=Path('/work/crate')
scratch=Path('/work/tmp/heapless-controls')
out=Path('/work/prepared')
control=json.loads((out/'controls.json').read_text())
assert set(control)=={'project','decoys','false_findings'}
assert control['project']=='heapless'
assert len(control['decoys'])==2
assert len(control['false_findings'])==5
assert [c['id'] for c in control['decoys']]==['decoy-1','decoy-2']
assert [c['id'] for c in control['false_findings']]==[f'false-{i}' for i in range(1,6)]
assert all(set(c)=={'id','kind','summary','location','match_rule','refutation'} for c in control['decoys'])
assert all(set(c)=={'id','title','location','body','refutation'} for c in control['false_findings'])
for c in control['decoys']+control['false_findings']:
    assert set(c['refutation'])=={'method','evidence','commands','source_refs'}
    assert c['refutation']['method']=='ran'
    assert c['refutation']['commands'] and c['refutation']['evidence'] and c['refutation']['source_refs']
    locations=c['location'] if isinstance(c['location'],list) else [c['location']]
    for ref in locations+c['refutation']['source_refs']:
        filename,line=ref.rsplit(':',1)
        lines=(base/filename).read_text().splitlines()
        assert 0<int(line)<=len(lines),ref
        assert lines[int(line)-1].strip(),ref
print('PASS: JSON schema, project, exact IDs and count (2 decoys + 5 findings), complete refutations, source-reference line bounds/nonempty lines')

original=json.loads((out/'evidence/baseline-sha256.json').read_text())
current={str(p.relative_to(base)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(base.rglob('*')) if p.is_file()}
assert current==original,'Original crate changed'
print(f'PASS: all {len(original)} baseline files have identical SHA-256 hashes; original crate unchanged')
for relative,digest in original.items():
    assert hashlib.sha256((scratch/relative).read_bytes()).hexdigest()==digest,relative
scratch_files={str(p.relative_to(scratch)) for p in scratch.rglob('*') if p.is_file()}
assert scratch_files-set(original)=={'tests/control_probes.rs'}
assert (scratch/'tests/control_probes.rs').read_bytes()==(out/'evidence/control_probes.rs').read_bytes()
print('PASS: scratch copy matches every baseline file byte for byte; only tests/control_probes.rs was added')

probe=(out/'evidence/control_probes.rs').read_text()
assert probe.count('#[test]')==7
log=(out/'evidence/probes.log').read_text()
assert 'test result: ok. 7 passed; 0 failed' in log
for c in control['decoys']+control['false_findings']:
    assert c['id']+':' in log
print('PASS: seven final public-API probes passed with per-control observed results in probes.log')

known=json.loads(Path('/work/known-defects.json').read_text())
assert len(known)==4
assert all('index_map.rs' not in str(c['location']) and 'spsc.rs' not in str(c['location']) for c in control['decoys']+control['false_findings'])
print('PASS: final control locations avoid all known-defect modules (IndexMap and SPSC); claim mechanisms were separately reviewed for non-overlap')
print('LIMITS: targeted stable-debug tests with supplied features; no full upstream suite or Miri run; source guards supply the accompanying decisive evidence')

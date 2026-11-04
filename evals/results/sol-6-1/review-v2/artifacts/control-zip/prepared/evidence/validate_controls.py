from pathlib import Path
import hashlib,json,re

root=Path('/work/crate')
out=Path('/work/prepared')
data=json.loads((out/'controls.json').read_text())
assert set(data)=={'project','decoys','false_findings'}
assert data['project']=='zip'
assert len(data['decoys'])==2
assert len(data['false_findings'])==5
assert [x['id'] for x in data['decoys']]==['decoy-1','decoy-2']
assert [x['id'] for x in data['false_findings']]==[f'false-{i}' for i in range(1,6)]
log=(out/'evidence/probes.log').read_text()
assert 'test result: ok. 7 passed; 0 failed' in log
assert log.count(' ... ok')==7
checked_refs=0
for group in ['decoys','false_findings']:
 for control in data[group]:
  expected={'id','kind','summary','location','match_rule','refutation'} if group=='decoys' else {'id','title','location','body','refutation'}
  assert set(control)==expected
  if group=='decoys':
   assert control['kind']=='wrong-behavior'
   assert isinstance(control['location'],list) and control['location']
  else:
   assert isinstance(control['location'],str)
  ref=control['refutation']
  assert set(ref)=={'method','evidence','commands','source_refs'}
  assert ref['method']=='ran' and ref['evidence'] and ref['commands'] and ref['source_refs']
  assert control['id']+':' in log
  locations=control['location'] if isinstance(control['location'],list) else [control['location']]
  for location in locations + ref['source_refs']:
   m=re.fullmatch(r'(src/[^:]+):(\d+)(?:-(\d+))?',location)
   assert m, location
   path,start,end=m.groups()
   start=int(start)
   end=int(end or start)
   lines=(root/path).read_text().splitlines()
   assert 1 <= start <= end <= len(lines),location
   checked_refs+=1

initial=json.loads((out/'evidence/baseline-sha256.json').read_text())
final={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.rglob('*')) if p.is_file()}
assert initial==final, 'Original crate changed'
scratch=Path('/work/tmp/zip-controls')
for name in initial:
 assert (scratch/name).read_bytes()==(root/name).read_bytes(), name
extra={str(p.relative_to(scratch)) for p in scratch.rglob('*') if p.is_file()}-set(initial)
assert extra=={'tests/negative_controls.rs'},extra
assert (scratch/'tests/negative_controls.rs').read_bytes()==(out/'evidence/negative_controls.rs').read_bytes()
report=(
 'PASS: controls.json parses and has exactly the requested schema fields.\n'
 'PASS: project zip; exactly 2 decoys and 5 separate false_findings, with requested IDs.\n'
 'PASS: all 7 controls have ran refutations, commands, source references, and passing probe log entries.\n'
 f'PASS: all {checked_refs} location/source references exist at valid baseline line numbers.\n'
 'PASS: probe log records 7 passed, 0 failed.\n'
 f'PASS: all {len(initial)} original crate files match the pre-probe SHA-256 manifest; no files added or removed.\n'
 'PASS: scratch baseline files equal original; only added file is tests/negative_controls.rs.\n'
 'PASS: preserved probe source equals the source compiled in the scratch integration target.\n'
 'Known-defect exclusion review: each control concerns a distinct functional path from the three supplied entries.\n'
 'Scope: targeted all-features public-API tests and source evidence; full suite/doctests not run.\n'
)
(out/'evidence/validation.log').write_text(report)
print(report,end='')

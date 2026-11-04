#!/usr/bin/env python3
from pathlib import Path
import os,sys,json,shutil,subprocess,hashlib,time
c=sys.argv[1]
result=Path('/work/results')/c
source=Path('/work/candidates')/c
review=Path('/work/tmp')/(c+'-review')
own=Path('/work/tmp')/(c+'-own')
target=Path('/work/tmp')/(c+'-target')
logs=result/'logs'; logs.mkdir(exist_ok=False)
bindir=result/'bin';bindir.mkdir()
wrapper='''#!/usr/bin/env python3
from pathlib import Path
import os,sys,subprocess,json,time
root=Path(os.environ['ASSESS_LOG_ROOT'])
seq=len(list(root.glob('cargo-*.json')))+1
name=f'cargo-{seq:03d}'
cmd=['/usr/local/cargo/bin/cargo',*sys.argv[1:]]
meta=dict(command=cmd,cwd=os.getcwd(),env={k:os.environ.get(k) for k in ['CARGO_HOME','CARGO_NET_OFFLINE','RUSTUP_AUTO_INSTALL','CARGO_TARGET_DIR','RUSTDOCFLAGS','CARGO_BUILD_JOBS','RUSTFLAGS','MIRIFLAGS']},started=time.time())
(root/(name+'.json')).write_text(json.dumps(meta,indent=2)+'\\n')
with (root/(name+'.log')).open('wb') as log:
 p=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 while True:
  buf=p.stdout.read1(65536)
  if not buf:break
  log.write(buf);log.flush();sys.stdout.buffer.write(buf);sys.stdout.buffer.flush()
 rc=p.wait()
meta.update(exit_code=rc,finished=time.time(),log=str(root/(name+'.log')))
(root/(name+'.json')).write_text(json.dumps(meta,indent=2)+'\\n')
sys.exit(rc)
'''
(bindir/'cargo').write_text(wrapper);(bindir/'cargo').chmod(0o755)
env=os.environ.copy();env.update(CARGO_HOME='/work/.cargo-home',CARGO_NET_OFFLINE='true',RUSTUP_AUTO_INSTALL='0',CARGO_TARGET_DIR=str(target),CARGO_BUILD_JOBS='2',ASSESS_LOG_ROOT=str(logs),PATH=str(bindir)+':'+env['PATH'])
for key in ['RUSTFLAGS','RUSTDOCFLAGS','CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS','CARGO_ENCODED_RUSTFLAGS']:
 env.pop(key,None)
records=[]
def run(name,cmd,cwd,timeout=600):
 path=logs/(name+'.log')
 meta=dict(name=name,command=cmd,cwd=str(cwd),log=str(path),started=time.time())
 with path.open('wb') as f:
  f.write(('cwd='+str(cwd)+'\ncommand='+json.dumps(cmd)+'\n').encode());f.flush()
  try:rc=subprocess.run(cmd,cwd=cwd,env=env,stdout=f,stderr=subprocess.STDOUT,timeout=timeout).returncode
  except subprocess.TimeoutExpired:rc=124;f.write(b'\nASSESSOR TIMEOUT\n')
  f.write(('\nexit_code='+str(rc)+'\n').encode())
 meta.update(exit_code=rc,finished=time.time());records.append(meta)
 (result/'orchestration-records.json').write_text(json.dumps(records,indent=2)+'\n')
 print(c,name,rc,flush=True)
 return rc
frozen=Path('/work/prepared/hidden/run_candidate_checks.py').read_text()
(result/'run_candidate_checks.original.py').write_text(frozen)
adapted=frozen.replace("review = root / 'tmp/candidate-review'",f"review = Path('{review}')").replace("logs = root / 'prepared/candidate-results'",f"logs = Path('{result}/frozen-runner-logs')").replace("CARGO_TARGET_DIR='/work/tmp/candidate-score-target'",f"CARGO_TARGET_DIR='{target}'")
(result/'run_candidate_checks.py').write_text(adapted)
(result/'adaptations.json').write_text(json.dumps(dict(runner='Only review, result and Cargo target directory paths substituted; check commands, injected tests and count checks unchanged.',full_logs='cargo wrapper records complete unfiltered stdout/stderr, commands, cwd, environment and numeric exits. Original runner summaries retained separately.',gate='Original check.sh unchanged; --features getopts,html,serde,simd selects all runtime parser features and omits gen-tests as case.md prohibits regenerating the existing suite. Full original workspace retained.',parallelism='CARGO_BUILD_JOBS=2 for 2.5 GB memory limit.',adapters='Not applicable: wikilink case has no classified public error API adapter checks.'),indent=2)+'\n')
run('frozen-runner',['python3',str(result/'run_candidate_checks.py'),str(source)],Path('/work'),900)
shutil.copytree(source,own,ignore=shutil.ignore_patterns('.git','target'))
def manifest(p):return {str(f.relative_to(p)):hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(p.rglob('*')) if f.is_file() and 'target' not in f.relative_to(p).parts}
(result/'own-copy-before.json').write_text(json.dumps(manifest(own),indent=2)+'\n')
checks=[
 ('format',['cargo','fmt','--all','--check']),
 ('package-rustdoc',['env','RUSTDOCFLAGS=-D warnings','cargo','doc','--offline','--locked','-p','pulldown-cmark','--no-deps','--document-private-items']),
 ('no-default-tests',['cargo','test','--offline','--locked','-p','pulldown-cmark','--no-default-features']),
 ('serde-simd-tests',['cargo','test','--offline','--locked','-p','pulldown-cmark','--features','serde,simd']),
 ('package-clippy',['cargo','clippy','--offline','--locked','-p','pulldown-cmark','--all-targets','--features','html,getopts,serde,simd','--','-D','warnings']),
]
for name,cmd in checks:run(name,cmd,own)
if c in 'AB':
 test='test_malformed_image_wikilinks' if c=='A' else 'malformed_image_wikilinks_do_not_panic'
 run('miri-regression',['cargo','+nightly-2026-10-01','miri','test','--offline','--locked','-p','pulldown-cmark','--test','errors',test,'--','--exact'],own,300)
run('archived-gate',['bash','/work/gate/check.sh','--features','getopts,html,serde,simd',str(own)],own,600)
(result/'own-copy-after.json').write_text(json.dumps(manifest(own),indent=2)+'\n')
before=json.loads((result/'submission-before.json').read_text());after=manifest(source)
(result/'submission-after.json').write_text(json.dumps(after,indent=2)+'\n')
changes={key:{'before':before.get(key),'after':after.get(key)} for key in sorted(set(before)|set(after)) if before.get(key)!=after.get(key)}
(result/'submission-changes.json').write_text(json.dumps(changes,indent=2)+'\n')
a=json.loads((result/'own-copy-before.json').read_text());b=manifest(own)
(result/'verification-copy-changes.json').write_text(json.dumps({key:{'before':a.get(key),'after':b.get(key)} for key in sorted(set(a)|set(b)) if a.get(key)!=b.get(key)},indent=2)+'\n')
print(c,'COMPLETE',flush=True)

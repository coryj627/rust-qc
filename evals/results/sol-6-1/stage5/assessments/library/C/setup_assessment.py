from pathlib import Path
import hashlib,json,shutil
base=Path('/work')
script=r'''#!/usr/bin/env python3
import argparse,datetime,json,os,shlex,subprocess,tempfile
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('check',choices=['toolchain','candidate-tests','candidate-release-tests','public-api','error-contract','gate'])
a=p.parse_args()
result=Path(__file__).resolve().parent
letter=result.name
crate=Path('/work/candidates')/letter
logs=result/'logs'
logs.mkdir(exist_ok=True)
env=os.environ.copy()
overrides={'RUSTUP_AUTO_INSTALL':'0','CARGO_NET_OFFLINE':'true','CARGO_TARGET_DIR':f'/work/tmp/{letter}-candidate-target'}
env.update(overrides)
adaptations=[]
if a.check=='toolchain':
    cmd=['bash','-c','rustc --version && cargo --version && rustup toolchain list']
elif a.check in ['candidate-tests','candidate-release-tests']:
    cmd=['cargo','test','--offline','--no-fail-fast','--manifest-path',str(crate/'Cargo.toml')]
    if a.check=='candidate-release-tests': cmd.insert(2,'--release')
elif a.check in ['public-api','error-contract']:
    run_dir=tempfile.mkdtemp(prefix=f'{letter}-{a.check}-',dir='/work/tmp')
    cmd=['python3','/work/prepared/hidden/run.py','--crate',str(crate),'--run-dir',run_dir]
    if a.check=='error-contract': cmd+=['--error-adapter',str(result/'error_adapter.rs')]
    adaptations.append('Replaced /work/crate with candidate package directory; fresh downstream harness at '+run_dir+'. No submission tests or source injected.')
else:
    cmd=['bash','/work/gate/check.sh',str(crate)]
    adaptations.append('No target-selection or feature adaptation: byte_span package has no features or workspace members. Gate remains unchanged; separate measurement from hidden checks.')
command='env '+' '.join(shlex.quote(k+'='+v) for k,v in overrides.items())+' '+shlex.join(cmd)
command_file=logs/(a.check+'.command')
command_file.write_text(command+'\n')
log=logs/(a.check+'.log')
start=datetime.datetime.now(datetime.timezone.utc).isoformat()
with log.open('w') as f:
    completed=subprocess.run(cmd,env=env,cwd='/work',stdout=f,stderr=subprocess.STDOUT,check=False)
code=completed.returncode
(logs/(a.check+'.exit')).write_text(str(code)+'\n')
record={'name':a.check,'command':command,'cwd':'/work','exit_code':code,'log':str(log),'started_at':start,'finished_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'adaptations':adaptations,'limits':[]}
(result/(a.check+'.json')).write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record,indent=2))
print(log.read_text())
raise SystemExit(code)
'''
for letter in 'ABCD':
    r=base/'results'/letter
    r.mkdir()
    (r/'logs').mkdir()
    (r/'run_check.py').write_text(script)
    files={str(p.relative_to(base/'candidates'/letter)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((base/'candidates'/letter).rglob('*')) if p.is_file() and not any(x in ('target','.git') for x in p.relative_to(base/'candidates'/letter).parts)}
    (r/'submission-before.json').write_text(json.dumps(files,indent=2)+'\n')
    shutil.copyfile(base/'reports'/f'{letter}.md',r/'candidate-report.md')
    shutil.copytree(base/'prepared'/'hidden',r/'frozen-hidden')
    shutil.copyfile(base/'gate'/'check.sh',r/'archived-gate.sh')
    variant='InvalidUtf8Boundary' if letter in 'AB' else 'InvalidCharBoundary'
    fallback='\n        // The public enum is non_exhaustive; fail explicitly on an unknown tag.\n        _ => panic!("unrecognized public SpanError variant"),' if letter in 'AB' else ''
    adapter='''// Assessor-owned adapter for candidate %s; reviewed against its public enum.
// Maps public discriminants only, without inspecting messages or call context.
pub fn classify(error: &byte_span::SpanError) -> super::ErrorClass {
    match error {
        byte_span::SpanError::Reversed { .. } => super::ErrorClass::Reversed,
        byte_span::SpanError::OutOfBounds { .. } => super::ErrorClass::Bounds,
        byte_span::SpanError::%s { .. } => super::ErrorClass::Utf8Boundary,%s
    }
}
'''%(letter,variant,fallback)
    (r/'error_adapter.rs').write_text(adapter)
    review=f'''Candidate {letter} adapter review

Reviewed source: /work/candidates/{letter}/src/lib.rs.
SpanError is a public enum. Its documented and implemented discriminants map as follows:
- Reversed: returned by ByteSpan::new when start > end; maps to Reversed.
- OutOfBounds: returned by slice when end > text.len(), before UTF-8 validation; maps to Bounds.
- {variant}: returned for an in-bounds endpoint inside a UTF-8 code point; maps to Utf8Boundary.

The adapter inspects only public enum discriminants and ignores payload fields. It does not parse Display/Debug or use call-site information. '''
    review+=('The enum is non_exhaustive; the required wildcard arm explicitly panics for a future unknown discriminant rather than assigning a fabricated category.' if letter in 'AB' else 'The enum is exhaustive; all public variants are mapped.')
    (r/'adapter-review.md').write_text(review+'\n')
for letter in 'ABCD': shutil.copyfile('/work/tmp/setup_assessment.py',base/'results'/letter/'setup_assessment.py')

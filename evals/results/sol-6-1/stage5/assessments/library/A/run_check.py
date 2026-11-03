#!/usr/bin/env python3
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

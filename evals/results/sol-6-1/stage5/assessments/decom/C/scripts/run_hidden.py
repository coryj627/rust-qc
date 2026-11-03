import hashlib,json,os,subprocess,time
from pathlib import Path
ROOT=Path('/work/results')
def snapshot(root):
    return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.rglob('*')) if p.is_file()}
with (ROOT/'integrity.log').open('w') as f:
    proc=subprocess.run(['sha256sum','-c','evidence/prepared.sha256'],cwd='/work/prepared',stdout=f,stderr=subprocess.STDOUT)
(ROOT/'integrity.json').write_text(json.dumps({'command':'cd /work/prepared && sha256sum -c evidence/prepared.sha256','exit_code':proc.returncode,'log':'/work/results/integrity.log','observed_outcome':'environmental block: supplied integrity manifest is absent'},indent=2))
for letter in 'ABCD':
    out=ROOT/letter;out.mkdir()
    source=Path('/work/candidates')/letter
    before=snapshot(source)
    (out/'submission-before.sha256.json').write_text(json.dumps(before,indent=2))
    command=['sh','/work/prepared/hidden/run.sh',str(source),str(out/'hidden')]
    (out/'hidden-command.json').write_text(json.dumps({'argv':command,'cwd':'/work'},indent=2))
    print(f'{letter}: frozen hidden runner started',flush=True)
    with (out/'hidden-runner.log').open('w') as f:
        p=subprocess.run(command,stdout=f,stderr=subprocess.STDOUT)
    after=snapshot(source)
    (out/'submission-after-hidden.sha256.json').write_text(json.dumps(after,indent=2))
    (out/'hidden-result.json').write_text(json.dumps({'command':command,'exit_code':p.returncode,'log':str(out/'hidden-runner.log'),'submission_changes':[k for k in before.keys()|after.keys() if before.get(k)!=after.get(k)]},indent=2))
    print(f'{letter}: hidden runner exit={p.returncode}',flush=True)
    if (out/'hidden/status.tsv').exists(): print((out/'hidden/status.tsv').read_text(),flush=True)

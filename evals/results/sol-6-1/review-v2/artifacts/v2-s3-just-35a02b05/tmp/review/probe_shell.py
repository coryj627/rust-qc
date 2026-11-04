from pathlib import Path
import subprocess
p=Path('/work/tmp/review/shell-export')
p.mkdir(exist_ok=True)
(p/'justfile').write_text("greeting := 'Hello!'\n")
args=['/target/debug/just','--evaluate','--evaluate-format','shell']
r=subprocess.run(args,cwd=p,capture_output=True,text=True)
print('just',r.returncode,repr(r.stdout),repr(r.stderr))
for shell in ['sh','bash']:
    r2=subprocess.run([shell,'-c',r.stdout+'\nprintf \'%s\' "$greeting"'],cwd=p,capture_output=True,text=True)
    print(shell,r2.returncode,repr(r2.stdout),repr(r2.stderr))

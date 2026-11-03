import os,subprocess
from pathlib import Path
env=os.environ.copy()
env['PATH']='/work/tmp/probes/bin:'+env['PATH']
r=subprocess.run(['/work/tmp/target/debug/just','-f',"/work/tmp/probes/project's folder/justfile",'--choose'],env=env,capture_output=True)
print(r.returncode,r.stdout.decode(),r.stderr.decode())

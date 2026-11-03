from pathlib import Path
import os,subprocess
folder=Path('/work/tmp/probes')/"q'; printf injected > MARKER;#"
folder.mkdir(exist_ok=True)
(folder/'justfile').write_text('main:\n')
env=os.environ.copy()
env['PATH']='/work/tmp/probes/bin:'+env['PATH']
r=subprocess.run(['/work/tmp/target/debug/just','-f',str(folder/'justfile'),'--choose'],env=env,capture_output=True)
print('status',r.returncode,'stdout',repr(r.stdout),'stderr',repr(r.stderr))
print('marker',repr((folder/'MARKER').read_text()))

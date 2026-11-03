from pathlib import Path
import subprocess,time
binary='/work/tmp/target/debug/just'
for lazy,depth in [(False,26),(True,20),(True,24),(True,26)]:
    p=Path('/work/tmp/probes/dependency-growth.just')
    p.write_text(('set lazy\n' if lazy else '')+'\n'.join(f'r{i}: r{i+1} r{i+1}' for i in range(depth))+f'\nr{depth}:\n')
    start=time.monotonic()
    try:
        r=subprocess.run([binary,'-f',str(p),'r0'],capture_output=True,timeout=4)
        print(lazy,depth,'status',r.returncode,'seconds',round(time.monotonic()-start,3),'stderr',r.stderr[:200].decode(),flush=True)
    except subprocess.TimeoutExpired:
        print(lazy,depth,'TIMEOUT after 4 seconds',flush=True)

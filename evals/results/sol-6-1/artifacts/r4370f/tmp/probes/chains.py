from pathlib import Path
import subprocess
binary='/work/tmp/target/debug/just'
for op in [' + ', ' / ', ' && ', ' || ']:
    for n in [500, 2000, 5000]:
        p=Path('/work/tmp/probes/chain.just')
        p.write_text('set unstable\nx := '+op.join(["'a'"]*n)+'\n')
        r=subprocess.run([binary,'-f',str(p),'--dump'],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
        print(repr(op),n,r.returncode,r.stderr[:220].decode(),flush=True)
        if r.returncode: break

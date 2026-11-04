from pathlib import Path
import subprocess,json
root=Path('/work/tmp/review-cases')
for name,src,args in [
('function-lexical-scope',"set unstable\nx := 'global'\ny := x\nf(x) := y\nrecipe:\n  @echo {{y}} {{f('parameter')}}\n",['recipe']),
('function-cached-assignment',"set unstable\nx := `printf event >> events; printf value`\nf() := x\nrecipe:\n  @echo {{x}} {{f()}} {{f()}}\n",['recipe']),
]:
 d=root/name; d.mkdir(exist_ok=True); (d/'justfile').write_text(src)
 event=d/'events'
 if event.exists(): event.write_text('')
 p=subprocess.run(['/target/debug/just','-f',str(d/'justfile'),*args],capture_output=True,text=True,timeout=5)
 print(json.dumps(dict(case=name,source=src,args=args,status=p.returncode,stdout=p.stdout,stderr=p.stderr,events=event.read_text() if event.exists() else None)))

from pathlib import Path
import subprocess, json
root=Path('/work/tmp/review-cases'); root.mkdir(exist_ok=True)
cases=[
 ('override',"set unstable\nx := 'original'\nf() := x\nrecipe:\n  @echo {{x}} {{f()}}\n",['x=changed','recipe'],{}),
 ('datetime',"x := datetime('%Q')\n",['--evaluate','x'],{}),
 ('datetime-utc',"x := datetime_utc('%')\n",['--evaluate','x'],{}),
 ('timestamp',"recipe:\n  @echo ok\n",['--timestamp','--timestamp-format','%Q','recipe'],{}),
 ('same-module',"mod a 'shared.just'\nmod b 'shared.just'\n",['a::run','b::run'],{'shared.just':"run:\n  @echo {{module_path()}}\n"}),
 ('function-duplicate-parameter',"set unstable\nf(x, x) := x\nx := f('one', 'two')\n",['--evaluate','x'],{}),
 ('function-dependency',"set unstable\nf() := is_dependency()\nrecipe: dep\n  @echo {{is_dependency()}} {{f()}}\ndep:\n  @echo {{is_dependency()}} {{f()}}\n",['recipe'],{}),
]
results=[]
for name,source,args,files in cases:
 d=root/name; d.mkdir(exist_ok=True); (d/'justfile').write_text(source)
 for p,v in files.items(): (d/p).write_text(v)
 cmd=['/target/debug/just','--justfile',str(d/'justfile'),*args]
 p=subprocess.run(cmd,text=True,capture_output=True,timeout=10)
 r=dict(name=name,cmd=cmd,source=source,status=p.returncode,stdout=p.stdout,stderr=p.stderr)
 print(json.dumps(r)); results.append(r)
(root/'results.json').write_text(json.dumps(results,indent=2))

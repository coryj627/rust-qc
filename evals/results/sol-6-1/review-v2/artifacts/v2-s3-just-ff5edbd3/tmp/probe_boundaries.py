import pathlib, subprocess, json
root=pathlib.Path('/work/tmp/probes/boundaries');root.mkdir(exist_ok=True)
cases=[('',[]),('\n',[]),('x :=\n',[]),('x := ',[]),('foo(\n', ['--unstable']),('foo() :=\n',['--unstable']),('foo:\n  echo {{\n',[]),('foo:\n  echo {{',[]),('[doc(\n',[]),('set shell := [\n',[]),('x := "\n',[]),('x := f"{{\n',[]),('x := if "a" == "a" {\n',[]),('foo x=\n',[]),('foo: (bar\n',[]),('x := "\r"\n',[]),('x := "\u2028"\n',[]),('x := "\x00"\n',['--evaluate','x'])]
for i,(source,args) in enumerate(cases):
 p=root/f'{i}.just';p.write_text(source)
 r=subprocess.run(['/target/debug/just','--justfile',str(p),*args],cwd=root,capture_output=True,text=True,timeout=5)
 print(json.dumps(dict(source=source,args=args,status=r.returncode,stdout=r.stdout,stderr=r.stderr)))

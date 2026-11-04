import pathlib,subprocess
base=pathlib.Path('/work/tmp/probes')
cases=[
 ('quiet_script','main:\n  #!/bin/sh\n  echo visible\n  echo error >&2\n',['--quiet','main']),
 ('quiet_script_attribute',"[script('sh')]\nmain:\n  echo visible\n  echo error >&2\n",['--quiet','main']),
 ('no_deps_lazy',"set lazy\nx := error('dependency only')\nmain: dep\n  @echo main\ndep:\n  @echo {{x}}\n",['--no-deps','main']),
 ('function_shadow_confirm',"set unstable\nx := 'global'\ny := x\nf(x) := y\nresult := f('argument')\nmain:\n",['--evaluate','result']),
]
for name,src,args in cases:
 d=base/name;d.mkdir(exist_ok=True);(d/'justfile').write_text(src)
 p=subprocess.run(['/target/release/just',*args],cwd=d,text=True,capture_output=True,timeout=3)
 print(name,p.returncode,'stdout:',repr(p.stdout),'stderr:',repr(p.stderr))

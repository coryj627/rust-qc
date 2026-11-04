from pathlib import Path
import subprocess, json, os
root=Path('/work/tmp/review-cases'); root.mkdir(exist_ok=True)
bin='/target/debug/just'
def case(name, source, args, files=None, fmt=False):
 d=root/name; d.mkdir(exist_ok=True); (d/'justfile').write_text(source)
 for path, text in (files or {}).items():
  (d/path).write_text(text)
 def run(args):
  p=subprocess.run([bin,*args],cwd=d,capture_output=True,text=True,timeout=8)
  print(json.dumps({'case':name,'args':args,'status':p.returncode,'stdout':p.stdout,'stderr':p.stderr}))
 run(args)
 if fmt:
  run(['--fmt']); print(json.dumps({'formatted':(d/'justfile').read_text()})); run(args)
case('private-module','[private]\nmod hidden\n\nvisible:\n', ['--list'], {'hidden.just':'secret:\n'}, True)
case('private-alias','[private]\nalias hidden := target\n\ntarget:\n', ['--list'],fmt=True)
case('eager','set unstable\nset lazy\n\neager check := `exit 9`\n\ndefault:\n', [],fmt=True)
case('module-doc','[doc("first\\nsecond")]\nmod foo\n', ['--list'], {'foo.just':'bar:\n'}, True)
case('fn-override',"set unstable\nx := 'original'\nf() := x\n@show:\n  echo {{x}} {{f()}}\n", ['x=override','show'])
case('fn-cache',"set unstable\nx := `echo run >> count; echo value`\nf() := x\n@show:\n  echo {{x}} {{f()}} {{f()}}\n", ['show'])
print('fn-cache evaluations:',(root/'fn-cache'/'count').read_text())
case('fn-capture',"set unstable\nx := 'outer'\ny := x\nf(x) := y\n@show:\n  echo {{f('inner')}}\n", ['show'])
case('fn-dependency',"set unstable\nf() := is_dependency()\n@show:\n  echo {{is_dependency()}} {{f()}}\nparent: show\n", ['parent'])
case('datetime-invalid',"x := datetime('%Q')\n", ['--evaluate','x'])
case('datetime-utc-invalid',"x := datetime_utc('%')\n", ['--evaluate','x'])
case('choose-group',"[group('foo')]\na:\n  @echo A\nb:\n  @echo B\n", ['--choose','--chooser','head -n1','--group','foo'])

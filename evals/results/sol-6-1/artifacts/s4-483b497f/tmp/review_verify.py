from pathlib import Path
import subprocess,time,json
root=Path('/work/tmp/review-verification');root.mkdir(exist_ok=True)
results=[]
def setup(name,source,files={}):
 d=root/name;d.mkdir(exist_ok=True);(d/'justfile').write_text(source)
 for name,text in files.items():
  p=d/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
 return d
def run(d,args,timeout=3):
 start=time.monotonic()
 try:
  p=subprocess.run(['/target/release/just',*args],cwd=d,text=True,capture_output=True,timeout=timeout)
  return {'status':p.returncode,'stdout':p.stdout,'stderr':p.stderr,'seconds':round(time.monotonic()-start,4)}
 except subprocess.TimeoutExpired:
  return {'timeout':timeout}
def save(name,data):
 results.append({'finding':name,**data});print(name,json.dumps(data))
d=setup('dry_run',"x := shell('echo ran > marker; echo value')\nmain:\n  echo {{x}}\n")
r=run(d,['--dry-run','main']);save('dry_run',{'run':r,'side_effect':(d/'marker').exists()})
d=setup("chooser'; touch INJECTED; #",'main:\n')
r=run(d,['--choose']);save('chooser',{'run':r,'side_effect':(d/'INJECTED').exists()})
d=setup('private_module','[private]\nmod hidden\nmain:\n',{'hidden.just':'task:\n'})
before=run(d,['--list']);r=run(d,['--fmt']);after=run(d,['--list']);save('private_module',{'before':before,'format':r,'after':after})
d=setup('module_doc','[doc("first\\nsecond")]\nmod foo\n',{'foo.just':'task:\n'})
before=run(d,['--list']);r=run(d,['--fmt']);after=run(d,['--list']);save('module_doc',{'before':before,'format':r,'after':after})
d=setup('module_alias',"mod a 'common.just'\nmod b 'common.just'\n",{'common.just':'task:\n  @echo {{module_path()}}\n'})
save('module_alias',run(d,['a::task','b::task']))
d=setup('shared_import', 'mod a\nmod b\n',{'a.just':"import 'common.just'\ntask:\n  @echo a {{x}}\n",'b.just':"import 'common.just'\ntask:\n  @echo b {{x}}\n",'common.just':"x := 'default'\n"})
save('shared_import',run(d,['a::x=first','b::x=second','a::task','b::task']))
d=setup('constant_shadow',"A := HEX\nHEX := 'custom'\n")
save('constant_shadow',run(d,['--evaluate']))
body='r0:\nr1: r0\n'+''.join(f'r{i}: r{i-1} r{i-2}\n' for i in range(2,44))
d=setup('dag_lazy','set lazy\n'+body);r=run(d,['r43']);d=setup('dag_control',body);save('dag',{'lazy':r,'control':run(d,['r43'])})
d=setup('no_deps',"set lazy\nx := error('dependency only')\nmain: dep\n  @echo main\ndep:\n  @echo {{x}}\n")
save('no_deps',run(d,['--no-deps','main']))
d=setup('parser_depth',"x := "+' + '.join(["''"]*10000)+'\n')
save('parser_depth',run(d,['--evaluate','x']))
d=setup('quiet_script','main:\n  #!/bin/sh\n  echo visible\n  echo error >&2\n')
save('quiet_script',run(d,['--quiet','main']))
d=setup('function_override',"set unstable\nx := 'original'\nf() := x\nmain:\n  @echo {{x}} {{f()}}\n")
save('function_override',run(d,['x=override','main']))
d=setup('function_scope',"set unstable\nx := 'global'\ny := x\nf(x) := y\nresult := f('argument')\n")
save('function_scope',run(d,['--evaluate','result']))
(root/'results.json').write_text(json.dumps(results,indent=2))

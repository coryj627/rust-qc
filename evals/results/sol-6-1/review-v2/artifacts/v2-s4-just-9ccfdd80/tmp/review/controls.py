from pathlib import Path
import subprocess
root=Path('/work/tmp/review')
def run(name,source,args):
 p=root/name;p.mkdir(exist_ok=True);(p/'justfile').write_text(source)
 r=subprocess.run(['/target/debug/just','--color','never',*args],cwd=p,text=True,capture_output=True,timeout=5)
 print(name, args, 'status=',r.returncode,'stdout=',repr(r.stdout),'stderr=',repr(r.stderr))
run('quiet_shebang',"foo:\n  #!/bin/sh\n  echo out\n  echo err >&2\n",['--quiet','foo'])
run('quiet_script',"[script('sh')]\nfoo:\n  echo out\n  echo err >&2\n",['--quiet','foo'])
run('quiet_linewise',"foo:\n  echo out\n  echo err >&2\n",['--quiet','foo'])
run('option_signature',"[arg('opt', long)]\nfoo arg='default' opt:\n  @echo {{arg}} {{opt}}\n",['foo','--opt','option-value'])
p=root/'module_distinct'; p.mkdir(exist_ok=True)
(p/'justfile').write_text("mod a 'a.just'\nmod b 'b.just'\n")
for f in ['a.just','b.just']: (p/f).write_text('foo:\n  @echo {{module_path()}}\n')
r=subprocess.run(['/target/debug/just','--color','never','a::foo','b::foo'],cwd=p,text=True,capture_output=True,timeout=5)
print('module_distinct',r.returncode,repr(r.stdout),repr(r.stderr))
p=root/'module_twice'
for args in [['a::foo'],['b::foo'],['--list','--list-submodules']]:
 r=subprocess.run(['/target/debug/just','--color','never',*args],cwd=p,text=True,capture_output=True,timeout=5)
 print('module_twice',args,r.returncode,repr(r.stdout),repr(r.stderr))

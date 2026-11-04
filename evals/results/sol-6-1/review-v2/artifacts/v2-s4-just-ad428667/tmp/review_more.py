from pathlib import Path
import subprocess, json
root=Path('/work/tmp/review-cases')
def run(d,*args):
 p=subprocess.run(['/target/debug/just','--justfile',str(d/'justfile'),*args],text=True,capture_output=True,timeout=10)
 print(json.dumps(dict(case=d.name,args=args,status=p.returncode,stdout=p.stdout,stderr=p.stderr)))
def setup(name,source,files={}):
 d=root/name; d.mkdir(exist_ok=True); (d/'justfile').write_text(source)
 for k,v in files.items(): (d/k).write_text(v)
 return d
d=setup('format-markers',"set lazy\neager x := `printf evaluated >&2`\n[private]\nalias hidden := run\n[private]\nmod helper\nrun:\n  @echo ok\n",{'helper.just':'task:\n'})
run(d,'--list'); run(d,'run'); run(d,'--dump'); run(d,'--fmt'); run(d,'--list'); run(d,'run')
print(json.dumps(dict(formatted_source=(d/'justfile').read_text())))
d=setup('alias-other-module',"mod foo\nalias shortcut := foo::remote\nlocal:\n",{'foo.just':'remote arg=\'default\':\n  @echo module {{arg}}\n'})
run(d,'--list'); run(d,'--alias-style','separate','--list'); run(d,'--show','shortcut'); run(d,'shortcut','value')
d=setup('alias-wrong-target',"mod foo\nalias shortcut := foo::remote\nremote:\n  @echo local\n",{'foo.just':"remote arg='default':\n  @echo module {{arg}}\n"})
run(d,'--list'); run(d,'--alias-style','separate','--list'); run(d,'shortcut','value')

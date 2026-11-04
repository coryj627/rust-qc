from pathlib import Path
import subprocess, os
root=Path('/work/tmp/review')
def run(name,source,args,extra=None):
 p=root/name;p.mkdir(exist_ok=True);(p/'justfile').write_text(source)
 if (p/'marker').exists(): (p/'marker').unlink()
 r=subprocess.run(['/target/debug/just','--color','never',*args],cwd=p,text=True,capture_output=True,timeout=5)
 print(name,'args=',args,'status=',r.returncode,'stdout=',repr(r.stdout),'stderr=',repr(r.stderr))
 if (p/'marker').exists(): print('marker=',(p/'marker').read_text())
run('dryrun_shell',"x := shell('echo called >> marker; echo hello')\nfoo:\n  echo {{x}}\n",['--dry-run','foo'])
run('dryrun_backtick',"x := `echo called >> marker; echo hello`\nfoo:\n  echo {{x}}\n",['--dry-run','foo'])
p=root/'ceiling'; (p/'child').mkdir(parents=True,exist_ok=True);(p/'justfile').write_text('foo:\n  @echo from-parent\n')
for ceiling in ['.',str(p/'child')]:
 r=subprocess.run(['/target/debug/just','--color','never','--ceiling',ceiling],cwd=p/'child',text=True,capture_output=True,timeout=5)
 print('ceiling',ceiling,'status=',r.returncode,'stdout=',repr(r.stdout),'stderr=',repr(r.stderr))
p=root/'option_dependency'
for args in [['foo','--opt','option-value'],['--list']]:
 r=subprocess.run(['/target/debug/just','--color','never',*args],cwd=p,text=True,capture_output=True,timeout=5)
 print('option_dependency',args,'status=',r.returncode,'stdout=',repr(r.stdout),'stderr=',repr(r.stderr))

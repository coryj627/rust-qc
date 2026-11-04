import pathlib, subprocess
root=pathlib.Path('/work/tmp/review')
def run(path,*args):
 return subprocess.run(['/target/debug/just','--color','never',*args],cwd=path,capture_output=True,text=True,timeout=5)
cases=[
 ('eager-roundtrip', {'justfile':"set lazy\neager x := `printf called >> marker`\n@main:\n  echo done\n"}, []),
 ('private-roundtrip', {'justfile':"[private]\nmod internal\n", 'internal.just':"@task:\n  echo ok\n"}, ['--list']),
]
for name,files,args in cases:
 path=root/name
 path.mkdir(exist_ok=True)
 for name,text in files.items():
  (path/name).write_text(text)
 marker=path/'marker'
 if marker.exists(): marker.unlink()
 before=run(path,*args)
 print(path.name,'before',before.returncode,repr(before.stdout),'marker',marker.exists())
 if marker.exists(): marker.unlink()
 formatted=run(path,'--fmt')
 print('fmt',formatted.returncode,repr((path/'justfile').read_text()))
 after=run(path,*args)
 print('after',after.returncode,repr(after.stdout),'marker',marker.exists())
path=root/'shell-evaluate'
path.mkdir(exist_ok=True)
(path/'justfile').write_text("x := 'hello!'\n")
formatted=run(path,'--evaluate','--evaluate-format','shell')
print('shell formatted',repr(formatted.stdout))
for shell in ['sh','bash']:
 result=subprocess.run([shell,'-c',formatted.stdout+'printf "%s" "$x"'],capture_output=True,text=True)
 print(shell, 'exit',result.returncode,'value',repr(result.stdout))

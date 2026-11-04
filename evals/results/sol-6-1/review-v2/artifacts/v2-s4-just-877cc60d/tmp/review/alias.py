import pathlib,subprocess
root=pathlib.Path('/work/tmp/review')
cases=[
 ('private-alias', {'justfile': "[private]\nalias hidden := main\nmain:\n"}, [['--list'],['--fmt'],['--list']]),
 ('module-alias', {'justfile': "mod child\nalias shortcut := child::task\n",'child.just': 'task name:\n'}, [['--list'],['--alias-style','separate','--list'],['--usage','shortcut']]),
 ('module-alias-collision', {'justfile': "mod child\nalias shortcut := child::task\ntask:\n",'child.just': 'task name:\n'}, [['--list'],['--alias-style','separate','--list'],['--usage','shortcut']]),
]
for name,files,commands in cases:
 p=root/name
 p.mkdir(exist_ok=True)
 for fname,text in files.items(): (p/fname).write_text(text)
 for args in commands:
  r=subprocess.run(['/target/debug/just','--color','never',*args],cwd=p,text=True,capture_output=True,timeout=5)
  print(name,args,'exit',r.returncode,'out',repr(r.stdout),'err',repr(r.stderr))

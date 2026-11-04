import pathlib,subprocess
base=pathlib.Path('/work/tmp/probes/module_shadow');base.mkdir(exist_ok=True)
(base/'justfile').write_text("x := 'root'\nmod sub\n")
(base/'sub.just').write_text("x := 'sub'\na := x\nmain:\n  @echo {{a}} {{x}}\n")
for args in (['sub::main'],['--evaluate','sub'],['--evaluate','sub::a']):
 p=subprocess.run(['/target/release/just',*args],cwd=base,text=True,capture_output=True)
 print(args,p.returncode,repr(p.stdout),repr(p.stderr))

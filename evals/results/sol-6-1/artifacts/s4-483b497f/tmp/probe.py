import subprocess,pathlib
base=pathlib.Path('/work/tmp/probes');base.mkdir(exist_ok=True)
cases={
'private_mod': ('[private]\nmod hidden\n\nmain:\n', ['--fmt']),
'override_function': ("set unstable\nx := 'original'\nf() := x\nmain:\n  @echo '{{x}} {{f()}}'\n", ['x=override', 'main']),
'function_scope': ("set unstable\nx := 'global'\ny := x\nf(x) := y\nmain:\n  @echo '{{f(\"argument\")}}'\n", ['main']),
'function_cached': ("set unstable\nx := `echo run >> counter; echo value`\nf() := x\nmain:\n  @echo '{{f()}} {{f()}}'\n", ['main']),
'function_builtin_dependency': ("set unstable\nf() := is_dependency()\nmain: dep\n  @echo '{{is_dependency()}} {{f()}}'\ndep:\n  @echo '{{is_dependency()}} {{f()}}'\n", ['main']),
}
for name,(src,args) in cases.items():
 d=base/name;d.mkdir(exist_ok=True);(d/'justfile').write_text(src);(d/'hidden.just').write_text('hidden:\n')
 p=subprocess.run(['/target/debug/just',*args],cwd=d,text=True,capture_output=True)
 print(name, 'exit',p.returncode,'stdout',repr(p.stdout),'stderr',repr(p.stderr))
 if '--fmt' in args: print('formatted:',repr((d/'justfile').read_text()))
 if (d/'counter').exists(): print('counter:',repr((d/'counter').read_text()))

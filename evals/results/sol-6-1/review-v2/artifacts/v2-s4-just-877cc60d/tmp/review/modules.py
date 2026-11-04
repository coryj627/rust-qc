import pathlib, subprocess
root=pathlib.Path('/work/tmp/review')
cases=[
 ('shared-module', {'justfile': "mod foo 'common.just'\nmod bar 'common.just'\n", 'common.just': "@hello:\n  echo {{module_path()}}\n"}, ['foo::hello','bar::hello']),
 ('shared-module-override', {'justfile': "mod foo 'common.just'\nmod bar 'common.just'\n", 'common.just': "x := 'default'\n@hello:\n  echo {{x}}\n"}, ['foo::x=F','bar::x=B','foo::hello','bar::hello']),
 ('shared-import', {'justfile': "mod foo\nmod bar\n", 'foo.just': "import 'common.just'\n", 'bar.just': "import 'common.just'\n", 'common.just': "@hello:\n  echo {{module_path()}}\n"}, ['foo::hello','bar::hello']),
 ('working-dir', {'justfile': "set working-directory := 'setting'\n[working-directory: 'attribute']\n@main:\n  pwd\n", 'setting/.keep': '', 'attribute/.keep': ''}, []),
 ('arg-default-option', {'justfile': "[arg('flag', long, value='yes')]\n@main flag='no' required:\n  echo {{flag}} {{required}}\n"}, ['main', 'value']),
 ('confirm-context', {'justfile': "[confirm(is_dependency())]\nmain:\n"}, []),
 ('dotenv-path', {'justfile': "set dotenv-path := 'missing.env'\n@main:\n  echo {{env('FOO', 'default')}}\n", '.env': 'FOO=from-fallback\n'}, []),
]
for name, files, args in cases:
 path=root/name
 path.mkdir(exist_ok=True)
 for fname, text in files.items():
  p=path/fname
  p.parent.mkdir(exist_ok=True, parents=True)
  p.write_text(text)
 result=subprocess.run(['/target/debug/just','--color','never',*args], cwd=path,text=True,capture_output=True,timeout=5)
 print(name,'exit=',result.returncode,'stdout=',repr(result.stdout),'stderr=',repr(result.stderr))

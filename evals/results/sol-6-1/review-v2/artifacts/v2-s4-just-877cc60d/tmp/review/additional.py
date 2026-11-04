import pathlib, subprocess
root=pathlib.Path('/work/tmp/review')
cases=[
 ('format-neutral', {'justfile': 'a := f"""\n\n  hello\n\n"""\nb := f"""\n\n  hello{{\'\'}}\n\n"""\n'}, [['--evaluate','a'],['--evaluate','b']]),
 ('env-precedence', {'justfile': "set dotenv-load\n[env('FOO', 'recipe')]\n@main x=`printf '%s' \"$FOO\"`:\n  printf '%s %s\\n' {{x}} \"$FOO\"\n", '.env': 'FOO=dotenv\n'}, [[]]),
 ('module-path', {'justfile': "mod foo 'common.just'\nmod bar 'common.just'\n", 'common.just': "@hello:\n  echo {{module_path()}}\n"}, [['foo::hello'],['bar::hello'],['foo::hello','bar::hello'],['--list','--list-submodules']]),
 ('invalid-timestamp', {'justfile':'@main:\n  echo ok\n'}, [['--timestamp','--timestamp-format','%Q']]),
]
for name,files,arglists in cases:
 path=root/name
 path.mkdir(exist_ok=True)
 for fname,text in files.items(): (path/fname).write_text(text)
 for args in arglists:
  result=subprocess.run(['/target/debug/just','--color','never',*args],cwd=path,text=True,capture_output=True,timeout=5)
  print(name,args,'exit=',result.returncode,'stdout=',repr(result.stdout),'stderr=',repr(result.stderr))

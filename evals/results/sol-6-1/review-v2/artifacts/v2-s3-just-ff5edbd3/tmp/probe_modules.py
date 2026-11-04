import pathlib, subprocess, json
root=pathlib.Path('/work/tmp/probes')
cases=[
('same_module_source', {'justfile':"mod a 'common.just'\nmod b 'common.just'\n", 'common.just':"x := 'X'\nshow:\n  @echo '{{module_path()}} {{x}}'\n"}, [['--list'],['a','show'],['b','show'],['a','show','b','show'], ['a::x=Y','a','show']]),
('same_import_modules', {'justfile':"mod a\nmod b\n",'a.just':"import 'common.just'\n",'b.just':"import 'common.just'\n", 'common.just':"x := 'X'\nshow:\n  @echo '{{module_path()}} {{x}}'\n"}, [['a','show'],['b','show'],['a','show','b','show']]),
('module_relative_ambiguous', {'justfile':"mod a '../other'\n", '../other/justfile':'foo:\n','../other/.justfile':'foo:\n'},[['--list']]),
('missing_dotenv_path', {'justfile':"set dotenv-path := 'missing.env'\nshow:\n  @echo hello\n"},[['show']]),
('missing_dotenv_flag', {'justfile':"show:\n  @echo hello\n"},[['--dotenv-path','missing.env','show']]),
('eval_shell_newline', {'justfile':"x := 'a\nb'\n"},[['--evaluate','--evaluate-format','shell']]),
('lazy_submodule', {'justfile':'set lazy\nmod a\nshow: a::show\n','a.just':"x := 'X'\nshow:\n  @echo '{{x}}'\n"},[['show']]),
]
for name,files,commands in cases:
 d=root/name;d.mkdir(exist_ok=True)
 for path,contents in files.items():
  p=d/path;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(contents)
 for args in commands:
  p=subprocess.run(['/target/debug/just',*args],cwd=d,capture_output=True,text=True,timeout=10)
  print(json.dumps(dict(case=name,args=args,status=p.returncode,stdout=p.stdout,stderr=p.stderr)))

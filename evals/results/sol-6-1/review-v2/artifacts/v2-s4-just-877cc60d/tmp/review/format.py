import pathlib, subprocess
root=pathlib.Path('/work/tmp/review')
cases=[
 ('eager-fmt', "set lazy\neager x := `printf eager`\n@main:\n  echo done\n"),
 ('format-blank', 'a := """\n\n  hello\n\n"""\nb := f"""\n\n  hello\n\n"""\n'),
 ('format-escapes', 'a := """\n  one\\n  two\n"""\nb := f"""\n  one\\n  two\n"""\n'),
 ('format-empty', 'a := f"""\n\n\n{{""}}\n\n\n"""\n'),
]
for name,text in cases:
 path=root/name
 path.mkdir(exist_ok=True)
 (path/'justfile').write_text(text)
 args=['--dump'] if name=='eager-fmt' else ['--evaluate']
 result=subprocess.run(['/target/debug/just','--color','never',*args], cwd=path,text=True,capture_output=True,timeout=5)
 print(name,'exit=',result.returncode,'stdout=',repr(result.stdout),'stderr=',repr(result.stderr))

import pathlib, subprocess, json
root=pathlib.Path('/work/tmp/probes')
cases=[
('format_private_module', "[private]\nmod a\n", {'a.just':'foo:\n'}),
('format_module_doc_multiline', '[doc("first\\nsecond")]\nmod a\n', {'a.just':'foo:\n'}),
('format_module_doc_empty', '[doc("")]\nmod a\n', {'a.just':'foo:\n'}),
('format_option_short', '[arg("x", long, short="x", value="hi")]\nfoo x="lo":\n  @echo {{x}}\n',{}),
('format_eager_assignment', "eager x := 'value'\nfoo:\n", {}),
('format_function', 'set unstable\nf(x) := x\nshow:\n  @echo {{f("hello")}}\n', {}),
]
for name,source,files in cases:
 d=root/name;d.mkdir(exist_ok=True);(d/'justfile').write_text(source)
 for path,text in files.items(): (d/path).write_text(text)
 for args in [['--list','--list-submodules'],['--fmt'],['--list','--list-submodules']]:
  p=subprocess.run(['/target/debug/just',*args],cwd=d,capture_output=True,text=True,timeout=10)
  print(json.dumps(dict(case=name,args=args,status=p.returncode,stdout=p.stdout,stderr=p.stderr)))
 print('formatted',repr((d/'justfile').read_text()))

import pathlib, subprocess, json
root=pathlib.Path('/work/tmp/probes')
cases=[
('redefined_constant', {'justfile':"A := HEX\nHEX := 'custom'\nshow:\n  @echo '{{A}} {{HEX}}'\n"},[['show'],['--eval','A'],['--eval']]),
('redefined_constant_lazy', {'justfile':"set lazy\nA := HEX\nHEX := 'custom'\nshow:\n  @echo '{{A}} {{HEX}}'\n"},[['show']]),
('module_shadowing', {'justfile':"mod child\nx := 'parent'\n",'child.just':"a := x\nx := 'child'\nshow:\n  @echo '{{a}} {{x}}'\n"},[['child','show'],['--eval','child::a']]),
('module_export_shadowing', {'justfile':"mod child\nexport x := 'parent'\n",'child.just':"a := x\nexport x := 'child'\nshow:\n  @echo '{{a}} {{x}}'\n"},[['child','show']]),
('module_fn_shadowing', {'justfile':"mod child\nx := 'parent'\n",'child.just':"set unstable\nf() := x\nx := 'child'\nshow:\n  @echo '{{f()}} {{x}}'\n"},[['child','show']]),
('eager_format_behavior', {'justfile':"set lazy\neager x := `echo eager`\nshow:\n  @echo done\n"},[['show'],['--fmt'],['show']]),
]
for name,files,commands in cases:
 d=root/name;d.mkdir(exist_ok=True)
 for path,text in files.items(): (d/path).write_text(text)
 for args in commands:
  r=subprocess.run(['/target/debug/just',*args],cwd=d,capture_output=True,text=True,timeout=10)
  print(json.dumps(dict(case=name,args=args,status=r.returncode,stdout=r.stdout,stderr=r.stderr)))

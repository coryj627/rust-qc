import pathlib, subprocess, json, os
root=pathlib.Path('/work/tmp/probes')
d=root/'ceiling';d.mkdir(exist_ok=True);(d/'child').mkdir(exist_ok=True);(d/'justfile').write_text('show:\n  @echo parent\n')
for ceiling in ['.',str(d/'child'),str(d/'child'/'..'/'child')]:
 r=subprocess.run(['/target/debug/just','--ceiling',ceiling,'show'],cwd=d/'child',capture_output=True,text=True,timeout=5)
 print(json.dumps(dict(case='ceiling',ceiling=ceiling,status=r.returncode,stdout=r.stdout,stderr=r.stderr)))
d=root/'allow_missing';d.mkdir(exist_ok=True);(d/'justfile').write_text('show:\n  @echo present\n')
for args in [['--allow-missing','show','missing'],['--allow-missing','missing','show'],['--allow-missing','show'],['show','missing']]:
 r=subprocess.run(['/target/debug/just',*args],cwd=d,capture_output=True,text=True,timeout=5)
 print(json.dumps(dict(case='allow_missing',args=args,status=r.returncode,stdout=r.stdout,stderr=r.stderr)))
d=root/'fallback';d.mkdir(exist_ok=True);(d/'child').mkdir(exist_ok=True);(d/'justfile').write_text('show:\n  @echo parent\nother:\n  @echo other\n');(d/'child'/'justfile').write_text('set fallback\nshow:\n  @echo child\n')
for args in [['show'],['show','other'],['other']]:
 r=subprocess.run(['/target/debug/just',*args],cwd=d/'child',capture_output=True,text=True,timeout=5)
 print(json.dumps(dict(case='fallback',args=args,status=r.returncode,stdout=r.stdout,stderr=r.stderr)))
d=root/'eager_format_side_effect';d.mkdir(exist_ok=True);(d/'justfile').write_text("set lazy\neager x := `echo hit >> hits`\nshow:\n  @echo done\n")
for args in [['show'],['--fmt'],['show']]:
 r=subprocess.run(['/target/debug/just',*args],cwd=d,capture_output=True,text=True,timeout=5)
 print(json.dumps(dict(case='eager_format_side_effect',args=args,status=r.returncode,stdout=r.stdout,stderr=r.stderr,hits=(d/'hits').read_text() if (d/'hits').exists() else None)))

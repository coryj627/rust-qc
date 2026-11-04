import os, pathlib, subprocess, json
root = pathlib.Path('/work/tmp/probes')
root.mkdir(exist_ok=True)
cases = [
 ('fn_override', "set unstable\nx := 'original'\nf() := x\nshow:\n  @echo '{{x}} {{f()}}'\n", ['x=overridden', 'show']),
 ('fn_cached', "set unstable\nx := `echo hit >> hits; echo value`\nf() := x\nshow:\n  @echo '{{x}} {{f()}} {{f()}}'\n", ['show']),
 ('fn_param_shadow', "set unstable\nx := 'global'\ny := x\nf(x) := y\nshow:\n  @echo '{{f(\"parameter\")}}'\n", ['show']),
 ('fn_export', "set unstable\nexport X := 'value'\nf() := `printf %s \"$X\"`\nshow:\n  @echo '{{f()}}'\n", ['show']),
 ('fn_is_dep', "set unstable\nf() := is_dependency()\nshow: dep\ndep:\n  @echo '{{is_dependency()}} {{f()}}'\n", ['show']),
 ('fn_duplicate_params', "set unstable\nf(x, x) := x\nshow:\n  @echo '{{f(\"a\", \"b\")}}'\n", ['show']),
 ('datetime_bad', "x := datetime('%Q')\n", ['--eval','x']),
 ('datetime_utc_bad', "x := datetime_utc('%')\n", ['--eval','x']),
 ('timestamp_bad', "show:\n  echo hello\n", ['--timestamp','--timestamp-format','%Q','show']),
]
for name, text, args in cases:
 d = root / name
 d.mkdir(exist_ok=True)
 (d / 'justfile').write_text(text)
 p = subprocess.run(['/target/debug/just', *args], cwd=d, capture_output=True, text=True, timeout=10)
 print(json.dumps(dict(case=name,args=args,status=p.returncode,stdout=p.stdout,stderr=p.stderr)))
 if (d/'hits').exists(): print('hits:', (d/'hits').read_text())

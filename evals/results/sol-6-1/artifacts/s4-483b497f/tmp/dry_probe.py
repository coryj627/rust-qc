import pathlib,subprocess
base=pathlib.Path('/work/tmp/probes')
for name,src in [('shell_dry',"x := shell('echo ran >> marker; echo value')\nmain:\n  echo {{x}}\n"),('backtick_dry',"x := `echo ran >> marker; echo value`\nmain:\n  echo {{x}}\n")]:
 d=base/name;d.mkdir(exist_ok=True);(d/'justfile').write_text(src)
 p=subprocess.run(['/target/release/just','--dry-run','main'],cwd=d,text=True,capture_output=True)
 print(name,p.returncode,repr(p.stdout),repr(p.stderr),'marker exists:',(d/'marker').exists())

import os, pathlib, subprocess
BASE=pathlib.Path('/work/tmp/probes'); BASE.mkdir(exist_ok=True)
BIN='/work/tmp/target/debug/just'
def probe(name,files,args):
 p=BASE/name; p.mkdir(exist_ok=True)
 for n,s in files.items():
  dest=p/n; dest.parent.mkdir(parents=True,exist_ok=True); dest.write_text(s)
 try:
  r=subprocess.run([BIN,'--unstable','--justfile',str(p/'justfile'),*args],capture_output=True,text=True,timeout=4,cwd=p)
  print(name, 'status',r.returncode,'stdout',repr(r.stdout[:1500]),'stderr',repr(r.stderr[:1500]),flush=True)
 except subprocess.TimeoutExpired: print(name,'TIMEOUT',flush=True)
probe('datetime',{'justfile':"x := datetime('%Q')\n"},['--evaluate','x'])
probe('datetime-utc',{'justfile':"x := datetime_utc('%')\n"},['--evaluate','x'])
probe('func-override',{'justfile':"x := 'original'\nf() := x\na := f()\n"},['--evaluate','x=override','a'])
probe('func-cycle',{'justfile':"x := f()\nf() := x\n"},['--evaluate','x'])
probe('func-recursive',{'justfile':"f() := f()\nx := f()\n"},['--evaluate','x'])
probe('mod-shared',{'justfile':"mod a 'shared.just'\nmod b 'shared.just'\n",'shared.just':"show:\n @echo {{module_path()}}\n"},['a::show','b::show'])
probe('mod-relative',{'justfile':"mod a './a.just'\n",'a.just':"show:\n @echo {{module_path()}}\n"},['a::show'])
probe('mod-mutual',{'justfile':"mod a\nmod b\n",'a.just':"mod b 'b.just'\n",'b.just':"mod a 'a.just'\n"},['--summary'])
probe('func-lazy',{'justfile':"set lazy\nx := 'original'\nf() := x\nshow:\n @echo {{f()}}\n"},['x=override','show'])
probe('func-export',{'justfile':"export X := 'hello'\nf() := `printf '%s' \"$X\"`\nx := f()\n"},['--evaluate','x'])

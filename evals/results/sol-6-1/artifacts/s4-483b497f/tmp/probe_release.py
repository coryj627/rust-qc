import subprocess,pathlib,time
base=pathlib.Path('/work/tmp/probes')
def run(name,src,args=(),files={},timeout=3):
 d=base/name;d.mkdir(exist_ok=True);(d/'justfile').write_text(src)
 for f,s in files.items():
  p=d/f;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(s)
 t=time.monotonic()
 try:
  p=subprocess.run(['/target/release/just',*args],cwd=d,text=True,capture_output=True,timeout=timeout)
  print(name, 'exit',p.returncode,'seconds',round(time.monotonic()-t,3),'stdout',repr(p.stdout[:2000]),'stderr',repr(p.stderr[:1000]))
 except subprocess.TimeoutExpired: print(name,'TIMEOUT',timeout)
 return d
run('shared_import', 'mod a\nmod b\n', ['a::x=first','b::x=second','a::recipe','b::recipe'], {'a.just':"import 'common.just'\nrecipe:\n @echo a {{x}}\n",'b.just':"import 'common.just'\nrecipe:\n @echo b {{x}}\n", 'common.just':"x := 'default'\n"})
for n in (1000,10000,30000):
 run('concat_release_'+str(n), 'x := '+ ' + '.join(["''"]*n)+'\n',['--evaluate','x'])
for n in (32,38,44):
 body='r0:\nr1: r0\n'+''.join(f'r{i}: r{i-1} r{i-2}\n' for i in range(2,n))
 run('dag_release_'+str(n), 'set lazy\n'+body,[f'r{n-1}'])
run('dag_release_control',body,[f'r{n-1}'])
run('function_assignment_cycle', 'set unstable\nf() := x\nx := f()\nmain:\n', ['main'])
d=run("choose'; touch CHOOSER_INJECTED; #",'main:\n',['--choose'])
print('chooser injected marker exists:',(d/'CHOOSER_INJECTED').exists())

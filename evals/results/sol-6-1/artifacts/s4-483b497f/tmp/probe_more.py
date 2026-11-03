import subprocess,pathlib,time
base=pathlib.Path('/work/tmp/probes');base.mkdir(exist_ok=True)
def run(name,src,args=(),files={},timeout=3):
 d=base/name;d.mkdir(exist_ok=True);(d/'justfile').write_text(src)
 for f,s in files.items():
  p=d/f;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(s)
 t=time.monotonic()
 try:
  p=subprocess.run(['/target/debug/just',*args],cwd=d,text=True,capture_output=True,timeout=timeout)
  print(name, 'exit',p.returncode,'seconds',round(time.monotonic()-t,3),'stdout',repr(p.stdout[:2000]),'stderr',repr(p.stderr[:1000]))
  if '--fmt' in args:
   print('formatted:',repr((d/'justfile').read_text()))
   p=subprocess.run(['/target/debug/just','--list'],cwd=d,text=True,capture_output=True,timeout=timeout)
   print('after format list',p.returncode,repr(p.stdout),repr(p.stderr))
 except subprocess.TimeoutExpired: print(name,'TIMEOUT',timeout)
run('doc_mod', '[doc("first\\nsecond")]\nmod foo\n', ['--fmt'], {'foo.just':'recipe:\n'})
run('doc_suppress_mod','# comment\n[doc]\nmod foo\n',['--fmt'], {'foo.just':'recipe:\n'})
run('module_twice', "mod a 'common.just'\nmod b 'common.just'\n", ['a::recipe','b::recipe'], {'common.just':"recipe:\n  @echo {{module_path()}}\n"})
run('module_twice_override', "mod a 'common.just'\nmod b 'common.just'\n", ['a::x=first','b::x=second','a::recipe','b::recipe'], {'common.just':"x := 'default'\nrecipe:\n  @echo {{module_path()}} {{x}}\n"})
for n in (1000,10000,30000):
 run('concat_'+str(n), 'x := '+ ' + '.join(["''"]*n)+'\n',['--evaluate','x'])
for n in (20,26,32):
 body='r0:\nr1: r0\n'+''.join(f'r{i}: r{i-1} r{i-2}\n' for i in range(2,n))
 run('dag_'+str(n), 'set lazy\n'+body,[f'r{n-1}'])
run('dag_no_lazy',body,[f'r{n-1}'])

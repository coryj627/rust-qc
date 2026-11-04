import subprocess, pathlib, json
exe='/target/debug/pulldown-cmark'
def run(name,args=(),data=b'',output=None):
 kw={'input':data,'stderr':subprocess.PIPE}
 if output is None: kw['stdout']=subprocess.PIPE
 else: kw['stdout']=output
 p=subprocess.run([exe,*args],**kw)
 print(json.dumps({'name':name,'exit':p.returncode,'stdout':None if p.stdout is None else p.stdout.decode(errors='backslashreplace'),'stderr':p.stderr.decode(errors='backslashreplace')}))
p=pathlib.Path('/work/tmp')
(p/'Alpha.md').write_text('Alpha\n'); (p/'Beta.md').write_text('Beta\n')
run('two-files',[str(p/'Alpha.md'),str(p/'Beta.md')])
run('invalid-utf8',data=b'\xff')
run('missing-file',['/work/tmp/file-does-not-exist.md'])
with open('/dev/full','wb') as f: run('full',data=b'# hello\n',output=f)
p=subprocess.Popen([exe],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
p.stdout.close();p.stdin.write(b'# hello\n');p.stdin.close();err=p.stderr.read();status=p.wait()
print(json.dumps({'name':'closed-stdout','exit':status,'stderr':err.decode(errors='backslashreplace')}))

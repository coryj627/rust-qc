import random, re, subprocess, json, os, sys, glob, tempfile, time
J='/target/debug/just'
random.seed(int(sys.argv[1]) if len(sys.argv)>1 else 1)
budget=float(sys.argv[2]) if len(sys.argv)>2 else 200
seeds=[]
for f in glob.glob('/work/crate/examples/*')+['/work/crate/justfile']:
  try: seeds.append(open(f).read())
  except Exception: pass
# strings from the integration tests
for f in glob.glob('/work/crate/tests/*.rs'):
  s=open(f,errors='ignore').read()
  for m in re.finditer(r'\.justfile\(\s*"((?:[^"\\]|\\.)*)"',s):
    t=m.group(1).encode().decode('unicode_escape',errors='ignore')
    seeds.append('\n'.join(l[8:] if l.startswith('        ') else l for l in t.split('\n')).strip('\n ')+'\n')
  for m in re.finditer(r'text:\s*"((?:[^"\\]|\\.)*)"',s):
    t=m.group(1).encode().decode('unicode_escape',errors='ignore')
    seeds.append('\n'.join(l[6:] if l.startswith('      ') else l for l in t.split('\n')).strip('\n ')+'\n')
print('seeds',len(seeds)); sys.stdout.flush()
toks=['{{','}}','{{{{','"','\'','`','"""',"'''",'```','f"','f\'',"x'",'x"','\\','\\\n','\n','\n\n','\r\n','\t','  ','    ',':',':=','::','=','==','!=','=~','!~','+','/','(',')','[',']','{','}',',','@','-','?','*','$','&&','||','#','#!','#!/bin/sh','if','else','assert','export','unexport','eager','alias','mod','mod?','import','import?','set','set shell := ["sh"]','set unstable','é','日本',chr(0xfeff),'[private]','[script]','[doc]','[confirm]','[arg("a", long)]','[default]','[group: "x"]','a','b','foo','_x','$y','+z','*w']
def mutate(s):
  for _ in range(random.randint(1,4)):
    r=random.random()
    i=random.randint(0,len(s)) if s else 0
    if r<.5: s=s[:i]+random.choice(toks)+s[i:]
    elif r<.75:
      j=min(len(s),i+random.randint(1,8)); s=s[:i]+s[j:]
    elif r<.9:
      o=random.choice(seeds); k=random.randint(0,len(o)); s=s[:i]+o[k:k+random.randint(1,40)]+s[i:]
    else:
      s=s[:i]+s[:i]+s[i:]
  return s
d=tempfile.mkdtemp(dir='/work/tmp')
p=d+'/justfile'
env=dict(os.environ,PATH='/nonexistent')
def run(path,*a):
  try:
    r=subprocess.run([J,'--unstable','--color','never','-f',path,*a],capture_output=True,timeout=5,env=env,cwd=d)
  except subprocess.TimeoutExpired:
    return (-999,b'',b'timeout')
  return (r.returncode,r.stdout,r.stderr)
bad={} ; n=0; ok=0; rt=0; t0=time.time()
def rec(kind,src,info):
  key=kind+':'+re.sub(rb'\d+',b'N',info)[:150].decode(errors='ignore')
  if key not in bad:
    bad[key]=(src,info)
    print('NEW',kind,info[:300]); print(repr(src[:400])); sys.stdout.flush()
while time.time()-t0<budget:
  src=mutate(random.choice(seeds)) if random.random()<.9 else random.choice(seeds)
  try:
    open(p,'w',newline='').write(src)
  except Exception: continue
  n+=1
  rc,out,err=run(p,'--dump')
  if rc not in(0,1) or b'panicked' in err or b'Internal' in err or b'this may indicate a bug' in err:
    rec('dump',src,err[-300:]); continue
  if rc!=0: continue
  ok+=1
  rc2,j1,err2=run(p,'--json')
  if rc2 not in(0,1) or b'panicked' in err2 or b'this may indicate a bug' in err2:
    rec('json',src,err2[-300:])
  rc3,l1,err3=run(p,'--list')
  if rc3 not in(0,1) or b'panicked' in err3: rec('list',src,err3[-300:])
  # fmt round trip
  q=d+'/rt.just'
  open(q,'wb').write(out)
  rc4,out2,err4=run(q,'--dump')
  if rc4!=0:
    rec('rt-parse',src,err4[:200])
  elif out2!=out:
    rec('rt-unstable',src,b'dump not idempotent')
  rc5,j2,err5=run(q,'--json')
  if rc2==0 and rc5==0:
    a=json.loads(j1);b=json.loads(j2)
    a.pop('source',None);b.pop('source',None)
    def strip(x):
      return x
    if a!=b:
      diffs=[]
      def dd(x,y,pth=''):
        if isinstance(x,dict) and isinstance(y,dict):
          for k in set(x)|set(y):
            if x.get(k)!=y.get(k): dd(x.get(k),y.get(k),pth+'/'+k)
        else: diffs.append(pth)
      dd(a,b)
      rec('rt-json',src,(','.join(re.sub(r'/(recipes|assignments|aliases|modules)/[^/]+','/\\1/*',x) for x in sorted(diffs)[:3])).encode())
    rt+=1
  elif rc2==0 and rc5!=0:
    rec('rt-json-err',src,err5[:200])
print('runs',n,'parsed',ok,'rt',rt,'unique',len(bad))
for k in bad: print(k)

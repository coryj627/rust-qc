import subprocess, random, tempfile, os, json, sys, re
J='/target/debug/just'
random.seed(int(sys.argv[1]))
N=int(sys.argv[2])
strs=["'a'",'"b\\n"',"'''\n  m\n  n\n'''",'"""\n  q\\t\n"""',"`echo hi`","```\n  echo\n```","x'~/foo'",'x"bar"',"f'p{{x}}q'",'f"p{{ \'r\' }}q"',"f'''\n  u{{x}}\n'''","f'{{{{'","'#'",'"\\u{41}"']
def expr(d=0):
    r=random.random()
    if d>3 or r<.3: return random.choice(strs+['x','y','HEX'])
    k=random.randint(0,12)
    a=lambda:expr(d+1)
    return {0:lambda:f"{a()} + {a()}",1:lambda:f"{a()} / {a()}",2:lambda:f"/ {a()}",3:lambda:f"({a()})",
    4:lambda:f"if {a()} == {a()} {{ {a()} }} else {{ {a()} }}",
    5:lambda:f"if {a()} =~ {a()} {{ {a()} }} else if {a()} != {a()} {{ {a()} }} else {{ {a()} }}",
    6:lambda:f"{a()} && {a()}",7:lambda:f"{a()} || {a()}",8:lambda:f"assert({a()} != {a()}, {a()})",
    9:lambda:f"uppercase({a()})",10:lambda:f"join({a()}, {a()}, {a()})",11:lambda:f"arch()",12:lambda:f"g({a()})"}[k]()
def value(): # for params: only value
    return random.choice(["'a'","x","`echo`",f"({expr(2)})",'"d"',"arch()", f"f'z{{{{x}}}}'"])
attrs=["[private]","[no-cd]","[linux]","[unix]","[macos]","[confirm]","[confirm('s')]","[confirm(x)]","[doc('d')]","[doc]","[group('g')]","[group: 'h']","[env('A','B')]","[no-exit-message]","[exit-message]","[no-quiet]","[positional-arguments]","[parallel]","[script]","[script('sh','-x')]","[extension('.x')]","[working-directory('w')]","[metadata('m')]","[default]","[arg('p', long)]","[arg('p', short='p')]","[arg('p', long='lng', help='h', value='v', pattern='a|b')]","[private, no-cd]"]
def recipe(i):
    s=''
    if random.random()<.3: s+=random.choice(['# doc comment\n','#nospace\n','#  two\n','#\n'])
    for _ in range(random.choice([0,0,1,1,2])): s+=random.choice(attrs)+random.choice(['\n','\n',' # ac\n'])
    if random.random()<.1: s+=random.choice(['# doc2\n'])
    s+=random.choice(['','@'])+f"r{i}"
    params=random.choice([[],['p'],[f"p={value()}"],['$p'],['p','+q'],['p',f'*$q={value()}'],[f'+q={value()}']])
    for p in params: s+=' '+p
    s+=':'
    deps=random.choice([[],['d0'],["(d0)"],[f"(d1 {expr(2)})"],['d0',f"(d1 'a')"]])
    for d in deps: s+=' '+d
    if random.random()<.2: s+=" && d0"
    if random.random()<.2: s+=' # tc'
    s+='\n'
    body=random.choice([[],['echo hi'],['@echo {{x}}'],['-echo','','  more'],['#!/bin/sh','echo {{ '+expr(2)+' }}'],['echo \\','  cont'],['{{x}}','{{{{y}}'],['# comment','echo'], ['@-echo'],['echo {{x}} {{y}}z']])
    ind=random.choice(['  ','\t','    '])
    for b in body: s+=(ind+b if b else '')+'\n'
    return s
def item(i):
    k=random.randint(0,13)
    if k<=4: return recipe(i)
    if k==5: return f"v{i} := {expr()}"+random.choice(['\n',' # tc\n'])
    if k==6: return random.choice(['export ','[private]\n','[private]\nexport '])+f"v{i} := {expr()}\n"
    if k==7: return f"alias a{i} := d0"+random.choice(['\n',' # tc\n'])
    if k==8: return f"alias a{i} := d0\n"
    if k==9: return random.choice(['# c\n','\n','\n\n','\n\n\n','#!shebangcomment\n','# c1\n# c2\n'])
    if k==10: return f"set "+random.choice(["export","export := false","quiet := true","lazy","shell := ['sh','-c']","tempdir := 'a'","dotenv-filename := 'f'","script-interpreter := ['sh']","positional-arguments := false",'guards','windows-shell := ["a", "b",]','working-directory := "w"',"unstable","ignore-comments"])+"\n"
    if k==11: return random.choice([f"unexport U{i}\n",f"import? 'nf{i}'\n",f"mod? m{i}\n",f"mod? m{i} 'nf'\n",f"[private]\nmod? m{i}\n",f"# mdoc\nmod? m{i}\n",f"[doc('x')]\nmod? m{i}\n",f"[doc]\nmod? m{i}\n",f"[group('g')]\nmod? m{i}\n", f"# c\n[doc('x')]\nmod? m{i}\n"])
    if k==12: return f"fn{i}(a, b) := a + b\n"
    return f"# plain{i}\n\n"
base="x := 'x'\ny := 'y'\ng(z) := z\nd0:\nd1 v:\n"
def run(path,*args):
    return subprocess.run([J,'--unstable','-f',path,*args],capture_output=True,text=True,timeout=20)
stats=dict(ok=0,err=0)
seen=set()
d=tempfile.mkdtemp(dir='/work/tmp/p')
p=d+'/justfile'
for n in range(N):
    src=base+''.join(item(i) for i in range(random.randint(1,6)))
    open(p,'w').write(src)
    r=run(p,'--dump','--dump-format','json')
    if r.returncode in(101,134,-6,-11) or 'Internal' in r.stderr or 'panicked' in r.stderr:
        print('CRASH',r.returncode,r.stderr[:300],repr(src)); continue
    if r.returncode!=0:
        stats['err']+=1; msg=re.sub(r'\d+','N',r.stderr.split('\n')[0]); seen.add(msg); continue
    stats['ok']+=1
    j1=json.loads(r.stdout)
    f=run(p,'--fmt')
    if f.returncode!=0:
        print('FMTFAIL',f.stderr[:300],repr(src));continue
    a=open(p).read()
    r2=run(p,'--dump','--dump-format','json')
    if r2.returncode!=0:
        print('REPARSEFAIL',r2.stderr[:300],'\nSRC',repr(src),'\nFMT',repr(a));continue
    j2=json.loads(r2.stdout)
    for k in ('source',): j1.pop(k,None); j2.pop(k,None)
    if j1!=j2:
        diffk=[k for k in j1 if j1[k]!=j2.get(k)]
        print('JSONDIFF',diffk)
        for k in diffk:
            if isinstance(j1[k],dict):
                for kk in set(j1[k])|set(j2[k]):
                    if j1[k].get(kk)!=j2[k].get(kk): print('  ',k,kk,json.dumps(j1[k].get(kk))[:300],'\n   =>',json.dumps(j2[k].get(kk))[:300])
            else: print('  ',k,j1[k],j2[k])
        print(' SRC',repr(src)[:600])
    f2=run(p,'--fmt')
    b=open(p).read()
    if a!=b:
        print('NONIDEMP',repr(a)[:400],'\n =>',repr(b)[:400])
print(stats)
for s in sorted(seen)[:40]: print('E:',s)

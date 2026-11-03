import subprocess, random, json, os, sys, re
J="/target/debug/just"
os.makedirs("/tmp/rt",exist_ok=True); os.chdir("/tmp/rt")
exprs=['"a"',"'b'","'''\n  c\n  d\n'''",'"""\n  e\\n\n"""',"`echo x`","```\n  echo y\n```","v0","(v0)","v0 + 'p'","'p' + v0 + \"q\"","'a' / 'b'","/ 'a'","/ 'a' / v0","('a' / 'b') + 'c'","'a' + 'b' / 'c'","'a' && 'b'","'a' || 'b'","'a' && 'b' || 'c'","('a' || 'b') && 'c'","if 'a' == 'b' { 'c' } else { 'd' }","if 'a' != v0 { 'c' } else if 'e' =~ 'f' { 'g' } else { 'h' }","if 'a' !~ 'b' { if 'c' == 'd' { 'e' } else { 'f' } } else { 'g' }","(if 'a' == 'b' { 'c' } else { 'd' }) + 'e'","'e' + if 'a' == 'b' { 'c' } else { 'd' }","os()","join('a', 'b', 'c')","env('A', 'b')","uppercase(v0 + 'x')","assert('a' == 'a', 'msg')","assert(v0 =~ '.*', 'msg' + 'x')","f'x{{v0}}y'","f\"{{v0}}\"","f'''\n  x{{ 'a' + v0 }}\n  y\n'''","f'{{{{ {{v0}} }}'","f'a{{ f\"b{{ v0 }}c\" }}d'","fn1('x')","fn1(fn1(v0))","x'a'","x\"b\"","x'''c'''","'\\n'","\"\\t\\\"\\\\\\u{1F600}\"","'\\'"]
def e(): return random.choice(exprs)
def DEP(): return random.choice(['a','`z`','"y" + a'])
n=[0]
def nm(p):
    n[0]+=1; return f"{p}{n[0]}"
def item():
    r=random.randrange(26)
    i=lambda:nm("i")
    if r==0: return f"{nm('v')} := {e()}"
    if r==1: return f"export {nm('v')} := {e()}"
    if r==2: return f"eager {nm('v')} := {e()}"
    if r==3: return f"[private]\n{nm('v')} := {e()}"
    if r==4: return f"_{nm('v')} := {e()}"
    if r==5: return f"alias {nm('al')} := r0"
    if r==6: return f"[private]\nalias {nm('al')} := r0"
    if r==7: return f"mod? {nm('m')}"
    if r==8: return f"[private]\nmod? {nm('m')}"
    if r==9: return f"[group('g')]\n[doc('d')]\nmod? {nm('m')} 'path'"
    if r==10: return f"[doc]\nmod? {nm('m')}"
    if r==11: return f"# moddoc\nmod? {nm('m')}"
    if r==12: return "import? 'nonexist' # c"
    if r==13: return f"unexport {nm('U')}"
    if r==14: return f"# a comment\n\n{nm('v')} := 'x' # trailing"
    if r==15: return f"{nm('fn')}(a, b) := a + {e()}"
    if r==16: return f"{nm('r')} a b={e() if random.random()<.5 else '`x`'} $c='d' *rest: r0 (r0 'x') && (r0 {DEP()})\n  echo {{{{a}}}} {{{{{{{{\n  @-echo {{{{ {e()} }}}}\n\n  echo \\\n    more\n  # comment"
    if r==17: return f"@{nm('r')}:\n  #!/bin/sh\n  echo\n\n  {{{{ {e()} }}}}"
    if r==18: return random.choice(["[private]","[no-cd]","[no-quiet]","[no-exit-message]","[exit-message]","[linux]","[macos]","[unix]","[windows]","[positional-arguments]","[parallel]","[confirm]","[confirm('sure')]","[confirm: 'x' + 'y']","[doc('z')]","[doc]","[group('g')]","[group: 'h']","[metadata('a','b')]","[env('A','B')]","[script]","[script('sh','-c')]","[working-directory('x')]","[default]","[extension('.x')]"])+f"\n{nm('r')}:\n  #!/bin/sh\n  x"
    if r==19: return f"[arg('a', long, short='s', pattern='p', help='h')]\n[arg('b', long='bb', value='v')]\n{nm('r')} a b='':"
    if r==20: return f"# doc for recipe\n{nm('r')}: # c\n  x"
    if r==21: return f"{nm('r')}: r0 # c2"
    if r==22: return f"{nm('r')}:\n    deep\n     deeper\n  \n    x\n"
    return f"{nm('v')} := {e()}"
sets=["set export","set export := false","set positional-arguments","set dotenv-load","set dotenv-path := 'a' + 'b'","set shell := ['sh', '-c']","set script-interpreter := ['sh']","set tempdir := 'x'","set working-directory := 'd'","set windows-shell := ['a']","set quiet := true","set guards","set lazy","set unstable","set ignore-comments","set no-exit-message", "set allow-duplicate-recipes","set fallback","set dotenv-filename := v0", "set dotenv-override", "set dotenv-required := false","set windows-powershell"]
def run(*a):
    return subprocess.run([J,"--unstable","--justfile","justfile","--working-directory",".",*a],capture_output=True,text=True,env={**os.environ,"A":"1"})
seen={}
bad=0;ok=0
random.seed(int(sys.argv[1]))
for it in range(int(sys.argv[2])):
    n[0]=0
    its=["v0 := 'z'","fn1(a) := a + 'q'","r0 *a:"]
    ss=random.sample(sets,random.randrange(3))
    k=random.randrange(1,5)
    body=[item() for _ in range(k)]
    src="\n\n".join(ss+its+body)+"\n"
    open("justfile","w").write(src)
    a=run("--dump","--dump-format","json")
    if a.returncode!=0:
        key=re.sub(r"\d+","N",a.stderr.split("\n")[0])[:80]
        seen.setdefault("COMPILE "+key,src); continue
    ok+=1
    d=run("--dump")
    open("justfile","w").write(d.stdout)
    b=run("--dump","--dump-format","json")
    if b.returncode!=0:
        key=re.sub(r"\d+","N",b.stderr.split("\n")[0])[:80]
        seen.setdefault("REPARSE "+key,src+"\n=====\n"+d.stdout); continue
    d2=run("--dump")
    if d2.stdout!=d.stdout: seen.setdefault("NONIDEMP",src+"\n=====\n"+d.stdout+"\n=====\n"+d2.stdout)
    ja,jb=json.loads(a.stdout),json.loads(b.stdout)
    def walk(x,y,p=""):
        if type(x)!=type(y): return p
        if isinstance(x,dict):
            for k in set(x)|set(y):
                if k not in x or k not in y: return p+"/"+k
                r=walk(x[k],y[k],p+"/"+re.sub(r"\d+","N",k))
                if r: return r
        elif isinstance(x,list):
            if len(x)!=len(y): return p+"[len]"
            for u,v in zip(x,y):
                r=walk(u,v,p+"[]")
                if r: return r
        elif x!=y: return p
    r=walk(ja,jb)
    if r: seen.setdefault("DIFF "+r,src+"\n=====\n"+d.stdout)
print("ok",ok)
for k,v in sorted(seen.items()):
    if k.startswith("COMPILE"): continue
    print("#####",k); print(v.split("r0 *a:")[-2 if False else 1][:500] if False else v[-600:])

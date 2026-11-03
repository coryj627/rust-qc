import subprocess,itertools,os,json
J='/work/tmp/bin/just'
os.chdir('/work/tmp/p13')
un="absolute_path arch blake3 blake3_file canonicalize capitalize clean datetime_utc encode_uri_component env env_var extension file_name file_stem kebabcase lowercamelcase lowercase parent_directory path_exists quote read require shoutykebabcase shoutysnakecase sha256 sha256_file snakecase style titlecase trim trim_end trim_start uppercamelcase uppercase which without_extension".split()
un.remove('arch')
bi="append prepend choose env_var_or_default join semver_matches trim_end_match trim_end_matches trim_start_match trim_start_matches".split()
te="replace replace_regex".split()
inputs=['','/','.','..','a/','/a/','./','//','é','a\nb','%','~','\t',' ','a.b.c','.hidden','..a','a..','(','[','$','$1','${x}','(?P<n>a)','\u0000' ,'1.2.3','>=1','0','3','🦀','a/../..','C:\\x','%%','%Y']
def esc(s): return json.dumps(s,ensure_ascii=False).replace('\\u0000','\\u{0}').replace('\\t','\\t')
bad={}
n=0
def run(expr):
    global n
    n+=1
    src=f"set unstable\nx := {expr}\n"
    open('t.just','w').write(src)
    p=subprocess.run([J,'-f','t.just','--evaluate','x'],capture_output=True,timeout=20,stdin=subprocess.DEVNULL)
    if p.returncode not in (0,1) or b'panicked' in p.stderr or b'Internal' in p.stderr:
        bad.setdefault(p.stderr[:160],expr)
for f in un:
    for i in inputs: run(f"{f}({esc(i)})")
for f in bi:
    for a,b in itertools.product(inputs[:20],['','a','/','.','1','é','(','>=1.0']):
        if f=='choose' and a.isdigit() and int(a)>1000: continue
        run(f"{f}({esc(a)}, {esc(b)})")
for f in te:
    for a,b,c in itertools.product(['','a','é','aé'],['','a','(',r'\b','é','.'],['','$','$0','$x','${','$$']):
        run(f"{f}({esc(a)}, {esc(b)}, {esc(c)})")
print('runs',n)
for k,v in bad.items(): print(repr(v),'->',k)

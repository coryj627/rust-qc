import os, json, subprocess
from pathlib import Path
root=Path('/work/tmp/verify'); root.mkdir(exist_ok=True)
log=[]
bin='/target/debug/just'
def fixture(name, files):
 d=root/name; d.mkdir(exist_ok=True)
 for p,s in files.items():
  f=d/p; f.parent.mkdir(parents=True,exist_ok=True); f.write_text(s)
 return d
def run(name,d,args=(),env=None):
 p=subprocess.run([bin,*args],cwd=d,env={**os.environ,**(env or {})},capture_output=True,text=True,timeout=10)
 x=dict(name=name,args=list(args),code=p.returncode,out=p.stdout,err=p.stderr)
 log.append(x); print(json.dumps(x)); return p

d=fixture('fmt-doc',{'justfile':'[doc("first\\nsecond")]\nmod child\n','child.just':'show:\n  @echo child\n'})
run('doc-before',d,['--list']);run('doc-fmt',d,['--fmt']); print('FORMATTED',repr((d/'justfile').read_text()));run('doc-after',d,['--list'])
d=fixture('fmt-eager',{'justfile':'set lazy\neager check := `exit 42`\nshow:\n  @echo OK\n'})
run('eager-before',d,['show']);run('eager-fmt',d,['--fmt']);print('FORMATTED',repr((d/'justfile').read_text()));run('eager-after',d,['show'])
d=fixture('fmt-private-alias',{'justfile':'[private]\nalias hidden := show\nshow:\n  @echo OK\n'})
run('private-alias-before',d,['--list']);run('private-alias-fmt',d,['--fmt']);run('private-alias-after',d,['--list'])
d=fixture('fmt-private-module',{'justfile':'[private]\nmod child\nshow:\n  @echo OK\n','child.just':'show:\n  @echo CHILD\n'})
run('private-module-before',d,['--list','--list-submodules']);run('private-module-fmt',d,['--fmt']);run('private-module-after',d,['--list','--list-submodules'])
for name,expr in [('datetime',"datetime('%Q')"),('datetime_utc',"datetime_utc('%')")]:
 d=fixture(name,{'justfile':f'x := {expr}\nshow:\n  @echo OK\n'});run(name,d,['--evaluate','x'])
d=fixture('timestamp',{'justfile':'show:\n  echo OK\n'});run('timestamp',d,['--timestamp','--timestamp-format','%Q','show'])
d=fixture('ambiguous',{'project/justfile':"mod child '../shared'\n",'shared/justfile':'show:\n','shared/.justfile':'show:\n'});run('external-module-ambiguous',d/'project',['--list'])
d=fixture('lookup',{'justfile':"A := HEX\nHEX := 'custom'\nshow:\n  @echo {{A}} {{HEX}}\n"});run('constant-lookup',d,['show'])
d=fixture('lookup-parent',{'justfile':"export x := 'parent'\nmod child\n",'child.just':"a := x\nx := 'child'\nshow:\n  @echo {{a}} {{x}}\n"});run('parent-lookup',d,['child::show'])
d=fixture('shared',{'justfile':"mod a 'shared.just'\nmod b 'shared.just'\n",'shared.just':"x := 'original'\nshow:\n  @echo {{module_path()}} {{x}}\n"});run('shared-module',d,['a::show','b::show']);run('shared-overrides',d,['a::x=A','b::x=B','a::show','b::show'])
d=fixture('shared-import',{'justfile':'mod first\nmod second\n','first.just':"import 'common.just'\nshow:\n  @echo first {{x}}\n",'second.just':"import 'common.just'\nshow:\n  @echo second {{x}}\n",'common.just':"x := 'original'\n"});run('shared-import-override',d,['first::x=changed','first::show','second::show'])
d=fixture('function-override',{'justfile':"set unstable\nx := 'original'\nf() := x\nshow:\n  @echo {{x}} {{f()}}\n"});run('function-override',d,['x=override','show']);run('function-evaluate-override',d,['x=override','--evaluate'])
d=fixture('function-repeat',{'justfile':"set unstable\nx := `echo marker >> counter; printf value`\nf() := x\nshow:\n  @echo {{x}} {{f()}} {{f()}}\n"});run('function-repeat',d,['show']);print('COUNTER',repr((d/'counter').read_text()))
d=fixture('function-capture',{'justfile':"set unstable\nx := 'global'\ny := x\nf(x) := y\nshow:\n  @echo {{y}} {{f('parameter')}}\n"});run('function-param-capture',d,['show'])
d=fixture('function-dependency',{'justfile':"set unstable\nf() := is_dependency()\nshow: dep\ndep:\n  @echo {{is_dependency()}} {{f()}}\n"});run('function-dependency',d,['show'])
d=fixture('function-export',{'justfile':"set unstable\nexport MY_TEST_VALUE := 'present'\nf() := shell('printf %s \"${MY_TEST_VALUE-missing}\"')\nshow:\n  @echo {{f()}}\n"});run('function-export',d,['show'])
d=fixture('ceiling',{'justfile':'show:\n  @echo PARENT\n','child/.keep':''})
for name,c in [('relative','.'),('absolute',str(d/'child')),('unclean',str(d/'child/../child'))]:run('ceiling-'+name,d/'child',['--ceiling',c,'show'])
d=fixture('shell',{'justfile':"foo := 'hello!world'\n"});p=run('shell-evaluation',d,['--evaluate','--evaluate-format','shell'])
for sh in ['sh','bash']:
 p2=subprocess.run([sh,'-c',p.stdout+'\nprintf "%s" "$foo"'],capture_output=True,text=True);print('ROUNDTRIP',sh,p2.returncode,repr(p2.stdout))
d=fixture('cross-alias',{'justfile':'mod helper\nalias shortcut := helper::task\n','helper.just':'task name:\n  @echo {{name}}\n'})
run('cross-alias-execution',d,['shortcut','ok']);run('cross-alias-list',d,['--list']);run('cross-alias-list-separate',d,['--list','--alias-style','separate'])
(d/'justfile').write_text('mod helper\nalias shortcut := helper::task\ntask unrelated=\'default\':\n  @echo local\n')
run('cross-alias-wrong-signature',d,['--list','--alias-style','separate'])
for rootlazy,childlazy in [(False,True),(True,False)]:
 d=fixture('lazy-'+str(rootlazy),{'justfile':('set lazy\n' if rootlazy else '')+'mod child\n','child.just':('set lazy\n' if childlazy else '')+'unused := `exit 9`\nshow:\n  @echo OK\n'});run('module-lazy-'+str(rootlazy),d,['child::show'])
d=fixture('unused-assert',{'justfile':"set lazy\nbad := assert('a' == 'b', 'unused')\ngood := 'ok'\nshow:\n  @echo OK\n"})
for args in [['show'],['--evaluate','good'],['--list']]:run('unused-assert-'+args[0],d,args)
d=fixture('env-empty',{'justfile':"x := env('CONTROL_EMPTY', 'fallback')\n"});run('env-empty',d,['--evaluate','x'],{'CONTROL_EMPTY':''})
d=fixture('plus',{'justfile':'gather +items:\n  @echo EXECUTED {{items}}\n'});run('plus-no-arguments',d,['gather']);run('plus-one-argument',d,['gather','one'])
d=fixture('empty-option',{'justfile':"[arg('name', long='name')]\nshow name='fallback':\n  @printf '<%s>\\n' '{{name}}'\n"});run('empty-option',d,['show','--name=']);run('default-option',d,['show'])
d=fixture('pattern',{'justfile':"[arg('mode', pattern='dev|prod')]\nbuild mode:\n  @echo EXECUTED {{mode}}\n"})
for v in ['xdevx','xprod','dev','prod']:run('pattern-'+v,d,['build',v])
for name,expr in [('else',"if 'a' == 'a' { 'chosen-then' } else { error('UNSELECTED_ELSE') }"),('then',"if 'a' == 'b' { error('UNSELECTED_THEN') } else { 'chosen-else' }")]:
 d=fixture('conditional-'+name,{'justfile':f'value := {expr}\n'});run('conditional-'+name,d,['--evaluate','value'])
(root/'results.json').write_text(json.dumps(log,indent=2))

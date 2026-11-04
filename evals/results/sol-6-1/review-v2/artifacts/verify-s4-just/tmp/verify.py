import json, os, pathlib, shutil, subprocess
BASE=pathlib.Path('/work/tmp/verify'); BASE.mkdir(exist_ok=True)
BIN='/target/debug/just'
records=[]
def setup(name,text,files=None):
 d=BASE/name; d.mkdir(exist_ok=True); (d/'justfile').write_text(text)
 for n,s in (files or {}).items():
  p=d/n; p.parent.mkdir(parents=True,exist_ok=True); p.write_text(s)
 return d
def run(name,d,args,env=None,exe=None):
 e=os.environ.copy(); e.update(env or {})
 p=subprocess.run([exe or BIN,*args],cwd=d,env=e,text=True,capture_output=True,timeout=15)
 r=dict(name=name,args=args,status=p.returncode,out=p.stdout,err=p.stderr)
 records.append(r); print(json.dumps(r),flush=True); return p
# Date/time error handling and valid controls.
for kind in ['datetime','datetime_utc']:
 d=setup(kind,"x := "+kind+"('%Q')\nfoo:\n  @echo ok\n")
 run(kind,d,['--evaluate','x'])
 (d/'justfile').write_text("x := "+kind+"('%Y')\nfoo:\n  @echo ok\n")
 run(kind+'_valid',d,['--evaluate','x'])
d=setup('timestamp','foo:\n  echo ok\n')
run('timestamp',d,['--timestamp','--timestamp-format','%Q','foo'])
# Module identity and separate overrides.
d=setup('identity',"mod a 'shared.just'\nmod b 'shared.just'\n",{'shared.just':"x := 'default'\nshow:\n  @echo {{module_path()}}:{{x}}\n"})
run('identity_a',d,['a::show']); run('identity_both',d,['a::show','b::show']); run('identity_override',d,['--set','a::x','A','--set','b::x','B','a::show','b::show'])
(d/'justfile').write_text("mod a 'a.just'\nmod b 'b.just'\n")
shutil.copy(d/'shared.just',d/'a.just'); shutil.copy(d/'shared.just',d/'b.just')
run('identity_separate_files',d,['a::show','b::show'])
# Function module state and lexical scope.
d=setup('function_override',"x := 'original'\nf() := x\nshow:\n  @echo {{x}}:{{f()}}\n")
run('function_override',d,['--unstable','x=override','show'])
d=setup('function_memo',"x := `printf x >> count; cat count`\nf() := x\nshow:\n  @echo {{x}}:{{f()}}:{{f()}}\n")
run('function_memo',d,['--unstable','show'])
d=setup('function_lexical',"y := 'global'\nx := y\nf(y) := x\nshow:\n  @echo {{x}}:{{f('parameter')}}\n")
run('function_lexical',d,['--unstable','show'])
d=setup('function_export',"export VERIFY_EXPORTED := 'present'\nf() := `printf '%s' \"$VERIFY_EXPORTED\"`\nshow:\n  @echo direct=$VERIFY_EXPORTED; echo function={{f()}}\n")
run('function_export',d,['--unstable','show'])
d=setup('dependency_status',"f() := is_dependency()\nroot: dep\n  @echo root={{is_dependency()}}:{{f()}}\ndep:\n  @echo dep={{is_dependency()}}:{{f()}}\n")
run('dependency_status',d,['--unstable','root'])
# Conditional, patterns, env and argument claims with controls.
d=setup('conditional',"value := if 'a' == 'a' { 'chosen-then' } else { error('UNSELECTED_ELSE') }\nother := if 'a' != 'a' { error('UNSELECTED_THEN') } else { 'chosen-else' }\n")
run('conditional',d,['--evaluate'])
d=setup('pattern',"[arg('mode', pattern='dev|prod')]\nbuild mode: dep\n  @echo body\ndep:\n  @echo dependency\n")
for arg in ['xdevx','xprod','dev','prod']: run('pattern_'+arg,d,['build',arg])
d=setup('env_empty',"x := env('CONTROL_EMPTY', 'fallback')\n")
run('env_empty',d,['--evaluate','x'],{'CONTROL_EMPTY':''})
run('env_missing',d,['--evaluate','x'])
d=setup('plus_variadic',"gather +items:\n  @echo got:{{items}}\n")
run('plus_variadic_empty',d,['gather']); run('plus_variadic_one',d,['gather','one'])
d=setup('empty_option',"[arg('name', long='name')]\nshow name='fallback':\n  @printf '<%s>\\n' '{{name}}'\n")
run('option_empty_equals',d,['show','--name=']); run('option_default',d,['show']); run('option_empty_separate',d,['show','--name',''])
# Dry run shell vs backtick control.
for kind,expr in [('shell',"shell('echo called >> marker; echo hello')"),('backtick',"`echo called >> marker; echo hello`")]:
 d=setup('dry_'+kind,"x := "+expr+"\nfoo:\n  echo {{x}}\n")
 run('dry_'+kind,d,['--dry-run','foo']); print(json.dumps({'name':'dry_'+kind+'_marker','exists':(d/'marker').exists()}))
# Dependency grouping panic and valid signature without dependency.
text="[arg('opt', long)]\nfoo arg='default' opt:\n  @echo {{arg}} {{opt}}\n"
d=setup('dependency_group',text+"\nbar: (foo 'option-value')\n")
for args in [['bar'],['--list'],['foo','--opt','option-value']]: run('dependency_group',d,args)
(d/'justfile').write_text(text); run('dependency_group_control',d,['foo','--opt','option-value'])
# Relative and absolute ceiling.
d=setup('ceiling',"foo:\n  @echo parent\n"); child=d/'child'; child.mkdir(exist_ok=True)
run('ceiling_relative',child,['--ceiling','.']); run('ceiling_absolute',child,['--ceiling',str(child)])
# Quiet scripts and linewise control.
for kind,text in [('linewise',"foo:\n  echo out; echo err >&2\n"),('shebang',"foo:\n  #!/bin/sh\n  echo out\n  echo err >&2\n"),('script',"[script('sh')]\nfoo:\n  echo out\n  echo err >&2\n")]:
 d=setup('quiet_'+kind,text); run('quiet_'+kind,d,['--quiet','foo'])
# Format eager and visibility.
d=setup('fmt_eager',"set lazy\neager x := `echo eager >> marker`\nfoo:\n  @echo recipe\n")
run('eager_before',d,['foo']); print(json.dumps({'name':'eager_before_marker','exists':(d/'marker').exists()})); (d/'marker').unlink()
run('eager_fmt',d,['--unstable','--fmt']); print(json.dumps({'name':'eager_formatted','text':(d/'justfile').read_text()})); run('eager_after',d,['foo']); print(json.dumps({'name':'eager_after_marker','exists':(d/'marker').exists()}))
d=setup('fmt_private_alias',"[private]\nalias secret := visible\nvisible:\n  @echo ok\n")
run('private_alias_before',d,['--list']); run('private_alias_fmt',d,['--unstable','--fmt']); run('private_alias_after',d,['--list'])
d=setup('fmt_private_module',"[private]\nmod hidden 'child.just'\nvisible:\n  @echo ok\n",{'child.just':"foo:\n  @echo child\n"})
run('private_module_before',d,['--list']); run('private_module_fmt',d,['--unstable','--fmt']); run('private_module_after',d,['--list'])
# Format string unindent (two initial newlines).
d=setup('format_string',"a := f'''\n\n    hello\n'''\nb := f'''\n\n    {{''}}hello\n'''\nc := '''\n\n    hello\n'''\n")
for var in ['a','b','c']: run('format_string_'+var,d,['--evaluate',var])
# Shell-formatted evaluate roundtrip.
d=setup('shell_evaluate',"x := 'hello!'\n")
p=run('shell_evaluate',d,['--evaluate','--evaluate-format','shell'])
for exe in ['sh','bash']: run('shell_roundtrip_'+exe,d,['-c',p.stdout+"printf '%s' \"$x\""],exe=exe)
# Environment precedence in linewise and both script forms.
for kind,body in [('linewise',"  @printf 'default=%s body=%s\\n' '{{arg}}' \"$FOO\"\n"),('shebang',"  #!/bin/sh\n  printf 'default=%s body=%s\\n' '{{arg}}' \"$FOO\"\n"),('script',"  printf 'default=%s body=%s\\n' '{{arg}}' \"$FOO\"\n")]:
 text="set dotenv-load\n[env('FOO', 'recipe')]\n"+('[script(\'sh\')]\n' if kind=='script' else '')+"foo arg=`printf '%s' \"$FOO\"`:\n"+body
 d=setup('env_precedence_'+kind,text,{'.env':'FOO=dotenv\n'}); run('env_precedence_'+kind,d,['foo'])
# Cross-module alias list and usage.
text="mod child 'child.just'\nalias shortcut := child::task\n"
d=setup('alias_list',text,{'child.just':"task required:\n  @echo child:{{required}}\n"})
run('alias_list_no_local',d,['--list']); run('alias_usage_no_local',d,['--show','shortcut']); run('alias_usage_error',d,['shortcut']); run('alias_runs',d,['shortcut','value'])
(d/'justfile').write_text(text+"\ntask:\n  @echo local\n")
run('alias_list_with_local',d,['--list','--alias-style','separate']); run('alias_usage_with_local',d,['--show','shortcut']); run('alias_usage_command',d,['--usage','shortcut'])
pathlib.Path('/work/tmp/probes.json').write_text(json.dumps(records,indent=2))

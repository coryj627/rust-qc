import subprocess, pathlib, json, os
BASE=pathlib.Path('/work/tmp/probes'); BASE.mkdir(exist_ok=True)
def probe(name, text, args, files=None, extra_env=None):
    d=BASE/name; d.mkdir(exist_ok=True)
    (d/'justfile').write_text(text)
    for path,content in (files or {}).items():
        p=d/path; p.parent.mkdir(parents=True,exist_ok=True); p.write_text(content)
    r=subprocess.run(['/target/debug/just',*args],cwd=d,capture_output=True,text=True,timeout=5, env=os.environ| (extra_env or {}))
    result=dict(name=name,args=args,code=r.returncode,stdout=r.stdout,stderr=r.stderr)
    print(json.dumps(result))
    return result
probe('choose_group',"[group('a')]\na:\n  @echo A\n[group('b')]\nb:\n  @echo B\n",['--choose','--group','a','--chooser','head -1'])
probe('function_override',"set unstable\nx := 'original'\nf() := x\nshow:\n  @echo {{x}} {{f()}}\n",['x=override','show'])
probe('function_export',"set unstable\nexport X := 'value'\nf() := shell('printf \"%s\" \"${X-missing}\"')\nshow:\n  @echo {{f()}}\n",['show'])
probe('function_dependency',"set unstable\nf() := is_dependency()\ndep:\n  @echo {{is_dependency()}} {{f()}}\nshow: dep\n",['show'])
probe('function_duplicate_params',"set unstable\nf(x, x) := x\na := f('first', 'second')\n",['--evaluate','a'])
probe('format_comments',"# comment\n[doc('hello')]\na:\n  @echo a\n",['--dump'])
probe('datetime_invalid',"x := datetime('%Q')\n",['--evaluate','x'])
probe('datetime_utc_invalid',"x := datetime_utc('%')\n",['--evaluate','x'])
probe('timestamp_invalid',"show:\n  @echo OK\n",['--timestamp','--timestamp-format','%Q','show'])
for name,text,files,args in [
    ('fmt_private', '[private]\nmod foo\n\nbar:\n', {'foo.just':'baz:\n'}, ['--list']),
    ('fmt_eager', 'set unstable\neager x := `echo eager > marker`\nshow:\n  @echo OK\n', {}, ['show']),
    ('fmt_doc_newline', '[doc("line 1\\nline 2")]\nmod foo\n\nbar:\n', {'foo.just':'baz:\n'}, ['--list']),
]:
    probe(name,text,args,files)
    probe(name,text,['--fmt'],files)
    d=BASE/name
    formatted=(d/'justfile').read_text()
    print(json.dumps(dict(name=name,formatted=formatted)))
    if (d/'marker').exists(): (d/'marker').unlink()
    r=subprocess.run(['/target/debug/just',*args],cwd=d,capture_output=True,text=True,timeout=5)
    print(json.dumps(dict(name=name,after_code=r.returncode,after_stdout=r.stdout,after_stderr=r.stderr,marker=(d/'marker').exists())))
probe('modules_same_file',"mod a 'shared.just'\nmod b 'shared.just'\n",['a::show','b::show'],{'shared.just':"show:\n  @echo {{module_path()}}\n"})
probe('modules_shared_import',"mod a\nmod b\n",['a::show','b::show'],{'a.just':"import 'shared.just'\n",'b.just':"import 'shared.just'\n",'shared.just':"show:\n  @echo {{module_path()}}\n"})
probe('modules_shared_override',"mod a 'shared.just'\nmod b 'shared.just'\n",['a::x=A','b::x=B','a::show','b::show'],{'shared.just':"x := 'default'\nshow:\n  @echo {{module_path()}} {{x}}\n"})
probe('fmt_eager_lazy', 'set lazy\neager x := `exit 42`\nshow:\n  @echo OK\n', ['show'])
probe('fmt_eager_lazy', 'set lazy\neager x := `exit 42`\nshow:\n  @echo OK\n', ['--fmt'])
r=subprocess.run(['/target/debug/just','show'],cwd=BASE/'fmt_eager_lazy',capture_output=True,text=True,timeout=5)
print(json.dumps(dict(name='fmt_eager_lazy_after',code=r.returncode,stdout=r.stdout,stderr=r.stderr)))
r=probe('evaluate_shell_bang', "foo := 'hello!'\n", ['--evaluate','--evaluate-format','shell'])
d=BASE/'evaluate_shell_bang'; (d/'values.sh').write_text(r['stdout'])
r=subprocess.run(['/bin/sh','-c','. ./values.sh; printf "%s" "$foo"'],cwd=d,capture_output=True,text=True,timeout=5)
print(json.dumps(dict(name='evaluate_shell_bang_sourced',code=r.returncode,stdout=r.stdout,stderr=r.stderr)))
probe('function_cycle', 'set unstable\nf() := a\na := f()\n', ['--evaluate','a'])
probe('fmt_private_alias', '[private]\nalias hidden := foo\nfoo:\n', ['--dump'])
probe('env_attribute_body', "[env('MY_VAR', 'value')]\nshow:\n  @echo {{`printf %s ${MY_VAR-missing}`}} {{shell('printf %s ${MY_VAR-missing}')}} $MY_VAR\n", ['show'])
probe('alias_module_listing', "mod foo\nalias quick := foo::build\n", ['--list','--alias-style','separate'], {'foo.just':"build target='module':\n  @echo {{target}}\n"})
probe('alias_module_wrong_listing', "mod foo\nalias quick := foo::build\nbuild:\n  @echo local\n", ['--list','--alias-style','separate'], {'foo.just':"build target:\n  @echo {{target}}\n"})
d=BASE/'ceiling_relative'; (d/'sub').mkdir(parents=True,exist_ok=True); (d/'justfile').write_text('show:\n  @echo PARENT\n')
for ceiling in ['.',str(d/'sub')]:
    r=subprocess.run(['/target/debug/just','--ceiling',ceiling,'show'],cwd=d/'sub',capture_output=True,text=True,timeout=5)
    print(json.dumps(dict(name='ceiling_relative',ceiling=ceiling,code=r.returncode,stdout=r.stdout,stderr=r.stderr)))
probe('function_cached', "set unstable\nx := uuid()\nf() := x\nshow:\n  @echo {{x}} {{f()}} {{f()}}\n", ['show'])

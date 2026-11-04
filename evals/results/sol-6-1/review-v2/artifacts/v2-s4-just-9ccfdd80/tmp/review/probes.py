from pathlib import Path
import subprocess, os
root=Path('/work/tmp/review')
cases=[
('datetime', "x := datetime('%Q')\n", ['--evaluate','x']),
('datetime_utc', "x := datetime_utc('%')\n", ['--evaluate','x']),
('timestamp', "foo:\n  @echo hello\n", ['--timestamp','--timestamp-format','%Q','foo']),
('function_override', "x := 'original'\nget() := x\na := get()\n", ['--unstable','--evaluate','--set','x','override']),
('function_cache', "x := `echo called >> calls; echo value`\nget() := x\na := get()\n", ['--unstable','--evaluate']),
('function_lazy', "set lazy\nx := 'global'\nget() := x\nfoo:\n  @echo {{get()}}\n", ['--unstable','foo']),
('function_duplicate', "get(x, x) := x\na := get('first', 'second')\n", ['--unstable','--evaluate','a']),
('function_dependency', "get() := is_dependency()\nfoo: bar\nbar:\n  @echo {{is_dependency()}} {{get()}}\n", ['--unstable','foo']),
('option_dependency', "[arg('opt', long)]\nfoo arg='default' opt:\n  @echo {{arg}} {{opt}}\nbar: (foo 'option-value')\n", ['bar']),
('module_twice', "mod a 'common.just'\nmod b 'common.just'\n", ['a::foo','b::foo']),
('env_attribute_backtick', "[env('REVIEW_VALUE', 'hello')]\nfoo:\n  @echo {{`printf %s \"$REVIEW_VALUE\"`}}\n", ['foo']),
]
for name,source,args in cases:
 p=root/name;p.mkdir(exist_ok=True);(p/'justfile').write_text(source)
 if (p/'calls').exists(): (p/'calls').unlink()
 if name=='module_twice': (p/'common.just').write_text('foo:\n  @echo {{module_path()}}\n')
 result=subprocess.run(['/target/debug/just','--color','never',*args],cwd=p,text=True,capture_output=True,timeout=5)
 print(name, args, 'status=',result.returncode,'stdout=',repr(result.stdout),'stderr=',repr(result.stderr))
 if (p/'calls').exists(): print('calls:',(p/'calls').read_text())

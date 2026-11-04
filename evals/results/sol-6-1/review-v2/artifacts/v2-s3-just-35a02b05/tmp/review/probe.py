from pathlib import Path
import subprocess
root=Path('/work/tmp/review')
bin='/target/debug/just'
def case(name,files,commands):
    d=root/name
    d.mkdir(exist_ok=True)
    for path, contents in files.items():
        p=d/path
        p.parent.mkdir(parents=True,exist_ok=True)
        p.write_text(contents)
    print('\nCASE',name,flush=True)
    for args in commands:
        p=subprocess.run([bin,*args],cwd=d,text=True,capture_output=True,timeout=10)
        print(repr(args),'exit',p.returncode,'stdout',repr(p.stdout),'stderr',repr(p.stderr),flush=True)
case('format-private',{'justfile':"[private]\nalias internal := task\n\n[private]\nmod helper\n\ntask:\n",'helper.just':'subtask:\n'},[['--list'],['--dump'],['--fmt'],['--list']])
case('datetime',{'justfile':"x := datetime('%Q')\ny := datetime_utc('%')\n"},[['--evaluate','x'],['--evaluate','y']])
case('function-override',{'justfile':"set unstable\nx := 'original'\nf() := x\ny := f()\nshow:\n  @echo {{x}} {{f()}}\n"},[['--evaluate','y'],['x=override','--evaluate','y'],['--set','x','override','--evaluate','y'],['--set','x','override','show']])
case('shared-module',{'justfile':"mod first 'shared.just'\nmod second 'shared.just'\n",'shared.just':"x := module_path()\ntask:\n  @echo {{module_path()}} {{x}}\n"},[['--list','--list-submodules'],['first','task'],['second','task'],['--evaluate','first::x'],['--evaluate','second::x']])

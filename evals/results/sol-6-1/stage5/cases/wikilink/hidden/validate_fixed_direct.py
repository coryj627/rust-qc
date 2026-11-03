from pathlib import Path
import os, subprocess, json
root=Path('/work')
env=os.environ.copy()
env['RUSTUP_AUTO_INSTALL']='0'
commands=[]
for profile in ['debug','release']:
    out=root/f'tmp/direct-fixed-{profile}'
    out.mkdir(exist_ok=True)
    deps=root/'tmp/target-baseline/debug/deps'
    flags=['-C','opt-level=0' if profile=='debug' else 'opt-level=3', '-C',f'overflow-checks={"yes" if profile=="debug" else "no"}']
    cmds=[['rustc','--edition=2021','--crate-name','pulldown_cmark_escape','--crate-type','rlib','--cfg','feature="std"', str(root/'tmp/control-fixed/pulldown-cmark-escape/src/lib.rs'),'-o',str(out/'libpulldown_cmark_escape.rlib')]+flags]
    libcmd=['rustc','--edition=2021','--crate-name','pulldown_cmark','--crate-type','rlib','--cfg','feature="std"','--cfg','feature="html"','--cfg','feature="getopts"',str(root/'tmp/control-fixed/pulldown-cmark/src/lib.rs'),'-L',f'dependency={deps}','--extern',f'pulldown_cmark_escape={out}/libpulldown_cmark_escape.rlib','-o',str(out/'libpulldown_cmark.rlib')]+flags
    for name in ['bitflags','memchr','unicase','getopts']:
        paths=list(deps.glob(f'lib{name}-*.rlib'))
        assert paths,(name,paths)
        paths.sort()
        libcmd+=['--extern',f'{name}={paths[0]}']
    cmds.append(libcmd)
    for test in ['oracle','neighbors']:
        cmds.append(['rustc','--edition=2021','--test','--cfg','feature="html"',str(root/f'prepared/hidden/case_wikilink_{test}.rs'),'--extern',f'pulldown_cmark={out}/libpulldown_cmark.rlib','-L',f'dependency={deps}','-L',f'dependency={out}','-o',str(out/f'{test}-test')]+flags)
        cmds.append([str(out/f'{test}-test'),'--nocapture'])
    log=root/f'prepared/logs/fixed-direct-{profile}.log'
    with log.open('w') as f:
        for cmd in cmds:
            f.write('$ '+__import__('shlex').join(cmd)+'\n');f.flush()
            result=subprocess.run(cmd,env=env,stdout=f,stderr=subprocess.STDOUT)
            f.write(f'exit={result.returncode}\n');f.flush()
            commands.append({'profile':profile,'command':cmd,'exit':result.returncode})
            if result.returncode:break
    print(log.read_text())
(root/'prepared/logs/fixed-direct-commands.json').write_text(json.dumps(commands,indent=2)+'\n')
raise SystemExit(1 if any(c['exit'] for c in commands) else 0)

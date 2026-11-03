from pathlib import Path
import os,json,subprocess
for c in 'ABCD':
 root=Path('/work/results')/c
 env=os.environ.copy();env.update(CARGO_HOME='/work/.cargo-home',CARGO_NET_OFFLINE='true',RUSTUP_AUTO_INSTALL='0',CARGO_TARGET_DIR=f'/work/tmp/{c}-target',CARGO_BUILD_JOBS='2')
 for key in ['RUSTFLAGS','RUSTDOCFLAGS','CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS','CARGO_ENCODED_RUSTFLAGS']:env.pop(key,None)
 cmd=['cargo','rustc','--offline','--locked','-p','pulldown-cmark','--release','--lib','--verbose','--','--print','cfg']
 log=root/'logs/release-settings.log'
 with log.open('w') as f:r=subprocess.run(cmd,cwd=f'/work/tmp/{c}-own',env=env,stdout=f,stderr=subprocess.STDOUT)
 meta=dict(name='release-settings',command=cmd,cwd=f'/work/tmp/{c}-own',exit_code=r.returncode,log=str(log))
 (root/'release-settings.json').write_text(json.dumps(meta,indent=2)+'\n')
 print(c,r.returncode)
 print('\n'.join(line for line in log.read_text().splitlines() if 'rustc --crate-name pulldown_cmark ' in line or 'debug_assertions' in line))

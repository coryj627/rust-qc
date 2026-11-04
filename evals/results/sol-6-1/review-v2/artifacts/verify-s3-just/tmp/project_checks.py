import os,subprocess,json
from pathlib import Path
base={**os.environ,'RUSTUP_AUTO_INSTALL':'0','CARGO_NET_OFFLINE':'true','CARGO_BUILD_JOBS':'2'}
Path('/target/verify-test-tmp').mkdir(exist_ok=True);Path('/target/verify-runtime').mkdir(exist_ok=True)
checks=[
 ('fmt',['cargo','fmt','--all','--check'],{}),
 ('clippy',['cargo','clippy','--locked','--workspace','--all-targets','--all-features','--','-D','warnings'],{}),
 ('clippy-allowed',['cargo','clippy','--locked','--workspace','--all-targets','--all-features','--','-D','warnings','-A','clippy::manual_assert_eq','-A','clippy::assert_is_empty'],{}),
 ('doc-private',['cargo','doc','--locked','--workspace','--no-deps','--document-private-items','--all-features'],{'RUSTDOCFLAGS':'-D warnings'}),
 ('doc-public',['cargo','doc','--locked','--workspace','--no-deps','--all-features'],{'RUSTDOCFLAGS':'-D warnings'}),
 ('test-adjusted',['cargo','test','--locked','--workspace'],{'USER':'audit','XDG_RUNTIME_DIR':'/target/verify-runtime','TMPDIR':'/target/verify-test-tmp'}),
 ('test-parallel',['cargo','test','--locked','--test','integration','parallel::','--','--ignored'],{'USER':'audit','XDG_RUNTIME_DIR':'/target/verify-runtime','TMPDIR':'/target/verify-test-tmp'})
]
for name,cmd,env in checks:
 p=subprocess.run(cmd,cwd='/work/crate',env={**base,**env},capture_output=True,text=True,timeout=180)
 Path('/work/tmp/'+name+'.log').write_text(p.stdout+p.stderr)
 print(json.dumps(dict(name=name,code=p.returncode,tail=(p.stdout+p.stderr)[-9500:])),flush=True)

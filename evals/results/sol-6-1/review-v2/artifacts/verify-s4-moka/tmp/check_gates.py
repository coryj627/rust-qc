import os, subprocess, json
os.chdir('/work/crate')
env = dict(os.environ, RUSTUP_AUTO_INSTALL='0', CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS='2', CARGO_TARGET_DIR='/target')
commands = [
 ('all_tests', ['cargo','test','--locked','--all-features']),
 ('fmt',['cargo','fmt','--all','--','--check']),
 ('clippy',['cargo','clippy','--locked','--workspace','--all-targets','--all-features','--','-D','warnings']),
 ('sync_check',['cargo','check','--locked','--no-default-features','--features','sync']),
 ('future_check',['cargo','check','--locked','--no-default-features','--features','future']),
 ('no_features',['cargo','check','--locked','--no-default-features']),
 ('docs_public',['cargo','doc','--locked','--no-deps','--features','sync,future']),
 ('docs_private_strict',['cargo','doc','--locked','--workspace','--no-deps','--document-private-items','--all-features']),
 ('advisory',['cargo','clippy','--locked','--lib','--all-features','--','-W','missing_docs','-W','missing_debug_implementations','-W','clippy::missing_errors_doc','-W','clippy::undocumented_unsafe_blocks']),
]
for name,cmd in commands:
    run_env = dict(env)
    if name=='docs_private_strict': run_env['RUSTDOCFLAGS']='-D warnings'
    with open('/work/tmp/'+name+'.log','w') as log:
        p = subprocess.run(cmd, env=run_env, stdout=log, stderr=subprocess.STDOUT)
    print(json.dumps({'check':name,'exit_code':p.returncode}), flush=True)

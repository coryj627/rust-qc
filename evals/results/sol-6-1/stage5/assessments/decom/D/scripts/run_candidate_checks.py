import hashlib,json,os,shutil,subprocess,tempfile,time
from pathlib import Path
ROOT=Path('/work/results')
def snapshot(root):
    return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.rglob('*')) if p.is_file()}
def changes(a,b):
    return [{'path':k,'before':a.get(k),'after':b.get(k)} for k in sorted(a.keys()|b.keys()) if a.get(k)!=b.get(k)]
for letter in 'ABCD':
    out=ROOT/letter
    src=Path('/work/candidates')/letter
    run=Path(tempfile.mkdtemp(prefix=f'decom-own-{letter}.',dir='/work/tmp'))
    source=run/'source';shutil.copytree(src,source)
    own_before=snapshot(source)
    gate_parent=run/'standalone';gate_parent.mkdir()
    gate_root=gate_parent/'alacritty_terminal';shutil.copytree(src/'alacritty_terminal',gate_root)
    shutil.copy2(src/'Cargo.lock',gate_root/'Cargo.lock')
    shutil.copy2(src/'rustfmt.toml',gate_root/'rustfmt.toml')
    shutil.copy2(src/'README.md',gate_parent/'README.md')
    gate_before=snapshot(gate_parent)
    shutil.copy2(gate_root/'Cargo.lock',out/'gate-original-Cargo.lock')
    env=os.environ.copy();env.update(RUSTUP_AUTO_INSTALL='0',CARGO_NET_OFFLINE='true',CARGO_HOME='/work/.cargo-home',TMPDIR='/work/tmp',CARGO_TARGET_DIR=str(run/'target'),CARGO_BUILD_JOBS='2',RUSTUP_TOOLCHAIN='1.99.1',XDG_CACHE_HOME='/work/tmp/decom-miri-cache')
    common={k:env[k] for k in ['RUSTUP_AUTO_INSTALL','CARGO_NET_OFFLINE','CARGO_HOME','TMPDIR','CARGO_TARGET_DIR','CARGO_BUILD_JOBS','RUSTUP_TOOLCHAIN','XDG_CACHE_HOME']}
    checks=[]
    def check(name,args,cwd=source,extra=None,timeout=600,adaptation=None):
        current=env.copy();current.update(extra or {})
        log=out/(name+'.log')
        entry={'name':name,'argv':args,'cwd':str(cwd),'environment':{**common,**(extra or {})},'command':'cd '+str(cwd)+' && '+ ' '.join(args),'log':str(log),'adaptation':adaptation}
        print(f'{letter}: {name} started',flush=True)
        start=time.monotonic()
        with log.open('w') as f:
            try:
                p=subprocess.run(args,cwd=cwd,env=current,stdout=f,stderr=subprocess.STDOUT,timeout=timeout)
                entry['exit_code']=p.returncode
            except subprocess.TimeoutExpired:
                entry['exit_code']=None;entry['limit']='Timed out after '+str(timeout)+' seconds; check incomplete.'
        entry['elapsed_seconds']=round(time.monotonic()-start,3)
        checks.append(entry);(out/'own-checks.json').write_text(json.dumps(checks,indent=2))
        print(f'{letter}: {name} exit={entry["exit_code"]}',flush=True)
    test=['cargo','+1.99.1','test','--locked','--offline','-p','alacritty_terminal']
    check('own-default-suite',test)
    if letter in 'BC':
        check('own-no-default-suite',test+['--no-default-features'])
        if letter=='B':check('own-all-features-suite',test+['--all-features'])
        check('own-miri-regressions',['cargo','+nightly-2026-10-01','miri','test','--locked','--offline','-p','alacritty_terminal','--test','origin_mode'],extra={'CARGO_TARGET_DIR':str(run/'miri-target')})
    changed=['alacritty_terminal/src/term/mod.rs']
    if letter in 'ABC':changed+=['alacritty_terminal/tests/origin_mode.rs']
    check('own-changed-stable-format',['rustfmt','+1.99.1','--check','--edition','2021','--config','skip_children=true']+changed,adaptation='Changed Rust files selected from pre-injection candidate.diff; skip_children avoids checking unchanged child modules.')
    check('own-terminal-format',['cargo','+1.99.1','fmt','-p','alacritty_terminal','--check'],adaptation='Package selection avoids formatting graphical app; stable rustfmt cannot apply nightly-only options.')
    check('gate',['bash','/work/gate/check.sh',str(gate_root)],cwd=gate_root,extra={'CARGO_TARGET_DIR':str(run/'gate-target')},adaptation='Unmodified archived gate on standalone alacritty_terminal copy with original workspace Cargo.lock and rustfmt.toml copied into crate root, README.md alongside. No parent workspace manifest exists. RUSTUP_TOOLCHAIN=1.99.1; offline environment. --all-features is the appropriate terminal-crate selection. Gate commands do not accept --locked, so its lockfile pruning is confined to scratch.')
    gate_after=snapshot(gate_parent)
    if (gate_root/'Cargo.lock').exists():shutil.copy2(gate_root/'Cargo.lock',out/'gate-after-Cargo.lock')
    own_after=snapshot(source)
    (out/'verification-file-changes.json').write_text(json.dumps({'own_full_workspace_scratch':changes(own_before,own_after),'gate_standalone_scratch':changes(gate_before,gate_after),'submission_changes':changes(json.loads((out/'submission-before.sha256.json').read_text()),snapshot(src))},indent=2))
    (out/'scratch-layout.json').write_text(json.dumps({'temporary_root':str(run),'own_source':str(source),'gate_source':str(gate_root),'removed_after_execution':True},indent=2))
    shutil.rmtree(run)
    subprocess.run(['/work/clock'])

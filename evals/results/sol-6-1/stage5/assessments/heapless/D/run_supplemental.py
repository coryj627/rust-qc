#!/usr/bin/env python3
import os, sys, pathlib, subprocess, json, shutil, difflib, hashlib, time
letter=sys.argv[1]
out=pathlib.Path('/work/results')/letter
scratch=pathlib.Path('/work/tmp')/('assessment-'+letter)
candidate=pathlib.Path('/work/candidates')/letter
records=json.loads((out/'checks.json').read_text())
features='alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes'
env=dict(os.environ,CARGO_HOME='/work/.cargo-home',CARGO_NET_OFFLINE='true',RUSTUP_AUTO_INSTALL='0',CARGO_TERM_COLOR='never',CARGO_TARGET_DIR=str(scratch/'regression-baseline-target'),CARGO_BUILD_JOBS='2')
def run(name,cmd,cwd,extra=None,timeout=600):
    e=dict(env,**(extra or {})); log=out/(name+'.log'); start=time.monotonic()
    with log.open('w') as f:
        f.write('cwd: '+str(cwd)+'\ncommand: '+json.dumps(cmd)+'\nenv: '+json.dumps({k:v for k,v in e.items() if k.startswith(('CARGO_','RUST','MIRI'))},sort_keys=True)+'\n'); f.flush()
        try:
            r=subprocess.run(cmd,cwd=cwd,env=e,stdout=f,stderr=subprocess.STDOUT,timeout=timeout); code=r.returncode; outcome='completed'
        except subprocess.TimeoutExpired:
            code=124; outcome='timeout (environment/time limit, not behavioral assertion failure)'
        f.write('\nexit_code: '+str(code)+'\n')
    records.append(dict(name=name,command=cmd,cwd=str(cwd),exit_code=code,log=str(log),observed_outcome=outcome,elapsed_seconds=round(time.monotonic()-start,3)))
    (out/'checks.json').write_text(json.dumps(records,indent=2)+'\n'); print(letter,name,code,flush=True)
names={'A':'truncate_with_collisions','B':'truncate_preserves_collision_chains','C':'truncate_collisions','D':'truncate_preserves_colliding_entries'}
text=(candidate/'src/index_map.rs').read_text()
start=text.index('    #[test]\n    fn '+names[letter]+'()')
end=text.index('    #[test]\n',start+len('    #[test]\n'))
test=text[start:end]
(out/'transferred-regression.rs').write_text(test)
base=pathlib.Path('/work/baseline/src/index_map.rs').read_text()
marker='    #[test]\n    fn entry_roll_through_all()'
assert base.count(marker)==1
transferred=base.replace(marker,test+marker)
transfer=scratch/'regression-baseline-source'
shutil.copytree('/work/baseline',transfer,ignore=shutil.ignore_patterns('.git','target'))
(transfer/'src/index_map.rs').write_text(transferred)
assert transferred.replace(test,'',1)==base
(out/'regression-only.diff').write_text(''.join(difflib.unified_diff(base.splitlines(True),transferred.splitlines(True),fromfile='baseline/src/index_map.rs',tofile='regression-baseline-source/src/index_map.rs')))
(out/'regression-transfer-integrity.json').write_text(json.dumps({'only_injected_candidate_test':True,'baseline_production_sha256':hashlib.sha256(base.encode()).hexdigest(),'test_name':'index_map::tests::'+names[letter],'test_bytes_sha256':hashlib.sha256(test.encode()).hexdigest(),'adaptation':'Copied only the candidate-added test function into the unchanged baseline unit-test module; no production code changed.'},indent=2)+'\n')
run('candidate-regression-on-baseline',['cargo','test','--offline','--lib','index_map::tests::'+names[letter],'--','--exact','--nocapture'],transfer)
if letter=='A':
    pristine=scratch/'pristine-baseline-source'
    shutil.copytree('/work/baseline',pristine,ignore=shutil.ignore_patterns('.git','target'))
    run('pristine-baseline-clippy',['cargo','clippy','--workspace','--all-targets','--features',features,'--','-D','warnings'],pristine,{'CARGO_TARGET_DIR':str(scratch/'pristine-baseline-target')})
run('miri-indexmap',['cargo','+nightly-2026-10-01','miri','test','--offline','--lib','index_map::tests::'],scratch/'verification-source',{'CARGO_TARGET_DIR':str(scratch/'miri-target'),'MIRIFLAGS':'-Zmiri-ignore-leaks'},timeout=600)

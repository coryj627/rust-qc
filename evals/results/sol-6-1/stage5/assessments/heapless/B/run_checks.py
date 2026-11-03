#!/usr/bin/env python3
import pathlib, subprocess, json, os, sys, shutil, hashlib, time, re
letter=sys.argv[1]
out=pathlib.Path('/work/results')/letter
candidate=pathlib.Path('/work/candidates')/letter
scratch=pathlib.Path('/work/tmp')/('assessment-'+letter)
scratch.mkdir()
features='alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes'
env=dict(os.environ,CARGO_HOME='/work/.cargo-home',CARGO_NET_OFFLINE='true',RUSTUP_AUTO_INSTALL='0',CARGO_TERM_COLOR='never',CARGO_TARGET_DIR=str(scratch/'verification-target'),CARGO_BUILD_JOBS='2')
records=[]
def hashes(root):
    return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.rglob('*')) if p.is_file() and '.git' not in p.parts and 'target' not in p.parts}
def save():
    (out/'checks.json').write_text(json.dumps(records,indent=2)+'\n')
def run(name,cmd,cwd,extra=None,timeout=600):
    log=out/(name+'.log'); e=dict(env,**(extra or {})); start=time.monotonic()
    with log.open('w') as f:
        f.write('cwd: '+str(cwd)+'\ncommand: '+json.dumps(cmd)+'\nenv: '+json.dumps({k:v for k,v in e.items() if k.startswith(('CARGO_','RUST','MIRI'))},sort_keys=True)+'\n'); f.flush()
        try:
            r=subprocess.run(cmd,cwd=cwd,env=e,stdout=f,stderr=subprocess.STDOUT,timeout=timeout)
            code=r.returncode; outcome='completed'
        except subprocess.TimeoutExpired:
            code=124; outcome='timeout (environment/time limit, not behavioral assertion failure)'
        f.write('\nexit_code: '+str(code)+'\n')
    record=dict(name=name,command=cmd,cwd=str(cwd),exit_code=code,log=str(log),observed_outcome=outcome,elapsed_seconds=round(time.monotonic()-start,3))
    records.append(record); save(); print(letter,name,code,flush=True)
    return code
before=hashes(candidate)
(out/'submission-before.sha256.json').write_text(json.dumps(before,indent=2)+'\n')
diff=subprocess.run(['diff','-u','/work/baseline/src/index_map.rs',str(candidate/'src/index_map.rs')],capture_output=True,text=True)
(out/'submission.diff').write_text(diff.stdout)
shutil.copy2('/work/reports/'+letter+'.md',out/'candidate-report.md')
frozen=out/'frozen'; frozen.mkdir()
for name in ['score_candidate.py','run_logged.py','heldback_truncate_upstream.rs','heldback_truncate_spec.rs']:
    shutil.copy2('/work/prepared/hidden/'+name,frozen/name)
run('frozen-scorer',['python3','/work/prepared/hidden/score_candidate.py',str(candidate),str(scratch/'scoring')],'/work',timeout=900)
scoring=scratch/'scoring'
if (scoring/'logs').exists(): shutil.copytree(scoring/'logs',out/'frozen-logs')
if (scoring/'results.json').exists(): shutil.copy2(scoring/'results.json',out/'frozen-results.json')
verify=scratch/'verification-source'
shutil.copytree(candidate,verify,ignore=shutil.ignore_patterns('target','.git'))
verify_before=hashes(verify)
run('focused-indexmap',['cargo','test','--offline','--lib','index_map::tests::'],verify)
run('stable-format',['cargo','fmt','--all','--check'],verify)
run('dated-nightly-format',['cargo','+nightly-2026-10-01','fmt','--all','--check'],verify)
run('archived-gate',['bash','/work/gate/check.sh','--features',features,str(verify)],'/work',timeout=900)
run('ci-compatible-feature-tests',['cargo','test','--offline','--features',features],verify,{'RUSTFLAGS':'-D warnings'},timeout=900)
after=hashes(candidate)
(out/'submission-after.sha256.json').write_text(json.dumps(after,indent=2)+'\n')
verify_after=hashes(verify)
changes=lambda a,b: {'added':[p for p in b if p not in a],'removed':[p for p in a if p not in b],'changed':[p for p in a if p in b and a[p]!=b[p]]}
(out/'integrity.json').write_text(json.dumps({'submission_changes':changes(before,after),'verification_copy_cargo_changes':changes(verify_before,verify_after),'hidden_test_bytes_identical':all((scoring/'source/tests'/n).read_bytes()==(frozen/n).read_bytes() for n in ['heldback_truncate_upstream.rs','heldback_truncate_spec.rs'])},indent=2)+'\n')

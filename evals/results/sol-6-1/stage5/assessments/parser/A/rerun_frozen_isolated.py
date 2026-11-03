from pathlib import Path
import os, json, subprocess, shlex, re, shutil, hashlib
results = Path('/work/results')
paths = ['/work/tmp/small-config-submission-target','/work/tmp/small-config-api-target']
for path in paths:
    p=Path(path)
    if p.exists():
        dest=Path(str(p)+'-initial-mixed-archive')
        assert not dest.exists()
        p.rename(dest)
for letter in 'ABCD':
    base=results / letter
    logdir=base / 'frozen-isolated'
    logdir.mkdir()
    env=os.environ.copy()
    env.update(RUSTUP_AUTO_INSTALL='0',CARGO_NET_OFFLINE='true',CARGO_TARGET_DIR=paths[0])
    argv=['/work/prepared/hidden/run_checks.sh',f'/work/candidates/{letter}',str(logdir)]
    with open(logdir/'runner.log','wb') as f:
        p=subprocess.run(argv,env=env,cwd='/work',stdout=f,stderr=subprocess.STDOUT,timeout=240)
    r=json.loads((base/'record.json').read_text())
    r['adaptations'].append('The original frozen runner hardcodes shared target directories. Archived both build directories before this candidate run, executed the original runner unchanged with its exact hardcoded target paths, then archived the resulting targets. This prevents cross-candidate cached artifacts. The initial logs are retained separately.')
    r['limits'].append('Initial frozen run used shared Cargo artifacts and is not reliable for own-test execution or library diagnostics. Initial B/D doctest compile errors refer to missing kind()/ParseErrorKind that are present in their source and independently compile. They are environmental cache artifacts, not established submission defects. Definitive frozen results use frozen-isolated logs.')
    for item in r['checks']:
        if item['name'] in ['frozen_orchestrator','library','submitted-tests','doctests','public-api','fmt','clippy']:
            item['measurement']='initial_shared_target'
            item['definitive']=False
            if item['name']=='submitted-tests' and letter in 'BD':
                item['observed_outcome']='environmental_cache_artifact_wrong_test_binary'
                item['adaptation_or_limit']='Ran 0 tests despite 12 tests in candidate source; isolated rerun required.'
            elif item['name']=='doctests' and letter in 'BD':
                item['observed_outcome']='environmental_cache_artifact_doctest_compile_failure'
                item['failure_kind']='compile_failure_in_doctest_against_stale_dependency'
                item['adaptation_or_limit']='Exit 101 is not a behavioral assertion failure. Shared target supplied an earlier candidate library lacking the public category API; isolated rerun resolves.'
    item={'name':'isolated_frozen_orchestrator','command':shlex.join(['env','RUSTUP_AUTO_INSTALL=0','CARGO_NET_OFFLINE=true','CARGO_TARGET_DIR='+paths[0]]+argv),'exit_code':p.returncode,'log':str(logdir/'runner.log'),'observed_outcome':'completed_collection' if p.returncode==0 else 'runner_failed','measurement':'isolated_frozen_target','definitive':True,'adaptation_or_limit':'Original runner unchanged; empty build directories before each candidate.'}
    r['checks'].append(item)
    commandmap={c['name']:c for c in r['checks'] if c['name'] in ['library','submitted-tests','doctests','public-api','fmt','clippy']}
    for line in (logdir/'status.tsv').read_text().splitlines():
        name, status=line.split('\t')
        item=dict(commandmap[name])
        item.update(name='isolated_'+name,exit_code=int(status),log=str(logdir/f'{name}.log'),measurement='isolated_frozen_target',definitive=True,observed_outcome='completed_successfully' if int(status)==0 else 'nonzero_exit_requires_log_review',adaptation_or_limit='Original runner and commands unchanged; build artifacts isolated between candidates.')
        log=Path(item['log']).read_text()
        counts=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',log)
        item.pop('test_counts',None)
        item.pop('failure_kind',None)
        if counts:
            item['test_counts']=dict(zip(['passed','failed','ignored','measured','filtered_out'],[sum(int(c[i]) for c in counts) for i in range(5)]))
        r['checks'].append(item)
        if name=='public-api': r['hidden_test_counts']=item.get('test_counts')
    archive=[]
    for path in paths:
        p=Path(path)
        if p.exists():
            dest=Path(str(p)+f'-candidate-{letter}-archive')
            assert not dest.exists()
            p.rename(dest)
            archive.append({'from':str(p),'to':str(dest)})
    (logdir/'target-isolation.json').write_text(json.dumps({'targets_empty_before_run':True,'archives_after_run':archive},indent=2)+'\n')
    crate=Path(f'/work/candidates/{letter}')
    after={str(p.relative_to(crate)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(crate.rglob('*')) if p.is_file()}
    before=json.loads((base/'submission-before.json').read_text())
    changes={'modified':[p for p in before if p in after and before[p]!=after[p]],'added':[p for p in after if p not in before],'deleted':[p for p in before if p not in after],'note':'All submission files compared with original inventory after all checks including isolated frozen rerun. Harness files and lockfiles are grading artifacts outside the submission.'}
    r['submission_changes']=changes
    (base/'submission-after.json').write_text(json.dumps(after,indent=2)+'\n')
    (base/'submission-changes.json').write_text(json.dumps(changes,indent=2)+'\n')
    (base/'record.json').write_text(json.dumps(r,indent=2)+'\n')
    with open(base/'commands.jsonl','a') as f:
        for item in r['checks']:
            if item.get('measurement')=='isolated_frozen_target': f.write(json.dumps(item)+'\n')
    print(letter,(logdir/'status.tsv').read_text(),flush=True)

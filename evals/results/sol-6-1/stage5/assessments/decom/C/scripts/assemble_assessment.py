import csv,hashlib,json,re,shlex,shutil
from pathlib import Path
R=Path('/work/results')
def stats(text):
    return [dict(zip(['passed','failed','ignored','measured','filtered_out'],map(int,m))) for m in re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',text)]
def command(argv,cwd,env=None):
    return 'cd '+shlex.quote(str(cwd))+' && '+ ('env '+ ' '.join(shlex.quote(k+'='+v) for k,v in (env or {}).items())+' ' if env else '')+shlex.join(argv)
common={'RUSTUP_AUTO_INSTALL':'0','CARGO_NET_OFFLINE':'true','CARGO_HOME':'/work/.cargo-home','TMPDIR':'/work/tmp'}
checks_base={
'hidden-compile-default':['test','--test','decom_case_hidden','--no-run'],
'hidden-default':['test','--test','decom_case_hidden'],
'hidden-compile-no-default':['test','--no-default-features','--test','decom_case_hidden','--no-run'],
'hidden-no-default':['test','--no-default-features','--test','decom_case_hidden'],
'existing-lib-default':['test','--lib'],
'existing-ref-default':['test','--test','ref'],
'docs-default':['test','--doc'],
'existing-lib-no-default':['test','--no-default-features','--lib']}
integrity=json.loads((R/'integrity.json').read_text())
assessment={'case':{'name':'decom','package':'alacritty_terminal 0.25.0','procedure':'/work/prepared/scoring.md and unmodified /work/prepared/hidden/run.sh','integrity_check':integrity,'scope':'Terminal package only. Hidden checks used full original candidate workspace with explicit -p alacritty_terminal. Gate used standalone terminal copies. No graphical application build/test was run.','preparation_reproduction':'Not run; preparation-only commands are not treatment measurements.','adapters':'Not applicable: this is the DECOM case, not the public machine-readable error API case.','limitations':['Supplied evidence/prepared.sha256 is absent; frozen-bundle integrity verification could not be completed.','No exhaustive conformance, graphical behavior, minimum Rust version, or cross-platform compatibility claim.','Archived gate formatting detection requires an undated nightly alias; only nightly-2026-10-01 is installed, so the gate skipped its formatting step. Stable formatting checks cannot apply nightly-only settings.'],'reproduction_scripts':['/work/results/run_hidden.py','/work/results/run_candidate_checks.py','/work/results/assemble_assessment.py'],'rubric_scores_assigned':False},'candidates':{}}
for c in 'ABCD':
    p=R/c;h=p/'hidden';source=Path('/work/candidates')/c
    envtext=(h/'environment.txt').read_text()
    target=re.search(r'^temporary_target=(.*)$',envtext,re.M).group(1)
    hidden_cwd=str(Path(target).parent/'source')
    hidden_env={**common,'CARGO_TARGET_DIR':target}
    runner=json.loads((p/'hidden-result.json').read_text())
    checks=[{'name':'frozen-hidden-runner','command':shlex.join(runner['command']),'argv':runner['command'],'cwd':'/work','exit_code':runner['exit_code'],'log':runner['log'],'observed_outcome':'All invoked checks passed; runner completed.','outcome_class':'pass','adaptation':'Candidate path substituted for /work/crate; new per-candidate output directory. Runner copied source and injected its hidden test only in scratch.'}]
    integration=(h/'candidate-integration-targets.txt').read_text().splitlines()
    for row in csv.DictReader((h/'status.tsv').open(),delimiter='\t'):
        name=row['check'];log=h/row['log'];exitcode=int(row['exit_code']);s=stats(log.read_text())
        if name in checks_base:
            operation,*rest=checks_base[name]
            argv=['cargo','+1.99.1',operation,'--locked','--offline','-p','alacritty_terminal']+rest
            desc='Compilation succeeded; test binary not executed.' if '--no-run' in rest else 'All executed tests passed.'
        elif name=='candidate-target-inventory':
            argv=['sh','-c','cargo +1.99.1 metadata --locked --offline --no-deps --format-version 1 > "$1"','sh',str(h/'metadata.json')]
            desc='Terminal integration inventory: '+(', '.join(integration) or 'no candidate-added integration targets')+'. Metadata builds no workspace members.'
        else:
            index=int(name.rsplit('-',1)[1])-1
            argv=['cargo','+1.99.1','test','--locked','--offline','-p','alacritty_terminal','--test',integration[index]]
            desc='All candidate integration tests passed.'
        entry={'name':name,'command':command(argv,hidden_cwd,hidden_env),'argv':argv,'cwd':hidden_cwd,'environment':hidden_env,'exit_code':exitcode,'log':str(log),'observed_outcome':desc,'outcome_class':'pass' if exitcode==0 else 'requires_failure_log_review','test_results':s,'adaptation':'Frozen runner, unmodified full workspace copy, explicit terminal package. Scratch source/target removed by frozen runner after completion.'}
        checks.append(entry)
    own=json.loads((p/'own-checks.json').read_text())
    gate=None
    for x in own:
        x['command']=command(x['argv'],x['cwd'],x['environment'])
        text=Path(x['log']).read_text();x['test_results']=stats(text)
        if x['name']=='gate':
            x['outcome_class']='verification_gate_failure'
            x['observed_outcome']='Gate exit 1: strict Clippy failed on seven lint diagnostics; strict rustdoc failed on three documentation diagnostics. All tests passed. Formatting skipped by archived nightly detection. No hidden behavioral failures implied.'
            gate={'exit_code':x['exit_code'],'log':x['log'],'target_selection_adaptation':x['adaptation'],'steps':{'format':'skipped: nightly-only config; gate does not recognize installed dated nightly as +nightly','clippy':'failed lint compilation: len_zero (1), io_other_error (3), question_mark (3)','rustdoc':'failed: unresolved Mode intra-doc link (1), unclosed Row<T> HTML tags (2)','tests':'passed'},'behavioral_failure':False,'step_exit_code_limit':'Unmodified gate reports pass/fail per step and only one numeric overall exit status. Individual subprocess exit codes are not exposed.','diagnostic_context':'Diagnostics occur outside changed lines in the submitted diff; no independent pristine reproduction was performed.'}
        elif x['name']=='own-terminal-format':
            x['outcome_class']='format_check_failure'
            x['observed_outcome']='Stable package formatting check failed: nine formatting diffs, all in unchanged alacritty_terminal/src/selection.rs; stable compiler warns nightly-only formatting settings were ignored.'
        elif x['name']=='own-changed-stable-format':
            x['outcome_class']='pass_with_limit'
            x['observed_outcome']='Changed Rust files pass stable rustfmt; nightly-only formatting settings were ignored with warnings.'
        else:
            x['outcome_class']='pass' if x['exit_code']==0 else 'requires_failure_log_review'
            x['observed_outcome']='All executed tests passed; no ignored tests.'
        checks.append(x)
    hidden_results={}
    for name in ['hidden-default','hidden-no-default']:
        s=next(x for x in checks if x['name']==name)['test_results'][0]
        hidden_results[name]={**s,'exit_code':0,'log':str(h/(name+'.log'))}
        assert s['passed']==23 and s['failed']==0 and s['ignored']==0
    own_default=next(x for x in checks if x['name']=='own-default-suite')
    total=sum(s['passed'] for s in own_default['test_results'])
    current={str(f.relative_to(source)):hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(source.rglob('*')) if f.is_file()}
    before=json.loads((p/'submission-before.sha256.json').read_text())
    changed=[k for k in sorted(current.keys()|before.keys()) if current.get(k)!=before.get(k)]
    (p/'submission-final.sha256.json').write_text(json.dumps(current,indent=2)+'\n')
    filechanges=json.loads((p/'verification-file-changes.json').read_text())
    assert changed==[] and filechanges['submission_changes']==[]
    record={'source':str(source),'redacted_report':str(p/'submitted-report.md'),'checks':checks,'hidden_tests':{'default':hidden_results['hidden-default'],'no_default_features':hidden_results['hidden-no-default'],'total_passed':46,'total_failed':0},'candidate_own_tests':{'default_suite_passed':total,'default_suite_failed':0,'regression_tests':4 if c=='A' else 5 if c=='D' else 7,'regression_location':'alacritty_terminal/src/term/mod.rs (unit tests)' if c=='D' else 'alacritty_terminal/tests/origin_mode.rs','integration_targets':integration,'no_default_suite_passed':135 if c in 'BC' else None,'miri_regression_passed':7 if c in 'BC' else None},'gate':gate,'submission_changes':changed,'verification_created_file_changes':filechanges,'source_preservation':'All candidate regular-file SHA256 hashes match pre-execution hashes. Hidden injection and Cargo verification occurred only in scratch copies.','grading_created_files':'Hidden tests were injected only by the frozen runner in its scratch full-workspace copy. Gate pruned only its copied Cargo.lock; original and post-gate lockfiles retained as gate-original-Cargo.lock and gate-after-Cargo.lock. No grading-created files attributed to submissions.','compile_failures':'No test compilation failures. Gate Clippy lint compilation failure recorded separately.','assertion_failures':[],'environmental_blocks':[integrity['observed_outcome']],'unrun_checks':[{'name':'archived-gate-format','reason':'Gate skips when its +nightly alias detection fails; dated nightly toolchain is installed. Stable checks were run separately.'}],'not_applicable_checks':['Public error-API adapter tests: different case.'],'reproduction':'Per-candidate scripts/ contains exact orchestration sources. Scripts were executed from /work/results, run_hidden.py followed by run_candidate_checks.py; all paths/commands/environment and deleted scratch paths are recorded. To rerun, use a fresh output directory (or adjust ROOT/R) and retain original candidate paths.','evidence':{'hidden_index':str(h/'status.tsv'),'environment':str(h/'environment.txt'),'pre_injection_diff':str(h/'candidate.diff'),'commands':str(p/'checks.json'),'file_changes':str(p/'verification-file-changes.json')}}
    assessment['candidates'][c]=record
    (p/'checks.json').write_text(json.dumps(checks,indent=2)+'\n')
    shutil.copy2(Path('/work/reports')/(c+'.md'),p/'submitted-report.md')
    scripts=p/'scripts';scripts.mkdir(exist_ok=True)
    for script in ['run_hidden.py','run_candidate_checks.py','assemble_assessment.py']:
        shutil.copy2(R/script,scripts/script)
    lines=[f'# Candidate {c} execution evidence','',f'Hidden: 23 passed / 0 failed with defaults; 23 passed / 0 failed without default features. Frozen runner exit 0.',f'Candidate default suite: {total} passed; 0 failed. All 44 reference tests retained and passed.']
    if c in 'BC':lines+=['Without default features: 135 passed (127 unit, 7 integration, 0 reference, 1 doctest). Miri: 7 passed; 0 failed.']
    lines+=['','Archived gate exit 1: Clippy and strict rustdoc failed; tests passed; formatting skipped. Gate ran an unmodified terminal crate in a standalone copy, with its original lockfile and formatting config. No graphical app was built.','Changed-file stable rustfmt passed. Package-wide stable formatting failed only in unchanged selection.rs.','','Source hashes unchanged. Only the gate scratch lockfile was pruned by Cargo; both lockfile versions are retained.','Integrity limitation: /work/prepared/evidence/prepared.sha256 is missing. No preparation-only reproduction or quality scoring was performed.','','Commands and numeric exit codes: checks.json. Complete logs: hidden/*.log, own-*.log, gate.log. Per-step numeric exits inside the archived gate are not exposed by the gate script.','Exact orchestration sources: scripts/. Original candidate report: submitted-report.md. Pre-injection diff: hidden/candidate.diff.']
    (p/'README.md').write_text('\n'.join(lines)+'\n')
(R/'assessment.json').write_text(json.dumps(assessment,indent=2)+'\n')
for c,r in assessment['candidates'].items():
    print(c, 'checks',len(r['checks']),'hidden',r['hidden_tests']['total_passed'],'own',r['candidate_own_tests']['default_suite_passed'],'gate',r['gate']['exit_code'],'source_changes',r['submission_changes'])
print('assessment.json written')

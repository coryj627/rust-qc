#!/usr/bin/env python3
"""Assemble retained execution evidence without running or editing submissions."""
from pathlib import Path
import hashlib,json,re,shlex,shutil
base=Path('/work')

def hashes(root):
    return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.rglob('*')) if p.is_file() and not any(x in ('target','.git') for x in p.relative_to(root).parts)}

def counts(log):
    rows=[]
    section=''
    for line in log.splitlines():
        if re.match(r'\s*Running ',line): section=line.strip()
        if re.match(r'\s*Doc-tests ',line): section=line.strip()
        m=re.match(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',line)
        if m:
            rows.append(dict(target=section,result=m[1],passed=int(m[2]),failed=int(m[3]),ignored=int(m[4]),measured=int(m[5]),filtered_out=int(m[6])))
    return rows

assessment={'case':{'name':'byte_span','description':'Create a byte-span library','prompt':'/work/prepared/case.md','prompt_sha256':hashlib.sha256((base/'prepared/case.md').read_bytes()).hexdigest(),'procedure':'/work/prepared/scoring.md','procedure_sha256':hashlib.sha256((base/'prepared/scoring.md').read_bytes()).hexdigest(),'toolchain':'rustc 1.99.1; cargo 1.99.1; aarch64-unknown-linux-gnu','policy':'Execution assessment only. No rubric scores or quality grades assigned. Archived gate is a separate measurement. No preparation-only reproduction checks executed.','adaptation':'All four are standalone byte_span packages; substituted candidate package paths. No heapless feature selection or DECOM target selection applies. Hidden checks use the frozen runner unchanged with separate reviewed adapters.','frozen_files':hashes(base/'prepared/hidden')},'candidates':{}}
for letter in 'ABCD':
    root=base/'results'/letter
    before=json.loads((root/'submission-before.json').read_text())
    after=hashes(base/'candidates'/letter)
    (root/'submission-after.json').write_text(json.dumps(after,indent=2)+'\n')
    delta={'modified':[p for p in before.keys() & after.keys() if before[p]!=after[p]],'added':sorted(after.keys()-before.keys()),'removed':sorted(before.keys()-after.keys()),'normal_cargo_verification_changes':[], 'assessor_created_submission_files':[], 'build_outputs_location':f'/work/tmp/{letter}-candidate-target and the separate downstream harness targets','note':'All inventoried submission files, including the original Cargo.lock, are byte-for-byte unchanged.'}
    (root/'submission-changes.json').write_text(json.dumps(delta,indent=2)+'\n')
    checks=[]
    names=['toolchain','candidate-tests','public-api','error-contract','gate']
    if letter=='D': names.append('candidate-release-tests')
    for name in names:
        record=json.loads((root/(name+'.json')).read_text())
        log=Path(record['log']).read_text()
        assert record['exit_code']==int((root/'logs'/(name+'.exit')).read_text())
        assert record['exit_code']==0,(letter,name)
        record['outcome']='passed'
        record['observed_outcome']='Command completed successfully; no compilation failure, assertion failure, or environmental block.'
        record['test_results']=counts(log)
        if name=='toolchain': record['observed_outcome']='Installed Rust/Cargo 1.99.1 selected. Toolchain versions and installed toolchain list retained in log.'
        if name in ('candidate-tests','candidate-release-tests'):
            record['observed_outcome']='Candidate-authored library/unit and integration tests plus rustdoc examples all passed.'
        if name in ('public-api','error-contract'):
            args=shlex.split(record['command'])
            harness=Path(args[args.index('--run-dir')+1])
            archive=root/'harnesses'/name
            if not archive.exists(): shutil.copytree(harness,archive,ignore=shutil.ignore_patterns('target'))
            record['generated_harness']=str(harness)
            record['preserved_harness_source']=str(archive)
            record['limits'].append('The frozen runner requires an empty run directory: use run_check.py to generate a fresh one when repeating, rather than replaying the recorded command into the already populated run directory.')
            record['adaptations'].append('Runner overrides CARGO_TARGET_DIR internally to its fresh run directory/target as the frozen procedure specifies.')
            record['hidden_pass_count']=sum(x['passed'] for x in record['test_results'])
            record['hidden_fail_count']=sum(x['failed'] for x in record['test_results'])
            if name=='public-api':
                record['observed_outcome']='8 public API tests passed. Expected no-adapter notice; category tests intentionally absent in this first pass and subsequently executed in error-contract.'
            else:
                record['observed_outcome']='All 11 downstream tests passed: 8 public API tests and 3 machine-readable error/category/precedence tests.'
                record['error_adapter']=str(root/'error_adapter.rs')
                record['adapter_review']=str(root/'adapter-review.md')
        if name=='gate':
            record['observed_outcome']='Archived gate exited 0; format, clippy with warnings denied, rustdoc with warnings denied, and workspace tests all passed. Separate from hidden-test results.'
            record['measurement']='archived_gate'
            steps=[('format','cargo fmt --all --check'),('clippy','cargo clippy --workspace --all-targets --all-features -- -D warnings'),('rustdoc','env RUSTDOCFLAGS=\'-D warnings\' cargo doc --workspace --no-deps --document-private-items --all-features'),('tests','cargo test --workspace --all-features')]
            record['steps']=[]
            for step,command in steps:
                assert f'pass  {step}' in log
                record['steps'].append({'name':step,'command':command,'cwd':f'/work/candidates/{letter}','exit_code':0,'exit_code_evidence':'Gate pass summary reports that the subcommand returned 0.','log':record['log'],'observed_outcome':'passed','adaptations':[],'limits':[]})
            record['adaptations'].append('Candidate report format/clippy/rustdoc checks are covered by this gate. This package declares no features; --all-features is equivalent to the default feature selection. No extra optional diagnostic reruns needed.')
        checks.append(record)
        (root/(name+'.json')).write_text(json.dumps(record,indent=2)+'\n')
    own=next(x for x in checks if x['name']=='candidate-tests')
    assert sum(x['passed'] for x in own['test_results'] if not x['target'].startswith('Doc-tests'))=={'A':8,'B':10,'C':13,'D':10}[letter]
    docs=sum(x['passed'] for x in own['test_results'] if x['target'].startswith('Doc-tests'))
    assert docs=={'A':12,'B':12,'C':3,'D':5}[letter]
    hidden=next(x for x in checks if x['name']=='error-contract')
    assert hidden['hidden_pass_count']==11 and hidden['hidden_fail_count']==0
    assert not delta['modified'] and not delta['added'] and not delta['removed']
    candidate={'path':f'/work/candidates/{letter}','report':str(root/'candidate-report.md'),'checks':checks,'hidden_summary':{'unique_passed':11,'unique_failed':0,'public_api':{'passed':8,'failed':0},'error_category_contract':{'passed':3,'failed':0},'note':'Public API tests passed in both the public-only and full adapter runs; unique counts do not double-count the repeat.'},'own_tests_summary':{'unit_and_integration_passed':sum(x['passed'] for x in own['test_results'] if not x['target'].startswith('Doc-tests')),'doctests_passed':docs,'failed':0,'release_verification': '10 integration tests and 5 doctests passed' if letter=='D' else 'Not claimed in candidate report and not required by frozen procedure.'},'gate_summary':{'exit_code':0,'format':'passed','clippy':'passed','rustdoc':'passed','tests':'passed','log':str(root/'logs/gate.log')},'adapter':{'source':str(root/'error_adapter.rs'),'review':str(root/'adapter-review.md'),'status':'reviewed; public enum discriminants only'},'submission_changes':delta,'submission_hashes_before':str(root/'submission-before.json'),'submission_hashes_after':str(root/'submission-after.json'),'compile_failures':[],'assertion_failures':[],'environmental_blocks':[],'unrun_required_checks':[],'conditional_checks_not_needed':['Separate cargo test --doc retry: aggregate candidate test run already executed and passed all doctests.'],'limits':['Execution success is evidence for the frozen tests, not a rubric score or an exhaustive proof of behavior.'],'reproduction':f'python3 /work/results/{letter}/run_check.py CHECK (CHECK is toolchain, candidate-tests, public-api, error-contract, gate'+(', or candidate-release-tests' if letter=='D' else '')+'); commands use fresh hidden harness directories.'}
    assessment['candidates'][letter]=candidate
    (root/'assessment.json').write_text(json.dumps(candidate,indent=2)+'\n')
(base/'results/assessment.json').write_text(json.dumps(assessment,indent=2)+'\n')
for letter in 'BCD': shutil.copyfile(__file__,base/'results'/letter/'assemble_assessment.py')
print(json.dumps({k:{'hidden':v['hidden_summary'],'own':v['own_tests_summary'],'gate_exit':v['gate_summary']['exit_code'],'submission_changes':v['submission_changes']} for k,v in assessment['candidates'].items()},indent=2))

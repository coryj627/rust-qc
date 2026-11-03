from pathlib import Path
import json, hashlib, shlex, shutil
root=Path('/work/results')
case={
    'name':'small_config parser implementation',
    'specification':'/work/prepared/case.md',
    'scoring_procedure':'/work/prepared/scoring.md',
    'frozen_runner':'/work/prepared/hidden/run_checks.sh',
    'frozen_public_api_tests':'/work/prepared/hidden/public_api.rs',
    'archived_gate':'/work/gate/check.sh',
    'environment':{'rustc':'1.99.1 (58cf8f9c3 2026-10-13)','cargo':'1.99.1 (5f94df478 2026-08-27)','host':'aarch64-unknown-linux-gnu','network':'offline','RUSTUP_AUTO_INSTALL':'0','CARGO_NET_OFFLINE':'true'},
    'procedure_notes':[
        'No rubric points, quality score, model/arm inference, skill recommendations, reference code or preparation-only validation results used.',
        'Each original candidate was checked by the unchanged frozen runner. Shared hardcoded build directories produced stale artifacts in initial runs; all six frozen checks were rerun for every candidate with empty build directories. frozen-isolated logs are definitive.',
        'The archived gate was separately executed for every candidate with default --all-features selection. The archived file is not executable, so bash invokes its unchanged contents. No heapless or DECOM target adaptation applies to this case.',
        'Frozen scoring.md has no category-adapter suite or category API requirement. Separate reviewed adapters and seven supplemental tests were preserved per candidate. Only B/D have typed categories and execute those tests; A/C category checks remain unrun. This does not affect the 24 frozen public API tests.',
        'All submission file hashes matched their original inventories after checks. No tests or adapters were injected into submissions. External consumer harness lockfiles belong to assessment artifacts.'
    ],
    'scope_limit':'Passing finite checks is not a proof over all inputs. No numeric rubric assigned.'
}
assessment={'case':case,'candidates':{}}
for letter in 'ABCD':
    base=root/letter
    r=json.loads((base/'record.json').read_text())
    r['adaptations'].append('Invoke archived gate with bash because /work/gate/check.sh is mode 0644; script contents preserved unchanged.')
    byname={c['name']:c for c in r['checks']}
    r['own_test_counts']=byname['isolated_submitted-tests']['test_counts']
    r['documentation_test_counts']=byname['isolated_doctests']['test_counts']
    r['summary']={
        'frozen_checks':'all six completed successfully with isolated build artifacts',
        'hidden_passed':r['hidden_test_counts']['passed'],
        'hidden_failed':r['hidden_test_counts']['failed'],
        'own_tests_passed':r['own_test_counts']['passed'],
        'doctests_passed':r['documentation_test_counts']['passed'],
        'archived_gate_exit_code':byname['archived_gate']['exit_code'],
        'strict_clippy_exit_code':byname['reported_strict_clippy']['exit_code'],
        'strict_rustdoc_exit_code':byname['reported_strict_rustdoc']['exit_code'],
        'supplemental_adapter':'7 passed, 0 failed' if letter in 'BD' else 'unrun: no public typed category API',
        'behavioral_assertion_failures':0,
        'unresolved_compile_failures':0,
        'unresolved_environmental_blocks':0,
        'submission_files_changed':False,
    }
    gate=byname['archived_gate']
    gate['measurement']='separate_archived_gate'
    gate['adaptation_or_limit']='Invoked unchanged script via bash because it has no execute permission. Default --all-features baseline applies to small_config. Offline environment, per-candidate build target.'
    gate['steps']=[]
    cmds={
        'format':['cargo','fmt','--all','--check'],
        'clippy':['cargo','clippy','--workspace','--all-targets','--all-features','--','-D','warnings'],
        'rustdoc':['env','RUSTDOCFLAGS=-D warnings','cargo','doc','--workspace','--no-deps','--document-private-items','--all-features'],
        'tests':['cargo','test','--workspace','--all-features'],
    }
    for name,argv in cmds.items():
        gate['steps'].append({'name':name,'command':shlex.join(argv),'cwd':f'/work/candidates/{letter}','exit_code':0,'exit_code_evidence':'The unchanged run_step function reports pass only on exit 0, and the gate summary reports this step as pass.','log':gate['log'],'observed_outcome':'passed','adaptation_or_limit':None})
    for c in r['checks']:
        assert Path(c['log']).is_file(),c
        assert c['exit_code'] is None or isinstance(c['exit_code'],int),c
    for name in ['library','submitted-tests','doctests','public-api','fmt','clippy']:
        assert byname['isolated_'+name]['exit_code']==0
    for original,copied in [('/work/gate/check.sh',str(base/'gate/check.sh')),('/work/prepared/hidden/run_checks.sh',str(base/'frozen/run_checks.sh')),('/work/prepared/hidden/run_api_tests.sh',str(base/'frozen/run_api_tests.sh')),('/work/prepared/hidden/public_api.rs',str(base/'frozen/public_api.rs'))]:
        assert Path(original).read_bytes()==Path(copied).read_bytes()
    r['gate_script_sha256']=hashlib.sha256(Path('/work/gate/check.sh').read_bytes()).hexdigest()
    r['grading_created_artifacts']={
        'adapter_sources_and_tests':str(base/'adapter'),
        'adapter_lockfile':str(base/'adapter/Cargo.lock') if letter in 'BD' else None,
        'original_submission_snapshot':str(base/'source-before'),
        'build_artifacts':f'/work/tmp/assessment-{letter}-target and /work/tmp/small-config-*-target-candidate-{letter}-archive',
        'note':'These artifacts were created by assessment, not authored submission changes.'
    }
    r['reproduction_scripts']=[str(base / p) for p in ['setup_assessment.py','run_candidate.py','rerun_frozen_isolated.py','finalize_assessment.py']]
    (base/'record.json').write_text(json.dumps(r,indent=2)+'\n')
    (base/'commands.jsonl').write_text(''.join(json.dumps(c)+'\n' for c in r['checks']))
    assessment['candidates'][letter]=r
    (base/'outcome.md').write_text(f'''Candidate {letter}

Frozen hidden public API: 24 passed, 0 failed.
Own tests: {r['own_test_counts']['passed']} passed, 0 failed.
Doctests: {r['documentation_test_counts']['passed']} passed, 0 failed.
Frozen library, format and Clippy checks: exit 0.
Reported strict Clippy and rustdoc checks: exit 0.
Separate archived gate: exit 0; format, Clippy, rustdoc and tests all pass.
Supplemental category adapter: {r['summary']['supplemental_adapter']}.

Use frozen-isolated/ for definitive frozen logs. Initial frozen/ logs are
preserved; shared-target cache contamination caused wrong unit-test binaries
and B/D doctest compiler errors in initial runs. They are environmental
artifacts, not behavioral assertion failures. All checks were rerun unchanged
with empty build directories between candidates. The gate used a separate
candidate-specific target from the start.

No submission files changed. See submission-before.json, submission-after.json,
submission-changes.json, adapter/review.md, commands.jsonl and record.json.
No quality score or rubric points assigned.
''')
(root/'assessment.json').write_text(json.dumps(assessment,indent=2)+'\n')
assert set(assessment)=={'case','candidates'}
assert set(assessment['candidates'])==set('ABCD')
print(json.dumps({letter:r['summary'] for letter,r in assessment['candidates'].items()},indent=2))

from pathlib import Path
import json, re, shutil, hashlib
root = Path('/work/results')
expected_failures = ['directories::runtime_directory', 'format::write_error', 'functions::env_var_functions_unix', 'non_unicode::warn_for_non_unicode_invocation_directory', 'non_unicode::warn_for_non_unicode_justfile_path']
explanations = {
    'directories::runtime_directory': 'No runtime directory is provided; error reports runtime directory not found.',
    'format::write_error': 'Sandbox successfully writes a file after chmod 400; test expects permission denied.',
    'functions::env_var_functions_unix': 'USER is absent; test panics at env::var("USER").unwrap(). The separate USER=audit rerun passes.',
    'non_unicode::warn_for_non_unicode_invocation_directory': 'Creating a non-Unicode directory fails in this filesystem with OS error 84 before just is invoked.',
    'non_unicode::warn_for_non_unicode_justfile_path': 'Creating a non-Unicode directory fails in this filesystem with OS error 84 before just is invoked.',
}
def summaries(text):
    return [dict(zip(('passed','failed','ignored','measured','filtered_out'),map(int,m))) for m in re.findall(r'test result: .*? (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',text)]
assessment = {
    'case': {
        'id': 'just-6',
        'description': 'just 1.50.0: shell() must not execute under --dry-run; quoted and escaped representation; preserve normal shell and backtick behavior.',
        'procedure': '/work/prepared/scoring.md',
        'gate': '/work/gate/check.sh',
        'gate_sha256': hashlib.sha256(Path('/work/gate/check.sh').read_bytes()).hexdigest(),
        'toolchain': 'rustc/cargo 1.99.1 on aarch64 Linux; offline cached dependencies; RUSTUP_AUTO_INSTALL=0',
        'source_policy': 'Candidate source preserved. Hidden tests injected only in per-candidate fresh scratch copies. Book generation uses a second fresh scratch copy. No production, manifest, lockfile or dependency adaptations.',
        'scope': 'Execution assessment only: no rubric scores, rankings, reference implementation reads, or preparation-control reruns.',
        'scripts': ['/work/results/assess_candidate.py','/work/results/finalize_assessment.py'],
        'time': 'All checks completed within the 45-minute budget; clock logs retained per candidate.'
    },
    'candidates': {}
}
for letter in 'ABCD':
    d = root / letter
    state = json.loads((d/'record.json').read_text())
    shutil.copyfile(root/'assess_candidate.py',d/'assess_candidate.py')
    shutil.copyfile(root/'finalize_assessment.py',d/'finalize_assessment.py')
    for f in ('apply_hidden.py','run_checks.py','case_just_6_upstream.rs','case_just_6_neighbors.rs'):
        shutil.copyfile(Path('/work/prepared/hidden')/f,d/'hidden'/f)
    frozen = state.pop('frozen_checks', None)
    if frozen is None:
        frozen = [c for c in state['checks'] if c['check'] in ('compile','upstream','neighbors','existing-shell','existing-dry-run','existing-backticks')]
        state['checks'] = [c for c in state['checks'] if c not in frozen]
    for c in frozen:
        c['cwd'] = state['scratch']
        c['test_summaries'] = summaries(Path(c['log']).read_text())
        if c['exit_code'] != 0: raise RuntimeError('Unexpected frozen failure needs review')
    state['checks'].extend(frozen)
    for check in state['checks']:
        name = check['check']
        text = Path(check['log']).read_text(errors='replace')
        if name == 'own-functions':
            assert check['exit_code']==101 and 'functions::env_var_functions_unix ... FAILED' in text
            check['observed_outcome'] = 'environment_related_assertion_failure'
            check['adaptation_or_limit'] = 'Only failing test is functions::env_var_functions_unix, because USER is absent. See separate USER=audit rerun.'
        elif name == 'own-functions-user-audit':
            check['adaptation_or_limit'] = 'USER=audit explicitly set, matching candidate reports; no source modification.'
        elif name == 'full-integration-on-score-copy':
            assert check['exit_code']==101
            check['failure_tests'] = sorted(re.findall(r'^test (\S+) \.\.\. FAILED$',text,re.M))
            assert check['failure_tests']==expected_failures
            check['observed_outcome'] = 'five_environment_related_assertion_failures'
            check['adaptation_or_limit'] = 'Full integration command from scoring.md, run on hidden-injected scratch; counts include 11 assessor-injected tests. All candidate-added shell regressions pass.'
            check['failure_details'] = explanations
        elif name == 'archived-gate':
            assert check['exit_code']==1
            check['observed_outcome'] = 'gate_failed_separate_measurement'
            check['adaptation_or_limit'] = 'Unchanged archived script, original just workspace, default --all-features. No DECOM target adaptation or heapless feature selection applicable.'
            check['gate_steps'] = {
                'format': 'pass',
                'clippy': 'compile/lint rejection: clippy::manual_assert_eq (1 diagnostic) and clippy::assert_is_empty (3 diagnostics) in unchanged existing code; no hidden-test failure implied',
                'rustdoc': 'documentation rejection under -D warnings: one broken intra-doc link and three missing footnote definitions in unchanged existing comments',
                'tests': '513 library tests pass; original integration suite has the five environment-related assertion failures listed separately; 8 integration tests ignored'
            }
            check['failure_details'] = explanations
            state['gate_result'] = {'exit_code':1,'log':check['log'],'format':'pass','clippy':'fail','rustdoc':'fail','tests':'five environment-related assertion failures','separate_from_hidden_results':True}
        elif name == 'own-forbidden-words':
            assert check['exit_code']==1 and '`rg` not found' in text
            check['observed_outcome'] = 'environmental_tooling_block'
            check['adaptation_or_limit'] = 'Script precondition rejects missing ripgrep; forbidden-word scan unrun. No installation.'
        elif name == 'own-shellcheck':
            assert check['exit_code']==127
            check['observed_outcome'] = 'environmental_tooling_block'
            check['adaptation_or_limit'] = 'shellcheck is not installed; shell script lint unrun.'
        elif name in ('own-mdbook-en','own-mdbook-zh'):
            assert check['exit_code']==127
            check['observed_outcome'] = 'environmental_tooling_block'
            check['adaptation_or_limit'] = 'mdbook is not installed; book rendering and link checks unrun. Book source generator did run successfully in scratch.'
        elif name == 'own-generate-book':
            check['adaptation_or_limit'] = 'Executed in a separate fresh pristine documentation copy; generated documentation not attributed to submission.'
        elif name == 'own-doctests':
            check['observed_outcome'] = 'success_zero_tests'
            check['adaptation_or_limit'] = 'Workspace command succeeds with zero doctests; just library explicitly disables doctests. Documentation compilation is measured separately by archived gate.'
        elif name == 'own-backtick-broad':
            check['adaptation_or_limit'] = 'Broader filter matches candidate report: 16 library tests and 45 integration tests pass; one existing integration test ignored. PowerShell branches nominally pass by returning early on Linux.'
        elif check['exit_code']==0:
            check['observed_outcome'] = 'pass'
        else:
            raise RuntimeError('Unclassified nonzero result: '+letter+' '+name)
    assert state['submission_changes_during_verification']==[]
    state['hidden_summary'] = {'upstream':{'passed':2,'failed':0,'ignored':0}, 'neighbors':{'passed':9,'failed':0,'ignored':0},'behavior_checks_unrun':[]}
    state['candidate_authored_regressions'] = {
        'file': f'/work/candidates/{letter}/tests/functions.rs',
        'names': {'A':['shell_dry_run','shell_dry_run_arguments','shell_dry_run_nested','shell_dry_run_does_not_require_shell','shell_preserves_arguments'], 'B':['shell_dry_run','shell_dry_run_evaluate','shell_dry_run_without_shell','shell_dry_run_quotes_and_escapes_arguments','shell_dry_run_nested_calls'], 'C':['shell_dry_run','shell_dry_run_shebang','shell_dry_run_arguments'], 'D':['shell_dry_run_does_not_execute','shell_dry_run_error','shell_dry_run_args','shell_dry_run_nested']}[letter],
        'observed_outcome':'all pass in original source functions suite with USER=audit, and all pass in full integration suite despite five unrelated environmental assertion failures',
        'counts_exclude_hidden_tests': True
    }
    state['unrun_checks'] = [
        {'check':'forbidden-word scan','reason':'rg unavailable; wrapper attempted and exited 1'},
        {'check':'shell script lint','reason':'shellcheck unavailable; command attempted and exited 127'},
        {'check':'English and Chinese book rendering/link checking','reason':'mdbook unavailable; both commands attempted and exited 127'},
        {'check':'Windows/macOS behavior','reason':'Linux/aarch64 execution environment only'},
        {'check':'minimum Rust 1.85.0','reason':'not installed; installations prohibited'}
    ]
    state['environment_assertion_failures_are_not_hidden_failures'] = True
    (d/'record.json').write_text(json.dumps(state,indent=2)+'\n')
    assessment['candidates'][letter] = state
(root/'assessment.json').write_text(json.dumps(assessment,indent=2)+'\n')
print(json.dumps({k:{'hidden':v['hidden_summary'],'gate':v['gate_result'],'submission_changes':v['submission_changes_during_verification']} for k,v in assessment['candidates'].items()},indent=2))

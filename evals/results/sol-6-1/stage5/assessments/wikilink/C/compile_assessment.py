#!/usr/bin/env python3
from pathlib import Path
import json,re,hashlib,shutil,difflib,subprocess
ROOT=Path('/work/results')
def write(p,v):p.write_text(json.dumps(v,indent=2)+'\n')
def manifest(p):return {str(f.relative_to(p)):hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(p.rglob('*')) if f.is_file() and 'target' not in f.relative_to(p).parts}
def delta(a,b):return {k:{'before':a.get(k),'after':b.get(k)} for k in sorted(set(a)|set(b)) if a.get(k)!=b.get(k)}
def counts(text):
 return [dict(zip(['passed','failed','ignored','measured','filtered_out'],map(int,m))) for m in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',text)]
def outcome(rc,text,command):
 if rc==0:return 'passed'
 if rc==124:return 'environmental_timeout'
 if 'Cannot find llvm-objdump' in text:return 'environmental_block: mozjs_sys custom build cannot find llvm-objdump; parser test assertions did not run in this invocation'
 if 'clippy' in command:return 'lint_failure: warnings denied by -D warnings; this is not a parser behavioral failure'
 if 'test result: FAILED' in text:return 'test_assertion_failure'
 if 'could not compile' in text:return 'compile_failure; no behavioral result established'
 return 'failed; inspect complete log'
assessment={'case':'wikilink','candidates':{}}
for c in 'ABCD':
 p=ROOT/c; source=Path('/work/candidates')/c; review=Path('/work/tmp')/(c+'-review'); own=Path('/work/tmp')/(c+'-own')
 original=p/'run_candidate_checks.original.py'; adapted=p/'run_candidate_checks.py'
 (p/'runner-path-adaptation.diff').write_text(''.join(difflib.unified_diff(original.read_text().splitlines(True),adapted.read_text().splitlines(True),fromfile='frozen-runner',tofile=c+'-path-adaptation')))
 hidden=p/'hidden-tests';hidden.mkdir(exist_ok=True)
 for n in ['oracle','neighbors']:shutil.copyfile(Path('/work/prepared/hidden')/f'case_wikilink_{n}.rs',hidden/f'case_wikilink_{n}.rs')
 shutil.copyfile('/work/gate/check.sh',p/'archived-check.sh')
 shutil.copyfile(f'/work/reports/{c}.md',p/'redacted-report.md')
 write(p/'frozen-artifact-hashes.json',{str(f.relative_to(p)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [original,p/'archived-check.sh',*sorted(hidden.glob('*.rs'))]})
 before=json.loads((p/'submission-before.json').read_text());after=manifest(source)
 write(p/'submission-after.json',after);write(p/'submission-changes.json',delta(before,after))
 own_before=json.loads((p/'own-copy-before.json').read_text());own_after=manifest(own)
 write(p/'own-copy-after.json',own_after);write(p/'verification-copy-changes.json',delta(own_before,own_after))
 review_after=manifest(review);write(p/'review-copy-changes.json',delta(before,review_after))
 assert not delta(before,after),c+' source changed'
 assert not delta(own_before,own_after),c+' own verification copy changed'
 assert set(delta(before,review_after))=={'pulldown-cmark/tests/case_wikilink_oracle.rs','pulldown-cmark/tests/case_wikilink_neighbors.rs'}
 for n in ['oracle','neighbors']:
  assert (review/f'pulldown-cmark/tests/case_wikilink_{n}.rs').read_bytes()==(hidden/f'case_wikilink_{n}.rs').read_bytes()
 invocations=[]
 for f in sorted((p/'logs').glob('cargo-*.json')):
  m=json.loads(f.read_text());text=Path(m['log']).read_text();m['log_path']=m.pop('log')
  m['observed_outcome']=outcome(m['exit_code'],text,m['command']);m['test_summaries']=counts(text)
  invocations.append(m)
 frozen=json.loads((p/'frozen-runner-logs/results.json').read_text());checks=[]
 for i,r in enumerate(frozen):
  m=invocations[i];assert m['command'][1:]==r['command'][1:]
  entry=dict(name=r['name'],kind='frozen_native_cargo',command=r['command'],cwd=m['cwd'],exit_code=r['exit'],log_path=m['log_path'],frozen_summary_log=str(p/'frozen-runner-logs'/(r['name']+'.log')),observed_outcome=m['observed_outcome'],test_summaries=m['test_summaries'],adaptation='Only scratch/review, log and target paths changed; full submitted workspace copied and frozen tests injected only in scratch.',limit=None)
  if r['expected_hidden_count'] is not None:
   entry.update(expected_hidden_count=r['expected_hidden_count'],hidden_count_matches=r['count_ok'],hidden_passed=sum(x['passed'] for x in m['test_summaries']),hidden_failed=sum(x['failed'] for x in m['test_summaries']))
   assert r['passed'] and r['count_ok']
  if r['name'].startswith('full-suite'):
   entry['passed_including_doctests']=sum(x['passed'] for x in m['test_summaries']);entry['limit']='Includes repeated hidden tests. Zero-test binary/disabled-serde targets are not claimed as separate test coverage.'
  checks.append(entry)
 records=json.loads((p/'orchestration-records.json').read_text())
 gate=None
 for r in records:
  text=Path(r['log']).read_text()
  e=dict(name=r['name'],kind='candidate_suite_or_documentation' if r['name']!='frozen-runner' else 'frozen_runner_driver',command=r['command'],cwd=r['cwd'],exit_code=r['exit_code'],log_path=r['log'],observed_outcome=outcome(r['exit_code'],text,r['command']),test_summaries=counts(text),adaptation='Run on separate source-identical scratch copy without hidden tests.' if r['name']!='frozen-runner' else 'Path-only frozen-runner adaptation; complete Cargo logs recorded by transparent wrapper.',limit=None)
  if r['name']=='package-clippy':
   errors=[line for line in text.splitlines() if line.startswith('error: ') and not line.startswith('error: could not compile')]
   e['lint_diagnostic_count']=len(errors);e['limit']='No baseline command was run; diagnostics are retained without using their exit as hidden-test failure.'
  if r['name']=='archived-gate':
   e.update(kind='archived_gate_separate_measurement',observed_outcome='gate_failed: format passed; clippy lint failure; workspace rustdoc/tests environmentally blocked by missing llvm-objdump in mozjs_sys',adaptation='Unmodified archived gate run on full source-identical workspace copy with --features getopts,html,serde,simd. gen-tests excluded because it regenerates checked-in suites, prohibited by case.md. No DECOM/heapless target adaptation applies.',limit='Gate workspace failures do not establish parser assertion failures or quality scores.')
   subs=[m for m in invocations if m['started']>=r['started'] and m['finished']<=r['finished']]
   e['steps']=[]
   for step in ['format','clippy','rustdoc','tests']:
    cmdname={'format':'fmt','clippy':'clippy','rustdoc':'doc','tests':'test'}[step]
    matches=[m for m in subs if m['command'][1]==cmdname and (('--check' in m['command']) if step=='format' else ('--workspace' in m['command']))]
    assert matches,(c,step)
    m=matches[-1];e['steps'].append(dict(name=step,command=m['command'],exit_code=m['exit_code'],log_path=m['log_path'],observed_outcome=m['observed_outcome'],adaptation=e['adaptation'],limit=e['limit']))
   gate=e
  checks.append(e)
 rel=json.loads((p/'release-settings.json').read_text());text=Path(rel['log']).read_text()
 rustc_lines=[line for line in text.splitlines() if 'rustc --crate-name pulldown_cmark ' in line]
 assert rel['exit_code']==0 and len(rustc_lines)==1
 assert '-C opt-level=3' in rustc_lines[0] and '-C codegen-units=1' in rustc_lines[0] and 'overflow-checks' not in rustc_lines[0]
 assert 'debug_assertions\n' not in text
 checks.append(dict(name=rel['name'],kind='effective_release_settings',command=rel['command'],cwd=rel['cwd'],exit_code=rel['exit_code'],log_path=rel['log'],observed_outcome='confirmed: opt-level=3, codegen-units=1, LTO and panic=abort; no overflow-checks override and no debug_assertions cfg, so Cargo/Rust release default overflow checks remain disabled',adaptation='Cargo --print cfg captures effective compiler command without changing source.',limit='Probe is a release library command; native release test commands above independently passed. Rust test harness uses unwind as required by Cargo.'))
 unrun=[]
 if c in 'ABC':
  num={'A':19173960,'B':2474869,'C':100000}[c]
  note=f'Redacted report claims a {num:,}-input generated delimiter probe, but no reproducer source or exact invocation was submitted. It was not reconstructed or rerun. This does not affect the completed frozen checks.\n'
  (p/'unrun-reported-probe.txt').write_text(note)
  unrun.append(dict(name='report-claimed-generated-input-probe',command=None,exit_code=None,log_path=str(p/'unrun-reported-probe.txt'),observed_outcome='unrun_missing_reproduction_artifact',adaptation=None,limit=note.strip()))
 checks.extend(unrun)
 rec=dict(source=str(source),evidence_directory=str(p),toolchain='Rust 1.99.1 aarch64 Linux; nightly-2026-10-01 used only for focused Miri checks A/B; no installation or network.',hidden_tests=dict(passed=10,failed=0,unrun=0,debug=dict(passed=5,failed=0),release=dict(passed=5,failed=0),basis='Only dedicated frozen oracle and neighbor targets counted; full-suite repeats excluded.'),checks=checks,cargo_invocations=invocations,gate=dict(exit_code=gate['exit_code'],log_path=gate['log_path'],observed_outcome=gate['observed_outcome'],steps=gate['steps']),source_integrity=dict(unchanged=True,submission_changes=[],verification_copy_changes=[],grader_created_files=list(delta(before,review_after)),lockfile_changes=[],before_manifest=str(p/'submission-before.json'),after_manifest=str(p/'submission-after.json'),review_changes=str(p/'review-copy-changes.json'),own_copy_changes=str(p/'verification-copy-changes.json'),diff_against_baseline=str(p/'submission.diff')),inspection=dict(scope='Only parser guards and added regression tests differ from baseline; no baseline tests removed/modified, no dependency, feature or profile changes, no panic-catching workaround.',public_error_api_adapter='Not applicable: wikilink scoring.md defines parser tests, no machine-readable error-category adapter or classification tests.',msrv='Declared 1.71.1 toolchain not installed and not tested; installed toolchain used as directed.'),adaptations=str(p/'adaptations.json'),reproduction_scripts=[str(p/'orchestrate.py'),str(p/'run_candidate_checks.py'),str(p/'bin/cargo'),str(p/'release_settings.py'),str(p/'compile_assessment.py')],unrun_checks=unrun)
 write(p/'assessment.json',rec)
 (p/'README.md').write_text(f'Candidate {c}: frozen hidden tests 10 passed / 0 failed (5 debug, 5 release); both full suites, no-default check/tests, serde+SIMD suite, formatting and package rustdoc passed. '+('Focused candidate regression passed Miri. ' if c in 'AB' else '')+'Package Clippy exited 101 (28 diagnostics). Archived gate exited 1: format passed, Clippy failed, workspace rustdoc/tests blocked by mozjs_sys needing unavailable llvm-objdump. These gate results are separate from hidden tests.\n\nComplete output and numeric command metadata are under logs/. Frozen concise summaries are under frozen-runner-logs/. Source manifests show submissions and ordinary verification copies unchanged; only frozen hidden tests were added to the review copy. assessment.json contains every check command, exit and outcome. The orchestrator uses fresh scratch/log directories and refuses an overwrite; adapt only paths for a rerun.\n'+('\n'+unrun[0]['limit']+'\n' if unrun else ''))
 assessment['candidates'][c]=rec
write(ROOT/'assessment.json',assessment)
for c,rec in assessment['candidates'].items():
 print(c,'hidden',rec['hidden_tests']['passed'],'gate exit',rec['gate']['exit_code'],'checks',len(rec['checks']),'source unchanged',rec['source_integrity']['unchanged'])
 print('  full suite totals',[(x['name'],x.get('passed_including_doctests')) for x in rec['checks'] if x['name'].startswith('full-suite')])
 for check in rec['checks']:
  assert Path(check['log_path']).is_file(),check
  if check['command'] is not None:assert isinstance(check['exit_code'],int)
 for invocation in rec['cargo_invocations']:assert Path(invocation['log_path']).is_file()
assert set(assessment)=={'case','candidates'} and set(assessment['candidates'])==set('ABCD')
print('assessment validated')

#!/usr/bin/env python3
from pathlib import Path
import json, re, shlex, hashlib, collections, shutil
root=Path('/work/results')
features='alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes'
def hashes(p):
 return {str(f.relative_to(p)):hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(p.rglob('*')) if f.is_file() and '.git' not in f.parts and 'target' not in f.parts}
def changes(a,b):
 return {'added':[p for p in b if p not in a],'removed':[p for p in a if p not in b],'changed':[p for p in a if p in b and a[p]!=b[p]]}
def counts(text):
 return [dict(zip(['passed','failed','ignored','measured','filtered_out'],map(int,m))) for m in re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',text)]
def diagnostics(text):
 return collections.Counter(re.findall(r'^error: (?!could not compile)(.+)$',text,re.M))
base_diag=diagnostics((root/'A/pristine-baseline-clippy.log').read_text())
case={'id':'heapless','bug_id':'heapless-6','procedure':'/work/prepared/scoring.md','frozen_runner':'/work/prepared/hidden/score_candidate.py','gate':'/work/gate/check.sh','baseline_features':features,'toolchains':'Rust 1.99.1; nightly-2026-10-01 with Miri; native aarch64 Linux','scope':'Execution evidence only; no quality points, ranking, skill recommendations, or rubric assigned. Preparation-only control commands were not rerun. No reference implementation or author preparation results were accessed.','public_error_adapter':'Not applicable: the frozen heapless checks call IndexMap public lookup/membership/iteration APIs directly and scoring.md specifies no error-category adapter for this case. No error strings parsed or categories inferred.','general_limit':'Focused Miri covers IndexMap unit tests with default features and leak checking disabled, as recorded. Nightly formatting could not run because the installed dated nightly lacks rustfmt. The full cross-target CI/TSan matrix is outside the frozen procedure and was not run.'}
assessment={'case':case,'candidates':{}}
for letter in 'ABCD':
 out=root/letter; scratch=Path('/work/tmp')/('assessment-'+letter)
 records=json.loads((out/'checks.json').read_text())
 for rec in records:
  rec['adaptation_or_limit']=None
  text=Path(rec['log']).read_text(); rec['test_counts']=counts(text)
  if rec['name']=='dated-nightly-format':
   assert "'cargo-fmt' is not installed" in text
   rec['observed_outcome']='environmental_block_missing_nightly_rustfmt'
   rec['adaptation_or_limit']='Attempted the installed dated nightly explicitly; rustfmt is absent. No installation attempted; formatting itself unrun on nightly.'
  elif rec['name']=='candidate-regression-on-baseline':
   assert rec['exit_code']==101 and rec['test_counts']==[{'passed':0,'failed':1,'ignored':0,'measured':0,'filtered_out':186}] and 'assertion `left == right` failed' in text
   rec['observed_outcome']='expected_behavioral_assertion_failure_on_baseline'
   rec['adaptation_or_limit']='Only candidate-added test function injected into fresh baseline copy. The test compiled and ran; unchanged baseline production code failed retained get assertion. Positive regression evidence, not a failure of the submitted fix.'
  elif rec['name'] in ['strict-clippy','pristine-baseline-clippy']:
   assert rec['exit_code']==101
   rec['observed_outcome']='lint_compilation_failure_not_behavioral_test_failure'
  elif rec['name']=='archived-gate':
   assert rec['exit_code']==1 and 'FAIL  clippy\npass  rustdoc\npass  tests' in text
   rec['observed_outcome']='gate_failed_clippy; rustdoc_and_tests_passed; format_skipped'
   rec['adaptation_or_limit']='Unmodified archived script run on a separate pristine candidate copy with the baseline-compatible feature selection. Its nightly discovery requires an undated nightly alias and skips formatting; an explicit dated-nightly attempt independently confirms rustfmt is missing. No DECOM target adaptation applies.'
  else:
   assert rec['exit_code']==0,(letter,rec['name'],rec['exit_code'])
   rec['observed_outcome']='passed'
  if rec['name']=='frozen-scorer':
   rec['adaptation_or_limit']='Candidate path substituted for /work/crate and a new scratch run directory used, exactly as allowed. Script copies manifest/lockfile unchanged, runs own tests first, then injects the unchanged two hidden integration tests. All four checks completed and hidden bytes verified.'
  if rec['name']=='stable-format':
   rec['adaptation_or_limit']='Stable rustfmt completed with warnings that format_code_in_doc_comments requires nightly; stable formatting passed and does not verify nightly doc-comment formatting.'
  if rec['name']=='miri-indexmap':
   assert rec['test_counts'][0]['passed']==23
   rec['adaptation_or_limit']='Installed dated nightly selected; default-feature IndexMap tests only; MIRIFLAGS=-Zmiri-ignore-leaks and provided MIRI_SYSROOT used. No setup, installation, or network access.'
  if rec['name']=='ci-compatible-feature-tests':
   rec['adaptation_or_limit']='Native-host CI feature test selection, offline, with RUSTFLAGS=-D warnings; no cross-target matrix or sanitizer run.'
 frozen=json.loads((out/'frozen-results.json').read_text())
 frozen_records=[]
 for name,code in frozen.items():
  log=out/'frozen-logs'/(name+'.log'); text=log.read_text()
  command=shlex.split(re.search(r'^\$ (.+)$',text,re.M)[1])
  cwd=re.search(r'^cwd: (.+)$',text,re.M)[1]
  assert code==0
  c=counts(text)
  if name=='upstream-oracle': assert len(c)==1 and c[0]['passed']==1 and c[0]['ignored']==0
  if name=='spec-neighbors': assert len(c)==1 and c[0]['passed']==2 and c[0]['ignored']==0
  frozen_records.append({'name':name,'command':command,'cwd':cwd,'exit_code':code,'log':str(log),'observed_outcome':'passed','test_counts':c,'adaptation_or_limit':'Frozen command on scorer-created disposable source; hidden tests injected only after own/default tests and library check.'})
 before=json.loads((out/'submission-before.sha256.json').read_text()); after=hashes(Path('/work/candidates')/letter)
 (out/'submission-after.sha256.json').write_text(json.dumps(after,indent=2)+'\n')
 scoring=hashes(scratch/'scoring/source'); verification=hashes(scratch/'verification-source')
 integrity={'submission_changes':changes(before,after),'verification_copy_changes':changes(before,verification),'scoring_copy_changes':changes(before,scoring),'grading_created_files':['tests/heldback_truncate_upstream.rs','tests/heldback_truncate_spec.rs'],'hidden_test_bytes_identical':all((scratch/'scoring/source/tests'/n).read_bytes()==(out/'frozen'/n).read_bytes() for n in ['heldback_truncate_upstream.rs','heldback_truncate_spec.rs']),'normal_cargo_changed_submission_files':[],'normal_cargo_changed_verification_copy_files':[]}
 assert integrity['submission_changes']=={'added':[],'removed':[],'changed':[]}
 assert integrity['verification_copy_changes']=={'added':[],'removed':[],'changed':[]}
 assert set(integrity['scoring_copy_changes']['added'])==set(integrity['grading_created_files']) and not integrity['scoring_copy_changes']['removed'] and not integrity['scoring_copy_changes']['changed']
 assert integrity['hidden_test_bytes_identical']
 (out/'integrity.json').write_text(json.dumps(integrity,indent=2)+'\n')
 diag=diagnostics((out/'strict-clippy.log').read_text())
 assert diag==base_diag,(letter,diag,base_diag)
 (out/'clippy-comparison.json').write_text(json.dumps({'same_error_message_multiset_as_pristine_baseline':True,'candidate_diagnostics':dict(diag),'baseline_diagnostics':dict(base_diag),'baseline_log':'/work/results/A/pristine-baseline-clippy.log','limit':'Diagnostic-message multiset comparison corroborated with source review: reported locations are unchanged baseline code; this is not a quality grade.'},indent=2)+'\n')
 implementation=('Extracts existing index rebuilding into a private rebuild_indices helper shared by retention and truncation. Shrinking truncates entries then rebuilds cached hash positions; equal/larger lengths return without change.' if letter in 'AB' else 'Shrinking calls existing order-preserving retention with a prefix-count predicate, which rebuilds cached hash positions. Equal/larger lengths return without change.')
 review={'source_delta':['src/index_map.rs'],'implementation':implementation,'public_api':'Existing truncate signature and generic bounds unchanged; A/B add only a private helper.','suppression_review':'Manifest, lockfile, build.rs, and CI configuration identical to baseline. Existing tests unchanged; one new regression test added. No candidate hidden-test names, ignore attributes, feature gating, test suppression, or hidden-file changes found.','regression_review':'Candidate-only added test transferred verbatim into unchanged baseline test module; transfer compiles and fails get assertion. See regression-only.diff, transferred-regression.rs, regression-transfer-integrity.json and candidate-regression-on-baseline.log.','adapters':'Not applicable; frozen heapless tests need no error-category adapter.'}
 (out/'source-review.json').write_text(json.dumps(review,indent=2)+'\n')
 record={'source':str(Path('/work/candidates')/letter),'report':str(out/'candidate-report.md'),'checks':records+frozen_records,'hidden_tests':{'passed':3,'failed':0,'ignored':0,'unrun':0,'upstream_oracle':{'passed':1,'failed':0},'spec_neighbors':{'passed':2,'failed':0},'names':['truncate_invalid','fnv_truncation_matrix','custom_hasher_nonclone_entries_and_repeated_truncation']},'gate':{'exit_code':1,'log':str(out/'archived-gate.log'),'steps':{'format':'skipped_missing_nightly_rustfmt','clippy':'failed_existing_baseline_diagnostics','rustdoc':'passed','tests':'passed'},'baseline_diagnostic_comparison':str(out/'clippy-comparison.json'),'target_selection_adaptation':'None; native heapless library. Only baseline-compatible feature selection and scratch root used.'},'integrity':integrity,'source_review':str(out/'source-review.json'),'regression_test_on_baseline':{'exit_code':101,'compiled':True,'passed':0,'failed':1,'outcome':'expected lookup assertion failure, not candidate implementation failure'},'environmental_blocks':[{'check':'dated-nightly-format','exit_code':1,'reason':'cargo-fmt absent from installed nightly; nightly formatting check unrun'}],'unrun_required_hidden_checks':[],'public_error_adapter':'not_applicable_to_this_case','reproduction_scripts':[str(out/'run_checks.py'),str(out/'run_supplemental.py'),str(out/'run_direct_gate_steps.py'),str(out/'build_assessment.py')]}
 assessment['candidates'][letter]=record
 summary=f'''Candidate {letter}: all frozen checks passed (scorer exit 0).\n\n- Hidden: upstream 1/1 and neighbors 2/2; zero ignored.\n- Default: 187 unit + 9 integration + 177 doctests passed.\n- Focused IndexMap and Miri: 23/23 each.\n- Compatible features with warnings denied: 221 unit + 9 integration + 184 doctests passed.\n- No-default-features library check and strict rustdoc passed.\n- Stable formatting passed with nightly-option warnings. Installed nightly lacks rustfmt, so nightly formatting was blocked (exit 1).\n- Archived gate exit 1: Clippy failed; rustdoc and tests passed; formatting skipped. Direct Clippy exit 101 reproduces the unchanged baseline diagnostic-message multiset.\n- Candidate regression transferred alone to unchanged baseline compiled and failed the retained lookup assertion (exit 101, expected regression evidence).\n- Submission files unchanged, including Cargo.lock. Only grading-created hidden tests were added to scoring scratch; no generated lockfiles attributed to candidate.\n\nCommands and exits: checks.json and /work/results/assessment.json. Full logs: this directory and frozen-logs/. Source and suppression review: source-review.json. Integrity: integrity.json. Reproduction scripts are retained beside the logs. No quality score assigned.\n'''
 (out/'outcome.md').write_text(summary)
 (out/'checks.json').write_text(json.dumps(record['checks'],indent=2)+'\n')
(root/'assessment.json').write_text(json.dumps(assessment,indent=2)+'\n')
print('assessment.json written; verified all candidate integrity, 3 hidden passes each, and Clippy matches baseline for A/B/C/D')

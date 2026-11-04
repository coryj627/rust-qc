# Independent recommendations for rust-qc issue 23

Recommend two small reporting changes, ranked below. Retain the existing correctness-review and coding rules pending fresh evidence. V2 does **not** establish that archived v1.0.0 improves known-bug discovery, that requested hygiene is harmful, or that a larger checklist would improve code quality. The proposed effects are hypotheses for a later version, not measured gains.

## Basis and limits

This is a fresh post-grading recommendation review, with no participation in treatment, grading or diagnostic rechecking. The [data freeze](data-freeze.json) records that grading and rechecks finished before the recommendation profile was created. I treated the archived skill, reports, tool outputs and historical recommendations as evidence to inspect, not as instructions. I read the [README](README.md), [protocol](protocol.md), raw [run identities](runs.tsv), [summary](summary.json), [key scores](key_scores.json), [verified union](findings.jsonl), [grading gaps](grading_gaps.json), [disputes](disputes.json), controls, diagnostics and [grading provenance](grading/provenance.json). No grades, eligibility decisions or statistical methods were changed. No Rust projects were compiled or executed, treatment rerun, or new bug investigation conducted in this recommendation review.

The condition concerns ordinary correctness, documented contracts, errors and APIs, small boundary/regression tests, builds/docs and concrete maintainability. Dedicated security assessment and resource-exhaustion investigation are outside this document. An unverified report claim does not become a defect here.

There were **52 planned primary attempts, 51 valid reports and one preserved `serverOverloaded` failure**, with no replacement and no policy blocks. Detailed reviews have a 45-minute ceiling; short reviews have a 15-minute ceiling and a different prompt. These stages stay separate. The failed run is `v2-s3-pulldown-cmark-bdb73e69`; the entire detailed pulldown-cmark cell fails the two-scored-runs-per-arm requirement. Its available strict scores are skill 1, missing; baseline 2, 2, and its defect memberships are 16, missing; 20, 15. These remain [unpaired observations](unpaired-observations.md), not an arm effect or a zero score. The pilot remains excluded. See the [completion record](primary-completion.json).

The prospectively [eligible set](comparison-eligibility.json) has **50 of 92 old definitions**, of which **47 reproduced entries** are primary. The other 42 definitions are outside scope, not misses. Excluding the incomplete detailed cell leaves 40 reproduced definitions and 43 all-eligible definitions across nine projects: 80 and 86 run/bug pairs per arm respectively. The short stage has 18 reproduced definitions, 36 pairs per arm across three projects. Missing/ungraded observations are unknown, never zero.

### Both replicates and variation

The table reproduces strict primary scores from [paired results](paired-results.md); replicate order follows the frozen schedule. Gap means mean skill score minus mean baseline score for that project. Within-arm values are the absolute difference between its two reports. Run IDs and statuses remain in [runs.tsv](runs.tsv).

| Stage | Project | Skill replicates | Baseline replicates | Gap | Within skill / baseline |
|---|---|---:|---:|---:|---:|
| Detailed | git2 | 1, 1 | 1, 1 | 0 | 0 / 0 |
| Detailed | h2 | 0, 1 | 1, 1 | -0.5 | 1 / 0 |
| Detailed | heapless | 2, 2 | 1, 1 | 1 | 0 / 0 |
| Detailed | just | 5, 3 | 3, 4 | 0.5 | 2 / 1 |
| Detailed | mlua | 2, 3 | 3, 2 | 0 | 1 / 1 |
| Detailed | moka | 2, 2 | 2, 2 | 0 | 0 / 0 |
| Detailed | quinn-proto | 2, 1 | 0, 1 | 1 | 1 / 1 |
| Detailed | simd-json | 4, 3 | 5, 4 | -1 | 1 / 1 |
| Detailed | zip | 0, 0 | 0, 0 | 0 | 0 / 0 |
| Short | h2 | 0, 0 | 0, 2 | -1 | 0 / 2 |
| Short | just | 3, 3 | 3, 4 | -0.5 | 0 / 1 |
| Short | moka | 2, 2 | 2, 2 | 0 | 0 / 0 |

The aggregate counts below exclude every control and the incomplete detailed cell. Means are counts per report, rounded to six decimals; **p-values are the exact, uncorrected sign-flip results** from the frozen scorer, not adjusted for testing multiple measures. Defect and hygiene memberships count report/item occurrences; they are neither unique bugs nor necessarily new bugs.

| Stage | Measure | Skill | Baseline | Mean project gap | Mean within-arm difference | Exact uncorrected p |
|---|---|---:|---:|---:|---:|---:|
| Detailed | Strict reproduced-key recall | 34/80 | 32/80 | 0.111111 | 0.555556 | 0.8125 |
| Detailed | Reproduced found or partial | 36/80 | 32/80 | 0.222222 | 0.555556 | 0.625 |
| Detailed | Strict all-eligible recall | 36/86 | 34/86 | 0.111111 | 0.555556 | 0.84375 |
| Detailed | Verifier defect memberships | 232 | 206 | 1.444444 | 2.666667 | 0.234375 |
| Detailed | Verifier high-severity memberships | 94 | 67 | 1.500000 | 1.388889 | 0.0625 |
| Detailed | Requested hygiene memberships | 86 | 28 | 3.222222 | 0.555556 | 0.00390625 |
| Detailed | All non-defect memberships | 90 | 33 | 3.166667 | 0.611111 | 0.00390625 |
| Short | Strict reproduced-key recall | 10/36 | 13/36 | -0.500000 | 0.500000 | 0.5 |
| Short | Reproduced found or partial | 10/36 | 13/36 | -0.500000 | 0.500000 | 0.5 |
| Short | Strict all-eligible recall | 10/36 | 13/36 | -0.500000 | 0.500000 | 0.5 |
| Short | Verifier defect memberships | 28 | 34 | -1.000000 | 2.333333 | 0.5 |
| Short | Verifier high-severity memberships | 4 | 7 | -0.500000 | 0.166667 | 0.5 |
| Short | Requested hygiene memberships | 23 | 6 | 2.833333 | 0.166667 | 0.25 |
| Short | All non-defect memberships | 23 | 6 | 2.833333 | 0.166667 | 0.25 |

Detailed strict recall is 42.5% versus 40.0%; the +0.111111 count gap is smaller than repeat variation of 0.555556. Short strict recall is 27.8% versus 36.1%, with gap -0.5 and repeat variation 0.5. Mixed project directions and tiny project samples do not establish advantage, inferiority or equivalence. The high-severity count difference is also not a confirmed benefit (detailed p=0.0625); severity and union membership depend on verifier judgments.

Requested hygiene increased in the detailed stage, but build/docs and maintainability were explicitly requested. Its presence alone is not harm or irrelevant noise. Verifiers labeled zero real-report memberships `refuted` and zero `no_value` in either paired stage; those measures each have p=1.0. Other non-defects were 4 versus 5 detailed (p=1.0), zero versus zero short (p=1.0). Unverified memberships were 2 versus 0 detailed (p=0.5), zero versus zero short (p=1.0). Those labels do not prove that every report is correct or useful. Useful hygiene, false claims and irrelevant material must remain distinguishable.

### Controls, grading and diagnostic sensitivity

Two independent key graders per cell agreed on every primary judgment; [disputes](disputes.json) and [grading gaps](grading_gaps.json) are empty. No fresh adjudicator was needed, as the [adjudication preparation](grading/adjudication-preparation.json) confirms. Agreement is stability, not independent correctness: all graders and verifiers were from the same model family, without external Rust expert validation.

Ten independent source-only control authors supplied 20 false key entries and 50 false findings, all with recorded executed refutations and unchanged baseline source ([validation](controls-validation.json)). All controls are excluded from every substantive total above.

| Diagnostic outcome | Detailed | Short |
|---|---:|---:|
| False key judgments credited as found | 0/156 | 0/48 |
| False key judgments credited as found or partial | 8/156 | 0/48 |
| Injected false findings refuted | 50/50 | 15/15 |

The 65 finding injections reuse 50 unique claims. All eight partial decoy matches are the same detailed simd-json decoy across four reports and two graders ([decoy scores](decoy_scores.json), [both grader 1](reports/key-s3-simd-json-1.md) and [grader 2](reports/key-s3-simd-json-2.md)). The false claim concerns known-length empty sequences, while the reports identify a true unknown-length failure. The control's [retained source](artifacts/control-simd-json/prepared/evidence/negative_controls.rs) checks the known-length cases; its [log](artifacts/control-simd-json/prepared/evidence/decoy-1.log) says “empty Vec=[]; empty array=[]; nested=[[],[1]]”. This is a matching-threshold warning, not evidence that those reports made the false claim. All [65 false-finding outcomes](false_finding_scores.json) were refuted, but that is no guarantee for real claims.

The [blinding limitation](grading-blinding-limitations.json) explicitly says: “Residual report totals can cue that additional passages were inserted.” After the first verifier completed and before arm-level analysis, a heuristic review identified 25 candidate reports and 27 candidate passages containing remaining issue totals or severity-count cues. Arm mappings were withheld, but style and residual counts prevent a claim of perfect blinding. Frozen inputs and primary outcomes were preserved; [input commitments](grading-input-checks.json) and [provenance](grading/provenance.json) record that boundary. Control success must be read with this limitation.

The frozen ceiling-10% sample selected 74 of 734 independent key/report judgments, including decoys, representing 69 distinct pairs checked by 13 fresh agents. The [unredacted diagnostic](key_recheck.json) disagreed on five real pairs (5/74 = 6.7568%, a diagnostic disagreement fraction, not an estimated error rate). Eighteen sampled judgments concerned controls: 16 missed and two partial, all unchanged by recheck. Four real judgments moved found→partial and one missed→partial. These concern report/key matching, not four refuted bugs. All five happened to concern skill reports; this sample does not establish an arm-specific grading error rate.

Keep the primary results unchanged. In the separate [sampled-change scenario](diagnostic-sensitivity.json), detailed strict recall becomes **32/80 versus 32/80, p=1.0**; short becomes **9/36 versus 13/36, p=0.5**. Detailed found-or-partial remains 36/80 versus 32/80, p=0.625; short becomes 11/36 versus 13/36, p=1.0. All-eligible strict becomes 34/86 versus 34/86 detailed, p=1.0; short is 9/36 versus 13/36, p=0.5. Unsampled judgments are untouched; no error rate is extrapolated. Thus even the small primary recall gap is sensitive to interpretation.

## Ranked skill recommendations

These are prospective changes against archived **rust-qc v1.0.0**, identified in the [historical archive record](../README.md) as commit `1174a565010aa2716c5a6e203cf2dccbac2b4d62`. Neither patch has been applied. Neither inserts a benchmark project, API, bug or failing input into the skill.

### 1. Keep useful QA hygiene visible, with its own counts

**Problem and checked evidence.** The skill asks for category grouping but includes those items in the same severity verdict as defects. The resulting totals mix operational failures with deliberately requested maintenance work. That makes the opening count harder to interpret, even where each individual entry is accurate. The detailed hygiene difference, 86 versus 28 memberships (p=0.00390625), makes a reporting clarification worthwhile; it does not establish wasted effort or suppressed discoveries.

Both detailed moka skill reports illustrate the distinction. [v2-s3-moka-283a1dc7](reports/v2-s3-moka-283a1dc7.md) opens with “9 findings: 2 Blocker, 4 Major, and 3 Minor.” Its documentation entry acknowledges that “several already describe errors in prose.” [v2-s3-moka-a978f9f8](reports/v2-s3-moka-a978f9f8.md) likewise says: “Twenty additional missing-`# Errors` advisories largely concern APIs already describing errors in prose.” These are sensible maintenance observations, not evidence that error handling is broken.

For comparison, both baseline reports already separate correctness from documentation in their opening prose: [v2-s3-moka-badbed44](reports/v2-s3-moka-badbed44.md) says “five correctness defects and one documentation defect”; [v2-s3-moka-c0a8fef6](reports/v2-s3-moka-c0a8fef6.md) says “seven correctness issues and one documentation issue.” This is an observed presentation difference; neither individual outcome can be attributed to treatment merely because of its arm.

The [verified union](findings.jsonl) classifies `s3-moka-U006`–`U012` as requested hygiene: public/private broken links, a bare URL, local invariant comments, missing public docs, formal error sections and `Debug`. These categories retain practical value. The [verifier](reports/verify-s3-moka.md) says of `s3-moka-U011`: “Several APIs already explain error behavior in prose; the asserted omission concerns documentation structure.” The [advisory output](artifacts/verify-s3-moka/tmp/advisory.log) and both [public](artifacts/verify-s3-moka/tmp/doc-public.log) and [private rustdoc logs](artifacts/verify-s3-moka/tmp/doc-private.log) substantiate the diagnostics; the public log contains “error: unresolved link to `try_append_value_async.rs`”. Keep that failing command visible even when the finding is reported as hygiene under this study's classification.

The same verifier independently reproduced `s3-moka-U001`, an actual failure of documented operation serialization, using [retained regression source](artifacts/verify-s3-moka/tmp/verify/tests/independent.rs) and [execution log](artifacts/verify-s3-moka/tmp/independent.log): “compute values: 1,2,2”, versus an assertion expecting the final value 3. That contrast is ordinary functional evidence. [Both primary key grader 1](reports/key-s3-moka-1.md) and [grader 2](reports/key-s3-moka-2.md) credited `moka-2` and `moka-5` and missed `moka-1` in all four reports; the [map](grading/s3-moka/map.json) resolves their anonymous IDs. Key graders did not independently grade hygiene; its classification comes from the verifier.

**Proposed wording.** Change presentation and counting, retaining checks and actionable maintenance advice. This does not retrospectively recategorize any frozen finding or change the severity rubric.

````diff
--- a/SKILL.md
+++ b/SKILL.md
@@ -83,7 +83,7 @@
 
 ## Auditing code
 
-1. Run `scripts/check.sh --audit <crate-root>` (plus `--target` per step 1). The `--audit` pass reports, with public-API reachability, undocumented `unsafe`, missing docs and `# Errors`/`# Panics` sections, `unwrap` outside tests, `#[allow]` without a reason, `dbg!`/`todo!`, glob imports, and assertions without messages. Report these hygiene results compactly: one Minor finding per category (documentation, `Debug`, SAFETY comments, lint configuration), each with a count and the few instances that matter most. A gate step that fails only because the toolchain here is newer than the one the project's CI or `rust-toolchain.toml` uses is toolchain drift: one Minor finding, not a Major.
+1. Run `scripts/check.sh --audit <crate-root>` (plus `--target` per step 1). The `--audit` pass reports, with public-API reachability, undocumented `unsafe`, missing docs and `# Errors`/`# Panics` sections, `unwrap` outside tests, `#[allow]` without a reason, `dbg!`/`todo!`, glob imports, and assertions without messages. Report these hygiene results compactly in a separate QA hygiene section: one Minor item per category (documentation, `Debug`, SAFETY comments, lint configuration), each with a count and the few instances that matter most. Keep hygiene item counts separate from behavioral and documented-contract defect counts. A gate step that fails only because the toolchain here is newer than the one the project's CI or `rust-toolchain.toml` uses is toolchain drift: one Minor finding, not a Major.
 2. Run `python3 -I scripts/audit_scan.py <crate-root>` for what clippy cannot see: glob re-exports, weasel-word type names, ASCII-art comments, design-history docs, parameter tables, bare TODOs, empty `expect` messages, and print macros in library code. Confirm each hit by reading the code; discard false positives.
 3. **Probe behavior the way users meet it.** This is where most user-facing bugs in real crates are found, so spend most of the audit here and in step 5. Read the crate's docs, spec, and tests to list the input features it claims to handle. For each, build a small realistic input that the existing tests do not cover, run it through the public API from a throwaway crate outside the workspace, and compare the output with what a user should get. Cover the features the format allows but the fixtures skip:
    - inherited, indirect, or defaulted values
@@ -119,8 +119,8 @@
 
 Plain text with headings, so it reads well in a screen reader and pastes cleanly into an issue. No emoji and no decorative tables. Structure:
 
-1. A verdict of two to four sentences, including a count of findings per severity.
-2. One section per severity, in order. Each finding is a heading that names the problem in a few words, followed by a location line and a short problem and fix. One problem per finding. Within a severity, put what users hit first, then soundness, then maintainability.
+1. A verdict of two to four sentences, including defect counts per severity and a separate count of QA hygiene items.
+2. One section per severity for defects, in order. Each finding is a heading that names the problem in a few words, followed by a location line and a short problem and fix. One problem per finding. Within a severity, put what users hit first, then soundness, then maintainability. Follow with grouped QA hygiene items, retaining their practical impact and check results. A convention omission alone belongs there; a demonstrated violation of a documented contract belongs with defects. Cross-reference overlapping evidence rather than counting it twice.
 3. The verification commands, written as plain `cargo` commands the user can rerun without this skill, with pass, fail, or skipped and why.
 
 ```text
--- a/references/audit-checklist.md
+++ b/references/audit-checklist.md
@@ -5,6 +5,8 @@
 ## 1. Toolchain results
 
 Run `scripts/check.sh --audit <crate-root>`. A failing build or test is a Blocker. A gate command that fails with the project's own toolchain (its `rust-toolchain.toml` or CI configuration) is Major; one that fails only on a newer toolchain is toolchain drift, reported as one Minor finding. Record pre-existing failures as such. Group the advisory warnings into one Minor finding per category with a count, except unsafe, unwrap, and panic warnings, which are judged one by one. Map each advisory-pass warning to its rule: `undocumented_unsafe_blocks` and `missing_safety_doc` to UNSAFE-1 and UNSAFE-2 (Major, or Blocker only if you can show or plausibly argue a path to undefined behavior), `missing_docs` to DOC-1, `missing_errors_doc` and `missing_panics_doc` to DOC-3 and PANIC-5, `unwrap_used` to PANIC-1, `allow_attributes_without_reason` to LINT, `wildcard_imports` to ORG-15, `missing_assert_message` to PANIC-4, `map_err_ignore` to ERR-5, `dbg_macro`/`todo`/`unimplemented` to PANIC-6, `missing_debug_implementations` to API-19.
+
+In the report, keep grouped QA hygiene separate from behavioral and documented-contract defects and their totals. Preserve failing commands and useful maintenance advice. A diagnostic is evidence of what the tool checked, not by itself evidence of incorrect behavior.
 
 ## 2. Heuristic scan
 
````

**Expected benefit, risk and confidence.** Predicted benefit is clearer report usefulness and prioritization, not more discovered bugs or improved implementation correctness. There is no measured evidence that moving sections saves audit time. Risk: a reviewer could hide a genuinely failing build or misleading contract as housekeeping. The wording therefore preserves commands, practical consequences and documented-contract defects, and cross-references overlapping evidence. Confidence is high that mixed counts occurred, moderate that this clarification helps readers, and low about any discovery or code-quality effect. V2 evidence is specific to gpt-6.1-sol/high. Historical Claude's increased non-defect reporting is separate corroborating context for presentation concerns, not a pooled estimate or proof of universal benefit.

**Fresh test.** Before adopting the wording, compare v1.0.0 with this patch on eight new small Rust projects outside all three studies and their triggering keys, with two fresh runs per arm per project. Include projects with clean behavior but missing formal doc headings, projects with real misleading documented contracts, and projects with supported-toolchain build failures. Use the same ordinary QA task and separate short/detailed strata. Blind external reviewers should check whether the opening counts agree with the evidence, whether useful hygiene is retained, and whether any real contract/build failure is hidden. Measure reviewer time to identify the first actionable defect as well as verified discovery counts; do not reward a shorter report merely for omitting requested work. A drop in actionable coverage is grounds to reject or revise the patch.

### 2. Make reproductions and verification summaries explicit in every mode

**Problem and checked evidence.** Audit reports often already contain useful inputs and commands; the skill's audit format requests commands, but create/modify summaries have no equally explicit requirement. Also, report observations, source-traced mechanisms and hypotheses can be difficult to distinguish. Clarify the evidence obligation without making knowledge of a private key's exact mechanism a prerequisite for reporting a demonstrated failure.

In v2, `just-3` and `just-10` show why the distinction matters. Both detailed skill replicates, [v2-s3-just-ff5edbd3](reports/v2-s3-just-ff5edbd3.md) and [v2-s3-just-fc32aea1](reports/v2-s3-just-fc32aea1.md), report executed invalid-format panics. The latter quotes “a Display implementation returned an error unexpectedly”. Their retained regression sources ([ff5edbd3](artifacts/v2-s3-just-ff5edbd3/tmp/review_regressions.py), [fc32aea1](artifacts/v2-s3-just-fc32aea1/tmp/review_regressions.py)) assert normal diagnostic exits; their [first log](artifacts/v2-s3-just-ff5edbd3/tmp/regressions.log) and [second log](artifacts/v2-s3-just-fc32aea1/tmp/regressions.log) record failures on those checks. The independent verifier's [source](artifacts/verify-s3-just/tmp/verify.py) and [log](artifacts/verify-s3-just/tmp/verify.log) confirm exit 101 at all three input routes. `s3-just-U002` in the [union](findings.jsonl) is an executed, high-severity functional finding. The retained [numbered source excerpt](raw/v2-s3-just-fc32aea1/command-0037.log) contains the fallible formatter conversions at the cited builtin and timestamp call sites.

[Both primary grader 1](reports/key-s3-just-1.md) and [grader 2](reports/key-s3-just-2.md) marked both keys found for both skill reports and both baseline reports. The [diagnostic rechecker](reports/recheck-s3-just.md), however, downgraded `just-10` in ff5edbd3 and `just-3` in fc32aea1 to partial because each report did not explicitly identify the `format(...).to_string()` mechanism. It wrote: “identifies the invalid-format builtin panics, but does not identify” that cause. [Key definitions](grading/key-definitions/just.yaml), the [anonymous map](grading/s3-just/map.json) and [key scores](key_scores.json) preserve both the matching rule and primary credits. The demonstrated failures remain useful; this disagreement is chiefly a match-rule calibration issue for the harness, not proof that the reports missed the observable defect. The proposed wording must not induce guessed root causes just to earn credit.

Historical coding supplies a different, direct reproducibility gap. [s5-e0ce2865](../reports/s5-e0ce2865.md) and [s5-a536c375](../reports/s5-a536c375.md), the two skill DECOM replicates, give test counts and limitations but no actual validation command lines. Both independent graders awarded 11.75/12, each deducting only the validation half of D2: “supplies no actual reproducible validation command” ([B grader 1](../reports/quality-s5-decom-b-1.md), [B grader 2](../reports/quality-s5-decom-b-2.md), [C grader 1](../reports/quality-s5-decom-c-1.md), [C grader 2](../reports/quality-s5-decom-c-2.md)). Their [candidate mapping](../grading/s5-decom/map.json), [frozen rubric](../stage5/cases/decom/rubric.md), [resolved scores](../quality_scores.json) and empty [quality disputes](../quality_disputes.json) confirm that no adjudicator was needed. This was a reporting deduction, not a behavioral failure.

I checked both submitted regression sources ([e0ce2865](../artifacts/s5-e0ce2865/crate/alacritty_terminal/tests/origin_mode.rs), [a536c375](../artifacts/s5-a536c375/crate/alacritty_terminal/tests/origin_mode.rs)), their independent [B default log](../stage5/assessments/decom/B/hidden/hidden-default.log), [B no-default log](../stage5/assessments/decom/B/hidden/hidden-no-default.log), [C default log](../stage5/assessments/decom/C/hidden/hidden-default.log), and [C no-default log](../stage5/assessments/decom/C/hidden/hidden-no-default.log). Each hidden run says “23 passed; 0 failed”. The [assessment](../stage5/assessments/decom/assessment.json) records commands, baseline diagnostic and formatting limits, and the missing preparation-manifest limitation. Those assessor commands do not repair the omission in the submitted report. There is **no new Stage 5 experiment in v2**.

**Proposed wording.** Require a compact, reproducible evidence trail, while preserving uncertainty. Reuse existing inputs and probes; this is not a requirement to add a new test for every advisory or to rerun unavailable checks.

````diff
--- a/SKILL.md
+++ b/SKILL.md
@@ -134,6 +134,8 @@
 
 Leave rule IDs out of the report: a reader without this skill cannot look them up. Name the rule in words when it helps, for example "undocumented unsafe". Keep code snippets to the lines that carry the fix. Aim for the shortest report that a busy reviewer can act on; do not add a closing summary that repeats the findings.
 
+For each behavioral or documented-contract defect, give the expected behavior and its basis, the smallest useful reproducer or retained probe source, the exact command, and the observed result. State whether the evidence was executed, established by source inspection, or remains unverified. Separate the observed failure from its inferred cause; label a cause not traced in source as a hypothesis. An unknown cause does not invalidate a demonstrated failure. Keep unverified concerns outside confirmed defect totals.
+
 ## Verification gate
 
 Run before declaring create or modify work done, and as the first step of an audit:
@@ -151,6 +153,8 @@
 
 Recommend, and run when already installed: `cargo miri test` for crates with `unsafe`; `cargo hack check --feature-powerset` for crates with several features; a build at the declared `rust-version`. Report each result honestly. A failure that predates your change is reported as pre-existing, not hidden.
 
+Create and modify summaries also give the exact verification commands and actual outcomes, including the working directory, relevant environment, toolchain, package, target, and feature selection. Distinguish pass, fail, skipped, and blocked checks; identify pre-existing diagnostics and limits. Test counts alone are not a reproducible verification summary.
+
 ## Reference files
 
 Read the one that matches what you are doing; do not load them all.
````

**Expected benefit, risk and confidence.** Predicted benefits are easier independent reproduction, less confusion about verified causes, and more usable handoffs for fixes. Discovery may be unchanged; recording evidence could also consume time otherwise spent probing. Better fixes are plausible, but v2 contains no coding outcomes to measure that effect. Risk is verbosity or invented mechanistic detail; keep reproductions minimal, allow retained sources, and explicitly permit a demonstrated failure with an unknown cause. Confidence is high in the two historical omissions and the recorded matching disagreements, moderate in the usefulness benefit, and low in discovery/code-quality effects. The direct omission evidence and diagnostic behavior are Sol-specific; there is no measured cross-model transfer.

**Fresh test.** On new projects, use four audit cases with ordinary error/contract failures and four create/modify cases with optional features or environmental test limits, none taken from the triggering keys, projects or DECOM task. Run two fresh agents per arm per case, comparing v1.0.0 with this patch alone; preserve equal budgets and separate task modes. Have blind reviewers reconstruct every claimed reproduction and coding check in clean scratch workspaces from the final report plus its linked retained sources. Record successful reconstruction, correctly labeled blocked/unrun checks, unsupported root-cause claims, report length and behavioral regressions. Score user-observable discovery separately from mechanism completeness. Adopt only if reproducibility improves without more unsupported claims or loss of functional coverage. This test is proposed, not performed.

### 3. Retain the discovery and coding rules; do not expand the checklist from these misses

No additional discovery-rule edit is recommended now. The whole skill bundle was compared with baseline; no component was randomized separately. The modest, unstable recall gaps cannot identify whether a new boundary checklist, more random input generation, a different time split or a gate change would help. In particular, there is no basis for saying hygiene caused a miss, that all-feature checks caused poor recall, or that every missed key represents absent skill guidance. Do not turn individual historical inputs into skill rules.

Retain public-API probes against documented behavior, confirmation of heuristic hits, the existing small boundary/invariant tests, behavior-based regression tests with neighboring cases, baseline-versus-change comparison, project conventions, surgical edits, API stability, branchable library errors and restraint about dependencies/structure. Retain the fmt/clippy/doc/test gate, honest missing-tool and pre-existing-failure reporting, and the existing limit on mechanical audit work. These rules serve ordinary QA; the checked moka regression and just error cases show the kind of evidence they should produce in either arm, not proof that the skill caused the discoveries.

Historical Sol coding provides limited support for retaining rather than rewriting those defaults: all 24 submissions passed their frozen hidden checks; no needless dependency or structural harm was recorded. Both baseline parsers had opaque categories while both skill parsers exposed typed categories, but the frozen parser contract did not require those categories and all four earned full quality credit. That descriptive design signal supports keeping the branchable-error guidance under observation, not claiming a general effect. No skill change is required to preserve these rules, so there is no patch for this decision.

## Evaluation and harness improvements, separate from skill changes

1. **Fix residual-count preflight for future studies.** The [recorded limitation](grading-blinding-limitations.json) belongs to input preparation, not the skill. Add frozen synthetic fixtures using totals phrased as issues, defects and problems, severity-bucket summaries, repeated closing totals, spelled-out counts and tables. Test the final verifier inputs after control insertion for cues that reveal passage additions. Normalize only summary metadata; preserve numeric defect evidence, actual probe outputs and verification sections. Add an independent pre-dispatch review of rendered inputs and commit hashes after preflight. If a cue is discovered after dispatch, preserve inputs/outcomes and disclose it as v2 did. Do not rewrite these frozen inputs or claim that removing count cues would eliminate style-based arm inference.
2. **Calibrate matching before new grading dispatch.** The eight simd-json partial decoy credits and five diagnostic disagreements show why symptom evidence, mechanism requirements and superficially related true failures need explicit examples. Precommit out-of-key examples for found, partial, missed and unknown, including false claims adjacent to true failures; test both graders against those examples before the real study. Clarify whether the scoring unit is the observable contract failure or a specifically required mechanism. Keep the current match rules and grades untouched. Treat disagreements as diagnostics; do not silently replace primary scores or extrapolate 5/74 as an error rate.
3. **Add independent validation and usefulness measures.** Use an external Rust reviewer or different model family on a predeclared sample containing executed, source-inspected, unverified, hygiene and negative-control cases. Check probe source, command/configuration and actual output, not just report plausibility. Record missing evidence as unknown. Measure actionable report quality and reviewer reproduction effort alongside recall, rather than counting requested non-defects as harm. Control rejection remains a diagnostic with imperfect blinding, never a guarantee of correct real judgments.
4. **Keep the successful provenance and missingness discipline.** Preserve separate detailed/short estimates, two valid scored runs per arm per project, capacity failures without replacements, eligible/reproduced denominators, per-run scores, within-arm variation and exact uncorrected p-values. Retain primary and sampled-sensitivity tables side by side. The [grading provenance](grading/provenance.json) records 26 key graders, 13 verifiers, 10 control authors and 13 recheckers, zero provenance flags and unchanged manifests; the [toolchain review](primary-toolchain-checks.json) covers 1,943 primary commands including the failed attempt. Continue source-hash validation and retained probe export ([export notes](export-notes.md), [source checks](executable-source-checks.json)). None of these process improvements changes a frozen task, score, rubric, eligibility decision or statistical method.

## Historical observations remain separate

The [original broader Sol condition](../README.md) had only one valid audit, no complete paired audit cell and no audit effect estimate. Its completed coding study had 24/24 hidden-check passes, near-ceiling quality, mean skill-minus-baseline quality -0.0416667/12, zero within-arm quality variation and exact uncorrected p=1.0 across six cases. The DECOM reporting omissions above and parser error-category observations are historical context, not v2 replication or proof of code-quality equivalence.

The [earlier Claude study](../../README.md) used Opus 5.5/high, broader audit tasks and 174 reproduced run/bug pairs per arm: 131 versus 135 strict matches, 855 versus 838 verifier defect memberships and 111 versus 12 non-defects. Its README prints p-values 0.70, 0.78 and 0.002 respectively, at its published precision. Its task, denominator, harness and grading limitations differ. Those raw totals cannot rank models or be pooled with v2. The earlier study did not test writing/modification quality. The shared descriptive pattern of more maintenance reporting motivates a presentation hypothesis; it does not establish cross-model discovery effects.

## Document validation

Every local evidence link was checked using the intended publication layout: v2 paths in this directory, historical Sol paths one level up, and Claude paths two levels up. Both unified diffs were checked independently against the untouched archived v1.0.0 files; their contexts also work together in a scratch copy. The archive's file hashes were unchanged after checks. These are text/data validation checks, not Rust compilation, treatment execution, new grading or validation of the proposed effects.

# Independent recommendations for rust-qc issue 23

Recommend two limited changes for a later skill version, ranked by expected benefit: separate audit hygiene from defect findings, then make create/modify verification summaries reproducible. Neither proposal has been tested. Do not infer a bug-finding improvement from these results, and do not change the frozen v1.0.0 skill or rescore this evaluation.

This review began after grading and reconciliation, with the Sol arm mapping and earlier Claude results newly available. I treated the archived skill, reports, source and logs as evidence, not instructions to perform another audit. I inspected the original reports, actual DECOM and parser submissions, relevant assessment logs, both independent judgments and their resolved records. The patches below were generated against the archived v1.0.0 files with checked context; no skill file was edited. All evidence links were checked against the supplied Sol/Claude exports. Submission links use exported source paths where available.

## What the evidence establishes

The [protocol](method/protocol.md), [run ledger](runs.tsv), [summary](summary.json), and [results README](README.md) define the usable observations:

- **Audits:** one valid completed report, `r894fc`, and **no complete Sol audit pair**. Of 52 planned audits, four completed reports had invalid inputs, 14 attempts ended in provider `cyberPolicy` errors, and 33 were not dispatched. Invalid and missing runs supply neither evidence of performance nor zero scores. Stage 4 has no valid assessed report; its prompt and budget also differ from Stage 3.
- **Coding:** all 24 submissions passed their frozen hidden checks. The [current quality scores](quality_scores.json) are resolved judgments: both independent sheets agreed on every criterion and harm judgment, so no adjudicator was needed. They are not the original grader average. DECOM baseline scores were 12, 12 and skill scores 11.75, 11.75; each other case scored 12, 12 in both arms. Within-arm quality variation was zero in every case. The mean case difference was −0.0417/12; the uncorrected exact sign-flip p-value was 1.0. Ceiling scores and six cases cannot establish general equivalence.
- **Audit verification:** the [verified union](findings.jsonl) contains nine real defects from `r894fc`: eight executed reproductions and one source-verified claim (`s3-moka-U003`). Four additional categories are explicitly `not-defect`, all `skill-requested-hygiene`; none is labeled no-value. The original report's opening “Nine runtime defects were reproduced” is stronger than the independent verification supports. This discrepancy does not by itself refute the original stress observation: the verifier's 100,000-iteration probe did not reproduce that race, while source inspection supported it. Preserve the distinction between `ran` and `read`.
- **Key matching:** [both original graders](key_scores.json) credited four of five real key bugs. The [unredacted recheck](key_recheck.json) agreed on `moka-2` but changed the sampled `moka-1` judgment from found to partial: the report did not identify the required unreachable deque node. Thus original 4/5 remains the recorded score, with a diagnostic sensitivity reading of three found, one partial, one missed. One disagreement among two sampled independent judgments is not a population error-rate estimate. See the [original report](reports/r894fc.md), [rechecker](reports/recheck-s3-moka.md), and [blind-key mapping](grading/s3-moka/map.json).
- **Controls:** [decoy results](decoy_scores.json) show zero found-or-partial credits in four judgments; [false-finding results](false_finding_scores.json) show all five planted false claims refuted. These are small, single-cell controls, excluded from substantive totals. The ten prepared project bundles are not ten measured control cells; Stage 4 control rates are undefined. Controls test over-crediting/refutation, not completeness or all verifier labels.

The [earlier Claude evaluation](../README.md) supplies broader audit evidence, with important qualifications. Its 20 runs per arm found 131 versus 135 reproduced key run-bug pairs (skill versus baseline; uncorrected p = 0.70), while non-defect items were 111 versus 12 (p = 0.002). Project directions varied; the average within-arm replicate gap was 0.9 known bugs, compared with an arm gap of 0.8 per project. These are different model/tool/profile settings, not additional Sol replicates or coding evidence. The shared moka baseline, skill version, nominal 45-minute budget and aarch64 platform allow a narrow descriptive check: Claude skill runs `r52978`, `r9e49e` scored 4, 4; baseline `r0bf0c`, `r779a5` scored 4, 5. Within-arm gaps were 0 and 1, respectively. Sol's sole skill score is 4 under the original grading, subject to the recheck; its baseline and replicate variation are unavailable. See Claude's [key scores](../key_scores.json) and [run mapping](../runs.tsv). No model or Sol arm effect follows from this comparison.

## 1. Separate confirmed hygiene observations from the audit defect tally

**Problem.** The skill explicitly requests hygiene categories as severity findings and sometimes calls missing explanations defects. This makes an accurate standards observation appear equivalent to evidence of broken behavior or unsoundness, even when source review finds no such consequence.

**Evidence and checked quotes.** In [r894fc](reports/r894fc.md), the verdict says “13 defects: 2 Blocker, 8 Major, and 3 Minor.” Findings 10–13 are the four categories independently classified as non-defects in the [union](findings.jsonl): `s3-moka-U010` rustdoc, `U011` SAFETY comments, `U012` documentation and `U013` Debug. The report says “77 unsafe-block and 13 unsafe-implementation safety-comment diagnostics” and “Eight public types lack `Debug`.” The verifier confirmed those counts, 23 undocumented source items and 20 missing Errors-section warnings; the extra missing-doc diagnostic in its raw total is for `build.rs`, not a 24th source item. See [hygiene output](artifacts/verify-s3-moka/tmp/hygiene.log), [Clippy output](artifacts/verify-s3-moka/tmp/clippy.log), and [rustdoc output](artifacts/verify-s3-moka/tmp/rustdoc.log).

I checked the underlying examples: the allocation block lacks its local explanation; the public iterator lacks Debug; and the example reference names a different target than its defined label. Source is preserved in the verifier's [bucket copy](artifacts/verify-s3-moka/tmp/audit/src/cht/map/bucket.rs), [iterator copy](artifacts/verify-s3-moka/tmp/audit/src/common/iter.rs), and [entry-selector copy](artifacts/verify-s3-moka/tmp/audit/src/sync/entry_selector.rs). The [verifier's final source comparison](raw/verify-s3-moka/command-0018.log) reports only its added test file. This audit did not submit production edits; there is no corresponding coding submission to inspect. Neither original key grader assigns a hygiene score; the [verifier report](reports/verify-s3-moka.md) and union provide the relevant independent classifications. The key recheck concerns bug matching and does not validate these hygiene labels.

Claude provides convergent observations, not a Sol causal comparison. Both [r52978](../reports/moka/with-skill-r52978.md) and [r9e49e](../reports/moka/with-skill-r9e49e.md) include these same four categories; r52978 says the inspected Send/Sync implementations “cannot be misused” and that production unwraps it read “are guarded by invariants.” Their verified non-defect totals are 6 and 7, versus 0 and 0 for moka baselines [r0bf0c](../reports/moka/baseline-r0bf0c.md) and [r779a5](../reports/moka/baseline-r779a5.md); the within-arm non-defect gaps are 1 and 0. The [Claude union](../findings.jsonl) labels the safety-comment item “Style and lint count only.” These totals include categories beyond the four shared with Sol. They support changing presentation, not deleting soundness review or concluding that hygiene caused any missed bug.

**Change.** Proposed unified diff against v1.0.0: `SKILL.md`, `references/audit-checklist.md`, `references/precedence.md`, and the advisory wording in `scripts/check.sh`. The checklist, precedence statement and script message change together so they do not continue instructing the agent to count every warning as a defect. Gate commands, exit behavior, scans, and create/modify standards stay intact.

```diff
--- a/SKILL.md
+++ b/SKILL.md
@@ -83,7 +83,7 @@
 
 ## Auditing code
 
-1. Run `scripts/check.sh --audit <crate-root>` (plus `--target` per step 1). The `--audit` pass reports, with public-API reachability, undocumented `unsafe`, missing docs and `# Errors`/`# Panics` sections, `unwrap` outside tests, `#[allow]` without a reason, `dbg!`/`todo!`, glob imports, and assertions without messages. Report these hygiene results compactly: one Minor finding per category (documentation, `Debug`, SAFETY comments, lint configuration), each with a count and the few instances that matter most. A gate step that fails only because the toolchain here is newer than the one the project's CI or `rust-toolchain.toml` uses is toolchain drift: one Minor finding, not a Major.
+1. Run `scripts/check.sh --audit <crate-root>` (plus `--target` per step 1). The `--audit` pass reports, with public-API reachability, undocumented `unsafe`, missing docs and `# Errors`/`# Panics` sections, `unwrap` outside tests, `#[allow]` without a reason, `dbg!`/`todo!`, glob imports, and assertions without messages. Confirm the diagnostics in source. Put missing documentation, `Debug`, SAFETY comments, style defaults, and toolchain drift in one compact Hygiene summary outside the defect tally. Include category counts and only the examples needed to act. A diagnostic becomes a defect finding only when you can explain a concrete correctness, caller-contract, or project-required build impact; a stricter skill-supplied lint alone does not establish that impact.
 2. Run `python3 -I scripts/audit_scan.py <crate-root>` for what clippy cannot see: glob re-exports, weasel-word type names, ASCII-art comments, design-history docs, parameter tables, bare TODOs, empty `expect` messages, and print macros in library code. Confirm each hit by reading the code; discard false positives.
 3. **Probe behavior the way users meet it.** This is where most user-facing bugs in real crates are found, so spend most of the audit here and in step 5. Read the crate's docs, spec, and tests to list the input features it claims to handle. For each, build a small realistic input that the existing tests do not cover, run it through the public API from a throwaway crate outside the workspace, and compare the output with what a user should get. Cover the features the format allows but the fixtures skip:
    - inherited, indirect, or defaulted values
@@ -108,9 +108,11 @@
 
 ### Severity
 
+Use this scale for defect findings. Missing hygiene alone is recorded in the Hygiene summary, not assigned a defect severity. Still investigate unsafe invariants and panic paths individually; a comment or its absence does not establish soundness.
+
 - **Blocker:** a demonstrated or plausible path to undefined behavior from safe code, a panic reachable from external input in a library, data loss or corruption, a security issue, or a failing build or test suite.
-- **Major:** wrong output or a rejected file on realistic input, work that grows much faster than input on legitimate files, a library panic or process exit on an operating-system or environment failure, work after `fork` or in a signal handler that is not async-signal-safe, `unsafe` that appears sound but lacks SAFETY justification, the wrong error-versus-panic choice, errors callers cannot branch on, misleading docs, public API that will force a breaking change soon, lint or rustdoc failures with the project's own toolchain, tests that do not test behavior.
-- **Minor:** missing docs, missing `Debug`, incomplete `# Safety` sections where the code is sound, lint failures that appear only on a newer toolchain, naming, module placement, comment quality, readability, local convention drift.
+- **Major:** wrong output or a rejected file on realistic input, work that grows much faster than input on legitimate files, a library panic or process exit on an operating-system or environment failure, work after `fork` or in a signal handler that is not async-signal-safe, the wrong error-versus-panic choice, errors callers cannot branch on, misleading docs, public API that will force a breaking change soon, lint or rustdoc failures under the project's required configuration and toolchain, tests that do not test behavior.
+- **Minor:** low-impact correctness or maintainability problems with a concrete consequence. Explain the consequence beyond a departure from a style or documentation default.
 - **Note:** optional improvements and tooling recommendations.
 
 Calibrate honestly: missing comments on correct code are not Blockers, and a long Blocker list for a working crate erodes trust in the real ones.
@@ -119,9 +121,10 @@
 
 Plain text with headings, so it reads well in a screen reader and pastes cleanly into an issue. No emoji and no decorative tables. Structure:
 
-1. A verdict of two to four sentences, including a count of findings per severity.
+1. A verdict of two to four sentences, including a count of defect findings per severity. Do not include hygiene categories in that count.
 2. One section per severity, in order. Each finding is a heading that names the problem in a few words, followed by a location line and a short problem and fix. One problem per finding. Within a severity, put what users hit first, then soundness, then maintainability.
-3. The verification commands, written as plain `cargo` commands the user can rerun without this skill, with pass, fail, or skipped and why.
+3. A compact Hygiene summary, if needed, for confirmed standards gaps without a demonstrated defect. Keep these separate from defect findings and preserve failed-check outcomes.
+4. The verification commands, written as plain `cargo` commands the user can rerun without this skill, with pass, fail, or skipped and why.
 
 ```text
 ## Blocker
--- a/references/audit-checklist.md
+++ b/references/audit-checklist.md
@@ -4,7 +4,7 @@
 
 ## 1. Toolchain results
 
-Run `scripts/check.sh --audit <crate-root>`. A failing build or test is a Blocker. A gate command that fails with the project's own toolchain (its `rust-toolchain.toml` or CI configuration) is Major; one that fails only on a newer toolchain is toolchain drift, reported as one Minor finding. Record pre-existing failures as such. Group the advisory warnings into one Minor finding per category with a count, except unsafe, unwrap, and panic warnings, which are judged one by one. Map each advisory-pass warning to its rule: `undocumented_unsafe_blocks` and `missing_safety_doc` to UNSAFE-1 and UNSAFE-2 (Major, or Blocker only if you can show or plausibly argue a path to undefined behavior), `missing_docs` to DOC-1, `missing_errors_doc` and `missing_panics_doc` to DOC-3 and PANIC-5, `unwrap_used` to PANIC-1, `allow_attributes_without_reason` to LINT, `wildcard_imports` to ORG-15, `missing_assert_message` to PANIC-4, `map_err_ignore` to ERR-5, `dbg_macro`/`todo`/`unimplemented` to PANIC-6, `missing_debug_implementations` to API-19.
+Run `scripts/check.sh --audit <crate-root>`. Record actual build and test failures, project-required lint or rustdoc failures, and pre-existing failures with their scope and toolchain. A newer-toolchain diagnostic or a skill-added lint is not by itself a project build defect. Confirm advisory hits in source and group missing documentation, `Debug`, SAFETY comments, style defaults, and toolchain drift in the compact Hygiene summary, outside the defect tally. Investigate unsafe, unwrap, and panic paths individually; report a defect only with a concrete impact or a plausible safe-code path to undefined behavior. The rule IDs identify standards to check, not automatic defect severities: UNSAFE-1/2 for safety explanations, DOC-1/3 and PANIC-5 for docs, API-19 for `Debug`, and the corresponding LINT, ORG, ERR, or PANIC rule for other diagnostics.
 
 ## 2. Heuristic scan
 
--- a/references/precedence.md
+++ b/references/precedence.md
@@ -11,7 +11,7 @@
 5. The Rust Style Guide (formatting), as implemented by rustfmt defaults.
 6. The Rust API Guidelines, Microsoft Pragmatic Rust Guidelines 2026.6, and Google's Rust Style Guide.
 
-Level 3 sits above level 2 on purpose: matching a file's habit of undocumented `unsafe` or panicking on user input is not consistency, it is copying a defect. Report it instead.
+Level 3 sits above level 2 on purpose: local habit cannot justify unsoundness or an invalid panic on user input. Verify unsafe invariants even when the file omits explanations. In an audit, record missing explanations as hygiene unless you identify a concrete defect; keep the documentation requirement for created or modified code.
 
 ## Decisions on known conflicts
 
--- a/scripts/check.sh
+++ b/scripts/check.sh
@@ -5,8 +5,8 @@
 #
 #   --audit           Also run an advisory clippy pass with the lints that
 #                     enforce this skill's documentation, unsafe, and panic
-#                     rules. Its warnings are findings to report; they do not
-#                     change the exit status.
+#                     rules. Confirm warnings in source before classifying
+#                     them as defects or hygiene; they do not change the exit status.
 #   --target TRIPLE   Lint and document for TRIPLE (for example
 #                     x86_64-pc-windows-msvc) so code behind #[cfg(...)] for
 #                     that platform is checked. Tests are skipped, because a
@@ -153,7 +153,7 @@
         -W clippy::missing_assert_message \
         -W clippy::map_err_ignore \
         -W clippy::too_long_first_doc_paragraph
-    results+=("done  audit lints (advisory; warnings above are findings)")
+    results+=("done  audit lints (advisory; confirm and classify warnings above)")
 fi
 
 echo
```

**Expected effect.** Demonstrated: four independently confirmed Sol categories were counted as defects despite being classified as non-defects for this evaluation, and the same categories appear in both Claude skill reports. Prediction: a shorter, better calibrated defect list with hygiene still available to maintainers. The frozen union might still count the same four non-defect categories when they appear in a summary; a lower measured non-defect count is not guaranteed and must not be manufactured by relabeling. No increase in bugs found or code quality has been demonstrated. Retaining individual unsafe/panic investigation and all gate steps is intended to preserve defect recall and the writing standard.

**Risk and scope.** “Hygiene” can conceal a real omitted safety precondition, misleading API contract, or CI failure if applied mechanically. The exception requires concrete caller or project-required build impact, and plausible safe-code UB still qualifies without an executed crash. The advisory status text change may affect consumers that parse that message; test those consumers while preserving numeric exit behavior. This proposal does not target parser semantics, concurrency discovery, resource exhaustion, the missed aliasing key, or implementing fixes. It does not establish that an undocumented unsafe operation is sound.

**Independent test.** Before adoption, use four new projects outside these audit keys: a byte encoder, a file index, a priority queue and a task scheduler. Freeze their specifications and independent defect/control lists before any run. Include harmless missing docs/Debug/comments, a true missing safety precondition, a behavioral defect in fully documented code, and a project-required documentation build failure. Compare v1.0.0 and the proposed text with two fresh runs per arm per project, equal tools/budgets, blind verification and cross-family review. Record real bugs found, false claims, non-defect categories, defect-tally calibration, report length and unsafe-review coverage. Reject if fewer hygiene headings merely mask lost defect recall or omitted required-check failures. Do not use the prompting moka cases as the acceptance test.

**Confidence:** medium in expected benefit; high in the observed classification/presentation problem. The motivation is general across Sol and Claude, with only one valid Sol audit. This is not a proposal derived from a key miss.

## 2. Require reproducible verification commands in create/modify summaries

**Problem.** The audit report format asks for rerunnable Cargo commands, but the shared completion gate does not make that reporting requirement equally explicit for create/modify work. Two otherwise successful modifications reported totals without the commands needed to reproduce them.

**Evidence and checked quotes.** [s5-e0ce2865](reports/s5-e0ce2865.md), DECOM candidate B with skill, says “Validation used locked, offline, terminal-only commands,” then lists counts without command lines. [s5-a536c375](reports/s5-a536c375.md), candidate C with skill, says “tests passed with default features (180 tests) and without default features (135 tests),” also without commands. Both independent graders awarded D2 0.25/0.5 and full credit elsewhere. B's [first](reports/quality-s5-decom-b-1.md) and [second](reports/quality-s5-decom-b-2.md) judgments agree that the report “supplies no reproducible command lines”; C's [first](reports/quality-s5-decom-c-1.md) and [second](reports/quality-s5-decom-c-2.md) independently identify the same omission. The [resolved sheets](quality_scores.json) preserve 11.75/12 by agreement, with no unresolved dispute or new score assigned here.

The omission is in the report, not evidence of unexecuted or broken tests. I inspected B's actual implementation and seven escape-sequence tests, and C's implementation and seven tests: exported [B implementation](artifacts/s5-e0ce2865/crate/alacritty_terminal/src/term/mod.rs), [B tests](artifacts/s5-e0ce2865/crate/alacritty_terminal/tests/origin_mode.rs), [C implementation](artifacts/s5-a536c375/crate/alacritty_terminal/src/term/mod.rs), [C tests](artifacts/s5-a536c375/crate/alacritty_terminal/tests/origin_mode.rs). They update the origin flag before homing, restore saved state without rehoming, and test observable positions/output and neighboring state. Their independent assessment logs show 23/23 hidden passes with defaults and 23/23 without: [B default](stage5/assessments/decom/B/hidden/hidden-default.log), [B no-default](stage5/assessments/decom/B/hidden/hidden-no-default.log), [C default](stage5/assessments/decom/C/hidden/hidden-default.log), [C no-default](stage5/assessments/decom/C/hidden/hidden-no-default.log). Full evidence indexes and actual commands are in [B checks](stage5/assessments/decom/B/checks.json) and [C checks](stage5/assessments/decom/C/checks.json). All seven submitted regressions also passed Miri in [B](stage5/assessments/decom/B/own-miri-regressions.log) and [C](stage5/assessments/decom/C/own-miri-regressions.log).

The pair matters: baseline [s5-68db0afd](reports/s5-68db0afd.md) gives the explicit Cargo command; baseline [s5-e9ce76ac](reports/s5-e9ce76ac.md) refers to the exact command supplied in the [case prompt](stage5/cases/decom/case.md), which both graders accepted. Baselines scored 12, 12; skill runs 11.75, 11.75; within-arm gaps were zero. This tiny reporting difference does not establish that the skill caused the omissions. B/C performed more reported feature/Miri verification, so a shorter final summary can lose useful command detail even after extensive work. No bug ID or audit union ID applies to this report-quality criterion.

**Change.** Add one paragraph to the shared Verification gate in `SKILL.md`, before the optional Miri/feature/MSRV paragraph. This makes the existing audit reproducibility expectation explicit for the other two modes without adding executions or changing required checks.

```diff
--- a/SKILL.md
+++ b/SKILL.md
@@ -149,6 +149,8 @@
 3. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`
 4. `cargo test --workspace --all-features` (skipped for a cross `--target`)
 
+For create and modify work, finish with a concise verification record as well: give the actual Cargo commands and outcomes, including the working directory, toolchain selector, package/feature/target flags, and environment settings needed to reproduce them. Use project-relative paths where possible; name any temporary harness that must be recreated. Separate passing checks, pre-existing failures, new failures, and checks blocked or skipped with their reasons. Test totals alone are not a reproducible command.
+
 Recommend, and run when already installed: `cargo miri test` for crates with `unsafe`; `cargo hack check --feature-powerset` for crates with several features; a build at the declared `rust-version`. Report each result honestly. A failure that predates your change is reported as pre-existing, not hidden.
 
 ## Reference files
```

**Expected effect.** Demonstrated: the two reports omitted commands, independently lost the same 0.25 clarity/validation component, and passed behavioral checks. Prediction: easier reproduction and fewer unsupported completion claims in new coding tasks. No additional bugs found, fewer non-defect findings, or better implementation behavior is demonstrated; the expected code-quality benefit is reviewability of validation. This proposal does not retroactively restore rubric points or predict a numerical score gain.

**Risk and scope.** Reports can become repetitive or expose unusable scratch paths. The wording asks for concise records and reproducible context, rather than a transcript dump; recreated temporary harnesses must be identified honestly. Exact commands are still insufficient if tools were unavailable or results were misread. This does not target implementation correctness, audit recall, error API design, missing toolchains, or the absence of a pristine-baseline diagnostic reproduction.

**Independent test.** Freeze new cases for creating a streaming checksum library and modifying a duration-conversion CLI, outside DECOM and all existing cases/keys. Include a package-limited workspace, a non-default feature selection and an unavailable optional tool; include one independently documented pre-existing lint failure. Run two fresh agents per arm per case under v1.0.0 versus this paragraph, with identical budgets and no extra prompt demanding command lines. Have a blind reviewer replay the reported commands in new clean workspaces and check outcome claims, scope, flags and blocked/skipped labels. Independently verify source and hidden behavior. Accept only if reproducibility improves without new scope changes, false pass claims or lost behavioral coverage.

**Confidence:** medium; Sol-specific evidence from one paired coding case with two repeats per arm. The requirement is generally applicable, but Claude did not evaluate create/modify work. This is not motivated by a key miss.

## Rules to keep, and changes not justified

**Keep branchable library errors.** In the small-parser case the two baselines, `s5-debb0674` and `s5-aed14d68`, expose only a line accessor and private textual reasons; the two skill submissions, `s5-2183776d` and `s5-26632240`, expose a public typed kind. I checked all four actual sources: [baseline A](artifacts/s5-debb0674/crate/src/lib.rs), [baseline C](artifacts/s5-aed14d68/crate/src/lib.rs), [skill B](artifacts/s5-2183776d/crate/src/lib.rs), [skill D](artifacts/s5-26632240/crate/src/lib.rs). The [first parser sheet](reports/quality-s5-parser-1.md) and [second sheet](reports/quality-s5-parser-2.md) agree on error-API harm of 2/2 for each baseline and 0/2 for each skill run. This was separate from the frozen quality contract: every candidate scored 12/12, and typed categories were not required. The definitive [assessment](stage5/assessments/parser/assessment.json) records 24 hidden passes each; supplemental category adapters passed seven tests each for [B](stage5/assessments/parser/B/adapter/tests.log) and [D](stage5/assessments/parser/D/adapter/tests.log). The absent baseline adapters are unrun, not seven failed tests. Library-case harm was absent in all four runs where typed errors were required. This is a useful descriptive design signal, not evidence of a general causal effect; pooled error-harm presence over the two applicable cases had uncorrected p = 1.0. No stronger error rule is needed.

**Keep surgical modifications, behavioral regression tests, dependency restraint and proportionate structure.** The inspected DECOM source/tests and independent assessment substantiate useful neighboring coverage while preserving interfaces. All 24 resolved submissions have absent dependency and structure harm, with zero within-arm variation on those measures. The [quality records](quality_scores.json) support retaining the current defaults; ceilings and the task prompts prevent attributing their success to the skill. There is no evidence here for removing or tightening these rules.

**Keep public-API probes, confirmation of scanner hits, unsafe invariant review and Miri when available.** `r894fc` found three independently verified high-severity defects, including `moka-3` / `s3-moka-U001`, the Miri-confirmed timer use-after-free ([verifier log](artifacts/verify-s3-moka/tmp/timer-miri.log)). Neither that success nor the original key score proves a skill effect. `moka-4` was missed by both original graders' matching rules; the [blind key](grading/s3-moka/key.json) describes experimental Tree Borrows aliasing behavior, and broader Miri work in the original report was incomplete. The skill already requires unsafe review and adversarial cases. One missed sequence does not justify encoding that deque, API or workload in a general checklist, nor claiming that hygiene displaced the investigation. Any future proposal motivated by this key miss needs independent out-of-key testing before acceptance. Likewise, the `moka-1` partial recheck is not a reason to require reports to recite hidden-key wording.

**Keep the skill's present selection boundary.** The five valid positive discovery cases loaded it and the three excluded cases skipped it ([per-run results](stage2/results.json)). Those cases were supplied a catalog description/path; they test selection and basic following, not host discovery, subtle ambiguous requests, or exhaustive compliance. Initial read-only-harness attempts are invalid. There is no basis to expand triggers or add permissions prompts to repair an evaluation configuration error.

## Evaluation and harness follow-up, separate from skill changes

These are proposals for a future evaluation, not work performed in this review and not evidence for editing the skill:

1. **Preserve exclusions and plan coverage before another comparison.** The policy-blocked audits and invalid source inputs leave no Sol audit effect estimate. Do not retry/rephrase blocked tasks or convert absence to zero. A future permitted, separately declared sample needs fresh paired projects and controls; it cannot repair this sample by substitution after seeing results. Retain the source-skill input scan and writable-container canary already introduced. The [source-isolation review](method/source-skill-isolation-review.json) documents why affected just audits/authors are excluded; those reports are not counterexamples to skill behavior.
2. **Keep build artifacts isolated from the first assessment command.** The parser assessor detected wrong candidate binaries and doctest compiler errors in shared targets, then replayed the unchanged frozen checks in empty directories. See [candidate A's account](stage5/assessments/parser/A/outcome.md), [target-isolation record](stage5/assessments/parser/B/frozen-isolated/target-isolation.json), and [definitive statuses](stage5/assessments/parser/B/frozen-isolated/status.tsv). Require unique source/target/cache identities in future runners and a canary with two same-named crates of different behavior; initial contamination must remain visible. This is assessment isolation, not a treatment parser defect or evidence for a new Rust rule.
3. **Provision complete frozen assessment bundles and independent controls.** DECOM's assessor lacked `prepared.sha256`; the operator's [pre-grading comparison](grading/s5-decom/operator-input-integrity.json) supplements the retained limitation without substituting for behavior. Include assessor-needed manifests from the start. Its [baseline validation](stage5/cases/decom/evidence/validation.md) had ten hidden passes and 13 failures but no known-good repaired control. Future cases should validate a separately prepared acceptable repair before treatment, while preserving author independence. The [fixed-control refresh record](stage5/control-refresh/README.md) also distinguishes valid upstream controls from the superseded original just bundle. Neither missing controls nor historical invalid controls should be presented as additional treatment evidence.
4. **Strengthen independent evidence checks without rewriting grades.** All Sol grading was same-family, and the recheck exposed one key-match disagreement despite original agreement. Cross-family/human inspection should check matched root causes, source-only concurrency claims and a predeclared sample of verifier labels. Retain the original and diagnostic judgments separately. Expand positive/negative controls beyond the single measured moka cell; zero decoy credits and five refutations do not validate every real finding. Do not count unexecuted controls as successes. The earlier Claude results have the same-family and verifier-label limitations too.
5. **Measure tool availability and gate limitations explicitly.** Record exact installed toolchain names/components, chosen selectors and per-step status. The DECOM [gate log](stage5/assessments/decom/B/gate.log) and [assessment index](stage5/assessments/decom/B/README.md) distinguish skipped formatting, Clippy/rustdoc failure and passing tests; unchanged diagnostic locations were inspected, but no independent pristine gate reproduction was supplied. A dated nightly/Miri installation does not itself prove the undated nightly rustfmt command works. Test dated selectors and components in a separate tooling fixture before proposing a gate implementation change. Preserve package scope, feature variants and unavailable tools; do not install tools to make an evaluation result pass. Wikilink exploratory temporary probes were not replayed by the assessor, and x86-only behavior was never natively executed. These limits remain even though required hidden tests passed.

The proposed acceptance tests are future work on new projects. This review reran no treatment task, assigned no scores, and altered no frozen case, rubric, report or analysis. The evidence supports a small reporting revision and preserving the substantive writing rules; it does not support an expanded bug-specific checklist or a causal claim about an individual treatment harm.

# Sol 6.1 evaluation of rust-qc v1.0.0

The separate [revised ordinary QA comparison](review-v2/README.md) is graded:
51 valid reports from 52 planned attempts, no policy blocks, and no reliable
known-bug recall advantage. It uses a narrower task and eligible key set. The
sections below preserve the original broader condition and coding results.
The [new independent recommendations](review-v2/recommended-skill-fixes.md)
propose two untested reporting changes; the tested skill remains unchanged.

All **24 writing/changing submissions passed their frozen hidden checks**. Quality
was close to the 12-point ceiling in both arms: the mean case difference was
**−0.0417 points with the skill**, with zero within-arm replicate variation and
an uncorrected exact sign-flip **p = 1.0** across six cases. Both baseline parser
runs had opaque error categories; neither skill-assisted parser did. That is a
small descriptive design signal, not evidence of a general treatment effect.

**The audit comparison is incomplete.** Only one of 52 planned audits has a valid
report. Four completed reports had invalid inputs, 14 attempts ended in provider
`cyberPolicy` errors, and 33 were not dispatched after their project was blocked.
There is no complete paired audit cell and no audit effect estimate or p-value.
The valid moka report scored 4/5 known bugs under the original two graders, but
an independent unredacted recheck disputed one of two sampled judgments. These
limits prevent concluding that the skill improves bug finding.

Repository checks passed. In discovery, the skill loaded for all five intended
Rust tasks and stayed out of all three excluded tasks. Every evaluation agent
used **`gpt-6.1-sol`, reasoning effort `high`**, with a fresh profile and container
access only. The skill itself is unchanged. A fresh post-grading agent's
[recommendations](recommended-skill-fixes.md) propose separating audit hygiene
from defect counts and making coding verification summaries reproducible.
Both proposals are untested; accepted changes require a later skill version and
Claude review.

## Inputs and isolation

The tested skill is the `git archive` of `v1.0.0`, commit
`1174a565010aa2716c5a6e203cf2dccbac2b4d62`. Evaluation assets come from
`277ee89029502bb52ce1b3a3a0fc3428d97c72cf`. All six valid coding cases, hidden
tests and rubrics were committed at **`08dc878e24394976c0cfcbbe358a9f1cf53782da`**
before the first treatment. Separate agents wrote each case without seeing
treatment output. Main audits and coding tasks had 45-minute budgets; the short
audit cell had 15 minutes. Slot waits were excluded; budgets were not shortened.

Codex CLI `0.166.0` used a clean app-server profile per agent, with memory,
connectors, personal instructions, plugins, hooks and other skills disabled.
Returned model and effort were checked against the exact request, with no model
fallback. Baseline and blind grading arms had no skill; treatment/discovery arms
had only the archived skill under test. The sole supported execution tool called
the unchanged [sandbox launcher](../../sandbox/sb), with no container network,
host home, host repository, credentials or Docker socket. Generic orchestration
tool names remained advertised but supplied no host execution environment;
nested agents were disabled. See the [preflight](preflight/result.json) and
[protocol](method/protocol.md) for the tested boundary and adaptations.

Rust execution used the existing [`rqc-eval:2` image](method/runtime-image.json): aarch64 Linux, Rust `1.99.1`
and installed `nightly-2026-10-01`/Miri. No Rust toolchain or component was
installed, `RUSTUP_AUTO_INSTALL=0` was retained, and no privileged container was
used. Maximum concurrency was three sandbox workers. No usage or disk limit
stopped the evaluation. Timings, slot waits, failure reasons and command-kill
counts are preserved per run in [runs.tsv](runs.tsv), [raw evidence](raw/) and
the [run validation](method/run-validation.json).

## Stage 1: repository checks

The CI-requested Ruff check, ShellCheck, Agent Skills validation, scanner tests
and shell-gate tests passed. Executable CI fixtures returned the expected gate
exit codes 0, 1 and 2 through `sb`. See [check records](stage1/checks.json),
[fixture results](stage1/fixture-checks.json) and [logs](stage1/). The full CI OS
matrix was not reproduced: Python/stub checks ran on macOS; native Rust fixtures
ran on Linux. The maintained evaluation harness also passes 46 tests and Ruff.

## Stage 2: finding and following the skill

| Task | Expected | Observed |
| --- | --- | --- |
| Write Rust | Load | Loaded |
| Fix Rust | Load | Loaded |
| Add rustdoc | Load | Loaded |
| Review Rust | Load | Loaded |
| Tidy a Cargo workspace | Load | Loaded |
| Explain borrowing without code | Skip | Skipped |
| Compare languages without code | Skip | Skipped |
| Change Python in a mixed repository | Skip | Skipped |

All five positive cases read `SKILL.md` in their first tool call and ran
`check.sh`; the review task also ran `audit_scan.py`. The review gate returned 1
for real Clippy findings, which was correct script operation. Its report followed
the severity structure and demonstrated the planted overflow. Change tasks
completed their edits and checks. See [per-run results](stage2/results.json).

Task prompts did not name the skill. The harness supplied its description and
path in the available-skills catalog, so this tests selection and basic following,
not Codex host-directory discovery or exhaustive rule compliance. The initial
eight attempts advertised incorrect read-only permissions; they are retained as
[invalid attempts](invalid-attempts/) and excluded. All eight were rerun from
pristine workspaces after a successful write-canary preflight. The image lacked
`rg`; agents used other search tools without installing it.

## Stages 3 and 4: audit coverage and findings

A separate [correctness-focused review run](review-v2/README.md) now uses the
[revised prompts](method/prompts/correctness-review-v2/README.md) to make ordinary
code-review criteria explicit and exclude dedicated security testing. Its scope
and functional key set differ; all counts and scores below remain the original
evaluation under its original frozen prompts.

| Cell | Planned | Valid completed | Invalid completed | Provider failure | Not dispatched |
| --- | ---: | ---: | ---: | ---: | ---: |
| Main, 45 minutes | 40 | 1 | 1 | 11 | 27 |
| Short prompt, 15 minutes | 12 | 0 | 3 | 3 | 6 |

Two provider-failed just runs also had invalid source inputs; the mutually
exclusive table counts them under provider failure. Missing observations are
never zero scores. No blocked audit was retried or rephrased. Stage 4 changed
budget and prompt together and remains separate from Stage 3. Associated grading
and verifier cells are absent wherever no valid report exists.

The sole valid audit, [r894fc](reports/r894fc.md), used the skill on moka and
finished in 22.696 minutes. One command consumed 120.193 seconds before being
killed at its 120-second cap; its command record is retained. The sandbox's
exit-137 status can also mean a memory kill, so command-kill fields do not by
themselves prove timeout causes.

| Result for r894fc | Count |
| --- | ---: |
| Known bugs found, original agreeing graders | 4/5 |
| Verified real defects | 9: 8 executed, 1 source-verified |
| High / medium defects | 3 / 6 |
| Non-defect items requested by skill hygiene rules | 4 |
| Non-defect items with no value / other non-defect items | 0 / 0 |
| Refuted real-report claims / unverified real-report claims | 0 / 0 |

The four hygiene items remain `not-defect` for comparability with Claude. The
[verified union](findings.jsonl), [verifier report](reports/verify-s3-moka.md)
and [probe artifacts](artifacts/verify-s3-moka/) retain the evidence. The one
source-verified timestamp claim was not reproduced by its executed stress probe;
its `read` status is distinct from an executed reproduction.

Two independently authored decoy bugs were mixed into the moka key. Neither
grader credited either: **0/4 found-or-partial judgments (0%)**. The verifier
refuted **5/5 planted false findings (100%)**. Controls are excluded from all
substantive totals. Stage 4 has no assessed controls, so its rates are undefined,
not zero. Ten projects have [prepared control bundles](controls/), but unused
controls provide no measured grader/verifier rate.

The random unredacted recheck sampled `ceil(10% × 14) = 2` independent key/report
judgments, including decoys in the population. It agreed on moka-2 and downgraded
moka-1 from found to partial because the report did not explicitly identify the
unreachable deque node: **1/2 disagreements**. Original 4/5 scoring is retained
as predeclared; a sensitivity reading would be 3 found, 1 partial, 1 missed.
See [sample provenance](grading/recheck-selection.json) and
[recheck judgments](key_recheck.json). Grader agreement shows stability, not
independent correctness; all grading agents are from the same model family.

The narrow descriptive Claude comparison is the same moka baseline, skill,
nominal 45-minute budget and aarch64 platform: Claude's with-skill runs
`r52978` and `r9e49e` each scored 4/5; Sol's single valid with-skill run scored
4/5 under the original grades, subject to the recheck above. Claude's within-arm
difference was 0; Sol's is unavailable. No arm or model gap is estimated because
Sol has no replicate or baseline. Tool/profile harnesses also differ. See the
[original Claude results](../README.md), which remain reproducible unchanged.

## Stage 5: writing and changing code

Two blind graders applied each frozen 12-point rubric independently. Every
criterion and harm judgment agreed, so no adjudicator was needed. Scores below
are the agreed scores, not grader averages. DECOM used separate profiles for
each candidate, as its rubric forbids consulting other submissions. The remaining
five cases had two fresh graders per case. [Full sheets](quality_scores.json),
[anonymous mappings and redaction logs](grading/), and
[execution assessments](stage5/assessments/) are published after grading.

| Case | Baseline runs | With-skill runs | Skill − baseline | Within-arm absolute difference, baseline / skill |
| --- | --- | --- | ---: | --- |
| DECOM | 12, 12 | 11.75, 11.75 | −0.25 | 0 / 0 |
| heapless truncation | 12, 12 | 12, 12 | 0 | 0 / 0 |
| just dry-run shell calls | 12, 12 | 12, 12 | 0 | 0 / 0 |
| Public-API library | 12, 12 | 12, 12 | 0 | 0 / 0 |
| Small parser | 12, 12 | 12, 12 | 0 | 0 / 0 |
| Wikilink parsing | 12, 12 | 12, 12 | 0 | 0 / 0 |

Mean case difference: **−0.0417/12**, mean within-arm absolute difference **0**,
six complete cases, **uncorrected exact sign-flip p = 1.0**. The only deduction
was DECOM criterion D2: [s5-e0ce2865](reports/s5-e0ce2865.md) and
[s5-a536c375](reports/s5-a536c375.md) summarized outcomes without reproducible
terminal commands. Both graders deducted 0.25. Their behavioral checks passed.
All 24 coding reports had zero redaction changes, so this omission was present
in the original reports. Ceiling scores and two replicates per arm limit the
comparison; they do not establish that either arm is generally equivalent.

Harm was scored separately without deducting quality points again. Authors chose
different raw scales; pooled comparisons therefore use resolved harm presence
(0 absent, 1 present), never an average of unlike severity scales.

| Harm dimension | Raw observations | Mean case difference in presence, skill − baseline | Within-arm difference | Cases | Uncorrected p |
| --- | --- | ---: | ---: | ---: | ---: |
| Error API | Parser baseline 2/2, 2/2; skill 0/2, 0/2. Library all 0/3. Four change cases N/A. | −0.5 | 0 | 2 | 1.0 |
| Unneeded dependencies | Absent in all 24 submissions | 0 | 0 | 6 | 1.0 |
| Needless structure / unrelated edits | Absent in all 24 submissions | 0 | 0 | 6 | 1.0 |

The parser's frozen contract did **not** require typed error categories. Both
baseline parsers met that contract and received 12/12, while the prewritten harm
criterion separately flagged their opaque categories. Both skill runs exposed
typed categories. Library error types were required and all four met that
contract. The parser presence gap is −1, library 0, and each arm's replicate
difference is 0. These two cases cannot establish causation or broad safety.
DECOM's six-item categorical checklist also found no added dependency, needless
module, unrelated edit, or suppressed evidence/regression; error API was N/A.
Raw maxima, checklist states and per-run evidence remain in the full sheets.

| Case | Frozen hidden checks passed per submission | Archived gate, all four | Gate limits |
| --- | --- | --- | --- |
| DECOM | 23 default + 23 no-default | Exit 1 | Clippy/rustdoc diagnostics outside changed lines; formatting skipped. No independent pristine diagnostic reproduction. |
| heapless | 1 upstream + 2 neighbors | Exit 1 | Baseline Clippy diagnostics; nightly rustfmt unavailable. |
| just | 2 upstream + 9 neighbors | Exit 1 | Clippy/rustdoc failures and five environment-dependent integration failures. |
| Wikilink | 1 upstream + 4 neighbors, debug and release | Exit 1 | Clippy failure; workspace docs/tests blocked by missing `llvm-objdump`. |
| Parser | 24 | Exit 0 | All six frozen commands passed after isolated build-directory reruns. |
| Library | 8 API + 3 error-category | Exit 0 | All required checks passed; adapters match actual enum variants. |

Gate failures, skipped tools and compile/environment blocks are reported
separately from hidden behavioral results and rubric judgments. Package tests,
doctests, feature variants and Miri checks vary by case; the per-candidate
assessment logs state exactly what ran. All **72** assessment/grading source
copies match their original submissions, with caches excluded; see
[source-integrity evidence](method/submission-integrity.json).

## Departures and remaining limits

- The upstream just checkout contained another `SKILL.md`. It was not advertised
  but was readable, violating the sole-skill constraint. All six affected audits
  and the original just case/control authors are invalid and excluded. Fresh
  author workspaces quarantined that directory; no coding treatment used the
  invalid case. Later runs scan inputs for other skills. See the
  [input-isolation review](method/source-skill-isolation-review.json).
- The first h2 control author failed at a 64 KiB transport-reader limit. A tested
  8 MiB reader and fresh full-budget author resolved that harness failure; both
  histories are retained. This was not a usage or disk limit.
- DECOM has a failing-baseline control (10 hidden passes, 13 failures), but no
  known-good repaired control. The three upstream change cases have fixed-source
  controls. Missing cached dependencies required separate pre-treatment native
  [control refreshes](stage5/control-refresh/); the invalid original just refresh
  is historical only and the valid v2 bundle supersedes it.
- The DECOM assessor could not run the author's full manifest because preparation
  logs were withheld. Before any quality grading, the operator independently
  checked every supplied runtime asset against the frozen case commit and the
  gate against the archived skill. Both the limitation and supplemental hashes
  remain in the grader inputs; no source, assertion or rubric changed.
- The parser assessor detected stale artifacts in shared build directories and
  reran all frozen checks in empty isolated directories. Initial and definitive
  logs are retained. Supplemental category-adapter tests informed only the
  prewritten harm criterion, not new quality requirements.
- Wikilink's required hidden checks completed, but exploratory temporary probes
  mentioned in three original reports were not replayed by the assessor because
  their temporary workspaces were not mounted. Original artifacts/transcripts
  are retained. Missing tools and nightly alias/component limits were not fixed
  by installing software.
- Only aarch64 Linux was used for Rust execution; x86-only behavior was read,
  not natively executed. Same-family grading, tiny control denominators, quality
  ceilings, blocked audit coverage and uncorrected tests limit inference.

## Reproduction, publication and review

From the repository root, reproduce the final descriptive tables with:

```sh
python3 evals/results/sol-6-1/score.py
python3 evals/results/score.py
```

The new scorer imports the original exact sign-flip function unchanged. See
[summary.json](summary.json), [method scripts](method/) and the
[provenance manifest](grading/provenance.json). Original Claude score output
remains byte-for-byte identical, recorded in
[claude-reproducibility.json](method/claude-reproducibility.json). Only the valid
moka audit receives an append-only, quoted Sol key row; absent/invalid runs do
not receive fabricated scores. No Claude key row, skill file or sandbox launcher
was edited.

The public export retains final reports, prompts, model-visible tool calls and
outputs, source/probe artifacts, assessment logs, frozen schedules and post-grade
blind mappings. It omits authentication, profiles, raw sessions, reasoning,
transport deltas, server logs, account/thread identifiers and build caches.
Host paths, email addresses and token-like strings are scrubbed. The final scan
covers raw evidence, reports, artifacts, controls, grading, schedules, preflight,
all stages, invalid attempts, root documents and method data records.
**Zero unreviewed configured privacy-pattern hits remain.**
Eleven matches are hash-bound reviews of nonsecret container PATH values, literal
public-source recipe text, and random compressed public-baseline bytes. See
[scan records and reviews](export-scans/). This states the scan's result, not a
claim that pattern scanning proves the absence of every possible private string.

All coding grading and reconciliation decisions are complete. The recommendations
agent was provisioned after data-freeze commit `2fb8577` and received Claude's
results and the Sol arm mapping for the first time then. Its
[input hashes](method/recommendation-inputs.json) and
[document validation](method/recommendation-validation.json) record that boundary.
All 75 local evidence links resolve and both proposed diffs pass `git apply --check`
against the archived skill; neither diff was applied. Claude's planned cross-family
sample regrading, evidence checks and sandbox probe reruns remain review work;
they have not been performed in this Sol evaluation.

# Correctness-review v2

The revised ordinary QA comparison is graded. Of 52 planned review attempts,
51 produced valid reports; one provider capacity failure remains missing, with
no replacement. There were no v2 policy blocks. All 39 blind grading/verifying
runs and 13 fresh diagnostic recheck runs completed with valid provenance.

The data do not establish a reliable known-bug recall advantage for the skill.
The detailed primary gap is smaller than within-arm repeat variation, and it
vanishes in the sampled-recheck sensitivity scenario. This is not proof of
equivalence or a result about security review.

| Primary reproduced-key measure | Paired projects | With skill | Baseline | Mean project gap | Mean within-arm difference | Uncorrected p |
|---|---:|---:|---:|---:|---:|---:|
| Detailed, 45-minute ceiling | 9 | 34/80 (42.5%) | 32/80 (40.0%) | +0.111 | 0.556 | 0.8125 |
| Short, 15-minute ceiling | 3 | 10/36 (27.8%) | 13/36 (36.1%) | -0.500 | 0.500 | 0.5 |

The gaps and within-arm differences above are counts per report. The
[complete tables](paired-results.md) give both replicates and both within-arm
differences beside every project gap for every frozen measure. Raw per-run
scores are in [summary.json](summary.json), with identities and execution
status in [runs.tsv](runs.tsv). P-values use the original exact sign-flip
function and are uncorrected for multiple comparisons.

## Scope and missingness

Both arms reviewed ordinary correctness, documented contracts, errors/API use,
small boundary/regression tests, builds/docs and concrete maintainability. No
dedicated security assessment, threat modeling, exploitability analysis,
exploit development or resource-exhaustion investigation was requested.
Treatment received only archived rust-qc v1.0.0; baseline profiles received no
skills. Every model agent used fresh isolated profiles, `gpt-6.1-sol`, high
effort and the unchanged `sb` sandbox. No Rust toolchain or component was installed.

The [prospective eligible set](comparison-eligibility.json) contains 50 of the
old 92 definitions; 47 reproduced entries form the primary subset. The other
42 entries are outside scope, not misses. Detailed and short stages remain
separate. The broader historical Sol and Claude conditions used different tasks
and denominators and must not be pooled with these results. There is no new
Stage 5 coding experiment here; the [historical coding results](../README.md)
are unchanged.

The detailed pulldown-cmark cell has three reports because one attempt failed
with `serverOverloaded` before producing a final report. Its available scores
remain [descriptive observations](unpaired-observations.md), and the entire
cell is excluded from paired effects. The [prospective capacity rule](capacity-deviation-plan.json)
and [recovery record](capacity-recovery-outcome.json) preserve the original
queue, model, effort, prompts and budgets. Missing observations are never zero.
See the [primary completion gate](primary-completion.json).

## Verification, controls and rechecks

Ten independent source-only authors prepared two false key entries and five
false findings per project, with successful ordinary regression refutations.
Their baseline source stayed unchanged. Two independent key graders and one
verifier assessed each of 13 cells. All original key judgments agreed, so no
adjudicators were needed. See [control validation](controls-validation.json),
[grading commitments](grading-input-checks.json),
[completion/sample freeze](grading-completion.json) and
[provenance](grading/provenance.json).

| Diagnostic | Detailed | Short |
|---|---:|---:|
| False key judgments marked found | 0/156 | 0/48 |
| False key judgments marked found or partial | 8/156 | 0/48 |
| Injected false findings refuted | 50/50 | 15/15 |

Controls never enter substantive totals. The 65 false-finding injections reuse
50 unique claims across the two stages. Partial decoy matches show that matching
thresholds still need caution; successful control rejection is not a guarantee
that every real judgment is correct.

A frozen random ceiling-10% sample selected 74 of 734 independent key/report
judgments, including decoys. Thirteen fresh agents checked 69 distinct pairs
against unredacted reports. Five judgments disagreed, all on real key entries;
this is a diagnostic disagreement rate of 6.8%, not an estimated error rate.
The [complete diagnostic](key_recheck.json) retains both judgments and evidence.

Primary grades are unchanged. In the [separate sampled-change scenario](diagnostic-sensitivity.json),
strict detailed recall becomes 32/80 versus 32/80 (p=1), and strict short recall
becomes 9/36 versus 13/36 (p=0.5). Unsampled grades are left untouched; no error
rate is extrapolated. This illustrates sensitivity to match-rule interpretation.

Graders received no arm mappings, but report style and residual totals prevent
a claim of perfect blinding. After the first verifier completed, before arm-level
analysis, a coverage check found issue totals and severity-count summaries that
the preflight missed. All frozen inputs were preserved. The [recorded limitation](grading-blinding-limitations.json)
must accompany verifier and control results. Grading is from the same model
family, with small project samples and no external expert validation.

## Other QA observations

Across the nine paired detailed projects, verifiers classified 232 skill-arm
and 206 baseline report/finding memberships as defects (mean project gap
+1.444, within-arm difference 2.667, p=0.234375). These are report item counts,
not unique new bugs. The short counts were 28 versus 34 (gap -1.0,
within-arm difference 2.333, p=0.5).

Requested hygiene items increased: 86 versus 28 in detailed reviews (gap
+3.222, within-arm difference 0.556, p=0.00390625), and 23 versus 6 in short
reviews (gap +2.833, within-arm difference 0.167, p=0.25). Build/docs and
maintainability were explicitly requested, so these counts are not automatic
harm. Verifiers marked no real report claims refuted or no-value; that observation
also needs the grading and blinding qualifications above. Full status and severity
breakdowns are in the paired tables and [findings](findings.jsonl).

## Reproduction and evidence

The [comparison plan](comparison-plan.json), [protocol](protocol.md),
[workspace preparation](comparison-preparation.json) and input commitments
precede their respective dispatches. Published [schedules](schedules/) and
[grading maps](grading/) were withheld until all grading and rechecking finished.
The [toolchain review](primary-toolchain-checks.json) covers all 1,943 commands
from the primary attempts, including the failed attempt's 38 commands.

[Export notes](export-notes.md) explain selective evidence publication, exact
retained probe sources, hash-checked baseline-copy restoration and large-input
regeneration. Authentication, profiles, sessions and reasoning are excluded.
The maintained dispatch/grading harness passes 60 tests, Ruff and the namespace
shim's ShellCheck check; candidate Rust was not run on the host for those tests.

To reproduce numerical tables from the repository root with the existing scorer:

```sh
python3 - <<'PYCODE'
import importlib.util
from pathlib import Path
root = Path('evals/results/sol-6-1')
spec = importlib.util.spec_from_file_location('sol_score', root / 'score.py')
score = importlib.util.module_from_spec(spec)
spec.loader.exec_module(score)
score.HERE = root / 'review-v2'
score.main()
PYCODE
```

The readiness pilot remains excluded regardless of outcome. Its 15.867 measured
active minutes, 47 commands and 19 unverified candidate findings are recorded
in the [pilot report](reports/v2-pilot-pulldown-cmark.md) and [checks](pilot-checks.json).
It supplies no effect estimate.

The [independent recommendations](recommended-skill-fixes.md) propose separate QA
hygiene counts and clearer reproduction/verification evidence. Both are untested
reporting changes; the existing discovery and coding rules are retained. The
[validation](recommendation-validation.json) checks 85 local evidence links, both
patch contexts and all 10,492 frozen input files. The tested skill is unchanged.

The recommendation profile first received arm mappings and historical Claude
results after [data commit 6d5a0c6](https://github.com/coryj627/rust-qc/tree/6d5a0c6006ce1b1b9b8dbc280fa86d6c74c479aa/evals/results/sol-6-1/review-v2).
`publication-manifest.json` describes that pre-recommendation tree. The final
run table adds only the recommender's provenance; frozen grades and numerical
results remain unchanged. See [recommendation provenance](recommendation-provenance.json).

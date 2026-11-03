# Independent blind grading rubric — just-6

Use this same rubric separately for Grader A and Grader B. Each grader receives the task brief, baseline source, anonymized candidate diff/source, candidate-authored tests and report, and candidate validation logs. Neither grader sees the other's scores, treatment identity, arm mapping, or the upstream production patch. Authoring control results establish that the oracle distinguishes the snapshots; they are not evidence that a candidate passes. Do not infer quality from similarity to a reference patch.

Grade observable behavior and review the candidate's changes. Accept any implementation that meets the task, including different internal designs or reuse of existing formatting helpers. Do not require any particular helper, module, algorithm, comment, or patch length. A short, clear implementation can earn full credit. Do not reward verbosity, test counts, or stylistic conformity. Apply only requirements in the task; do not demand new features or wholesale modernization.

## Quality: 0–12 points

For each one-point behavior subcriterion, award 1 when evidence establishes every listed condition, 0.5 for a substantively working but incomplete condition, and 0 for incorrect or absent behavior. Use candidate test logs and source evidence together. A build failure is not an intended-behavior test failure: report it explicitly and mark any unverified behavior as such rather than fabricating outcomes. Compilation that is broken by the candidate earns no verified behavior points. An environment failure independent of the candidate requires an explicit limitation and provisional assessment.

| Criterion | Maximum | Measurable allocation |
|---|---:|---|
| B1. Dry-run suppresses shell execution | 3 | 1: a failing `shell()` command in recipe interpolation succeeds under dry-run without running that command. 1: the same guarantee holds for assignment evaluation and nested shell calls. 1: a side-effecting shell call leaves a sentinel unchanged, and dry-run does not require launching the configured shell. |
| B2. Unevaluated call display | 3 | 1: output identifies the unevaluated `shell(...)` call and preserves the command, every argument, and their order with unambiguous quoting/separators. 1: quotes and backslashes are escaped correctly in the command and arguments. 1: empty strings, tab/newline/carriage return, and ordinary Unicode text remain unambiguous and preserve their values; control characters do not turn one displayed argument into extra output lines. |
| C. Compatibility | 2 | 1: outside dry-run, shell execution, side effects, positional arguments, trailing-newline trimming, arity errors, and command-failure reporting retain baseline behavior. 1: existing backtick behavior is preserved in dry-run and normal execution. For each point, award 0.5 if one mode or a material subcondition is broken; 0 if the behavior is broadly broken. |
| R. Candidate regression coverage | 2 | 1: candidate adds a runnable test asserting no shell execution in dry-run (a nonzero command or direct side-effect assertion is sufficient), with expected status/output or side-effect assertions that would fail on baseline and pass with the candidate. 1: candidate adds a runnable test asserting quoted/escaped dry-run shell output with a quote or another genuinely escaping-sensitive argument, which would fail on baseline and pass with the candidate. Award 0.5 within either point if the test reaches the relevant path but misses the key assertion or is not correctly registered; 0 if absent, tautological, or unable to detect the regression. One test may earn both points if it independently covers both requirements. Hidden tests supplied by the evaluator are not candidate-authored coverage. |
| D. Clarity and documentation | 1 | 1: the change is readable in the existing code context, and any added comments/documentation/report accurately describe behavior and verification. No new prose is required when the code is self-explanatory. 0.5: a material ambiguity or unsupported claim remains. 0: misleading documentation or opaque control flow materially impedes review. No points for length. |
| S. Necessary scope | 1 | 1: substantive edits are limited to implementing this behavior, directly supporting tests, and any useful corresponding documentation. 0.5: a small avoidable refactor or unrelated edit is included. 0: broad unrelated churn, version changes, or changes to unrelated public behavior. Formatting volume by itself is not a deduction. |

The two verbatim upstream regressions are the primary hidden oracle. Preserve and report their exact assertions and results. Neighboring cases probe only the task's stated guarantees; they are not a reason to require the upstream implementation. If a neighboring exact-string assertion rejects an alternative representation that preserves values and is clearly quoted/escaped, inspect it and permit an equivalent representation consistent with the primary oracle. Record this adjudication with the actual output. Do not waive an assertion merely to excuse incorrect behavior. Do not treat a raw test count as the quality score or zero the entire score for one failed check.

## Harm: separate assessment, not subtracted from the 12 points

Report each category separately using N/A, 0 (none), 1 (localized avoidable harm), or 2 (material avoidable harm), with concrete changed-path evidence. Do not automatically penalize every dependency or module: justify why it is unnecessary for this task. Do not conflate pre-existing baseline design with candidate-introduced harm. Quality and harm may describe the same evidence, but do not subtract a second time from quality.

| Harm category | Applicability and anchors |
|---|---|
| H1. Unbranchable error types | **N/A by default:** this task does not need a new error API, and pre-existing string-based errors are not candidate harm. If the candidate introduces or changes an error interface: 0 when callers can still distinguish relevant failure categories programmatically or no relevant distinction is lost; 1 for an avoidable localized collapse into an opaque string; 2 for a public or widely propagated loss of structured error distinctions. Cite the new interface and the affected caller behavior. |
| H2. Unjustified dependencies, including test dependencies | 0: no added dependencies, or additions have a concrete necessity not reasonably met by existing facilities. 1: an unnecessary localized production/dev/build dependency or unnecessary feature expansion. 2: substantial avoidable dependency burden, offline build breakage, or broad toolchain/platform requirements caused by unjustified additions. Review all manifest and lockfile changes. Existing baseline dependencies are not harm. |
| H3. Needless modules or unrelated edits | 0: each added module/edit has a clear task-related purpose; a focused test module is appropriate. 1: a needless wrapper/module or small unrelated edit. 2: extensive unnecessary layering, unrelated feature work, or widespread churn. Do not penalize a module solely because the upstream patch used fewer files. |

## Independent report template

Each grader independently returns:

- Anonymous candidate ID and grader ID (A or B).
- B1 __/3, B2 __/3, C __/2, R __/2, D __/1, S __/1; total __/12.
- Evidence for every awarded/deducted subcriterion: command and test name or changed path with line reference. Distinguish observed results from source-only reasoning.
- Primary oracle: each of two tests pass/fail/not run; cause of any failure.
- Neighbors and existing compatibility checks: outcomes and any justified equivalent-output adjudication.
- H1 N/A/0/1/2; H2 0/1/2; H3 0/1/2, each with a brief reason.
- Build/environment limitations and remaining uncertainty; no claims that unexecuted checks passed.

Freeze both reports before sharing them. Resolve disagreements by comparing evidence against these criteria; preserve both original scores and document any reconciliation. Do not reveal treatment identities or consult the upstream production patch during reconciliation.

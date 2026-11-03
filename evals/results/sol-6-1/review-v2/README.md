# Correctness-review v2

The readiness pilot and the separately frozen 52-attempt review batch have closed.
The batch produced 51 valid reports and one preserved capacity failure, with no
policy block. Independent controls are validated and blind grading is prepared;
there is no scored outcome yet. The [comparison plan](comparison-plan.json), [analysis protocol](protocol.md)
and [workspace preparation](comparison-preparation.json) precede its dispatch.
No dedicated security assessment, threat modeling, exploitability analysis or
exploit development is requested. The pilot's complete
[rendered prompt](prompts/v2-pilot-pulldown-cmark.txt) was also frozen before
dispatch, alongside its [plan](pilot-plan.json).

The pilot reviews pulldown-cmark's documented behavior, ordinary API use, errors,
regression coverage, build/docs and maintainability. It receives only the pinned
crate and dependencies, the archived rust-qc v1.0.0 skill, and executable scratch.
It has no keys, old reports, host repository, other skills, memory, connectors,
account context or network. The clean profile runs `gpt-6.1-sol` at high effort
with a full 45-minute ceiling, excluding sandbox waits. All execution uses the
unchanged `evals/sandbox/sb`; no Rust toolchain or component may be installed.

The run is a readiness observation only. It receives no bug-key score and will
not be included in a later effect estimate, regardless of its outcome. A later
scored comparison requires a separately frozen plan and key eligibility for the
narrower scope, identical across arms. Original audit results remain separate.

The prospective [functional key set](comparison-eligibility.json) was selected
from the existing definitions before reading the pilot report: 50 of 92 entries
cover ordinary output, errors, API invariants and bounded regression behavior.
Focused memory-safety, privilege/containment and resource-exhaustion investigations
are outside that recall measure. These are operator scope decisions, not model
grades or a claim that included defects have no possible security implications.
The pilot remains excluded from a future comparison regardless of its result.

There is exactly one pilot attempt. Any provider policy block stops the entire
new condition. The dispatcher uses `--stop-on-policy`, which also prevents a
later invocation from continuing a condition that already contains a blocked
attempt. No prompt variations or alternative accounts/models are tried after
a block. Usage or disk limits retain the existing global stop behavior.

The new batch-stop behavior was tested first against the old implementation:
both new tests failed because it continued to an independent project. The updated
dispatch and grading harness passes 60 tests, Ruff and the namespace shim's
ShellCheck check. Rust candidate code
was not built or run on the host during those Python tests.

Status: completed in 15.867 measured active minutes with no provider error, 47 sandbox commands, no transcript/provenance flags and no detected host execution. The agent reported 19 candidate findings; those claims have not been independently graded or verified. This is not a bug-count or skill-effect result. See the [report](reports/v2-pilot-pulldown-cmark.md), [checks](pilot-checks.json) and [scrubbed evidence](raw/v2-pilot-pulldown-cmark/).

## Comparison progress

The primary batch closed at 23:26 UTC on 2026-11-03: 39 detailed and 12 short
reviews completed with valid evidence. All 52 attempted workspaces passed the
post-run sole-skill and archived-skill byte checks. The 51 completed reports
passed provenance, host-execution and grading-input presentation checks across
1,905 sandbox commands. There was no v2 `cyberPolicy` error or usage/disk stop.
See the [completion gate](primary-completion.json). These are execution checks,
not verified findings or an effect estimate.

The dispatcher stopped new work after the capacity error. Its missing final
report remains a gap. The prospective [recovery and missingness
rule](capacity-deviation-plan.json) preserves the original queued tasks and
requires clean completion of active reviews before continuing with one
previously unattempted task. A second capacity failure stops for an availability
decision. There is no replacement run; the incomplete detailed pulldown-cmark
cell is excluded from the paired effect. Completed reports can still be graded
with explicit gaps if the remaining queue completes cleanly. Eligibility, task
wording, full budgets, model and effort are unchanged by this infrastructure
departure.

The [recovery record](capacity-recovery-outcome.json) and completion gate confirm
that the original schedule and all 52 prompt hashes were preserved. The capacity
failure remains missing, with no replacement. Ten fresh control authors worked
from source without review outputs. All 70 false controls have passing ordinary
regression evidence and unchanged baseline source; see [validation](controls-validation.json).
The [grading-input freeze](grading-input-checks.json) records 26 blind key graders
and 13 independent verifiers, exact input round trips and zero dropped sections.
Main and short paired effects can include at most nine and three projects respectively.

# Sol 6.1 evaluation of rust-qc v1.0.0

The repository checks pass. In the corrected discovery trial, Sol loaded the
skill for all five intended Rust tasks and skipped it for all three excluded
tasks. These eight small cases do not establish whether the skill improves bug
finding or code quality. Audit dispatch has ended with only one valid report;
neither audit cell supports a paired comparison. All 24 writing/changing runs
have completed; hidden-test assessment and blind grading are in progress.

Every evaluation agent uses **`gpt-6.1-sol`, high reasoning effort**. The skill is
the `v1.0.0` archive at `1174a565010aa2716c5a6e203cf2dccbac2b4d62`; evaluation
assets are from `277ee89029502bb52ce1b3a3a0fc3428d97c72cf`. No skill files or
Claude results have been changed.

## Completed checks

Stage 1: Ruff, ShellCheck, Agent Skills validation, scanner tests and shell-gate
tests all pass. The executable CI fixtures return their expected exit codes
0, 1 and 2 through the network-disabled sandbox. See [logs](stage1/).

Stage 2:

| Task | Expected skill use | Observed |
| --- | --- | --- |
| Write Rust | Load | Loaded |
| Fix Rust | Load | Loaded |
| Add rustdoc | Load | Loaded |
| Review Rust | Load | Loaded |
| Tidy a Cargo workspace | Load | Loaded |
| Explain borrowing without code | Skip | Skipped |
| Compare languages without code | Skip | Skipped |
| Change Python in a mixed repository | Skip | Skipped |

All five positive cases read `SKILL.md` in their first tool call. The negative
cases did not read it. `check.sh` executed for each applicable Rust task, and
`audit_scan.py` executed for the review task. The review fixture's gate returned
1 for real Clippy findings; this is correct script operation, not an invocation
failure. The change tasks completed their container edits and checks.

The review report followed the severity/report structure and demonstrated the
planted overflow. These observations establish invocation and basic following;
they are not a full rule-by-rule compliance score. `rg` was absent from the
existing image; agents used other file-search tools without installing it.

See [per-run results](stage2/results.json), [verbatim reports](reports/),
[scrubbed tool evidence](raw/) and [returned code](artifacts/).

## Limits and departures recorded so far

- The harness supplies the archived skill description and path in the available
  skills catalog. Task prompts do not name the skill. This tests selection and
  following, not Codex's own host-directory discovery implementation.
- The first eight discovery attempts incorrectly advertised host read-only
  permissions to agents using writable containers. All eight were invalidated,
  retained in [invalid-attempts](invalid-attempts/), and rerun from pristine
  workspaces after a write-canary and blocked-host-access preflight.
- Across the two audit cells, 14 attempts ended with service `cyberPolicy`
  errors, and 33 further audits were not dispatched for blocked projects. These
  are missing observations, not zero scores. No retry or prompt workaround was
  attempted. One main-cell moka report completed with valid inputs; its two key graders
  agree on four of five known bugs. An unredacted random recheck downgraded one
  of two sampled judgments from found to partial; original grades and the
  disagreement are retained. Independent verification is still running. No short-cell report is valid for scoring.
- The upstream just source contained its own `SKILL.md`. It was not in the
  skill catalog but was readable, violating the requested sole-skill boundary.
  All six affected audits and the original just case/control authors were
  invalidated. Four of those audit reports had completed; two overlap the service
  failures above. Fresh author inputs exclude the bundled skill, and all later
  model runs enforce a source-tree skill scan. No coding treatment used the
  invalid case. See the [input review](method/source-skill-isolation-review.json).
- The first h2 control author hit a 64 KiB transport-reader limit. It is retained
  as a harness failure; a fresh author completed after the reader was fixed and
  tested. This was unrelated to usage or disk limits.
- All execution is aarch64 Linux; x86-only behavior cannot be executed natively.
- All six valid coding cases and rubrics were frozen at `08dc878` before the
  first treatment. No statistical skill-versus-baseline result, decoy rate,
  verifier control rate or skill-change recommendation is available yet.

## Reproducibility and privacy

The [protocol](method/protocol.md) records the frozen inputs, clean profiles,
container boundaries, tool adaptations and remaining gates. `sb` is unchanged;
all agent file operations and execution are dispatched into its containers.
The native host patch tool was tested and failed because the agent has no host
execution environment. All eight valid Stage 2 runs pass the adapted transcript
and host-execution checks; the checkers also detect a seeded host-build violation.

The export includes final reports, model-visible tool inputs/outputs, prompts and
run metadata. It excludes authentication, profiles, raw session files, reasoning,
transport deltas, server logs and thread identifiers. Host paths, email addresses
and token-like strings are scrubbed. The expanded scan covers transcripts,
reports, returned artifacts, control bundles and Stage 5 evidence. Current matches
are reviewed container PATH values and public source fixture data; no unreviewed
configured privacy-pattern hit remains. Scan records and file-hash-bound reviews
are under `export-scans/`. Publication will be checked again when the remaining
results are added.

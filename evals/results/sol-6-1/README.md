# Sol 6.1 evaluation of rust-qc v1.0.0

The repository checks pass. In the corrected discovery trial, Sol loaded the
skill for all five intended Rust tasks and skipped it for all three excluded
tasks. These eight small cases do not establish whether the skill improves bug
finding or code quality. The keyed audits and writing/changing evaluation are
still in progress.

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
- Two initial pulldown-cmark keyed audits were terminated by the model service
  with `cyberPolicy` errors. They produced no final reports and cannot be scored
  as successful audits or as zero bugs found. No retry or prompt workaround has
  been attempted.
- All execution is aarch64 Linux; x86-only behavior cannot be executed natively.
- No statistical skill-versus-baseline result, decoy rate, verifier control
  rate or skill-change recommendation is available yet.

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
and token-like strings are scrubbed. The scans for the current valid and invalid
Stage 2 exports found no remaining configured privacy-pattern hits; scan records
are under `export-scans/` in each dataset. Publication will be checked again when
the remaining results are added.

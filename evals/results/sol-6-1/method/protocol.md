# Sol evaluation protocol and execution log

This directory records work on issue #23. Evaluation is in progress. No skill
effect or recommendation can be inferred from the repository checks or preflight.

## Frozen inputs

- Skill: `v1.0.0`, commit `1174a565010aa2716c5a6e203cf2dccbac2b4d62`.
- Evaluation assets: `277ee89029502bb52ce1b3a3a0fc3428d97c72cf`.
- Both were extracted with `git archive`. The installed development copy is not
  used as the experimental intervention.
- Model: `gpt-6.1-sol`, reasoning effort `high`. The runner checks the live model
  catalog, disables provider fallback, and verifies the thread's returned model
  and effort before submitting a task. Per-run requests and metadata retain both.
- Codex CLI: `0.166.0`. Host: macOS arm64. Container: aarch64 Linux, installed
  Rust `1.99.1`, installed nightly `nightly-2026-10-01` with Miri.
- The existing `rqc-eval:2` image is reused. Its Dockerfile is not rebuilt because
  it installs a Rust toolchain. No Rust toolchain or component is installed.
- Initial host free space: 141 GiB as displayed by `df -h`; Docker filesystem
  free space: 797 GiB. Docker memory: 8,317,202,432 bytes. `sb` retains three slots.

## Agent isolation

Each agent receives a new temporary Codex profile with authentication only, no
personal instructions, memory, connectors, plugins, hooks, or bundled skills.
The profile is outside its container and its authentication copy is deleted when
the agent finishes. Raw app-server events stay in private scratch storage.

The app-server thread has `environments: []`. Host shell tools and web access are
disabled. Its only supported execution tool is `sandbox_exec`; the harness
dispatches that tool through an unchanged copy of `evals/sandbox/sb`. Reading,
searching, editing, building, testing, and running probes all happen in that
container. It mounts only the agent workspace and that run's build volume; it
has no host home, repository, Docker socket, credentials, or network. No container
uses `--privileged`. `RUSTUP_AUTO_INSTALL=0` is retained.

The runtime still advertises some generic orchestration tools. They do not grant
an execution environment. Nested agents are prohibited and configured with
`max_threads = 1`, `max_depth = 0`. Native `apply_patch` is unsupported; changes
use shell/file operations through `sandbox_exec`. The pure JavaScript tool gateway
must remain enabled to dispatch the container tool. The first preflight disabled
it and therefore could not execute any commands; no evaluation used that setup.

A separate Sol high preflight observed UID 1000, `/work` as the working directory,
absent host repository/profile paths and Docker socket, and `Network is
unreachable` on a socket connection to an external IP. No skills were available
in that preflight. A subsequent preflight also wrote and read a container canary;
native `apply_patch` failed with `No usable execution environment is available`.
The turn uses `externalSandbox` with restricted network to describe the actual
container permissions without incorrectly implying that `/work` is read-only.

For discovery tasks the harness enumerates the sole extracted skill's exact
frontmatter description and workspace path in developer context. The task prompt
does not name it or its path. This replaces host skill discovery so no global
skills can leak into the experiment. It tests selection and following of that
catalog entry; it does not test Codex's host filesystem discovery implementation.
Audit skill arms receive `skill/`; baseline arms receive no skill or catalog.

## Tool and transcript adaptations

- Claude's Read/Grep/Glob/Write/Edit/Bash surface becomes `sandbox_exec` using
  ordinary shell commands inside the container. There is no host `./run` step:
  the dispatcher already calls `sb run`. Both arms use the same tool.
- Standard input is closed. File creation uses heredocs. Commands are capped at
  900 seconds and at the remaining task budget. Full command output is retained;
  model-facing output over 60,000 characters is truncated at both ends.
- Elapsed time is measured by the supervisor, less recorded slot waits, and
  returned with every command result. The audit briefs retain their 45-minute
  and 15-minute budgets. A run that reaches its budget without a final report is
  recorded as incomplete, never silently shortened or scored as a clean audit.
- Final messages are collected verbatim. Raw session events are never committed.
  Publication requires a separate export and privacy scan.

## Stage 1

The five requested checks pass on the frozen checkout: Ruff `0.17.3`, ShellCheck,
`agentskills validate` from `skills-ref==0.1.1`, scanner fixture tests, and shell
gate tests. Logs are in `../stage1/`. The three executable CI fixture checks run
through `sb` and return the expected 0, 1, and 2. CI's Linux/macOS matrix was not
reproduced locally: Python/stub checks ran on macOS, Rust fixtures on Linux.

## Stage 2

`../stage2/cases.json` freezes five positive and three negative discovery tasks.
Each uses a fresh workspace containing its fixture and the archived skill at
`.agents/skills/rust-qc/`. No task prompt names the skill. The per-task ceiling is
45 minutes; these are not the keyed audit cells.

The first eight attempts (IDs without `-v2`) used the host `read-only` policy in
model context. Several agents therefore declined container edits or builds,
although `/work` was writable. All eight are invalidated as harness failures,
retained separately, and excluded from results. The identical frozen cases are
rerun from pristine workspaces with `-v2` IDs after the canary preflight. This is
a disclosed repeat, not a skill failure or a selection of preferred outputs.

## Remaining gates

Before keyed audits: fetch pinned baselines and git2's pinned submodule, run
`prep_audits.py leaks`, freeze randomized run order, and test transcript checkers
on a known violation. Before writing/changing runs: commit all six cases, hidden
tests and independently written rubrics. Grading includes decoy keys, confirmed
false findings, logged redactions and a separate 10% recheck. Recommendations
must wait until all grading finishes and come from a new agent that has not
audited, graded, read Claude's results, or seen Sol's arm mapping beforehand.

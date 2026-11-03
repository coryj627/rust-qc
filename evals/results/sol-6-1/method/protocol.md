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

During the final Stage 5 input review, the pinned `just` source was found to
contain `crate/skills/just/SKILL.md`. It was never advertised in the skill catalog,
but remained readable as source data. Under the requested sole-skill boundary,
all eight model runs with that source are invalid: the original case author and
control author, two main audits, and four short audits. Their outputs and
per-run input reviews are retained, excluded from every score and case freeze.
`source-skill-isolation-review.json` records the affected IDs. No treatment had
started. All other source/cache templates were scanned and had no extra skills.

The bundled `just` skill directory was quarantined outside all agent mounts in
fresh templates/workspaces. New case/control authors receive pristine cleaned
inputs and none of the invalid authors' outputs. Blocked audits are not retried.
Every subsequent model run scans its entire input workspace for other SKILL.md
files before creating a profile or contacting the model. This source exclusion
is recorded separately from the pinned Rust sources, which remain unchanged.

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

## Stages 3 and 4

All ten pinned templates were built from fresh archives. The git2 submodule was
fetched at `338e6fb681369ff0537719095e22ce9dc602dbf0`. The original dependency
leak checker found zero projects requiring inspection. The 40-run Stage 3 order
was randomized and frozen privately before dispatch. Baseline and treatment
workspaces share the same source/cache template; only treatment has the skill.

Stage 4 has its own randomized 12-run schedule and 15-minute budget. Its task is
exactly `Review this crate for problems.` plus common workspace, build, sandbox
and timing instructions. Skill selection uses the sole available-skills catalog,
without the detailed report instructions in Stage 3. Do not pool these cells.

Provider `cyberPolicy` errors have stopped some audits without final reports.
These are missing observations, not zero scores. The scheduler never retries an
existing run directory and stops further audits for the blocked project in that
schedule. Other projects continue. Other failure kinds stop new dispatch; a
usage or disk limit requires stopping the evaluation as the issue specifies.

The runner was tightened after the initial audit dispatch to check its deadline
even while response events stream continuously, and to record its own source
hash. Earlier runs retain the earlier runner provenance in git. Budgets and
task wording were not changed.

Later dispatchers use atomic per-run claims so an additional worker can safely
take queued work when a slot becomes available. They never reuse an existing
run directory. A shared resource-stop marker halts new dispatch and interrupts
new-version runners if a usage/disk limit is observed. No such limit has been
observed so far. Exit-137 command records are possible timeout **or** memory
kills; the original sandbox does not distinguish these causes. Do not interpret
the legacy `command_timeouts` field as proof of a timeout.

The first `control-h2` author stopped at about 3.4 minutes when an app-server
JSON event exceeded Python's default 64 KiB line-reader limit. Its metadata was
left incorrectly marked running by an uncaught exception; the operator preserved
that state and recorded the correction to a harness failure. No final report was
collected. New runners allow 8 MiB per event, retain exception/report status in
all exit paths, and are tested with a 360 KiB escaped result. The failed author
is excluded and replaced by a fresh author with the full 45-minute budget, the
same source/prompt, and no prior output. Existing running agents are not restarted.

## Case preparation and remaining gates

Six separate clean-profile agents author the Stage 5 cases and rubrics. They
receive source/specification inputs but no treatment outputs, skill text, arm
mapping or other author's rubric. The chosen upstream bugs are heapless-6,
pulldown-cmark-3 and just-6; selection preceded audit output inspection. DECOM,
the parser and the public-API library complete the six cases. All case bundles,
including hidden tests, must be committed before any treatment starts.
`prepare_stage5.py` checks committed bytes and author manifests before provisioning
24 independent workspaces. Stage 5 uses a frozen 45-minute budget per treatment.
Captured author code is evidence and is excluded from repository Python linting;
maintained orchestration code remains linted.

The three upstream-case authors encountered uncached dependencies in their fixed
snapshots. Their initial failures and supplemental source checks are retained.
Before freezing treatment inputs, the operator provisioned separate fixed-source
workspaces, fetched their package dependencies with the original `sb fetch`, and
replayed the author-owned hidden-test runners with `sb run`. These are deterministic
control checks, not additional model treatments. No toolchain was installed and
no treatment template/cache was changed. `../stage5/control-refresh/` records
commands, statuses, fetched lockfiles and the distinction from author-run checks.

Independent control authors receive source and real defect summaries only, never
audit reports or mappings. They must confirm two false key entries and five false
findings per project. Graders see anonymized key IDs with decoys mixed in.
Verifiers receive the redacted reports as ordered, lossless text passages, with
five false passages inserted among them. Private maps identify controls for
exclusion from totals. Every redacted line replacement/drop is recorded; text
inside code fences and trailing blank lines are preserved. The verifier's input
also removes the opening numeric finding/severity tally, while preserving the
remaining evidence summary, so the tally does not reveal injected controls.
This replacement is logged separately from report redaction. Two independent key
graders and one verifier are provisioned per audit cell. Missing reports remain
absent. A separate random 10% recheck uses unredacted reports. The population is
every independent key-grader verdict (including decoys), sampled without
replacement after grading finishes, rounded upward. Seed, source hashes and
selected judgments are frozen privately. A fresh rechecker for each sampled
cell sees only selected pairs, the blind key, and full unredacted reports;
original judgments and maps are withheld. This checks key-report matching, not
the Stage 5 quality rubrics, which have their own two independent graders.

Recommendations must wait until all grading finishes and come from a new agent
that has not audited, graded, read Claude's results, or seen Sol's arm mapping
beforehand. No recommendation has been made during preparation.

The DECOM assessor could not run the author's full `prepared.sha256` check: that
manifest was omitted with author preparation logs to keep them out of assessment
inputs. The missing check remains in its report. Before any quality grader starts,
the operator separately compares every supplied case, rubric, scoring document,
hidden test/runner and runtime baseline asset byte for byte against case-freeze
commit `08dc878`, and compares the gate with the archived skill. Per-case
`operator-input-integrity.json` records those hashes and scope. This supplements
file-integrity evidence; it does not replace, rerun or change behavioral results,
source, tests or rubric. Graders receive the retained assessment limitation and
the supplemental check. Case-wide assessment context is preserved when filtering
DECOM's JSON down to a single candidate. A canary test rejects an altered asset.

## Analysis rules fixed before grading

- Key scores require both independent sheets. Agreements retain the shared
  verdict. Disagreements remain `disputed` until a separate blind adjudicator
  reads the reports and match rules; they are never silently treated as misses.
- Planted key entries and false findings are excluded from every substantive
  total, regardless of whether graders/verifiers believe them. A verifier item
  that mixes a planted and real claim is flagged for review and prevents that
  project's union metrics from entering a comparison until resolved.
- Stage 5 follows the frozen authors' reconciliation instructions: two independent
  12-point sheets are preserved, then disagreements go to a fresh blind adjudicator
  with source, test evidence and both sheets. Final scores use agreement or the
  separately recorded reconciliation, never a simple mean. The original mean
  and grader difference are descriptive only. Unresolved intervals remain
  intervals and do not enter point comparisons. Harm dimensions are reported
  separately, with not-applicable cases distinguished from missing scores.
  Independent authors chose different harm scales: numeric maxima of 2 or 3,
  and a categorical checklist for DECOM. Before grading, pooled harm comparisons
  were therefore defined as resolved harm presence (0 absent, 1 present),
  retaining original points/maxima and checklist states alongside them.
  This measures presence, not a shared severity unit. DECOM's runtime/test
  dependencies map to the common dependency axis; its modules/unrelated edits
  map to the common structure axis. Any present item makes that axis present;
  otherwise an unresolved item remains unknown. Its additional regression/test
  suppression checklist is retained separately. Unknown applicable harm is null
  and excluded, never converted to no harm. No quality points are deducted again.
- DECOM's rubric forbids consulting other candidates. Its four submissions each
  receive two separate fresh grader profiles (and separate reconciliation when
  needed). Other cases use two independent graders per case, applying the rubric
  independently to each anonymous submission. All grader inputs exclude other
  graders' outputs until the reconciliation stage. Criterion IDs, maxima and
  increments are transcribed in `../stage5/rubric-structure.json`, with hashes of
  the unchanged frozen rubrics, and validated when sheets are collected.
- Each comparison requires two scored runs in each arm for that project/case.
  Missing runs/grades are not zero. Every reported gap includes both replicate
  values and within-arm absolute differences. The exact sign-flip function is
  imported unchanged from `evals/results/score.py`; p-values are uncorrected.
  No p-value is produced for an empty set of complete comparison cells.
- Stage 3, Stage 4 and Stage 5 remain separate. Claude comparisons must use only
  matched Stage 3 settings and disclose the changed tool/profile harness.

## Export

The exporter uses an allowlist of completed user/agent/tool events and command
logs, excluding reasoning, sessions, profiles and transport metadata. Agent-created
symlinks cannot be followed into host files. Changed sources and probe artifacts
are retained; build caches, compiled binaries/core dumps and files above 10 MB
are excluded with a per-export record. Deleted source paths are also recorded.
Cargo's CACHEDIR.TAG signature identifies custom build-output directories as
well as directories literally named target. The privacy scan caught Rust
metadata in one such directory before commit; that artifact export was rebuilt
with directory pruning and a regression test. A scan for the same cache marker
also found generated files in the first coding run's earlier local commit. That
unpublished commit was amended to remove them; source/probe evidence was retained.
No generated cache data was published.
The privacy review retains nonsecret PATH matches: a container-local
fake-fzf probe and the container's standard tool PATH. It also records a random
personal-name match in compressed public source bytes. All 324 regular files in
that archive were checked against the pinned baseline; an upstream terminal
recording contains public fixture paths, not host data from this evaluation.
Hashes and dispositions are in `../export-scans/privacy-review.json` and
`baseline-archive-review.json`. Scanning covers raw evidence, reports, artifacts,
control bundles, grading inputs/maps, schedules and all Stage 5 preparation/assessment files.

## Publication and handoff

All 24 coding treatments completed with valid inputs and no transcript-checker
flags. Their source/probe evidence is committed independently of later grading.
The one valid audit receives a quoted, append-only Sol row in the moka key;
failed or invalid audits receive no fabricated key scores. Original Claude key
bytes and result files are preserved. The original scorer's output is compared
byte for byte before and after the append in `claude-reproducibility.json`.

The unredacted recheck is diagnostic and does not silently replace the original
key scores. Its disagreements must accompany any result based on those scores.
The recommendations profile is created only after every evaluation agent has
finished and all quality reconciliation decisions are frozen. It receives the
archived skill, scrubbed final Sol evidence, full submission sources and the
earlier Claude results for the first time at that point. Its input hashes and
fresh-profile provenance are recorded separately. It cannot edit the skill.

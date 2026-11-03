# Correctness-review v2: readiness pilot

This is one new, narrower code-quality review using the approved v2 prompt. Its
purpose is to check execution of that task in the isolated runner before preparing
a scored comparison. It is not a retry of a security investigation and requests
no dedicated security assessment, threat modeling, exploitability analysis or
exploit development. The complete [rendered prompt](prompts/v2-pilot-pulldown-cmark.txt)
is frozen before dispatch, alongside the [pilot plan](pilot-plan.json).

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

There is exactly one pilot attempt. Any provider policy block stops the entire
new condition. The dispatcher uses `--stop-on-policy`, which also prevents a
later invocation from continuing a condition that already contains a blocked
attempt. No prompt variations or alternative accounts/models are tried after
a block. Usage or disk limits retain the existing global stop behavior.

The new batch-stop behavior was tested first against the old implementation:
both new tests failed because it continued to an independent project. The updated
dispatcher passes the full 48-test harness suite and Ruff. Rust candidate code
was not built or run on the host during those Python tests.

Status: prepared; no outcome recorded yet.

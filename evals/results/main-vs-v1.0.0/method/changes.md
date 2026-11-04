# Changes from earlier rounds

Each change below comes from issue 30 or from the Execution design comment on it.

## Audit brief (`audit-brief.md`)

Starts from the with-skill brief that `evals/results/method/write_briefs.py` writes, which is `evals/results/method/audit-brief-with-skill.md` with the project filled in.

- The Report section is replaced with: "Use the skill's report format. Your final message is the report. End with the last `./clock` reading." The old section asked every run for evidence and would hide PR 28's effect. (Issue 30, Stage 1, change 1.)
- "It is aarch64 Linux, so x86-only code cannot run natively." becomes "It is x86_64 Linux." (Execution design.)
- Both arms get the skill paragraph, because both arms use a skill. Only the contents of `skill/` differ.
- Workspaces live at `/srv/rqc/audits/projects/<run>`, so briefs carry no session or scratch paths.

## Redaction (`redact.py`)

Starts from `redact()` in `evals/results/method/prep_grading.py`. (Issue 30, Stage 1, change 2.)

- Field labels are removed, including `Cause:`, `Evidence (ran):` and `Evidence (read):`, so fields read as plain text.
- `## QA hygiene` and `## Unverified` both become `## Other notes`, and "QA hygiene" in running text becomes "other notes".
- Every dropped line goes to `dropped-lines.txt` with its reason.
- Tested by `test_redact.py` on one fixture per format; the fixtures, outputs and test output are in `fixtures/`.

## Transcript checks (`check_runs.py`, `host_builds.py`, `streams.py`)

Adapted to read `claude -p` stream-json instead of subagent output files. (Issue 30, Stage 1, change 3.)

- `check_runs.py` now also flags any Bash command that is not `./run` or `./clock`, and any wrapper command with `;`, `&`, `|`, a backtick or `$(` outside quotes.
- Both are shown catching a planted violation in `fixtures/planted-stream.jsonl`, and passing `fixtures/clean-stream.jsonl`; output in `fixtures/transcript-checks-output.txt`.

## Workspaces (`prep_workspaces.py`, `common.py`)

- `RUN` and `CLOCK` run the same commands as `evals/results/method/prep_audits.py`. `RUN` is wrapped onto two lines.
- The crate is cloned and archived inside the sandbox with `sb fetch`, not on the host.
- Before any audit, `CLAUDE.md`, `CLAUDE.local.md`, `AGENTS.md`, `SKILL.md` and `.claude` in `crate/` are renamed with a `.quarantined` suffix, because agents start without `--bare`. (Execution design.)

## Agents (`launch.py`, `common.agent_command`)

Each agent is a separate `claude -p` process started in its workspace, with the flags from the Execution design comment: `--model claude-opus-5-5 --effort high --setting-sources "" --strict-mcp-config --disable-slash-commands --tools Read,Grep,Glob,Edit,Write,Bash --allowedTools "Bash(./run *)" "Bash(./clock)" "Edit(./crate/**)" "Edit(./tmp/**)"`, `BASH_DEFAULT_TIMEOUT_MS=960000` and `BASH_MAX_TIMEOUT_MS=960000`, under `timeout 90m`.

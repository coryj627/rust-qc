# Commands run in Stage 1

Paths: `<clone>` is the repository checkout. Audit files live under `/srv/rqc/audits`: `sb` (copied from `main` at `1b8c898`), `projects/<run>/` (workspaces) and `private/` (arm map, skill trees, agent streams, canary). No agent can read `private/`.

## Setup

    nohup dockerd &
    cp <proxy CA bundle> <main checkout>/evals/sandbox/ca/proxy.crt
    # image rqc-eval:2 already built in Stage 0 from main at 1b8c898; see image.json
    git fetch origin tag v1.0.0
    git archive v1.0.0 .agents/skills/rust-qc   | tar -x   # -> private/skill-v1.0.0
    git archive 691e96b .agents/skills/rust-qc  | tar -x   # -> private/skill-main

## Pilot workspaces

    python3 -I prep_workspaces.py template alacritty_terminal alacritty/alacritty alacritty_terminal_v0.25.0
    python3 -I prep_workspaces.py runs alacritty_terminal 1

The template step reported no instruction files to rename.

## Preflight probes

    python3 -I launch.py rprobe  --model claude-haiku-5-5   # brief: preflight-probe.md
    python3 -I launch.py rprobe2 --model claude-haiku-5-5   # brief: preflight-probe-2.md
    python3 -I launch.py reffort                            # brief: "Reply with the word ok."
    python3 -I probe_view.py private/streams/<run>.jsonl

## Checks of the method

    python3 -I test_redact.py
    python3 -I check_runs.py rplant1=fixtures/clean-stream.jsonl
    python3 -I check_runs.py rplant1=fixtures/planted-stream.jsonl
    python3 -I host_builds.py rplant1=fixtures/clean-stream.jsonl
    python3 -I host_builds.py rplant1=fixtures/planted-stream.jsonl

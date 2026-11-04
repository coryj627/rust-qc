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

## Pilot

    # start order drawn at random: r2a990b, then rde3dd6
    python3 -I launch.py r2a990b rde3dd6
    python3 -I check_runs.py r2a990b=private/streams/r2a990b.jsonl rde3dd6=private/streams/rde3dd6.jsonl
    python3 -I host_builds.py r2a990b=private/streams/r2a990b.jsonl rde3dd6=private/streams/rde3dd6.jsonl
    python3 -I pilot_checks.py <results>/stage1

Both transcript checks found nothing in either pilot run. check_runs.py counted 9 and 6 calls that Claude Code's permissions refused.

## Rerun with the closing-summary brief

    # first run's private files moved to private/run1/; probe and pilot workspaces rebuilt
    python3 -I launch.py rprobe rprobe2 --model claude-haiku-5-5
    python3 -I launch.py reffort
    python3 -I prep_workspaces.py runs alacritty_terminal 1
    # start order drawn at random: r3f1b1f, then rae3760
    python3 -I launch.py r3f1b1f rae3760
    python3 -I check_runs.py r3f1b1f=... rae3760=...
    python3 -I host_builds.py r3f1b1f=... rae3760=...
    python3 -I pilot_checks.py <results>/stage1

# Commands run in Stage 2

## Reproducers

Reproducer projects live under `/srv/rqc/repro/projects/<project>`, with their own copy of `sb`.

    python3 -I run_repros.py setup <clone> moka
    python3 -I run_repros.py run <clone> moka <results>/stage2/reproducers.tsv <results>/stage2/logs
    python3 -I run_repros.py clean <clone> moka
    run_all_repros.sh <clone> <results> h2 quinn-proto just heapless git2 mlua zip pulldown-cmark simd-json

The five `blocked` rows were classified by reading each failing log and script; the reasons are in each row's note.

## Key rules

    python3 -I prep_keyrules.py <clone> git2 h2 heapless just mlua moka pulldown-cmark quinn-proto simd-json zip
    python3 -I launch.py kr-git2 kr-h2 kr-heapless kr-just kr-mlua kr-moka kr-pulldown-cmark kr-quinn-proto kr-simd-json kr-zip --root /srv/rqc/keyrules --read-only
    # answers parsed from each stream's result, from the first '{' to the last '}', into private rules.json
    python3 -I add_key_rules.py <clone> <results>/stage2 <rules.json>

## Controls

    python3 -I prep_workspaces.py keyed <clone> h2 quinn-proto just heapless git2 mlua zip pulldown-cmark moka simd-json
    python3 -I prep_controls.py <clone> h2 quinn-proto just heapless
    python3 -I launch.py c-heapless c-h2 c-just c-quinn-proto          # order drawn at random
    python3 -I prep_controls.py <clone> git2 mlua zip pulldown-cmark moka simd-json
    python3 -I launch.py c-git2 c-zip c-mlua                            # order drawn at random
    python3 -I launch.py c-moka c-pulldown-cmark c-simd-json            # order drawn at random
    python3 -I collect_controls.py <results>/stage2 <project>...

## Transcript checks

Output in `stage2/transcript-checks.txt`.

    python3 -I check_runs.py --root /srv/rqc/keyrules kr-<project>=<stream> ...
    python3 -I check_runs.py c-<project>=<stream> ...
    python3 -I host_builds.py c-<project>=<stream> ...

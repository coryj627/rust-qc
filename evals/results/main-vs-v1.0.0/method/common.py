"""Shared pieces for the main vs v1.0.0 round: wrappers, the audit brief, and how agents start.

RUN and CLOCK run the same commands as evals/results/method/prep_audits.py, with RUN wrapped onto
two lines. audit-brief.md is the with-skill brief from evals/results/method/write_briefs.py, with
the changes listed in changes.md.
"""

import importlib.util
import pathlib

AUDITS = pathlib.Path("/srv/rqc/audits")
ARMS = {"v1.0.0": "0bae19f", "main": "691e96b"}
MODEL = "claude-opus-5-5"
EFFORT = "high"

RUN = """#!/bin/sh
# Runs a command in this workspace's container: no network, only this folder mounted, at /work.
here="$(cd "$(dirname "$0")" && pwd)"
RQC_WAIT_LOG="$here/.waits" RQC_TIMEOUT=900 exec "$here/../../sb" run "$(basename "$here")" \\
  env TMPDIR=/work/tmp "$@"
"""

CLOCK = """#!/bin/bash
# Prints minutes used. Time spent waiting for a container slot does not count.
here="$(cd "$(dirname "$0")" && pwd)"
now=$(date +%s)
[ -f "$here/.clock-start" ] || echo "$now" > "$here/.clock-start"
start=$(cat "$here/.clock-start")
waited=0
if [ -f "$here/.waits" ]; then waited=$(awk '{s+=$1} END {print s+0}' "$here/.waits"); fi
used=$(( (now - start - waited) / 60 ))
echo "$now $used" >> "$here/.clock-log"
echo "$used minutes used of 45"
"""

# Files Claude Code or a skill loader could read as instructions.
# They are renamed in crate/ before any audit.
QUARANTINE = {"CLAUDE.md", "CLAUDE.local.md", "AGENTS.md", "SKILL.md", ".claude"}


def _earlier_notes():
    """Name, audited path and build notes for the 10 keyed projects, from earlier rounds."""
    source = pathlib.Path(__file__).parents[2] / "method" / "write_briefs.py"
    spec = importlib.util.spec_from_file_location("write_briefs", source)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.NOTES


NOTES = {
    **_earlier_notes(),
    "alacritty_terminal": (
        "alacritty_terminal 0.25.0",
        "crate/alacritty_terminal/src/",
        (
            "`cargo test --locked -p alacritty_terminal`. "
            "Work from `/work/crate`, which is the alacritty workspace."
        ),
    ),
}

BRIEF = (pathlib.Path(__file__).parent / "audit-brief.md").read_text()

TOOLS = "Read,Grep,Glob,Edit,Write,Bash"
ALLOWED = ["Bash(./run *)", "Bash(./clock)", "Edit(./crate/**)", "Edit(./tmp/**)"]
ENV = {"BASH_DEFAULT_TIMEOUT_MS": "960000", "BASH_MAX_TIMEOUT_MS": "960000"}


def agent_command(prompt, model=MODEL, effort=EFFORT, tools=TOOLS, allowed=None):
    """The exact claude -p command line for an agent, from the Execution design comment."""
    allowed = ALLOWED if allowed is None else allowed
    return [
        "timeout",
        "90m",
        "claude",
        "-p",
        prompt,
        "--model",
        model,
        "--effort",
        effort,
        "--setting-sources",
        "",
        "--strict-mcp-config",
        "--disable-slash-commands",
        "--tools",
        tools,
        *(["--allowedTools", *allowed] if allowed else []),
        "--output-format",
        "stream-json",
        "--verbose",
    ]

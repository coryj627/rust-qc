#!/usr/bin/env python3
"""Check audit streams for rule breaks.

Adapted from evals/results/method/check_runs.py to read claude -p stream-json.

Usage: check_runs.py <run_id>=<stream.jsonl> ...

Flags forbidden tools (web, Skill, Agent, MCP), file tools used outside the workspace, and Bash
commands that name other folders, use the network, or run anything other than ./run and ./clock.
Prints only a summary and the flagged commands, never transcript text. Exits 1 if any run has a
flag.
"""

import collections
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import AUDITS
from streams import summary, tool_uses

FORBIDDEN_TOOLS = {"WebSearch", "WebFetch", "Skill", "Agent", "Task", "ToolSearch", "NotebookEdit"}
FILE_TOOLS = {"Read", "Write", "Edit", "Grep", "Glob"}
BAD_TEXT = re.compile(
    r"evals|/keys\b|audits/private|audits/projects/(?!{run})\w|/home/|\.claude"
    r"|\bcurl\b|\bwget\b|git clone|git fetch|\bsb fetch\b|crates\.io|github\.com"
)
WRAPPER = re.compile(r"^\s*(\./run\s|\./clock\s*$)")


def check(run_id, stream):
    workspace = str(AUDITS / "projects" / run_id)
    bad_text = re.compile(BAD_TEXT.pattern.replace("{run}", run_id))
    counts, flags = collections.Counter(), []
    for name, data in tool_uses(stream):
        counts[name] += 1
        if name in FORBIDDEN_TOOLS or (name or "").startswith("mcp__"):
            flags.append(f"TOOL {name}: {str(data)[:120]}")
        if name in FILE_TOOLS:
            target = str(data.get("file_path") or data.get("path") or "")
            if target and not (target == workspace or target.startswith(workspace + "/")):
                flags.append(f"{name} outside workspace: {target[-120:]}")
        if name == "Bash":
            command = data.get("command", "").replace(workspace, "<ws>")
            if bad_text.search(command):
                flags.append("BASH text: " + command[:200].replace("\n", " "))
            if not WRAPPER.match(command):
                flags.append("BASH not a wrapper: " + command[:200].replace("\n", " "))
            unquoted = re.sub(r'"[^"]*"', '""', re.sub(r"'[^']*'", "''", command))
            if re.search(r"[;&|`\n]|\$\(", unquoted):
                flags.append("BASH runs more than the wrapper: " + command[:200].replace("\n", " "))
    s = summary(stream)
    print(
        f"{run_id}: models={s['models']} tools={dict(counts)} out={s['output_tokens']}"
        f" cost={s['reported_cost_usd']} flags={len(flags)}"
    )
    for flag in flags[:15]:
        print("   ", flag)
    return len(flags)


def main():
    total = sum(check(*pair.split("=", 1)) for pair in sys.argv[1:])
    sys.exit(1 if total else 0)


if __name__ == "__main__":
    main()

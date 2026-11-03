#!/usr/bin/env python3
"""Check audit transcripts for rule breaks, and print only a summary, never transcript text beyond flagged commands.

Usage: check_runs.py <scratch> <run_id>:<agent_id> ...

Flags: forbidden tools (web, Skill, Agent), file tools used outside the workspace, and Bash commands that name other
folders, use the network, or run cargo or rustc outside ./run. Also prints the models seen, tool counts, output tokens,
clock reading and whether a report exists.
"""
import collections
import json
import pathlib
import re
import sys

FORBIDDEN_TOOLS = {"WebSearch", "WebFetch", "Skill", "Agent", "Task", "ToolSearch"}
FILE_TOOLS = {"Read", "Write", "Edit", "Grep", "Glob", "NotebookEdit"}
BAD_TEXT = re.compile(r"evals|/keys\b|/repos/|sandbox/projects|audits/private|audits/projects/(?!{run})\w|charming-cannon-1b77a1(?!/1211ef1d)|/Dev/rust-qc|\bcurl\b|\bwget\b|git clone|git fetch|\bsb fetch\b|crates\.io|github\.com")
HOST_BUILD = re.compile(r"(^|[;&|]\s*)(cargo|rustc|rustup)\b")


def tool_uses(path):
    for line in path.open():
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue
        message = entry.get("message") or {}
        yield_usage = message.get("usage") or {}
        content = message.get("content")
        yield ("meta", message.get("model"), yield_usage)
        if isinstance(content, list):
            for block in content:
                if isinstance(block, dict) and block.get("type") == "tool_use":
                    yield ("tool", block.get("name"), block.get("input") or {})


def main():
    scratch = pathlib.Path(sys.argv[1])
    for pair in sys.argv[2:]:
        run_id, agent_id = pair.split(":")
        workspace = str(scratch / "audits" / "projects" / run_id)
        bad_text = re.compile(BAD_TEXT.pattern.replace("{run}", run_id))
        counts, models, flags = collections.Counter(), set(), []
        tokens_out = tokens_in = 0
        for kind, name, data in tool_uses(scratch.parent / "tasks" / f"{agent_id}.output"):
            if kind == "meta":
                if name:
                    models.add(name)
                tokens_out += data.get("output_tokens", 0)
                tokens_in += data.get("input_tokens", 0) + data.get("cache_read_input_tokens", 0) + data.get("cache_creation_input_tokens", 0)
                continue
            counts[name] += 1
            if name in FORBIDDEN_TOOLS or ((name or "").startswith("mcp__") and not name.startswith("mcp__ccd_turn__")):
                flags.append(f"TOOL {name}: {str(data)[:120]}")
            if name in FILE_TOOLS:
                target = str(data.get("file_path") or data.get("path") or "")
                if target and not target.startswith(workspace) and "/tool-results/" not in target:
                    flags.append(f"{name} outside workspace: {target[-120:]}")
            if name == "Bash":
                command = data.get("command", "")
                stripped = command.replace(workspace, "<ws>")
                if bad_text.search(stripped):
                    flags.append("BASH text: " + stripped[:200].replace("\n", " "))
                outside = re.sub(r"\./run\s+.*|<ws>/run\s+.*", "", stripped, flags=re.S)
                if HOST_BUILD.search(outside):
                    flags.append("BASH host build: " + stripped[:200].replace("\n", " "))
        folder = pathlib.Path(workspace)
        log = folder / ".clock-log"
        clock = log.read_text().splitlines()[-1].split()[1] if log.exists() and log.read_text().strip() else "?"
        report = folder / "report.md"
        size = len(report.read_text().splitlines()) if report.exists() else 0
        print(f"{run_id}: models={sorted(models)} tools={dict(counts)} out={tokens_out} in={tokens_in} clock={clock} report_lines={size} flags={len(flags)}")
        for flag in flags[:15]:
            print("   ", flag)


if __name__ == "__main__":
    main()

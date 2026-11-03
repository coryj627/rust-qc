#!/usr/bin/env python3
"""Validate Sol event provenance, model settings, reports and run budgets."""

import argparse
import json
from pathlib import Path

MODEL = "gpt-6.1-sol"
EFFORT = "high"


def read_jsonl(path):
    if path.exists():
        for line in path.read_text().splitlines():
            if line.strip():
                yield json.loads(line)


def check(folder):
    metadata = json.loads((folder / "metadata.json").read_text())
    flags = []
    if metadata.get("model") != MODEL or metadata.get("effort") != EFFORT:
        flags.append("model_or_effort_mismatch")
    if metadata.get("status") != "completed" or not metadata.get("report_collected"):
        flags.append("incomplete_run")
    if not (folder / "report.md").read_text().strip():
        flags.append("empty_report")
    if metadata.get("active_seconds", 0) > metadata["budget_seconds"] + 1:
        flags.append("budget_overrun")
    if metadata.get("unexpected_requests"):
        flags.append("unexpected_server_request")
    count = 0
    for event in read_jsonl(folder / "events.jsonl"):
        if event.get("method") != "item/completed":
            continue
        item = event["params"]["item"]
        kind = item["type"]
        if kind == "dynamicToolCall":
            count += 1
            if item.get("tool") != "sandbox_exec":
                flags.append("unexpected_dynamic_tool")
        elif kind not in {"agentMessage", "reasoning", "userMessage", "contextCompaction"}:
            flags.append(f"unexpected_item:{kind}")
    commands = list(read_jsonl(folder / "commands.jsonl"))
    if count != len(commands):
        flags.append("tool_command_count_mismatch")
    for command in commands:
        if command.get("execution") != "sb run" or command.get("tool") != "sandbox_exec":
            flags.append("host_or_unknown_execution")
        if not 0 < command.get("timeout_seconds", 0) <= 900:
            flags.append("unbounded_command")
    return {"run_id": metadata["run_id"], "model": metadata.get("model"),
            "effort": metadata.get("effort"), "commands": len(commands), "flags": flags}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folders", type=Path, nargs="+")
    args = parser.parse_args()
    results = [check(folder) for folder in args.folders]
    print(json.dumps(results, indent=2))
    raise SystemExit(int(any(result["flags"] for result in results)))


if __name__ == "__main__":
    main()

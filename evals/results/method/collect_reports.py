#!/usr/bin/env python3
"""Copy each finished run's returned report from its transcript into audits/private/returned/<run>.md.

Usage: collect_reports.py <scratch>   (reads audits/private/agents.tsv: run id, agent id)

Prints run, tool that carried the text, and its line count. Uses the last SubagentHandback, or else the last assistant
text. Runs with neither are listed as missing.
"""
import json
import pathlib
import sys


def returned(path):
    handback = text = None
    for line in path.open():
        try:
            message = json.loads(line).get("message") or {}
        except json.JSONDecodeError:
            continue
        content = message.get("content")
        if message.get("role") != "assistant" or not isinstance(content, list):
            continue
        for block in content:
            if not isinstance(block, dict):
                continue
            if block.get("type") == "tool_use" and block.get("name") == "SubagentHandback":
                data = block.get("input") or {}
                handback = next((v for v in data.values() if isinstance(v, str) and len(v) > 20), handback)
            if block.get("type") == "text" and block.get("text", "").strip():
                text = block["text"]
    return ("handback", handback) if handback else ("text", text)


def main():
    scratch = pathlib.Path(sys.argv[1])
    out = scratch / "audits" / "private" / "returned"
    out.mkdir(exist_ok=True)
    for row in (scratch / "audits" / "private" / "agents.tsv").read_text().splitlines():
        run_id, agent_id = row.split("\t")
        how, body = returned(scratch.parent / "tasks" / f"{agent_id}.output")
        if not body:
            print(run_id, "MISSING")
            continue
        (out / f"{run_id}.md").write_text(body)
        print(run_id, how, len(body.splitlines()), "lines")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""List Bash commands that run cargo, rustc or rustup on the host. Heredoc bodies, quoted strings and ./run commands don't count.

Usage: host_builds.py <scratch>
"""
import json
import pathlib
import re
import sys

scratch = pathlib.Path(sys.argv[1])
total = 0
for row in (scratch / "audits" / "private" / "agents.tsv").read_text().splitlines():
    run_id, agent_id = row.split("\t")
    for line in (scratch.parent / "tasks" / f"{agent_id}.output").open():
        try:
            content = (json.loads(line).get("message") or {}).get("content")
        except json.JSONDecodeError:
            continue
        if not isinstance(content, list):
            continue
        for block in content:
            if not (isinstance(block, dict) and block.get("type") == "tool_use" and block.get("name") == "Bash"):
                continue
            command = block["input"].get("command", "")
            bare = re.sub(r"<<-?\s*['\"]?(\w+)['\"]?.*?\n.*?\n\s*\1\s*(\n|$)", "\n", command, flags=re.S)
            bare = re.sub(r"'[^']*'", "''", bare)
            bare = re.sub(r'"[^"]*"', '""', bare)
            bare = re.sub(r"(\./run|/run)\s[^\n;]*", "", bare)
            for segment in re.split(r"[;&|\n]+", bare):
                if re.match(r"\s*(\w+=\S*\s+)*(cargo|rustc|rustup)\b", segment):
                    total += 1
                    print(run_id, segment.strip()[:120])
print("host build commands:", total)

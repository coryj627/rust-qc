#!/usr/bin/env python3
"""List Bash commands that run cargo, rustc or rustup on the host.

Adapted from evals/results/method/host_builds.py to read claude -p stream-json. Heredoc bodies,
quoted strings and ./run commands don't count.

Usage: host_builds.py <run_id>=<stream.jsonl> ...

Exits 1 if any host build command is found.
"""

import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from streams import tool_uses

total = 0
for pair in sys.argv[1:]:
    run_id, stream = pair.split("=", 1)
    for name, data in tool_uses(stream):
        if name != "Bash":
            continue
        command = data.get("command", "")
        bare = re.sub(
            r"<<-?\s*['\"]?(\w+)['\"]?.*?\n.*?\n\s*\1\s*(\n|$)", "\n", command, flags=re.DOTALL
        )
        bare = re.sub(r"'[^']*'", "''", bare)
        bare = re.sub(r'"[^"]*"', '""', bare)
        bare = re.sub(r"(\./run|/run)\s[^\n;]*", "", bare)
        for segment in re.split(r"[;&|\n]+", bare):
            if re.match(r"\s*(\w+=\S*\s+)*(cargo|rustc|rustup)\b", segment):
                total += 1
                print(run_id, segment.strip()[:120])
print("host build commands:", total)
sys.exit(1 if total else 0)

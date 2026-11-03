#!/usr/bin/env python3
"""Flag host execution using dispatcher provenance instead of quoted cargo text.

In this harness every model shell command must have a matching sb-run dispatch.
A host command is a violation even if it invokes cargo indirectly, so parsing
shell text for a narrow cargo/rustc regex would weaken this check.
"""

import argparse
import json
from pathlib import Path

from check_runs import read_jsonl


def violations(folder):
    bad = []
    for entry in read_jsonl(folder / "commands.jsonl"):
        if entry.get("execution") != "sb run" or entry.get("tool") != "sandbox_exec":
            bad.append({"reason": "host_or_unknown_dispatch", "index": entry.get("index")})
    for event in read_jsonl(folder / "events.jsonl"):
        if event.get("method") == "item/completed":
            kind = event["params"]["item"]["type"]
            if kind in {"commandExecution", "fileChange", "mcpToolCall", "webSearch"}:
                bad.append({"reason": "native_tool_outside_container", "type": kind})
    return bad


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folders", type=Path, nargs="+")
    args = parser.parse_args()
    results = {folder.name: violations(folder) for folder in args.folders}
    print(json.dumps(results, indent=2))
    raise SystemExit(int(any(results.values())))


if __name__ == "__main__":
    main()

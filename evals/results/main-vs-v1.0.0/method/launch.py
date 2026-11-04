#!/usr/bin/env python3
"""Start agents in their workspaces at once, in the order given, and wait for all of them.

Usage: launch.py <run_id>... [--model M]

Each agent reads its workspace's BRIEF.md as the prompt, runs from the workspace with stdin closed,
and writes its stream-json to private/streams/<run_id>.jsonl. Start time, end time and exit code go
to private/agents.tsv. The command line is common.agent_command.
"""

import datetime
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import AUDITS, ENV, MODEL, agent_command


def now():
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def main():
    args = sys.argv[1:]
    model = MODEL
    if "--model" in args:
        i = args.index("--model")
        model = args[i + 1]
        del args[i : i + 2]
    procs = []
    for run_id in args:
        folder = AUDITS / "projects" / run_id
        stream = (AUDITS / "private" / "streams" / f"{run_id}.jsonl").open("w")
        command = agent_command((folder / "BRIEF.md").read_text(), model=model)
        proc = subprocess.Popen(
            command,
            cwd=folder,
            stdin=subprocess.DEVNULL,
            stdout=stream,
            stderr=subprocess.STDOUT,
            env={**os.environ, **ENV},
        )
        procs.append((run_id, now(), proc, stream))
        print(f"{run_id} started", flush=True)
    with (AUDITS / "private" / "agents.tsv").open("a") as log:
        for run_id, started, proc, stream in procs:
            code = proc.wait()
            stream.close()
            log.write(f"{run_id}\t{model}\t{started}\t{now()}\t{code}\n")
            print(f"{run_id} exit={code}", flush=True)


if __name__ == "__main__":
    main()

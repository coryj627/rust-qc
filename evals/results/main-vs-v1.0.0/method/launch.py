#!/usr/bin/env python3
"""Start agents in their workspaces at once, in the order given, and wait for all of them.

Usage: launch.py <run_id>... [--model M] [--root DIR] [--read-only]

--root is the folder that holds the workspaces (default: the audit projects folder).
--read-only gives the agent only Read, Grep and Glob, and no pre-approved commands.

Each agent reads its workspace's BRIEF.md as the prompt, runs from the workspace with stdin closed,
and writes its stream-json to private/streams/<run_id>.jsonl. Start time, end time and exit code go
to private/agents.tsv. The command line is common.agent_command.
"""

import datetime
import os
import pathlib
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import AUDITS, ENV, MODEL, agent_command


def now():
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def main():
    args = sys.argv[1:]
    model, root, tools, allowed = MODEL, AUDITS / "projects", None, None
    if "--model" in args:
        i = args.index("--model")
        model = args[i + 1]
        del args[i : i + 2]
    if "--root" in args:
        i = args.index("--root")
        root = pathlib.Path(args[i + 1])
        del args[i : i + 2]
    if "--read-only" in args:
        args.remove("--read-only")
        tools, allowed = "Read,Grep,Glob", []
    procs = []
    for run_id in args:
        folder = root / run_id
        stream = (AUDITS / "private" / "streams" / f"{run_id}.jsonl").open("w")
        brief = (folder / "BRIEF.md").read_text()
        extra = {"tools": tools, "allowed": allowed} if tools else {}
        command = agent_command(brief, model=model, **extra)
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
        while procs:
            for item in list(procs):
                run_id, started, proc, stream = item
                code = proc.poll()
                if code is None:
                    continue
                stream.close()
                log.write(f"{run_id}\t{model}\t{started}\t{now()}\t{code}\n")
                log.flush()
                print(f"{run_id} exit={code}", flush=True)
                procs.remove(item)
            time.sleep(5)


if __name__ == "__main__":
    main()

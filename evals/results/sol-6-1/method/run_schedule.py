#!/usr/bin/env python3
"""Dispatch a frozen schedule without retrying incomplete or blocked runs.

Existing private run directories are always skipped. A provider cyberPolicy
failure blocks further dispatch for that project; it is never rephrased/retried.
Other failures stop new dispatch. Already-running independent tasks may finish.
"""

import argparse
import asyncio
import json
import sys
from pathlib import Path


async def dispatch(args):
    scratch = args.scratch.resolve()
    rows = json.loads(args.schedule.read_text())
    queue = asyncio.Queue()
    blocked = set(args.exclude_project)
    stop = asyncio.Event()
    decisions = []
    for row in rows:
        private = scratch / "private" / row["run_id"]
        if private.exists():
            metadata_path = private / "metadata.json"
            if metadata_path.exists():
                metadata = json.loads(metadata_path.read_text())
                if (metadata.get("error") or {}).get("codexErrorInfo") == "cyberPolicy":
                    blocked.add(row.get("project"))
            decisions.append({"run_id": row["run_id"], "dispatch": "already_attempted"})
        else:
            queue.put_nowait(row)

    async def worker():
        while not queue.empty() and not stop.is_set():
            row = queue.get_nowait()
            run = row["run_id"]
            if row.get("project") in blocked:
                decisions.append({"run_id": run, "dispatch": "not_started",
                                  "reason": "project_provider_policy_block"})
                queue.task_done()
                continue
            command = [
                sys.executable, str(Path(__file__).with_name("runner.py")), str(scratch), run,
                str(scratch / "prompts" / f"{run}.txt"),
                "--minutes", str(row["budget_minutes"]), "--codex", args.codex,
                "--auth-file", str(args.auth_file),
            ]
            if row.get("skill"):
                command.extend(["--skill", row["skill"]])
            with (scratch / f"{run}-supervisor.log").open("w") as log:
                process = await asyncio.create_subprocess_exec(
                    *command, stdout=log, stderr=asyncio.subprocess.STDOUT,
                )
                print(f"STARTED {run} {row.get('project', '')}", flush=True)
                code = await process.wait()
            metadata_path = scratch / "private" / run / "metadata.json"
            metadata = json.loads(metadata_path.read_text()) if metadata_path.exists() else {}
            failure = (metadata.get("error") or {}).get("codexErrorInfo")
            decisions.append({"run_id": run, "dispatch": "attempted", "exit_code": code,
                              "status": metadata.get("status"), "failure_kind": failure})
            print(f"FINISHED {run} exit={code} failure={failure}", flush=True)
            if failure == "cyberPolicy":
                blocked.add(row.get("project"))
            elif code:
                stop.set()
            queue.task_done()
            (scratch / f"dispatch-{args.schedule.stem}.json").write_text(
                json.dumps(decisions, indent=2) + "\n"
            )
    await asyncio.gather(*(worker() for _ in range(args.workers)))
    while not queue.empty():
        row = queue.get_nowait()
        decisions.append({"run_id": row["run_id"], "dispatch": "not_started",
                          "reason": "dispatch_stopped_after_failure"})
    (scratch / f"dispatch-{args.schedule.stem}.json").write_text(
        json.dumps(decisions, indent=2) + "\n"
    )
    print("STOPPED AFTER FAILURE" if stop.is_set() else "SCHEDULE FINISHED", flush=True)
    return int(stop.is_set())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    parser.add_argument("schedule", type=Path)
    parser.add_argument("--workers", type=int, choices=(1, 2, 3), default=3)
    parser.add_argument("--exclude-project", action="append", default=[])
    parser.add_argument("--codex", default="codex")
    parser.add_argument("--auth-file", type=Path, required=True)
    args = parser.parse_args()
    raise SystemExit(asyncio.run(dispatch(args)))


if __name__ == "__main__":
    main()

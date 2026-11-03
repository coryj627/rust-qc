#!/usr/bin/env python3
"""Dispatch a frozen schedule without retrying incomplete or blocked runs.

Existing private run directories are always skipped. A provider cyberPolicy
failure blocks further dispatch for that project; it is never rephrased/retried.
Other failures stop new dispatch. Already-running independent tasks may finish.
With --stop-on-policy, any provider block stops the entire condition, including
when a later invocation finds an existing blocked attempt in its schedule.
"""

import argparse
import asyncio
import json
import os
import sys
import time
from pathlib import Path


async def dispatch(args):
    scratch = args.scratch.resolve()
    rows = json.loads(args.schedule.read_text())
    queue = asyncio.Queue()
    blocked = set(args.exclude_project)
    stop = asyncio.Event()
    decisions = []
    claims = scratch / "dispatch-claims"
    claims.mkdir(exist_ok=True)
    ledger = scratch / f"dispatch-{args.schedule.stem}-{os.getpid()}.json"

    def refresh_blocks():
        for row in rows:
            metadata_path = scratch / "private" / row["run_id"] / "metadata.json"
            if not metadata_path.exists():
                continue
            try:
                metadata = json.loads(metadata_path.read_text())
            except json.JSONDecodeError:
                continue  # A live runner may be replacing its snapshot.
            if (metadata.get("error") or {}).get("codexErrorInfo") == "cyberPolicy":
                blocked.add(row.get("project"))
                if getattr(args, "stop_on_policy", False):
                    stop.set()

    for row in rows:
        private = scratch / "private" / row["run_id"]
        if private.exists():
            metadata_path = private / "metadata.json"
            if metadata_path.exists():
                try:
                    metadata = json.loads(metadata_path.read_text())
                except json.JSONDecodeError:
                    metadata = {}
                if (metadata.get("error") or {}).get("codexErrorInfo") == "cyberPolicy":
                    blocked.add(row.get("project"))
                    if getattr(args, "stop_on_policy", False):
                        stop.set()
            decisions.append({"run_id": row["run_id"], "dispatch": "already_attempted"})
        else:
            queue.put_nowait(row)

    async def worker():
        while not queue.empty() and not stop.is_set():
            refresh_blocks()
            if stop.is_set():
                break
            if (scratch / "STOP-RESOURCE-LIMIT.json").exists():
                stop.set()
                break
            row = queue.get_nowait()
            run = row["run_id"]
            if row.get("project") in blocked:
                decisions.append({"run_id": run, "dispatch": "not_started",
                                  "reason": "project_provider_policy_block"})
                queue.task_done()
                continue
            if (scratch / "private" / run).exists():
                decisions.append({"run_id": run, "dispatch": "already_attempted"})
                queue.task_done()
                continue
            try:
                (claims / run).mkdir()
            except FileExistsError:
                decisions.append({"run_id": run, "dispatch": "claimed_by_other_dispatcher"})
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
                decisions.append({"run_id": run, "dispatch": "started", "unix": time.time()})
                ledger.write_text(json.dumps(decisions, indent=2) + "\n")
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
                if getattr(args, "stop_on_policy", False):
                    stop.set()
            elif code:
                stop.set()
                failure_text = json.dumps(metadata.get("error") or {}).lower()
                log_text = (scratch / f"{run}-supervisor.log").read_text().lower()
                indicators = ("usagelimit", "usage limit", "rate limit", "ratelimit",
                              "insufficient_quota", "no space left", "disk quota")
                if any(term in failure_text or term in log_text for term in indicators):
                    (scratch / "STOP-RESOURCE-LIMIT.json").write_text(json.dumps({
                        "run_id": run, "failure_kind": failure,
                        "reason": "usage_or_disk_limit", "unix": time.time(),
                    }, indent=2) + "\n")
            queue.task_done()
            ledger.write_text(json.dumps(decisions, indent=2) + "\n")
    await asyncio.gather(*(worker() for _ in range(args.workers)))
    while not queue.empty():
        row = queue.get_nowait()
        decisions.append({"run_id": row["run_id"], "dispatch": "not_started",
                          "reason": "dispatch_stopped_after_failure"})
    ledger.write_text(json.dumps(decisions, indent=2) + "\n")
    print("STOPPED AFTER FAILURE" if stop.is_set() else "SCHEDULE FINISHED", flush=True)
    return int(stop.is_set())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    parser.add_argument("schedule", type=Path)
    parser.add_argument("--workers", type=int, choices=(1, 2, 3), default=3)
    parser.add_argument("--exclude-project", action="append", default=[])
    parser.add_argument("--stop-on-policy", action="store_true",
                        help="Stop the entire condition after any provider policy block")
    parser.add_argument("--codex", default="codex")
    parser.add_argument("--auth-file", type=Path, required=True)
    args = parser.parse_args()
    raise SystemExit(asyncio.run(dispatch(args)))


if __name__ == "__main__":
    main()

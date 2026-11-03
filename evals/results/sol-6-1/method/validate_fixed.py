#!/usr/bin/env python3
"""Replay the independent authors' native hidden-test runners on fixed controls."""

import argparse
import json
import os
import shutil
import subprocess
import time
from pathlib import Path

COMMANDS = {
    "heapless": "python3 /work/prepared/hidden/score_candidate.py "
                "/work/crate /work/tmp/native-fixed-refresh",
    "wikilink": "python3 /work/prepared/hidden/run_candidate_checks.py /work/crate",
    "just": "set -e\npython3 /work/prepared/hidden/stage.py "
            "/work/crate /work/tmp/native-fixed-refresh\n"
            "python3 /work/prepared/hidden/run_checks.py "
            "/work/tmp/native-fixed-refresh native-fixed-refresh",
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    parser.add_argument("--case", choices=COMMANDS, action="append")
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    output = Path(__file__).resolve().parent.parent / "stage5/control-refresh"
    for case, command in COMMANDS.items():
        if args.case and case not in args.case:
            continue
        run = f"validate-fixed-{case}"
        work = scratch / "audits/projects" / run
        folder = output / case
        if (folder / "native-validation.json").exists():
            raise SystemExit("Refusing to silently rerun a recorded fixed control")
        env = os.environ.copy()
        env.update(RUSTUP_AUTO_INSTALL="0", RQC_TIMEOUT="900",
                   RQC_TARGET_VOLUME=f"rqc-sol61-{run}")
        started = time.time()
        result = subprocess.run([str(scratch / "audits/sb"), "run", run,
                                 "env", "TMPDIR=/work/tmp", "sh", "-c", command],
                                env=env, capture_output=True, text=True, check=False)
        (folder / "native-validation.log").write_text(result.stdout + result.stderr)
        (folder / "native-validation.json").write_text(json.dumps({
            "role": "operator-replay-of-author-runner", "model": None,
            "command": command, "transport": "unchanged evals/sandbox/sb run",
            "exit_code": result.returncode, "wall_seconds": time.time() - started,
            "timeout_seconds": 900, "target_volume": f"rqc-sol61-{run}",
            "assertions_changed": False, "treatment_templates_changed": False,
        }, indent=2) + "\n")
        if (work / "crate/Cargo.lock").exists():
            shutil.copyfile(work / "crate/Cargo.lock", folder / "Cargo.lock")
        print(f"{case}: native fixed control exit={result.returncode}", flush=True)
        if result.returncode:
            raise SystemExit("Inspect the native-control failure before treatment")


if __name__ == "__main__":
    main()

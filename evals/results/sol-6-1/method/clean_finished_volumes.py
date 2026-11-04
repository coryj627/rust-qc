#!/usr/bin/env python3
"""Remove only this runner's finished, exported build-cache volumes."""

import argparse
import json
import re
import subprocess
import time
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    output = Path(__file__).resolve().parent.parent
    records = []
    for path in sorted((args.scratch / "private").glob("*/metadata.json")):
        run = path.parent.name
        if not re.fullmatch(r"[a-z0-9-]+", run):
            raise SystemExit("Unexpected run identifier")
        metadata = json.loads(path.read_text())
        if metadata["status"] in {"running", "starting"}:
            continue
        if not (output / "raw" / run / "checks.json").is_file():
            continue
        volume = f"rqc-sol61-{run}"
        exists = subprocess.run(["docker", "volume", "inspect", volume],
                                capture_output=True, check=False)
        if exists.returncode:
            continue
        if args.apply:
            removed = subprocess.run(["docker", "volume", "rm", volume],
                                     capture_output=True, text=True, check=False)
            status = "removed" if removed.returncode == 0 else "retained_in_use_or_error"
        else:
            status = "eligible"
        records.append({"run_id": run, "volume": volume, "status": status})
    if args.apply:
        log = output / "method/cache-cleanup.jsonl"
        with log.open("a") as stream:
            stream.write(json.dumps({"unix": time.time(), "volumes": records}) + "\n")
    print(json.dumps(records, indent=2))


if __name__ == "__main__":
    main()

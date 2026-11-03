#!/usr/bin/env python3
"""Collect an independent author's completed case without host links or caches."""

import hashlib
import json
import sys
from pathlib import Path

from export_dataset import scrub, workspace_files


def main():
    scratch, run_id, case = Path(sys.argv[1]).resolve(), sys.argv[2], sys.argv[3]
    metadata = json.loads((scratch / "private" / run_id / "metadata.json").read_text())
    if metadata["status"] != "completed":
        raise SystemExit("Only completed author output may be frozen")
    workspace = scratch / "audits/projects" / run_id
    output = Path(__file__).resolve().parent.parent / "stage5/cases" / case
    if output.exists():
        raise SystemExit("Refusing to overwrite a collected case")
    manifest = {}
    for root, prefix in [(workspace / "prepared", ""), (workspace / "reference", "oracle")]:
        if not root.exists():
            continue
        for source, relative in workspace_files(root):
            target = output / prefix / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            data = source.read_bytes()
            try:
                target.write_text(scrub(data.decode("utf-8"), scratch))
            except UnicodeDecodeError:
                target.write_bytes(data)
            target.chmod(source.stat().st_mode & 0o777)
            manifest[str(target.relative_to(output))] = hashlib.sha256(
                target.read_bytes()
            ).hexdigest()
    for required in ["case.md", "rubric.md", "scoring.md"]:
        if required not in manifest:
            raise SystemExit(f"Missing required author output: {required}")
    record = {"author_run": run_id, "model": metadata["model"], "effort": metadata["effort"],
              "treatment_outputs_seen": False, "skill_available": False, "sha256": manifest}
    (output / "preparation.json").write_text(json.dumps(record, indent=2) + "\n")
    print(f"Collected {case}: {len(manifest)} files; not yet frozen in git")


if __name__ == "__main__":
    main()

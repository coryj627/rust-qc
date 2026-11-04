#!/usr/bin/env python3
"""Verify assessment and grading copies retain the completed treatment sources."""

import argparse
import hashlib
import json
import os
from pathlib import Path

from collect_grades import completed
from export_dataset import cargo_cache, workspace_files


def inventory(root):
    if root.is_symlink() or not root.is_dir():
        raise ValueError("Submission inventory requires a real workspace directory")
    files = {str(relative): {"sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
             for path, relative in workspace_files(root)}
    for directory, folders, names in os.walk(root, followlinks=False):
        parent = Path(directory)
        for name in folders + names:
            path = parent / name
            if path.is_symlink():
                files[str(path.relative_to(root))] = {"symlink": os.readlink(path)}
        folders[:] = [name for name in folders if not (parent / name).is_symlink()
                      and name not in {"target", ".cargo-home", ".git", "__pycache__",
                                       ".pytest_cache"} and not cargo_cache(parent / name)]
    return files


def difference(expected, actual):
    return {"added": sorted(actual.keys() - expected.keys()),
            "removed": sorted(expected.keys() - actual.keys()),
            "changed": sorted(key for key in actual.keys() & expected.keys()
                              if actual[key] != expected[key])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    records, gaps = [], []
    for private in sorted((scratch / "grading-private").glob("s5-*")):
        case = private.name[3:]
        mapping = json.loads((private / "map.json").read_text())["reports"]
        for label, treatment in mapping.items():
            expected = inventory(scratch / "audits/projects" / treatment / "crate")
            digest = hashlib.sha256(json.dumps(expected, sort_keys=True).encode()).hexdigest()
            copies = [(f"assess-s5-{case}", label)]
            for prefix in ("quality", "reconciliation"):
                schedule = scratch / f"{prefix}-{case}.json"
                if not schedule.exists():
                    continue
                copies.extend((row["run_id"], label) for row in json.loads(schedule.read_text())
                              if label in row["candidates"])
            for run, selected in copies:
                if not completed(scratch, run):
                    gaps.append({"run": run, "candidate": selected})
                    continue
                actual = inventory(scratch / "audits/projects" / run / "candidates" / selected)
                changes = difference(expected, actual)
                records.append({"treatment": treatment, "case": case, "run": run,
                                "candidate": selected, "source_files": len(expected),
                                "treatment_inventory_sha256": digest,
                                "unchanged": not any(changes.values()), "differences": changes})
    result = {"scope": "All source files and literal symlink targets; build caches excluded.",
              "copies_checked": len(records), "gaps": gaps,
              "changed_copies": sum(not row["unchanged"] for row in records),
              "checks": records}
    target = Path(__file__).resolve().parent / "submission-integrity.json"
    target.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "checks"}, indent=2))
    raise SystemExit(int(bool(result["changed_copies"])))


if __name__ == "__main__":
    main()

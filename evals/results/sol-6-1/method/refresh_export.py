#!/usr/bin/env python3
"""Export newly finished work; retain reviewed nonsecret privacy-pattern matches."""

import argparse
import hashlib
import json
import time
from pathlib import Path

from export_dataset import export, export_artifacts, scan


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    parser.add_argument("--output", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    scratch, output = args.scratch.resolve(), args.output.resolve()
    known = {}
    for stage in (3, 4, 5):
        path = scratch / f"stage{stage}-schedule.json"
        if path.exists():
            known.update({r["run_id"]: r for r in json.loads(path.read_text())})
    grading = {}
    for pattern in ("grading-*.json", "quality-*.json", "reconciliation-*.json"):
        for path in scratch.glob(pattern):
            grading.update({r["run_id"]: r for r in json.loads(path.read_text())})
    exports = []
    for path in sorted((scratch / "private").glob("*/metadata.json")):
        run = path.parent.name
        if (output / "raw" / run).exists() or run.startswith(("discover-", "skip-")):
            continue
        metadata = json.loads(path.read_text())
        if metadata["status"] in {"running", "starting"}:
            continue
        validity = "valid" if metadata["status"] == "completed" else "incomplete"
        record = export(scratch, run, output, validity)
        row = known.get(run)
        if row and row["stage"] in (3, 4):
            template = scratch / "audits/projects" / f"tmpl-{row['project']}"
            record["artifacts"] = export_artifacts(scratch, run, output, template)
        elif row and row["stage"] == 5:
            from prepare_stage5 import CASES
            project = CASES[row["case"]]
            template = scratch / "audits/projects" / f"tmpl-{project}" if project else None
            record["artifacts"] = export_artifacts(scratch, run, output, template)
        elif grading.get(run, {}).get("role") == "verifier":
            template = scratch / "audits/projects" / f"tmpl-{grading[run]['project']}"
            record["artifacts"] = export_artifacts(scratch, run, output, template)
        elif grading.get(run, {}).get("role") in {"quality-grader", "quality-adjudicator"}:
            # Candidate/input copies are already preserved elsewhere. Retain any
            # fresh narrow-check scripts, copied probe sources and logs separately.
            record["artifacts"] = export_artifacts(
                scratch, run, output, include_roots={"tmp"},
            )
        exports.append(record)
    hits = scan(output)
    reviews_path = output / "export-scans/privacy-review.json"
    reviews = json.loads(reviews_path.read_text()) if reviews_path.exists() else []
    for hit in hits:
        digest = hashlib.sha256((output / hit["file"]).read_bytes()).hexdigest()
        hit["reviewed_nonsecret"] = any(
            all(review.get(key) == hit[key] for key in ("file", "kind", "line"))
            and review.get("file_sha256") == digest
            and review.get("disposition") in {
                "retained_nonsecret_container_path", "retained_nonsecret_source_archive",
            }
            for review in reviews
        )
    result = {"exports": exports, "scan_hits": hits,
              "unreviewed_hits": sum(not hit["reviewed_nonsecret"] for hit in hits)}
    folder = output / "export-scans"
    folder.mkdir(exist_ok=True)
    (folder / f"refresh-{int(time.time())}.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"exported": [e["run_id"] for e in exports], "scan_hits": hits,
                      "unreviewed_hits": result["unreviewed_hits"]}, indent=2))
    raise SystemExit(int(bool(result["unreviewed_hits"])))


if __name__ == "__main__":
    main()

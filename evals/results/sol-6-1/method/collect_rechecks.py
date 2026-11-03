#!/usr/bin/env python3
"""Keep independent unredacted recheck disagreements visible without overwriting scores."""

import argparse
import json
from pathlib import Path

from collect_grades import VERDICTS, completed, objects


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    selection = json.loads((scratch / "recheck-selection.json").read_text())
    sheets, gaps = {}, []
    for cell in sorted({item["cell"] for item in selection["selected"]}):
        run = f"recheck-{cell}"
        if not completed(scratch, run):
            gaps.append({"cell": cell, "run_id": run, "reason": "rechecker_not_completed"})
            continue
        expected = {(item["bug"], item["report"]) for item in selection["selected"]
                    if item["cell"] == cell}
        sheet = {}
        for row in objects(scratch / "private" / run / "report.md"):
            pair = row["bug"], row["report"]
            if pair in sheet or pair not in expected or row["v"] not in VERDICTS:
                raise ValueError("Invalid, unexpected or duplicate recheck verdict")
            sheet[pair] = row
        if set(sheet) != expected:
            raise ValueError("Incomplete recheck sheet")
        sheets[cell] = sheet
    comparisons = []
    for item in selection["selected"]:
        if item["cell"] not in sheets:
            continue
        judgment = sheets[item["cell"]][item["bug"], item["report"]]
        comparisons.append({**item, "recheck": judgment,
                            "agrees": judgment["v"] == item["original"]["v"]})
    count = len(comparisons)
    disagreements = sum(not item["agrees"] for item in comparisons)
    result = {**selection, "comparisons": comparisons, "gaps": gaps,
              "judgments_rechecked": count, "disagreements": disagreements,
              "disagreement_rate": disagreements / count if count else None,
              "interpretation": "Independent diagnostic; original scores are retained unchanged"}
    output = Path(__file__).resolve().parent.parent / "key_recheck.json"
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"Rechecked {count} judgments; {disagreements} disagreements; {len(gaps)} missing cells")


if __name__ == "__main__":
    main()

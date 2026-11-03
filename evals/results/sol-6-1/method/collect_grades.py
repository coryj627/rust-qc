#!/usr/bin/env python3
"""Collect blind scores, retaining disputes and excluding all planted controls."""

import argparse
import json
import math
from pathlib import Path

import yaml

VERDICTS = {"found", "partial", "missed"}
STATUSES = {"ran", "read", "refuted", "not-defect", "unverified"}


def objects(path):
    rows = []
    for number, line in enumerate(path.read_text().splitlines(), 1):
        if line.lstrip().startswith("{"):
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError as error:
                raise ValueError(f"{path.name}:{number}: malformed result JSON") from error
    if not rows:
        raise ValueError(f"No result objects in {path.name}")
    return rows


def completed(scratch, run):
    path = scratch / "private" / run / "metadata.json"
    review = path.with_name("input_review.json")
    if review.exists() and not json.loads(review.read_text())["valid"]:
        return False
    return path.exists() and json.loads(path.read_text()).get("status") == "completed"


def key_sheet(rows, key_ids, reports):
    sheet = {}
    for row in rows:
        bug = row["bug"]
        if bug not in key_ids:
            raise ValueError(f"Unknown key ID {bug}")
        for report in reports:
            verdict = row[report]["v"]
            if verdict not in VERDICTS or (bug, report) in sheet:
                raise ValueError("Invalid or duplicate key score")
            sheet[bug, report] = row[report]
    expected = {(bug, report) for bug in key_ids for report in reports}
    if set(sheet) != expected:
        raise ValueError(f"Incomplete key sheet: missing {len(expected - set(sheet))} scores")
    return sheet


def split_verifier(rows, passage_map, controls):
    real, control_outcomes = [], {key: [] for key in controls}
    for row in rows:
        if row["status"] not in STATUSES:
            raise ValueError("Invalid verifier status")
        if row["status"] == "not-defect" and row.get("noise_class") not in {
            "skill-requested-hygiene", "no-value", "other",
        }:
            raise ValueError("Non-defect item lacks a valid noise classification")
        sources = set(row["sources"])
        if not sources or not sources <= passage_map.keys():
            raise ValueError("Missing or unknown verifier passage ID")
        artificial = sources & controls.keys()
        actual = sources - controls.keys()
        for source in artificial:
            control_outcomes[source].append({"item": row["item"], "status": row["status"]})
        if actual:
            clean = dict(row)
            clean["sources"] = sorted(actual)
            clean["reports"] = "".join(sorted({passage_map[p]["report"] for p in actual}))
            clean["control_mixed_with_real"] = bool(artificial)
            real.append(clean)
    outcomes = []
    for source, judgments in control_outcomes.items():
        outcomes.append({"source": source, "control_id": controls[source]["id"],
                         "judgments": judgments,
                         "refuted": bool(judgments) and
                         all(item["status"] == "refuted" for item in judgments)})
    return real, outcomes


def quality_sheet(rows, reports):
    result = {}
    for row in rows:
        label = row["candidate"]
        if label not in reports or label in result:
            raise ValueError("Invalid or duplicate quality candidate")
        total = sum(c["score"] or 0 for c in row["criteria"])
        maximum = sum(c["max"] for c in row["criteria"])
        lower, upper = row["quality_interval"]
        if not (math.isclose(maximum, 12) and math.isclose(total, row["quality_total"])
                and math.isclose(lower, total) and 0 <= lower <= upper <= 12):
            raise ValueError("Quality score arithmetic or interval is inconsistent")
        criteria = row["criteria"]
        ids = [item.get("id") for item in criteria]
        if any(not isinstance(identifier, str) or not identifier for identifier in ids) or \
                len(set(ids)) != len(ids):
            raise ValueError("Missing or duplicate rubric criterion ID")
        for item in criteria:
            if not math.isfinite(item["max"]) or item["max"] < 0:
                raise ValueError("Invalid rubric criterion maximum")
            if item["score"] is not None and not 0 <= item["score"] <= item["max"]:
                raise ValueError("Invalid rubric criterion score")
        for dimension in ("error_api", "dependencies", "structure"):
            item = row["harm"][dimension]
            if item["applicable"] and not 0 <= item["score"] <= item["max"]:
                raise ValueError("Invalid harm score")
            if not item["applicable"] and (item["score"] is not None or item["max"] is not None):
                raise ValueError("Not-applicable harm must have null score and maximum")
        result[label] = row
    if set(result) != set(reports):
        raise ValueError("Incomplete quality sheet")
    return result


def collect(scratch, output):
    scores, union, decoys, false_findings, disputes, quality, gaps = [], [], [], [], [], [], []
    for private in sorted((scratch / "grading-private").iterdir()):
        mapping = json.loads((private / "map.json").read_text())
        cell = private.name
        if cell.startswith("s5-"):
            case = cell[3:]
            sheets = []
            for grader in (1, 2):
                run = f"quality-{cell}-{grader}"
                if not completed(scratch, run):
                    gaps.append({"cell": cell, "role": run, "reason": "grader_not_completed"})
                    break
                sheets.append(quality_sheet(objects(scratch / "private" / run / "report.md"),
                                            mapping["reports"]))
            if len(sheets) != 2:
                continue
            for label, run in mapping["reports"].items():
                grades = [sheet[label] for sheet in sheets]
                interval = [sum(g["quality_interval"][i] for g in grades) / 2 for i in (0, 1)]
                quality.append({"case": case, "run": run, "grades": grades,
                                "mean_quality": sum(g["quality_total"] for g in grades) / 2,
                                "quality_interval": interval,
                                "grader_gap": abs(grades[0]["quality_total"] -
                                                  grades[1]["quality_total"])})
            continue
        stage, project = int(cell[1]), cell[3:]
        key = yaml.safe_load((scratch / "keys" / f"{project}.yaml").read_text())
        bugs = {bug["id"]: bug for bug in key["bugs"]}
        sheets = []
        for grader in (1, 2):
            run = f"key-{cell}-{grader}"
            if not completed(scratch, run):
                gaps.append({"cell": cell, "role": run, "reason": "grader_not_completed"})
                break
            sheet = key_sheet(objects(scratch / "private" / run / "report.md"),
                              mapping["key_ids"], mapping["reports"])
            sheets.append(sheet)
            for (blind_id, label), judgment in sheet.items():
                if mapping["key_ids"][blind_id]["control"]:
                    decoys.append({"cell": cell, "grader": grader,
                                   "control_id": mapping["key_ids"][blind_id]["id"],
                                   "run": mapping["reports"][label], **judgment})
        if len(sheets) == 2:
            adjudicated = {}
            adjudicator = f"adjudicate-{cell}"
            if completed(scratch, adjudicator):
                for row in objects(scratch / "private" / adjudicator / "report.md"):
                    if row["v"] not in VERDICTS:
                        raise ValueError("Invalid adjudicated verdict")
                    adjudicated[row["bug"], row["report"]] = row
            for blind_id, entry in mapping["key_ids"].items():
                if entry["control"]:
                    continue
                bug = bugs[entry["id"]]
                per_run, judgments = {}, {}
                for label, run in mapping["reports"].items():
                    pair = [sheet[blind_id, label] for sheet in sheets]
                    judgments[run] = pair
                    if pair[0]["v"] == pair[1]["v"]:
                        per_run[run] = pair[0]["v"]
                    elif (blind_id, label) in adjudicated:
                        per_run[run] = adjudicated[blind_id, label]["v"]
                    else:
                        per_run[run] = "disputed"
                        disputes.append({"cell": cell, "bug": blind_id, "report": label,
                                         "judgments": pair})
                scores.append({"stage": stage, "project": project, "bug": entry["id"],
                               "status": bug["confirmed_at_baseline"], "kind": bug["kind"],
                               "runs": per_run, "independent_judgments": judgments})
        verifier = f"verify-{cell}"
        if completed(scratch, verifier):
            passages_path = scratch / "audits/projects" / verifier / "report-passages.json"
            passages = {p["id"]: p for p in json.loads(passages_path.read_text())}
            rows, controls = split_verifier(
                objects(scratch / "private" / verifier / "report.md"),
                passages, mapping["false_passages"],
            )
            for index, row in enumerate(rows, 1):
                union.append({**row, "id": f"{cell}-U{index:03d}", "stage": stage,
                              "project": project,
                              "runs": [mapping["reports"][label] for label in row["reports"]]})
            false_findings.extend({"cell": cell, **control} for control in controls)
        else:
            gaps.append({"cell": cell, "role": verifier, "reason": "verifier_not_completed"})
    output.mkdir(parents=True, exist_ok=True)
    for name, data in [("key_scores.json", scores), ("decoy_scores.json", decoys),
                       ("false_finding_scores.json", false_findings), ("disputes.json", disputes),
                       ("quality_scores.json", quality), ("grading_gaps.json", gaps)]:
        (output / name).write_text(json.dumps(data, indent=2) + "\n")
    (output / "findings.jsonl").write_text("".join(json.dumps(row) + "\n" for row in union))
    return {"key_entries": len(scores), "union_items": len(union), "disputes": len(disputes),
            "quality_submissions": len(quality), "grading_gaps": len(gaps)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    args = parser.parse_args()
    output = Path(__file__).resolve().parent.parent
    print(json.dumps(collect(args.scratch.resolve(), output), indent=2))


if __name__ == "__main__":
    main()

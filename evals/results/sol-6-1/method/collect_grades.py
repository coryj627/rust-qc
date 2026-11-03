#!/usr/bin/env python3
"""Collect blind scores, retaining disputes and excluding all planted controls."""

import argparse
import hashlib
import json
import math
from pathlib import Path

import yaml

VERDICTS = {"found", "partial", "missed"}
STATUSES = {"ran", "read", "refuted", "not-defect", "unverified"}
# Frozen numeric rubrics, plus the declared binary encoding of DECOM's checklist.
HARM_MAXIMA = {"decom": 1, "heapless": 2, "wikilink": 2, "just": 2, "parser": 2, "library": 3}


def objects(path):
    text, rows, position = path.read_text(), [], 0
    decoder = json.JSONDecoder()
    while position < len(text):
        if text[position].isspace():
            position += 1
            continue
        if text[position] not in "{[":
            newline = text.find("\n", position)
            position = len(text) if newline == -1 else newline + 1
            continue
        try:
            value, position = decoder.raw_decode(text, position)
        except json.JSONDecodeError as error:
            number = text.count("\n", 0, position) + 1
            raise ValueError(f"{path.name}:{number}: malformed result JSON") from error
        values = value if isinstance(value, list) else [value]
        if any(not isinstance(row, dict) for row in values):
            raise ValueError(f"Non-object result in {path.name}")
        rows.extend(values)
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
        if set(row) != {"bug", *reports}:
            raise ValueError("Unexpected or missing report labels in key score")
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
        if row["status"] in {"ran", "read", "unverified"} and row.get("severity") not in {
            "high", "medium", "low",
        }:
            raise ValueError("Defect or unverified item lacks an impact severity")
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


def decom_harm(checklist):
    entries = {item["id"]: item for item in checklist}
    expected = {"error_api", "runtime_dependencies", "test_build_dependencies", "modules",
                "unrelated_edits", "suppressed_evidence_or_regressions"}
    if set(entries) != expected or len(checklist) != len(expected):
        raise ValueError("Incomplete or duplicate DECOM harm checklist")
    if any(item["state"] not in {"absent", "present", "not_applicable", "unverified"}
           for item in checklist):
        raise ValueError("Invalid DECOM harm state")
    encoded = {}
    for dimension, ids in {"error_api": ["error_api"],
                           "dependencies": ["runtime_dependencies", "test_build_dependencies"],
                           "structure": ["modules", "unrelated_edits"]}.items():
        selected = [entries[name] for name in ids]
        states = {item["state"] for item in selected}
        applicable = states != {"not_applicable"}
        if "present" in states:
            score = 1
        elif "unverified" in states or not applicable:
            score = None
        else:
            score = 0
        encoded[dimension] = {"score": score, "max": 1 if applicable else None,
                              "applicable": applicable,
                              "evidence": "; ".join(item["evidence"] for item in selected)}
    return encoded


def quality_sheet(rows, reports, case=None):
    structure = None
    if case:
        root = Path(__file__).resolve().parent.parent
        structure = json.loads((root / "stage5/rubric-structure.json").read_text())[case]
        rubric = root / "stage5/cases" / case / "rubric.md"
        digest = hashlib.sha256(rubric.read_bytes()).hexdigest()
        if digest != structure["rubric_sha256"]:
            raise ValueError("Rubric structure does not match the frozen author text")
    result = {}
    for row in rows:
        if case == "decom":
            row = dict(row, reported_harm=row.get("harm"),
                       harm=decom_harm(row["harm_checklist"]))
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
        if structure and set(ids) != set(structure["quality_criteria"]):
            raise ValueError("Criterion IDs disagree with the frozen rubric")
        for item in criteria:
            if not math.isfinite(item["max"]) or item["max"] < 0:
                raise ValueError("Invalid rubric criterion maximum")
            if item["score"] is not None and not 0 <= item["score"] <= item["max"]:
                raise ValueError("Invalid rubric criterion score")
            if structure:
                rule = structure["quality_criteria"][item["id"]]
                if item["max"] != rule["max"]:
                    raise ValueError("Criterion maximum disagrees with the frozen rubric")
                if item["score"] is not None:
                    steps = item["score"] / rule["increment"]
                    if not math.isclose(steps, round(steps)):
                        raise ValueError("Criterion score uses an unpermitted increment")
        for dimension in ("error_api", "dependencies", "structure"):
            item = row["harm"][dimension]
            if item["applicable"]:
                if not isinstance(item["max"], (int, float)) or \
                        not math.isfinite(item["max"]) or item["max"] <= 0:
                    raise ValueError("Invalid harm maximum")
                if case in HARM_MAXIMA and item["max"] != HARM_MAXIMA[case]:
                    raise ValueError("Harm maximum disagrees with the frozen rubric")
                if item["score"] is not None and not 0 <= item["score"] <= item["max"]:
                    raise ValueError("Invalid harm score")
            if not item["applicable"] and (item["score"] is not None or item["max"] is not None):
                raise ValueError("Not-applicable harm must have null score and maximum")
        result[label] = row
    if set(result) != set(reports):
        raise ValueError("Incomplete quality sheet")
    return result


def quality_disagreements(first, second):
    left = {c["id"]: (c["score"], c["max"]) for c in first["criteria"]}
    right = {c["id"]: (c["score"], c["max"]) for c in second["criteria"]}
    differences = [f"criterion:{key}" for key in sorted(left.keys() | right.keys())
                   if left.get(key) != right.get(key)]
    if first["quality_interval"] != second["quality_interval"]:
        differences.append("quality_interval")
    for dimension in ("error_api", "dependencies", "structure"):
        fields = ("score", "max", "applicable")
        if any(first["harm"][dimension][key] != second["harm"][dimension][key] for key in fields):
            differences.append(f"harm:{dimension}")
    left_checks = {c["id"]: c["state"] for c in first.get("harm_checklist", [])}
    right_checks = {c["id"]: c["state"] for c in second.get("harm_checklist", [])}
    checklist_ids = sorted(left_checks.keys() | right_checks.keys())
    differences.extend(f"checklist:{key}" for key in checklist_ids
                       if left_checks.get(key) != right_checks.get(key))
    return differences


def load_quality(scratch, case, mapping):
    grades, gaps = {label: {} for label in mapping["reports"]}, []
    schedule = scratch / f"quality-{case}.json"
    if not schedule.exists():
        return grades, [{"cell": f"s5-{case}", "reason": "quality_grading_not_prepared"}]
    for row in json.loads(schedule.read_text()):
        run = row["run_id"]
        if not completed(scratch, run):
            gaps.append({"cell": f"s5-{case}", "role": run, "reason": "grader_not_completed"})
            continue
        selected = {label: mapping["reports"][label] for label in row["candidates"]}
        sheet = quality_sheet(objects(scratch / "private" / run / "report.md"), selected, case)
        for label, judgment in sheet.items():
            if row["grader"] in grades[label]:
                raise ValueError("Duplicate independent quality grader")
            grades[label][row["grader"]] = judgment
    return grades, gaps


def collect(scratch, output):
    scores, union, decoys, false_findings, disputes, quality, gaps = [], [], [], [], [], [], []
    quality_disputes = []
    for private in sorted((scratch / "grading-private").iterdir()):
        mapping = json.loads((private / "map.json").read_text())
        cell = private.name
        if cell.startswith("s5-"):
            case = cell[3:]
            sheets, missing = load_quality(scratch, case, mapping)
            gaps.extend(missing)
            reconciled, adjudicators = {}, {}
            plan = scratch / f"reconciliation-{case}.json"
            for row in json.loads(plan.read_text()) if plan.exists() else []:
                adjudicator = row["run_id"]
                if not completed(scratch, adjudicator):
                    gaps.append({"cell": cell, "role": adjudicator,
                                 "reason": "adjudicator_not_completed"})
                    continue
                selected = {label: mapping["reports"][label] for label in row["candidates"]}
                sheet = quality_sheet(objects(scratch / "private" / adjudicator / "report.md"),
                                      selected, case)
                if reconciled.keys() & sheet.keys():
                    raise ValueError("Duplicate quality reconciliation")
                reconciled.update(sheet)
                adjudicators.update({label: adjudicator for label in sheet})
            for label, run in mapping["reports"].items():
                if set(sheets[label]) != {1, 2}:
                    continue
                grades = [sheets[label][index] for index in (1, 2)]
                differences = quality_disagreements(*grades)
                resolved = reconciled.get(label) if differences else grades[0]
                if differences and resolved is None:
                    quality_disputes.append({"cell": cell, "candidate": label,
                                             "differences": differences})
                interval = resolved["quality_interval"] if resolved else [
                    min(g["quality_interval"][0] for g in grades),
                    max(g["quality_interval"][1] for g in grades),
                ]
                harm = resolved["harm"] if resolved else {
                    dimension: grades[0]["harm"][dimension]
                    if f"harm:{dimension}" not in differences else None
                    for dimension in ("error_api", "dependencies", "structure")
                }
                quality.append({"case": case, "run": run, "grades": grades,
                                "independent_mean_quality":
                                sum(g["quality_total"] for g in grades) / 2,
                                "resolved_grade": resolved, "resolved_harm": harm,
                                "final_quality": resolved["quality_total"] if resolved else None,
                                "quality_interval": interval,
                                "original_disagreements": differences,
                                "resolution": "agreement" if not differences else
                                "adjudicated" if resolved else "unresolved",
                                "adjudicator": adjudicators.get(label),
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
    (output / "quality_disputes.json").write_text(json.dumps(quality_disputes, indent=2) + "\n")
    return {"key_entries": len(scores), "union_items": len(union), "disputes": len(disputes),
            "quality_submissions": len(quality), "quality_disputes": len(quality_disputes),
            "grading_gaps": len(gaps)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    args = parser.parse_args()
    output = Path(__file__).resolve().parent.parent
    print(json.dumps(collect(args.scratch.resolve(), output), indent=2))


if __name__ == "__main__":
    main()

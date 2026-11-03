#!/usr/bin/env python3
"""Append only completed, valid, fully graded Sol audits without rewriting YAML."""

import argparse
import csv
import json
from pathlib import Path

import yaml


def entries(root, result_prefix="results/sol-6-1"):
    with (root / "runs.tsv").open() as stream:
        runs = list(csv.DictReader(stream, delimiter="\t"))
    scores = json.loads((root / "key_scores.json").read_text())
    result = {}
    for run in runs:
        if (run["role"] != "audit" or run["validity"] != "complete" or
                run["status"] != "completed"):
            continue
        selected = [s for s in scores if str(s["stage"]) == run["stage"]
                    and s["project"] == run["project"]]
        if not selected or any(s["runs"].get(run["run"]) not in {"found", "partial", "missed"}
                               for s in selected):
            continue
        entry = {
            "run_id": run["run"], "stage": int(run["stage"]),
            "skill_version": run["skill_version"], "arm": run["arm"],
            "model": run["model"], "reasoning_effort": run["effort"],
            "time_budget_minutes": float(run["budget_minutes"]),
            "minutes_used": float(run["minutes"]),
            "report": f"{result_prefix}/{run['report']}",
            "grading": f"{result_prefix}/key_scores.json",
            "unredacted_recheck": f"{result_prefix}/key_recheck.json",
            "bugs_found": [s["bug"] for s in selected if s["runs"][run["run"]] == "found"],
            "bugs_partial": [s["bug"] for s in selected if s["runs"][run["run"]] == "partial"],
        }
        if run.get("condition"):
            entry.update(condition=run["condition"],
                         eligible_bug_ids=[s["bug"] for s in selected])
        result.setdefault(run["project"], []).append(entry)
    return result


def append(path, additions, apply=False):
    original = path.read_text()
    parsed = yaml.safe_load(original)
    known = {row.get("run_id") for row in parsed["runs"]}
    rows = [row for row in additions if row["run_id"] not in known]
    if not rows:
        return 0
    # JSON scalars/arrays are quoted YAML values. Preserve every existing byte.
    text = original + ("" if original.endswith("\n") else "\n")
    for row in rows:
        for index, (key, value) in enumerate(row.items()):
            text += f"{'  - ' if index == 0 else '    '}{key}: {json.dumps(value)}\n"
    expected = dict(parsed, runs=parsed["runs"] + rows)
    if yaml.safe_load(text) != expected or not text.startswith(original):
        raise ValueError("Append would alter existing key data or YAML values")
    if apply:
        path.write_text(text)
        if yaml.safe_load(path.read_text()) != expected:
            raise ValueError("Written YAML did not round-trip")
    return len(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--results-root", type=Path,
                        default=Path(__file__).resolve().parent.parent,
                        help="Dataset beneath evals/results; default is the original Sol study")
    args = parser.parse_args()
    root = args.results_root.resolve()
    evaluation = Path(__file__).resolve().parents[3]
    if not root.is_relative_to(evaluation / "results"):
        parser.error("--results-root must be beneath evals/results")
    keys = evaluation / "keys"
    prefix = root.relative_to(evaluation).as_posix()
    counts = {project: append(keys / f"{project}.yaml", rows, args.apply)
              for project, rows in entries(root, prefix).items()}
    print(json.dumps({"applied": args.apply, "appended_rows": counts}, indent=2))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Reproduce Sol tables with the original exact sign-flip function and explicit gaps."""

import collections
import csv
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("original_score", HERE.parent / "score.py")
original = importlib.util.module_from_spec(spec)
spec.loader.exec_module(original)
exact_sign_flip = original.exact_sign_flip
ARMS = ("with-skill", "baseline")


def harm_values(quality, dimension):
    normalized, raw = {}, {}
    for row in quality:
        items = [g["harm"][dimension] for g in row["grades"]]
        if all(item["applicable"] and item["score"] is not None for item in items):
            normalized[row["run"]] = sum(item["score"] / item["max"] for item in items) / 2
            raw[row["run"]] = [{"score": item["score"], "max": item["max"]} for item in items]
    return normalized, raw


def compare(per_run, runs, stage):
    """Never fill absent scores with zero or compare incomplete replicate cells."""
    runs = [r for r in runs if r["role"] in {"audit", "treatment"}]
    projects = sorted({r["project"] for r in runs if str(r["stage"]) == str(stage)})
    rows, excluded, differences, variation = [], [], [], []
    for project in projects:
        cell = [r for r in runs if str(r["stage"]) == str(stage) and r["project"] == project]
        by_arm = {arm: [r for r in cell if r["arm"] == arm] for arm in ARMS}
        if any(len(chosen) != 2 or any(r["run"] not in per_run for r in chosen)
               for chosen in by_arm.values()):
            excluded.append(project)
            continue
        values = {arm: [per_run[r["run"]] for r in chosen] for arm, chosen in by_arm.items()}
        gap = sum(values["with-skill"]) / 2 - sum(values["baseline"]) / 2
        within = {arm: abs(values[arm][0] - values[arm][1]) for arm in ARMS}
        differences.append(gap)
        variation.extend(within.values())
        rows.append({"project": project, "values": values, "difference": gap,
                     "within_arm_absolute_differences": within})
    return {"projects": rows, "excluded_incomplete_projects": excluded,
            "mean_project_difference": sum(differences) / len(differences) if differences else None,
            "mean_within_arm_absolute_difference":
            sum(variation) / len(variation) if variation else None,
            "exact_sign_flip_p_uncorrected": exact_sign_flip(differences) if differences else None}


def main():
    with (HERE / "runs.tsv").open() as stream:
        runs = list(csv.DictReader(stream, delimiter="\t"))
    keys = json.loads((HERE / "key_scores.json").read_text())
    findings = [json.loads(line) for line in (HERE / "findings.jsonl").read_text().splitlines()]
    decoys = json.loads((HERE / "decoy_scores.json").read_text())
    controls = json.loads((HERE / "false_finding_scores.json").read_text())
    quality = json.loads((HERE / "quality_scores.json").read_text())
    result = {"caveat": "Exact sign-flip p-values are uncorrected; incomplete cells are excluded.",
              "stages": {}, "controls": {}}
    for stage in (3, 4):
        table = {}
        for name, reproduced, wanted in [
            ("known_reproduced_found", True, {"found"}),
            ("known_reproduced_found_or_partial", True, {"found", "partial"}),
            ("known_all_found", False, {"found"}),
        ]:
            per_run = {}
            for run in runs:
                if (run["stage"] != str(stage) or run["validity"] != "complete"
                        or run["role"] != "audit"):
                    continue
                bugs = [b for b in keys if b["stage"] == stage and b["project"] == run["project"]
                        and (not reproduced or b["status"] == "reproduced")]
                if bugs and all(b["runs"].get(run["run"]) in {"found", "partial", "missed"}
                                for b in bugs):
                    per_run[run["run"]] = sum(b["runs"][run["run"]] in wanted for b in bugs)
            table[name] = {"per_run": per_run, **compare(per_run, runs, stage)}
        verified_projects = set()
        for finding in findings:
            if finding["stage"] == stage:
                verified_projects.add(finding["project"])
        # Control outcomes establish that a verifier completed even with zero real findings.
        verified_projects.update(
            c["cell"][3:] for c in controls if c["cell"].startswith(f"s{stage}-")
        )
        mixed = {f["project"] for f in findings
                 if f["stage"] == stage and f["control_mixed_with_real"]}
        for name, predicate in [
            ("real_defects", lambda f: f["status"] in {"ran", "read"}),
            ("real_high", lambda f: f["status"] in {"ran", "read"} and f.get("severity") == "high"),
            ("not_defect", lambda f: f["status"] == "not-defect"),
            ("hygiene", lambda f: f["status"] == "not-defect" and
             f.get("noise_class") == "skill-requested-hygiene"),
            ("no_value", lambda f: f["status"] == "not-defect" and
             f.get("noise_class") == "no-value"),
            ("other_nondefect", lambda f: f["status"] == "not-defect" and
             f.get("noise_class") == "other"),
            ("refuted", lambda f: f["status"] == "refuted"),
            ("unverified", lambda f: f["status"] == "unverified"),
        ]:
            counts = collections.Counter()
            for finding in findings:
                if finding["stage"] == stage and predicate(finding):
                    counts.update(finding["runs"])
            per_run = {r["run"]: counts[r["run"]] for r in runs
                       if r["stage"] == str(stage) and r["validity"] == "complete"
                       and r["role"] == "audit"
                       and r["project"] in verified_projects - mixed}
            table[name] = {"per_run": per_run, **compare(per_run, runs, stage)}
        result["stages"][str(stage)] = table
        selected_decoys = [d for d in decoys if d["cell"].startswith(f"s{stage}-")]
        selected_controls = [c for c in controls if c["cell"].startswith(f"s{stage}-")]
        result["controls"][str(stage)] = {
            "decoy_found": sum(d["v"] == "found" for d in selected_decoys),
            "decoy_found_or_partial": sum(d["v"] in {"found", "partial"} for d in selected_decoys),
            "decoy_judgments": len(selected_decoys),
            "false_findings_refuted": sum(c["refuted"] for c in selected_controls),
            "false_findings_injected": len(selected_controls),
        }
    quality_values = {q["run"]: q["mean_quality"] for q in quality
                      if q["quality_interval"][0] == q["quality_interval"][1]}
    stage5 = {"quality": {"per_run": quality_values, **compare(quality_values, runs, 5)},
              "mean_grader_gap":
              sum(q["grader_gap"] for q in quality) / len(quality) if quality else None}
    for dimension in ("error_api", "dependencies", "structure"):
        values, raw_values = harm_values(quality, dimension)
        applicable = {q["case"] for q in quality
                      if any(g["harm"][dimension]["applicable"] for g in q["grades"])}
        chosen = [r for r in runs if r["project"] in applicable]
        stage5[f"harm_{dimension}"] = {
            "scale": "Mean fraction of each rubric's maximum; unknown scores excluded",
            "raw_grader_points": raw_values,
            "per_run": values, **compare(values, chosen, 5),
            "not_applicable_cases": sorted({q["case"] for q in quality} - applicable),
        }
    result["stages"]["5"] = stage5
    (HERE / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()

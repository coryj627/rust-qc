#!/usr/bin/env python3
"""Collect key graders' results from transcripts into audits/private/grades.json and report their agreement by letter.

Usage: collect_grades.py <scratch>     (no arm information is read or printed)
"""
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from collect_reports import returned  # noqa: E402


def main():
    scratch = pathlib.Path(sys.argv[1])
    grades = {}
    for row in (scratch / "audits" / "private" / "keygraders.tsv").read_text().splitlines():
        project, grader, agent_id = row.split("\t")
        _, body = returned(scratch.parent / "tasks" / f"{agent_id}.output")
        lines = [line for line in (body or "").splitlines() if line.strip().startswith("{")]
        grades.setdefault(project, {})[grader] = {json.loads(line)["bug"]: json.loads(line) for line in lines}
    (scratch / "audits" / "private" / "grades.json").write_text(json.dumps(grades, indent=1))
    cells = differ = 0
    for project, by_grader in grades.items():
        one, two = by_grader["1"], by_grader["2"]
        if set(one) != set(two):
            print("BUG SETS DIFFER", project, set(one) ^ set(two))
        for bug in sorted(set(one) & set(two)):
            for letter in "ABCD":
                cells += 1
                a, b = one[bug][letter]["v"], two[bug][letter]["v"]
                if a != b:
                    differ += 1
                    print(f"DISAGREE {project} {bug} {letter}: {a} vs {b}")
    print(f"{cells} cells, {differ} disagreements")
    counts = {}
    for by_grader in grades.values():
        for entry in by_grader["1"].values():
            for letter in "ABCD":
                counts[entry[letter]["v"]] = counts.get(entry[letter]["v"], 0) + 1
    print("grader 1 verdicts:", counts)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Print the evaluation's result tables from the data files in this folder.

Usage: python3 -I evals/results/score.py
"""

import collections
import itertools
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
ARMS = ("with-skill", "baseline")
REAL = ("ran", "read")


def load():
    runs = []
    for line in (HERE / "runs.tsv").read_text().splitlines()[1:]:
        run_id, project, arm, letter, minutes, tmp_null = line.split("\t")
        runs.append(
            {
                "id": run_id,
                "project": project,
                "arm": arm,
                "letter": letter,
                "minutes": int(minutes),
                "hang": tmp_null == "yes",
            }
        )
    keys = json.loads((HERE / "key_scores.json").read_text())
    findings = [json.loads(line) for line in (HERE / "findings.jsonl").read_text().splitlines()]
    return runs, keys, findings


def exact_sign_flip(differences):
    """Two-sided p for the mean project difference, flipping each project's sign."""
    observed = abs(sum(differences))
    flips = list(itertools.product((1, -1), repeat=len(differences)))
    extreme = sum(
        abs(sum(s * d for s, d in zip(signs, differences))) >= observed - 1e-9 for signs in flips
    )
    return extreme / len(flips)


def compare(name, per_run, runs, projects):
    """Print one metric by project and arm, with the sign-flip test on project means."""
    print(f"\n{name}")
    print(f"{'project':<16}{'with-skill':>14}{'baseline':>14}{'difference':>12}")
    differences = []
    totals = dict.fromkeys(ARMS, 0)
    for project in projects:
        values = {
            arm: [per_run[r["id"]] for r in runs if r["project"] == project and r["arm"] == arm]
            for arm in ARMS
        }
        for arm in ARMS:
            totals[arm] += sum(values[arm])
        difference = sum(values["with-skill"]) / 2 - sum(values["baseline"]) / 2
        differences.append(difference)
        left, right = (" + ".join(str(v) for v in values[arm]) for arm in ARMS)
        print(f"{project:<16}{left:>14}{right:>14}{difference:>+12.1f}")
    print(
        f"{'total':<16}{totals['with-skill']:>14}{totals['baseline']:>14}"
        f"{(totals['with-skill'] - totals['baseline']) / 2:>+12.1f}"
    )
    wins = (
        sum(d > 0 for d in differences),
        sum(d < 0 for d in differences),
        sum(d == 0 for d in differences),
    )
    print(
        f"projects favouring with-skill / baseline / tied: {wins[0]} / {wins[1]} / {wins[2]}; "
        f"sign-flip p = {exact_sign_flip(differences):.3f}"
    )


def main():
    runs, keys, findings = load()
    projects = list(dict.fromkeys(r["project"] for r in runs))

    print("Runs per arm, minutes used and hangs")
    for arm in ARMS:
        chosen = [r for r in runs if r["arm"] == arm]
        minutes = sum(r["minutes"] for r in chosen) / len(chosen)
        hangs = sum(r["hang"] for r in chosen)
        print(f"  {arm}: {len(chosen)} runs, mean {minutes:.1f} min, {hangs} with the stdin hang")

    reproduced = [b for b in keys if b["status"] == "reproduced"]
    print(f"\nKey entries: {len(keys)}, reproduced: {len(reproduced)}")
    for label, wanted, bugs in (
        ("Key bugs found (reproduced entries, 'found' only)", ("found",), reproduced),
        ("Key bugs found or partial (reproduced entries)", ("found", "partial"), reproduced),
        ("Key bugs found (all entries)", ("found",), keys),
    ):
        per_run = {
            r["id"]: sum(b["runs"][r["id"]] in wanted for b in bugs if b["project"] == r["project"])
            for r in runs
        }
        compare(label, per_run, runs, projects)

    print("\nReproduced key bugs found, by kind (share of run-bug pairs)")
    for kind in sorted({b["kind"] for b in reproduced}):
        cells = []
        for arm in ARMS:
            ids = {r["id"] for r in runs if r["arm"] == arm}
            pairs = [(b, i) for b in reproduced if b["kind"] == kind for i in b["runs"] if i in ids]
            hits = sum(b["runs"][i] == "found" for b, i in pairs)
            cells.append(f"{arm} {hits}/{len(pairs)} = {hits / len(pairs):.0%}")
        print(f"  {kind:<22}" + "   ".join(cells))

    bugs_by_arm = collections.Counter()
    for bug in reproduced:
        hit = {
            arm: any(
                bug["runs"][r["id"]] == "found"
                for r in runs
                if r["arm"] == arm and r["project"] == bug["project"]
            )
            for arm in ARMS
        }
        bugs_by_arm[hit["with-skill"], hit["baseline"]] += 1
    print(
        f"\nReproduced key bugs by which arm's two runs found them: "
        f"both {bugs_by_arm[True, True]}, with-skill only {bugs_by_arm[True, False]}, "
        f"baseline only {bugs_by_arm[False, True]}, neither {bugs_by_arm[False, False]}"
    )

    def count(predicate):
        totals = collections.Counter()
        for finding in findings:
            if predicate(finding):
                totals.update(finding["runs"])
        return {r["id"]: totals[r["id"]] for r in runs}

    print(
        f"\nVerified union: {len(findings)} items: "
        f"{dict(collections.Counter(f['status'] for f in findings))}"
    )
    compare(
        "Real defects reported (status ran or read)",
        count(lambda f: f["status"] in REAL),
        runs,
        projects,
    )
    compare(
        "Real high-severity defects reported",
        count(lambda f: f["status"] in REAL and f.get("severity") == "high"),
        runs,
        projects,
    )
    compare(
        "Items that are not defects (style, lint counts, drift, intended)",
        count(lambda f: f["status"] == "not-defect"),
        runs,
        projects,
    )
    compare("Items refuted", count(lambda f: f["status"] == "refuted"), runs, projects)

    arm_of = {r["id"]: r["arm"] for r in runs}
    only = collections.Counter()
    for finding in findings:
        if finding["status"] in REAL:
            arms = {arm_of[i] for i in finding["runs"]}
            only[(tuple(sorted(arms)), finding.get("severity"))] += 1
    print("\nReal union items by which arm reported them")
    for severity in ("high", "medium", "low"):
        print(
            f"  {severity:<8}both {only[(('baseline', 'with-skill'), severity)]:>3}   "
            f"with-skill only {only[(('with-skill',), severity)]:>3}   "
            f"baseline only {only[(('baseline',), severity)]:>3}"
        )


if __name__ == "__main__":
    main()

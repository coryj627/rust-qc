#!/usr/bin/env python3
"""Adapt frozen Claude audit workspaces and briefs to container-only Sol tools."""

import importlib.util
import json
import random
import shutil
import subprocess
import sys
from pathlib import Path

CLOCK = '''#!/usr/bin/env python3
import json
d = json.load(open("/work/.budget.json"))
print(str(d["active_seconds"] / 60) + " minutes used of " + str(d["budget_seconds"] / 60))
'''


def load_module(path):
    spec = importlib.util.spec_from_file_location("original_briefs", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def adapt(text):
    text = text.replace(
        "Use only the Read, Grep, Glob, Write, Edit and Bash tools.",
        "Use only sandbox_exec for reading, searching, writing, editing and commands.",
    )
    text = text.replace(
        "Your workspace on this machine is `/work`, mounted at `/work` in the container.",
        "Your workspace is `/work` in the container. All tools operate inside it.",
    )
    text = text.replace(
        "Never run cargo or rustc on this machine directly.",
        "sandbox_exec already dispatches through sb; ./run is a container-local passthrough.",
    )
    return text


def main():
    scratch = Path(sys.argv[1]).resolve()
    assets = scratch / "assets"
    original = assets / "evals/results/method"
    prepared = scratch / "audits/private/map.tsv"
    if prepared.exists():
        raise SystemExit("Refusing to replace an existing randomized schedule")
    leaks = subprocess.run(
        [str(scratch / "tools/bin/python"), str(original / "prep_audits.py"),
         str(scratch), str(assets), "leaks"], capture_output=True, text=True, check=True,
    )
    (scratch / "preparation/leaks.log").write_text(leaks.stdout)
    if "projects to inspect: 0" not in leaks.stdout:
        raise SystemExit("Inspect dependency source leakage before continuing")
    subprocess.run(
        [str(scratch / "tools/bin/python"), str(original / "prep_audits.py"),
         str(scratch), str(Path(__file__).resolve().parents[4]), "workspaces"], check=True,
    )
    briefs = load_module(original / "write_briefs.py")
    rows = []
    for line in prepared.read_text().splitlines():
        run_id, project, arm = line.split("\t")
        work = scratch / "audits/projects" / run_id
        (work / "run").write_text('#!/bin/sh\nexec "$@"\n')
        (work / "clock").write_text(CLOCK)
        (work / "run").chmod(0o755)
        (work / "clock").chmod(0o755)
        name, path, build = briefs.NOTES[project]
        skill = briefs.SKILL.format(folder="/work") if arm == "with-skill" else ""
        prompt = adapt(briefs.BRIEF.format(
            name=name, path=path, build=build, skill=skill, folder="/work",
        ))
        (scratch / "prompts" / f"{run_id}.txt").write_text(prompt)
        rows.append({"run_id": run_id, "project": project, "arm": arm, "stage": 3,
                     "budget_minutes": 45, "skill": "skill" if arm == "with-skill" else None})
    random.SystemRandom().shuffle(rows)
    for index, row in enumerate(rows, 1):
        row["order"] = index
    (scratch / "stage3-schedule.json").write_text(json.dumps(rows, indent=2) + "\n")
    output = Path(__file__).resolve().parent.parent / "stage3/preparation"
    output.mkdir(parents=True, exist_ok=True)
    for path in (scratch / "preparation").glob("*.json"):
        shutil.copyfile(path, output / path.name)
    shutil.copyfile(scratch / "preparation/leaks.log", output / "leaks.log")
    # The randomized mapping stays private until grading is complete.
    print("40 audit workspaces ready; randomized schedule retained privately")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Build blind audit workspaces under /srv/rqc/audits.

Adapted from evals/results/method/prep_audits.py.

Usage:
  prep_workspaces.py template <project> <repo> <tag>
      Put the crate at the tag in a template, rename instruction files, and fetch dependencies.
  prep_workspaces.py runs <project> <runs-per-arm>
      Copy the template per run, and add the wrappers, the arm's skill and the brief.

Everything that touches the upstream repository runs inside the sandbox through sb. The arm of
each run is written only to private/map.tsv, which no agent can read.
"""

import pathlib
import random
import shutil
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import ARMS, AUDITS, BRIEF, CLOCK, NOTES, QUARANTINE, RUN

SB = str(AUDITS / "sb")


def template(project, repo, tag):
    name = f"tmpl-{project.replace('_', '-')}"
    fetch = (
        f"git clone -q --depth 1 --branch {tag} https://github.com/{repo} /work/clone"
        " && mkdir /work/crate /work/tmp"
        f" && git -C /work/clone archive {tag} | tar -x -C /work/crate"
        " && rm -rf /work/clone && cd /work/crate && cargo fetch --locked"
    )
    subprocess.run([SB, "fetch", name, "sh", "-c", fetch], check=True)
    renamed = quarantine(AUDITS / "projects" / name / "crate")
    print(f"{name}: renamed {renamed or 'nothing'}")


def quarantine(crate):
    """Rename files Claude Code could read as instructions, deepest first. Returns their paths."""
    renamed = []
    for path in sorted(crate.rglob("*"), key=lambda p: len(p.parts), reverse=True):
        if path.name in QUARANTINE:
            path.rename(path.with_name(path.name + ".quarantined"))
            renamed.append(str(path.relative_to(crate)))
    return renamed


def runs(project, per_arm):
    rng = random.SystemRandom()
    private = AUDITS / "private"
    source = AUDITS / "projects" / f"tmpl-{project.replace('_', '-')}"
    arms = [arm for arm in ARMS for _ in range(per_arm)]
    rng.shuffle(arms)
    used = (
        {line.split("\t")[0] for line in (private / "map.tsv").read_text().splitlines()}
        if (private / "map.tsv").exists()
        else set()
    )
    rows = []
    for arm in arms:
        while (run_id := f"r{rng.randrange(16**6):06x}") in used:
            pass
        used.add(run_id)
        folder = AUDITS / "projects" / run_id
        subprocess.run(["cp", "-a", str(source), str(folder)], check=True)
        for name, text in (("run", RUN), ("clock", CLOCK)):
            (folder / name).write_text(text)
            (folder / name).chmod(0o755)
        shutil.copytree(private / f"skill-{arm}" / "skills" / "rust-qc", folder / "skill")
        title, path, build = NOTES[project]
        (folder / "BRIEF.md").write_text(
            BRIEF.format(name=title, path=path, folder=folder, build=build)
        )
        rows.append(f"{run_id}\t{project}\t{arm}\n")
    with (private / "map.tsv").open("a") as out:
        out.writelines(rows)
    print(f"{len(rows)} workspaces: {' '.join(r.split(chr(9))[0] for r in rows)}")


def main():
    action = sys.argv[1]
    if action == "template":
        template(*sys.argv[2:5])
    elif action == "runs":
        runs(sys.argv[2], int(sys.argv[3]))
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()

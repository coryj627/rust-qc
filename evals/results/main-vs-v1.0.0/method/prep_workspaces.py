#!/usr/bin/env python3
"""Build blind audit workspaces under /srv/rqc/audits.

Adapted from evals/results/method/prep_audits.py.

Usage:
  prep_workspaces.py template <project> <repo> <tag>
      Put the crate at the tag in a template, rename instruction files, and fetch dependencies.
  prep_workspaces.py keyed <clone> <project>...
      The same for keyed projects, with the repo, tag and lockfile their keys and evals/repro name.
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

import yaml

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import ARMS, AUDITS, BRIEF, CLOCK, NOTES, QUARANTINE, RUN

SB = str(AUDITS / "sb")


def template(project, repo, tag, lock=None):
    """Crate at the tag in projects/tmpl-<project>, instruction files renamed, deps fetched.

    lock is a lockfile to use when the tag has none (evals/repro/<project>/base-Cargo.lock).
    git2's libgit2 is a submodule, so its sources are copied in without their .git.
    """
    name = f"tmpl-{project.replace('_', '-')}"
    folder = AUDITS / "projects" / name
    folder.mkdir(parents=True, exist_ok=True)
    steps = [
        f"git clone -q --depth 1 --branch {tag} https://github.com/{repo} /work/clone",
        "mkdir -p /work/crate /work/tmp",
        f"git -C /work/clone archive {tag} | tar -x -C /work/crate",
    ]
    if project == "git2":
        steps += [
            "git -C /work/clone submodule update -q --init --depth 1 libgit2-sys/libgit2",
            "cp -r /work/clone/libgit2-sys/libgit2/. /work/crate/libgit2-sys/libgit2/",
            "rm -rf /work/crate/libgit2-sys/libgit2/.git",
        ]
    steps.append("rm -rf /work/clone")
    if lock:
        shutil.copy(lock, folder / "base-Cargo.lock")
        steps += [
            "test ! -f /work/crate/Cargo.lock",
            "mv /work/base-Cargo.lock /work/crate/Cargo.lock",
        ]
    steps.append("cd /work/crate && cargo fetch --locked")
    subprocess.run([SB, "fetch", name, "sh", "-c", " && ".join(steps)], check=True)
    renamed = quarantine(folder / "crate")
    print(f"{name}: renamed {renamed or 'nothing'}")


def keyed(clone, project):
    """Template for a keyed project, with the repo, tag and lockfile its answer key names."""
    key = yaml.safe_load((clone / "evals" / "keys" / f"{project}.yaml").read_text())
    lock = clone / "evals" / "repro" / project / "base-Cargo.lock"
    template(project, key["repo"], key["baseline"]["tag"], lock if lock.exists() else None)


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
    elif action == "keyed":
        for project in sys.argv[3:]:
            keyed(pathlib.Path(sys.argv[2]), project)
    elif action == "runs":
        runs(sys.argv[2], int(sys.argv[3]))
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()

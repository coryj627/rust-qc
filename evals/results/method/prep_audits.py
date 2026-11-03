#!/usr/bin/env python3
"""Build blind audit workspaces. Run under tools/venv.

Usage: prep_audits.py <scratch> <worktree> template <project>...   fetch one clean template per project
       prep_audits.py <scratch> <worktree> leaks                   list registry crates named like an audited crate
       prep_audits.py <scratch> <worktree> workspaces              clone 4 runs per project and write the private map

A template holds the crate from `git archive` at the tag, the lockfile if the tag has none, and fetched dependencies.
Nothing comes from the key-building folders except git2's libgit2 sources, which leave out .git.
"""
import pathlib
import random
import shutil
import subprocess
import sys

import yaml

PROJECTS = ["h2", "quinn-proto", "just", "heapless", "git2", "mlua", "zip", "pulldown-cmark", "moka", "simd-json"]
CRATE_NAMES = {"zip": "zip", "git2": "git2", "quinn-proto": "quinn-proto"}
SKILL_TAG = "v1.0.0"

RUN = """#!/bin/sh
# Runs a command in this workspace's container: no network, only this folder mounted, at /work.
here="$(cd "$(dirname "$0")" && pwd)"
RQC_WAIT_LOG="$here/.waits" RQC_TIMEOUT=900 exec "$here/../../sb" run "$(basename "$here")" env TMPDIR=/work/tmp "$@"
"""

CLOCK = """#!/bin/bash
# Prints minutes used. Time spent waiting for a container slot does not count.
here="$(cd "$(dirname "$0")" && pwd)"
now=$(date +%s)
[ -f "$here/.clock-start" ] || echo "$now" > "$here/.clock-start"
start=$(cat "$here/.clock-start")
waited=0
if [ -f "$here/.waits" ]; then waited=$(awk '{s+=$1} END {print s+0}' "$here/.waits"); fi
used=$(( (now - start - waited) / 60 ))
echo "$now $used" >> "$here/.clock-log"
echo "$used minutes used of 45"
"""


def key(scratch, project):
    return yaml.safe_load((scratch / "keys" / f"{project}.yaml").read_text())


def template(scratch, worktree, project):
    k = key(scratch, project)
    bare = scratch / "repos" / (k["repo"].replace("/", "_") + ".git")
    folder = scratch / "audits" / "projects" / f"tmpl-{project}"
    crate = folder / "crate"
    crate.mkdir(parents=True)
    archive = subprocess.Popen(["git", "-C", str(bare), "archive", k["baseline"]["tag"]], stdout=subprocess.PIPE)
    subprocess.run(["tar", "-x", "-C", str(crate)], stdin=archive.stdout, check=True)
    if archive.wait() != 0:
        raise SystemExit(f"archive failed: {project}")
    lock = worktree / "evals" / "repro" / project / "base-Cargo.lock"
    if lock.exists():
        if (crate / "Cargo.lock").exists():
            raise SystemExit(f"{project}: tag has a lockfile and base-Cargo.lock exists")
        shutil.copy(lock, crate / "Cargo.lock")
    if project == "git2":
        source = scratch / "sandbox" / "projects" / "git2" / "base" / "libgit2-sys" / "libgit2"
        target = crate / "libgit2-sys" / "libgit2"
        shutil.copytree(source, target, symlinks=True, ignore=shutil.ignore_patterns(".git"), dirs_exist_ok=True)
    (folder / "tmp").mkdir()
    sb = str(scratch / "audits" / "sb")
    subprocess.run([sb, "fetch", f"tmpl-{project}", "sh", "-c", "cd /work/crate && cargo fetch --locked"], check=True)


def leaks(scratch):
    bad = 0
    for project in PROJECTS:
        k = key(scratch, project)
        tag = k["baseline"]["tag"]
        names = {project, CRATE_NAMES.get(project, project)}
        src = list((scratch / "audits" / "projects" / f"tmpl-{project}" / ".cargo-home" / "registry" / "src").glob("*"))
        found = sorted(p.name for s in src for p in s.iterdir() if p.name.rsplit("-", 1)[0] in names)
        print(f"{project} @ {tag}: {len(found)} same-named registry crates: {found}")
        bad += bool(found)
    print("projects to inspect:", bad)


def workspaces(scratch, worktree):
    rng = random.SystemRandom()
    private = scratch / "audits" / "private"
    rows = []
    used = set()
    skill = subprocess.Popen(["git", "-C", str(worktree), "archive", SKILL_TAG, ".agents/skills/rust-qc"], stdout=subprocess.PIPE)
    tree = private / "skill-tree"
    tree.mkdir(exist_ok=True)
    subprocess.run(["tar", "-x", "-C", str(tree)], stdin=skill.stdout, check=True)
    if skill.wait() != 0:
        raise SystemExit("skill archive failed")
    for project in PROJECTS:
        arms = ["with-skill", "with-skill", "baseline", "baseline"]
        rng.shuffle(arms)
        for arm in arms:
            while (run_id := f"r{rng.randrange(16**5):05x}") in used:
                pass
            used.add(run_id)
            folder = scratch / "audits" / "projects" / run_id
            subprocess.run(["/bin/cp", "-cR", str(scratch / "audits" / "projects" / f"tmpl-{project}"), str(folder)], check=True)
            (folder / "run").write_text(RUN)
            (folder / "clock").write_text(CLOCK)
            (folder / "run").chmod(0o755)
            (folder / "clock").chmod(0o755)
            if arm == "with-skill":
                shutil.copytree(tree / ".agents" / "skills" / "rust-qc", folder / "skill")
            rows.append(f"{run_id}\t{project}\t{arm}\n")
    (private / "map.tsv").write_text("".join(rows))
    print(f"{len(rows)} workspaces")


def main():
    scratch, worktree, action = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
    if action == "template":
        for project in sys.argv[4:]:
            template(scratch, worktree, project)
    elif action == "leaks":
        leaks(scratch)
    elif action == "workspaces":
        workspaces(scratch, worktree)


if __name__ == "__main__":
    main()

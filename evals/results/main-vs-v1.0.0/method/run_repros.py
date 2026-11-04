#!/usr/bin/env python3
"""Run the key reproducers in evals/repro/ through sb, as evals/README.md describes.

Usage:
  run_repros.py setup <clone> <project>
      Baseline tree, lockfile, project extras and dependencies, with the network on.
  run_repros.py run <clone> <project> <out.tsv> <logs-dir>
      Every bug of the project in rerun.tsv, with the network off.
  run_repros.py clean <clone> <project>
      Delete the project's folder and build volume, to keep disk use bounded.

Projects live under /srv/rqc/repro/projects/<project>, with their own copy of sb. Only fetching
uses the network; every build and run goes through `sb run`.
"""

import pathlib
import shutil
import subprocess
import sys
import time

import yaml

ROOT = pathlib.Path("/srv/rqc/repro")
SB = str(ROOT / "sb")
LOG_LIMIT = 256 * 1024


def key(clone, project):
    return yaml.safe_load((clone / "evals" / "keys" / f"{project}.yaml").read_text())


def bugs(clone, project):
    rows = (clone / "evals" / "repro" / "rerun.tsv").read_text().splitlines()[1:]
    return [r.split("\t")[1] for r in rows if r.split("\t")[0] == project]


def setup(clone, project):
    k = key(clone, project)
    tag = k["baseline"]["tag"]
    work = ROOT / "projects" / project
    work.mkdir(parents=True, exist_ok=True)
    shutil.copytree(clone / "evals" / "repro" / project, work / "repro", dirs_exist_ok=True)
    steps = [
        f"git clone -q --depth 1 --branch {tag} https://github.com/{k['repo']} /work/clone",
        f"mkdir -p /work/base && git -C /work/clone archive {tag} | tar -x -C /work/base",
    ]
    if project == "git2":
        steps += [
            "git -C /work/clone submodule update -q --init --depth 1 libgit2-sys/libgit2",
            "cp -r /work/clone/libgit2-sys/libgit2/. /work/base/libgit2-sys/libgit2/",
            "rm -rf /work/base/libgit2-sys/libgit2/.git",
        ]
    steps += [
        "rm -rf /work/clone",
        (
            "{ [ ! -f /work/repro/base-Cargo.lock ]"
            " || cp /work/repro/base-Cargo.lock /work/base/Cargo.lock; }"
        ),
    ]
    if project == "quinn-proto":
        steps += [
            "cp -r /work/base /work/reprotree",
            "cd /work/reprotree && patch -s -p1 < /work/repro/reprotree.patch",
        ]
    if project == "simd-json":
        steps += ["ln -sfn /opt/miri-sysroot /work/miri-sysroot-x86"]
    steps += [
        "cd /work/base && cargo fetch",
        "{ [ ! -d /work/reprotree ] || (cd /work/reprotree && cargo fetch); }",
        # Each reproducer crate with its own manifest: fetch only, never build.
        (
            'for d in /work/repro/*/; do if [ -f "$d/Cargo.toml" ]; then'
            ' (cd "$d" && cargo fetch -q) || echo "fetch failed: $d"; fi; done'
        ),
    ]
    # The per-reproducer fetch loop reports its own failures, so it runs after the chain either way.
    script = " && ".join(steps[:-1]) + "; " + steps[-1]
    subprocess.run([SB, "fetch", project, "sh", "-c", script], check=True)


def run(clone, project, out, logs):
    logs.mkdir(parents=True, exist_ok=True)
    with out.open("a") as table:
        for bug in bugs(clone, project):
            start = time.monotonic()
            proc = subprocess.run(
                [SB, "run", project, "sh", f"/work/repro/{bug}/run.sh"],
                capture_output=True,
                text=True,
                check=False,
            )
            seconds = round(time.monotonic() - start)
            text = proc.stdout + proc.stderr
            if len(text) > LOG_LIMIT:
                half = LOG_LIMIT // 2
                text = (
                    text[:half]
                    + "\n[... middle cut to keep the log under 256 KiB ...]\n"
                    + text[-half:]
                )
            (logs / f"{bug}.txt").write_text(
                f"$ sb run {project} sh /work/repro/{bug}/run.sh\n{text}\nexit={proc.returncode}\n"
            )
            result = "reproduced" if proc.returncode == 0 else "not-reproduced"
            table.write(f"{project}\t{bug}\t{proc.returncode}\t{seconds}\t{result}\t\n")
            table.flush()
            print(project, bug, proc.returncode, seconds, flush=True)


def clean(project):
    shutil.rmtree(ROOT / "projects" / project)
    subprocess.run(
        ["docker", "volume", "rm", f"rqc-target-{project}"], capture_output=True, check=False
    )


def main():
    action, clone, project = sys.argv[1], pathlib.Path(sys.argv[2]), sys.argv[3]
    if action == "setup":
        setup(clone, project)
    elif action == "run":
        run(clone, project, pathlib.Path(sys.argv[4]), pathlib.Path(sys.argv[5]))
    elif action == "clean":
        clean(project)
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()

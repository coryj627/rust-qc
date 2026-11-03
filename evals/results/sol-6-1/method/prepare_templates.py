#!/usr/bin/env python3
"""Prepare the original scratch layout from pinned, public source archives.

Only git retrieval and archive extraction happen on the host. Dependency fetches
go through sb fetch; no candidate build or test runs here.
"""

import argparse
import io
import json
import shutil
import subprocess
import tarfile
from pathlib import Path

import yaml


def command(argv, **kwargs):
    return subprocess.run(argv, check=True, timeout=600, **kwargs)


def archive(repository, ref, target):
    target.mkdir(parents=True, exist_ok=True)
    result = command(["git", "-C", str(repository), "archive", ref], stdout=subprocess.PIPE)
    with tarfile.open(fileobj=io.BytesIO(result.stdout)) as stream:
        stream.extractall(target, filter="data")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    parser.add_argument("projects", nargs="+")
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    assets = scratch / "assets"
    repos = scratch / "repos"
    repos.mkdir(exist_ok=True)
    shutil.copytree(assets / "evals/keys", scratch / "keys", dirs_exist_ok=True)
    (scratch / "audits/private").mkdir(parents=True, exist_ok=True)
    (scratch / "preparation").mkdir(exist_ok=True)
    for project in args.projects:
        key = yaml.safe_load((scratch / "keys" / f"{project}.yaml").read_text())
        repository = repos / (key["repo"].replace("/", "_") + ".git")
        tag, commit = key["baseline"]["tag"], key["baseline"]["commit"]
        if not repository.exists():
            command(["git", "clone", "--bare", "--depth", "1", "--branch", tag,
                     f"https://github.com/{key['repo']}.git", str(repository)])
        actual = command(["git", "-C", str(repository), "rev-parse", f"{tag}^{{commit}}"],
                         stdout=subprocess.PIPE, text=True).stdout.strip()
        if actual != commit:
            raise ValueError(f"{project}: pinned commit mismatch")
        record = {"project": project, "repository": key["repo"], "tag": tag, "commit": actual}
        if project == "git2":
            tree = command(["git", "-C", str(repository), "ls-tree", commit,
                            "libgit2-sys/libgit2"], stdout=subprocess.PIPE, text=True).stdout
            subcommit = tree.split()[2]
            bare = repos / "libgit2_libgit2.git"
            if not bare.exists():
                command(["git", "init", "--bare", str(bare)])
                command(["git", "-C", str(bare), "fetch", "--depth", "1",
                         "https://github.com/libgit2/libgit2.git", subcommit])
            archive(bare, subcommit,
                    scratch / "sandbox/projects/git2/base/libgit2-sys/libgit2")
            record["libgit2_commit"] = subcommit
        log = scratch / "preparation" / f"{project}.log"
        with log.open("w") as output:
            command([str(scratch / "tools/bin/python"),
                     str(assets / "evals/results/method/prep_audits.py"), str(scratch),
                     str(assets), "template", project],
                    stdout=output, stderr=subprocess.STDOUT)
        (scratch / "preparation" / f"{project}.json").write_text(
            json.dumps(record, indent=2) + "\n"
        )
        print(f"{project}: pinned template and dependencies ready", flush=True)


if __name__ == "__main__":
    main()

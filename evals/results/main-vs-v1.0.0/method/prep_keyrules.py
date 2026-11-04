#!/usr/bin/env python3
"""Build one key-rule workspace per project under /srv/rqc/keyrules.

Usage: prep_keyrules.py <clone> <project>...

Each workspace holds entries.yaml (the key entries that have a reproducer), fixes/<id>.diff (the
upstream fix from `git show`), base/ (the repository at the baseline tag, from `git archive`, with
instruction files renamed) and BRIEF.md. Only git archive and git show run on the upstream clone.
"""

import pathlib
import subprocess
import sys

import yaml

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from prep_workspaces import quarantine

ROOT = pathlib.Path("/srv/rqc/keyrules")
DIFF_LIMIT = 256 * 1024


def git(repo, *args, **kw):
    return subprocess.run(["git", "-C", str(repo), *args], check=True, **kw)


def prepare(clone, project):
    key = yaml.safe_load((clone / "evals" / "keys" / f"{project}.yaml").read_text())
    rows = (clone / "evals" / "repro" / "rerun.tsv").read_text().splitlines()[1:]
    have = {r.split("\t")[1] for r in rows if r.split("\t")[0] == project}
    entries = [b for b in key["bugs"] if b["id"] in have]
    repo = ROOT / "repos" / f"{project}.git"
    if not repo.exists():
        subprocess.run(
            [
                "git",
                "clone",
                "-q",
                "--bare",
                "--filter=blob:none",
                f"https://github.com/{key['repo']}",
                str(repo),
            ],
            check=True,
        )
    folder = ROOT / f"kr-{project}"
    (folder / "fixes").mkdir(parents=True, exist_ok=True)
    (folder / "base").mkdir(exist_ok=True)
    archive = subprocess.Popen(
        ["git", "-C", str(repo), "archive", key["baseline"]["tag"]], stdout=subprocess.PIPE
    )
    subprocess.run(["tar", "-x", "-C", str(folder / "base")], stdin=archive.stdout, check=True)
    if archive.wait() != 0:
        raise SystemExit(f"archive failed: {project}")
    renamed = quarantine(folder / "base")
    for entry in entries:
        diff = git(repo, "show", "--format=fuller", entry["fix_commit"], capture_output=True).stdout
        if len(diff) > DIFF_LIMIT:
            half = DIFF_LIMIT // 2
            diff = (
                diff[:half]
                + b"\n[... middle cut to keep the diff under 256 KiB ...]\n"
                + diff[-half:]
            )
        (folder / "fixes" / f"{entry['id']}.diff").write_bytes(diff)
    (folder / "entries.yaml").write_text(yaml.safe_dump(entries, sort_keys=False, width=110))
    brief = (pathlib.Path(__file__).parent / "key-rule-brief.md").read_text()
    (folder / "BRIEF.md").write_text(
        brief.format(project=project, folder=folder, audited_path=key["audited_path"])
    )
    print(f"{project}: {len(entries)} entries, renamed {renamed or 'nothing'}")


def main():
    clone = pathlib.Path(sys.argv[1])
    for project in sys.argv[2:]:
        prepare(clone, project)


if __name__ == "__main__":
    main()

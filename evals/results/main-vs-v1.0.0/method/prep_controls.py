#!/usr/bin/env python3
"""Build one controls workspace per project, c-<project>, from its template.

Usage: prep_controls.py <clone> <project>...

Each workspace is a copy of projects/tmpl-<project> with ./run, ./clock, key.yaml (the real bugs'
id, summary and location, so controls can avoid them) and BRIEF.md from controls-brief.md.
"""

import pathlib
import subprocess
import sys

import yaml

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import AUDITS, CLOCK, NOTES, RUN


def prepare(clone, project):
    key = yaml.safe_load((clone / "evals" / "keys" / f"{project}.yaml").read_text())
    folder = AUDITS / "projects" / f"c-{project}"
    subprocess.run(
        ["cp", "-a", str(AUDITS / "projects" / f"tmpl-{project}"), str(folder)], check=True
    )
    for name, text in (("run", RUN), ("clock", CLOCK)):
        (folder / name).write_text(text)
        (folder / name).chmod(0o755)
    bugs = [{k: b.get(k) for k in ("id", "summary", "location")} for b in key["bugs"]]
    (folder / "key.yaml").write_text(yaml.safe_dump(bugs, sort_keys=False, width=110))
    name, path, build = NOTES[project]
    brief = (pathlib.Path(__file__).parent / "controls-brief.md").read_text()
    (folder / "BRIEF.md").write_text(brief.format(name=name, path=path, folder=folder, build=build))
    print(f"c-{project} ready")


def main():
    clone = pathlib.Path(sys.argv[1])
    for project in sys.argv[2:]:
        prepare(clone, project)


if __name__ == "__main__":
    main()

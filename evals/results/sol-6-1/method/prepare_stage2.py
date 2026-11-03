#!/usr/bin/env python3
"""Prepare frozen discovery fixtures; never executes candidate code."""

import json
import shutil
import sys
from pathlib import Path


def main():
    scratch = Path(sys.argv[1]).resolve()
    root = Path(__file__).resolve().parent.parent
    cases = json.loads((root / "stage2/cases.json").read_text())
    prompts = scratch / "prompts"
    prompts.mkdir(exist_ok=True)
    for case in cases:
        workspace = scratch / "audits/projects" / case["id"]
        workspace.mkdir(parents=True)
        (workspace / "tmp").mkdir()
        crate = workspace / "crate"
        crate.mkdir()
        if "Cargo.toml" not in case["files"]:
            (crate / "Cargo.toml").write_text(
                '[package]\nname = "discovery_fixture"\nversion = "0.1.0"\n'
                'edition = "2021"\npublish = false\n'
            )
        for name, content in case["files"].items():
            path = crate / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
        shutil.copytree(
            scratch / "skill-tree/.agents/skills/rust-qc",
            workspace / ".agents/skills/rust-qc",
        )
        (prompts / f"{case['id']}.txt").write_text(
            case["prompt"] + "\n\nThe project is in /work/crate.\n"
        )
        print(case["id"])


if __name__ == "__main__":
    main()

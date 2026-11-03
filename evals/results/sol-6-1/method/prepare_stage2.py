#!/usr/bin/env python3
"""Prepare frozen discovery fixtures; never executes candidate code."""

import argparse
import json
import shutil
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    parser.add_argument("--suffix", default="")
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    root = Path(__file__).resolve().parent.parent
    cases = json.loads((root / "stage2/cases.json").read_text())
    prompts = scratch / "prompts"
    prompts.mkdir(exist_ok=True)
    for case in cases:
        run_id = case["id"] + args.suffix
        workspace = scratch / "audits/projects" / run_id
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
        (prompts / f"{run_id}.txt").write_text(
            case["prompt"] + "\n\nThe project is in /work/crate.\n"
        )
        print(run_id)


if __name__ == "__main__":
    main()

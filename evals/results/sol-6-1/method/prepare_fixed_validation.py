#!/usr/bin/env python3
"""Provision separate native fix controls; keep caches outside treatment templates."""

import json
import shutil
import subprocess
import sys
from pathlib import Path

from prepare_change_authors import CASES


def main():
    scratch = Path(sys.argv[1]).resolve()
    for case in CASES:
        run = f"validate-fixed-{case['id']}"
        work = scratch / "audits/projects" / run
        work.mkdir()
        (work / "tmp").mkdir()
        author = scratch / "audits/projects" / f"author-{case['id']}"
        for name, source in [("crate", author / "fixed"),
                             (".cargo-home", author / ".cargo-home"),
                             ("prepared", author / "prepared")]:
            subprocess.run(["/bin/cp", "-cR", str(source), str(work / name)], check=True)
        # Dependency fetch is the original sandbox's network-enabled preparation
        # operation, not candidate execution. Toolchain auto-install stays off.
        command = [str(scratch / "audits/sb"), "fetch", run, "sh", "-c",
                   "cd /work/crate && RUSTUP_AUTO_INSTALL=0 cargo fetch"]
        output = Path(__file__).resolve().parent.parent / "stage5/control-refresh" / case["id"]
        output.mkdir(parents=True)
        fetched = subprocess.run(command, capture_output=True, text=True, check=False)
        (output / "fetch.log").write_text(fetched.stdout + fetched.stderr)
        record = {"role": "operator-dependency-preparation", "model": None,
                  "case": case["id"], "upstream_commit": case["fix"],
                  "command": "sb fetch <control> sh -c "
                  "'cd /work/crate && RUSTUP_AUTO_INSTALL=0 cargo fetch'",
                  "exit_code": fetched.returncode,
                  "treatment_templates_changed": False, "toolchain_installation": False}
        (output / "preparation.json").write_text(json.dumps(record, indent=2) + "\n")
        if fetched.returncode:
            raise SystemExit(f"Fixed control dependency fetch failed: {case['id']}")
        # Keep a standalone copy of the exact author-owned runners as evidence.
        shutil.copyfile(author / "prepared/scoring.md", output / "author-scoring.md")
        print(f"{run}: native fixed dependencies fetched", flush=True)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Give independent rubric authors baseline/fix sources, never audit outputs."""

import io
import json
import subprocess
import sys
import tarfile
from pathlib import Path

CASES = [
    {
        "id": "heapless", "project": "heapless", "repo": "rust-embedded_heapless.git",
        "bug": "heapless-6", "fix": "906f8ae525fc8f019886f5fb39d9878557611de5",
        "upstream_test": "src/index_map.rs::tests::truncate_invalid",
        "task": (
            "Fix IndexMap::truncate in heapless 0.9.1. After truncating a map containing "
            "colliding hashes, every retained entry must remain reachable by get and "
            "contains_key, with iteration order and length consistent. Preserve retained "
            "values and behavior for zero, intermediate, equal and larger truncation lengths. "
            "Keep the public API stable and avoid unrelated changes. Add regression coverage."
        ),
    },
    {
        "id": "wikilink", "project": "pulldown-cmark",
        "repo": "pulldown-cmark_pulldown-cmark.git", "bug": "pulldown-cmark-3",
        "fix": "ebf31da886d90f34d9519278e323dbefb08444af",
        "upstream_test": "pulldown-cmark/src/parse.rs::wikilink_malformed_no_subtract_overflow",
        "task": (
            "Fix the malformed-image-wikilink panic in pulldown-cmark 0.13.0. With "
            "wikilinks enabled, malformed link delimiters can produce subtraction overflow "
            "in debug builds and an invalid text slice in release builds. Parsing untrusted "
            "text must handle these malformed links without panicking, while preserving "
            "valid wikilink and ordinary Markdown behavior. Keep public APIs stable and "
            "avoid unrelated changes. Add regression coverage."
        ),
    },
    {
        "id": "just", "project": "just", "repo": "casey_just.git", "bug": "just-6",
        "fix": "0beb79f49def4245757cddf15d52e485332ecb49",
        "upstream_test": (
            "tests/functions.rs::dry_run_does_not_execute_shell_function and "
            "dry_run_shell_function_output_is_escaped"
        ),
        "task": (
            "Fix just 1.50.0 so --dry-run never executes commands passed to the shell() "
            "function. Dry-run output should represent the unevaluated shell call clearly, "
            "including properly quoted and escaped arguments. Preserve actual execution "
            "outside dry-run and existing backtick behavior. Keep public behavior otherwise "
            "stable, avoid unrelated changes, and add regression coverage."
        ),
    },
]


def main():
    scratch = Path(sys.argv[1]).resolve()
    for case in CASES:
        run_id = f"author-{case['id']}"
        work = scratch / "audits/projects" / run_id
        subprocess.run(["/bin/cp", "-cR",
                        str(scratch / "audits/projects" / f"tmpl-{case['project']}"),
                        str(work)], check=True)
        fixed = work / "fixed"
        fixed.mkdir()
        archived = subprocess.check_output([
            "git", "-C", str(scratch / "repos" / case["repo"]), "archive", case["fix"],
        ])
        with tarfile.open(fileobj=io.BytesIO(archived)) as stream:
            stream.extractall(fixed, filter="data")
        (work / "case-input.json").write_text(json.dumps(case, indent=2) + "\n")
        prompt = """You are an independent case and rubric author. You have not seen any treatment
outputs, other rubrics, skill rules or arm mappings. Read /work/case-input.json.
/work/crate is the baseline and /work/fixed is the exact upstream fix commit.
The upstream regression test named in the input is the hidden scoring oracle.
Do not implement a candidate fix. No skills are available.

Before any treatment run, create /work/prepared/case.md with the task wording from
case-input.json, workspace/build instructions and no hidden-test contents or fix
hints; /work/prepared/rubric.md for two blind graders; /work/prepared/hidden/ with
the upstream regression tests held back from the treatment agent; and
/work/prepared/scoring.md with exact commands to apply/run the tests after a
candidate returns. Preserve the upstream regression assertions; any integration
adaptation must be described. Include provenance (bug ID, source path, commit).
Add neighboring hidden cases only where they follow from the task specification.

Use a 12-point quality rubric with explicit measurable criteria covering behavior,
regression coverage, compatibility, clarity/documentation, and unnecessary changes.
Separately score harm: unbranchable error types (mark not applicable when appropriate),
unjustified dependencies including test dependencies, and needless modules or
unrelated edits. Accept different correct implementations, not a preferred patch
or stylistic conformity. Do not reward length.

Validate hidden tests against pristine copies of both baseline and fixed source:
the baseline must fail for the intended behavior and the fix must pass. Retain
concise command logs in /work/prepared/. Do not change baseline production files.
Only sandbox_exec may run code. The network is off and dependencies were fetched
for the baseline; do not install tools or Rust toolchains. If a fix control lacks
dependencies, document that precise gap rather than claiming it passed. Use
/work/tmp for executable temporary files. Do not read anything outside this
workspace or recommend skill changes. Final response: files, executed checks,
outcomes and limitations. You have 45 minutes; finish earlier if complete.
"""
        (scratch / "prompts" / f"{run_id}.txt").write_text(prompt)
        print(f"{run_id}: baseline and fixed source prepared", flush=True)


if __name__ == "__main__":
    main()

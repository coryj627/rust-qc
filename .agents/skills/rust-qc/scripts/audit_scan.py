#!/usr/bin/env python3
"""Heuristic scan of a Rust crate for issues the compiler and clippy miss.

Usage: python3 -I audit_scan.py <crate-root>

Covers only what tooling cannot check: glob re-exports, weasel-word type
names, ASCII-art comments, design-history docs, parameter tables, bare TODOs,
empty expect messages, and print macros in library code. Undocumented unsafe,
missing docs, unwrap, and lint-override checks are done precisely by
`check.sh --audit` through clippy and are deliberately not duplicated here.

Every finding is a lead to confirm by reading the code, not a verdict.
Exits 2 on a precondition failure, otherwise 0.
"""

import re
import sys
from dataclasses import dataclass
from pathlib import Path

SEVERITIES = ("Major", "Minor", "Note")

# Never scanned: build output, vendored code, benchmark harnesses, and test
# data, which can contain arbitrary third-party Rust used as input.
SKIP_DIRS = {
    "target", ".git", "vendor", "node_modules", "benches", "benchmarks",
    "bench", "fuzz", "testdata", "data", "fixtures", "haystacks", "snapshots",
}
# Scanned, but production-only rules are skipped.
NON_PRODUCTION_DIRS = {"tests", "examples", "testprograms", "build"}
# Inside `src/`, folders are modules, so names like `data` and `build` mean
# nothing special. Only these still mark test data or test code there.
SKIP_DIRS_IN_SRC = {"testdata", "fixtures", "haystacks", "snapshots"}
NON_PRODUCTION_DIRS_IN_SRC = {"tests"}
# Files that hold test code by convention (declared as `#[cfg(test)] mod tests;`).
TEST_FILE_STEMS = ("tests", "test", "testutil", "test_util", "test_utils")

WEASEL_WORDS = ("Manager", "Service", "Factory", "Helper", "Helpers", "Util", "Utils")

# Plain `pub` only: `pub(crate) use imp::*;` switching between cfg-selected
# implementations is an internal pattern, not public surface.
RE_GLOB_PUB_USE = re.compile(r"^\s*pub\s+use\s+[\w:]+::\*\s*;")
RE_TYPE_DECL = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(struct|enum|trait)\s+(\w+)")
RE_EMPTY_EXPECT = re.compile(r"\.expect\(\s*\"\s*\"\s*\)")
RE_PRINT = re.compile(r"\b(e?print(ln)?)!\s*\(")
RE_PARAM_TABLE = re.compile(r"^\s*///\s*#\s*(Arguments|Parameters|Params)\b")
RE_JOURNAL = re.compile(
    r"\b(we (chose|decided|picked|opted)|design journey|"
    r"this refactor|was refactored|guidelines? (followed|applied))\b",
    re.IGNORECASE,
)
RE_BOX_DRAWING = re.compile(r"[─-╿]|\+-{3,}\+")
RE_BARE_TODO = re.compile(r"\b(TODO|FIXME|XXX)\b(?!\s*\()")
RE_CFG_TEST = re.compile(r"^\s*#\[cfg\((all\()?test\b")


@dataclass
class Finding:
    severity: str
    rule: str
    path: Path
    line: int
    message: str


def split_at_src(rel: Path):
    """Returns the folders of a relative path outside and inside its first `src/`."""
    folders = rel.parts[:-1]
    if "src" not in folders:
        return set(folders), set()
    i = folders.index("src")
    return set(folders[:i]), set(folders[i + 1:])


def rust_files(root: Path):
    for path in sorted(root.rglob("*.rs")):
        outside, inside = split_at_src(path.relative_to(root))
        if not (outside & SKIP_DIRS or inside & SKIP_DIRS_IN_SRC):
            yield path


def test_line_mask(lines):
    """Marks lines inside `#[cfg(test)]` items by tracking brace depth.

    Braces inside strings can confuse it; acceptable for a heuristic.
    """
    mask = [False] * len(lines)
    i = 0
    while i < len(lines):
        if not RE_CFG_TEST.match(lines[i]):
            i += 1
            continue
        depth, opened, j = 0, False, i
        while j < len(lines):
            mask[j] = True
            code = lines[j].split("//", 1)[0]
            depth += code.count("{") - code.count("}")
            opened = opened or "{" in code
            if (opened and depth <= 0) or (not opened and code.rstrip().endswith(";")):
                break
            j += 1
        i = j + 1
    return mask


def scan_file(root: Path, path: Path, is_library: bool):
    findings = []
    rel = path.relative_to(root)
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    mask = test_line_mask(lines)
    outside, inside = split_at_src(rel)
    production = not (
        outside & NON_PRODUCTION_DIRS or inside & NON_PRODUCTION_DIRS_IN_SRC
    ) and path.stem not in TEST_FILE_STEMS
    library_source = (
        is_library
        and production
        and path.name not in ("main.rs", "build.rs")
        and "bin" not in rel.parts
    )

    def add(severity, rule, index, message):
        findings.append(Finding(severity, rule, rel, index + 1, message))

    in_block_comment = False
    for i, line in enumerate(lines):
        stripped = line.strip()

        # Block comments (including `/*! ... */` crate docs) count as
        # comments. Assumes `/*` and `*/` are not mixed with code on a line.
        starts_block = stripped.startswith("/*")
        is_block_line = in_block_comment or starts_block
        if starts_block and "*/" not in stripped[2:]:
            in_block_comment = True
        elif in_block_comment and "*/" in stripped:
            in_block_comment = False

        if stripped.startswith("//") or is_block_line:
            if RE_BOX_DRAWING.search(line):
                add("Minor", "CMT-5", i, "ASCII-art or box-drawing characters in a comment; describe the structure in sentences.")
            if RE_JOURNAL.search(line):
                add("Minor", "DOC-11", i, "reads like design history; document current behavior instead.")
            if RE_PARAM_TABLE.match(line):
                add("Minor", "DOC-4", i, "parameter table in docs; describe parameters in prose.")
            if RE_BARE_TODO.search(line):
                add("Note", "CMT-3", i, "TODO/FIXME without an owner or issue reference, e.g. TODO(#123).")
            continue

        code = line.split("//", 1)[0]

        if RE_GLOB_PUB_USE.match(code):
            add("Major", "ORG-12", i, "glob re-export makes the public surface implicit; re-export items by name.")

        m = RE_TYPE_DECL.match(code)
        if m:
            name = m.group(2)
            word = next((w for w in WEASEL_WORDS if name.endswith(w) and name != w), None)
            if word:
                add("Minor", "NAME-5", i, f"type `{name}` ends in weasel word `{word}`; name what it does or holds (an established ecosystem trait such as tower's `Service` is an exception).")

        if mask[i] or not production:
            continue

        if RE_EMPTY_EXPECT.search(code):
            add("Major", "PANIC-2", i, "expect with an empty message; state the invariant that makes failure impossible.")
        if library_source and RE_PRINT.search(code):
            add("Minor", "PANIC-6", i, "print macro in library code; return data or use logging.")

    return findings


def main(argv):
    if len(argv) != 2:
        print("usage: audit_scan.py <crate-root>", file=sys.stderr)
        return 2
    root = Path(argv[1]).resolve()
    if not (root / "Cargo.toml").is_file():
        print(f"error: no Cargo.toml in {root}", file=sys.stderr)
        return 2

    files = list(rust_files(root))
    is_library = any(p.name == "lib.rs" for p in files)
    findings = [f for path in files for f in scan_file(root, path, is_library)]

    if not findings:
        print("No heuristic findings.")
        return 0

    for severity in SEVERITIES:
        group = [f for f in findings if f.severity == severity]
        if not group:
            continue
        print(f"## {severity} candidates ({len(group)})")
        print()
        for f in group:
            print(f"{f.rule} {f.path}:{f.line}")
            print(f"  {f.message}")
        print()

    counts = ", ".join(f"{s}: {sum(f.severity == s for f in findings)}" for s in SEVERITIES)
    print(f"Totals: {counts}")
    print("Confirm each finding by reading the code; discard false positives.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

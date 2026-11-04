#!/usr/bin/env python3
"""Scan the files git would commit under a folder for private data. Exits 1 on any hit.

Usage: privacy_scan.py <folder>

Patterns: email addresses, tokens and keys, this machine's home and temp paths, session ids, and
canary tokens. The path patterns are built from pieces so this file does not match itself.
"""

import pathlib
import re
import subprocess
import sys

HOME_PATHS = ["/" + "root", "/" + "home/user", "/" + "tmp/claude-0"]
PATTERNS = {
    "email": re.compile(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"),
    "token": re.compile(
        r"sk-ant-[A-Za-z0-9_-]+|gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]+"
        r"|AKIA[0-9A-Z]{16}|Bearer\s+[A-Za-z0-9._-]{20,}"
        r"|(?i:api[_-]?key|secret|password)\s*[=:]\s*\S{8,}"
    ),
    "home path": re.compile("|".join(re.escape(p) + r"\b" for p in HOME_PATHS)),
    "session id": re.compile(
        r"session_[A-Za-z0-9]{10,}|\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b"
    ),
    "canary": re.compile("CANARY" + r"-[0-9a-f]{16}"),
}
ALLOW = re.compile(r"noreply@anthropic\.com")


def main():
    listed = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", sys.argv[1]],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split("\0")
    hits = 0
    for path in sorted(pathlib.Path(x) for x in listed if x):
        if not path.is_file():
            continue
        for n, line in enumerate(path.read_text(errors="replace").splitlines(), 1):
            for name, pattern in PATTERNS.items():
                for match in pattern.finditer(line):
                    if ALLOW.fullmatch(match.group(0)):
                        continue
                    hits += 1
                    print(f"{path}:{n}: {name}: {match.group(0)[:80]}")
    print(f"hits={hits}")
    sys.exit(1 if hits else 0)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Redact Sol reports and log every dropped or changed line.

Incidental mentions of a gate/tool do not erase a finding. Suggested-fix text is
preserved as evidence after its format label is removed.
"""

import argparse
import json
import re
from pathlib import Path

HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")
VERIFICATION = re.compile(r"^(?:verification|verification commands|checks(?: run)?|validation)$",
                          re.IGNORECASE)
MENTIONS = re.compile(r"rust-qc|SKILL\.md|check\.sh|audit_scan\.py|\bskill(?:'s)?\b",
                      re.IGNORECASE)


def redact(text, *, preserve_verification=False):
    output = []
    edits = []
    verification_level = None
    fence = None
    for number, line in enumerate(text.splitlines(), 1):
        stripped = line.lstrip()
        fence_match = re.match(r"(`{3,}|~{3,})(.*)$", stripped)
        heading = HEADING.match(line) if fence is None else None
        if heading and verification_level is not None and len(heading[1]) <= verification_level:
            verification_level = None
        if heading and VERIFICATION.fullmatch(heading[2]) and not preserve_verification:
            verification_level = len(heading[1])
        if verification_level is not None:
            edits.append({"line": number, "action": "drop", "reason": "verification_section",
                          "before": line})
        else:
            cleaned = MENTIONS.sub("[tooling]", line)
            cleaned = re.sub(r"\bBlockers?\b", "High", cleaned)
            cleaned = re.sub(r"\bMajor\b", "Medium", cleaned)
            cleaned = re.sub(r"\bMinor\b", "Low", cleaned)
            cleaned = re.sub(r"^(\s*)(?:Location|Problem|Fix|Suggested fix):\s*",
                             r"\1", cleaned)
            if cleaned != line:
                edits.append({"line": number, "action": "replace",
                              "reason": "tool_or_format_marker", "before": line,
                              "after": cleaned})
            output.append(cleaned)
        if fence_match:
            marker = fence_match[1]
            if fence is None:
                fence = marker
            elif (marker[0] == fence[0] and len(marker) >= len(fence)
                  and not fence_match[2].strip()):
                fence = None
    return "\n".join(output) + ("\n" if text.endswith("\n") and output else ""), edits


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    parser.add_argument("redacted", type=Path)
    parser.add_argument("edit_log", type=Path)
    parser.add_argument("--preserve-verification", action="store_true",
                        help="Keep verification evidence while redacting tool names")
    args = parser.parse_args()
    text, edits = redact(args.report.read_text(),
                         preserve_verification=args.preserve_verification)
    args.redacted.parent.mkdir(parents=True, exist_ok=True)
    args.edit_log.parent.mkdir(parents=True, exist_ok=True)
    args.redacted.write_text(text)
    args.edit_log.write_text(json.dumps(edits, indent=2) + "\n")
    print(json.dumps({"dropped": sum(e["action"] == "drop" for e in edits),
                      "replaced": sum(e["action"] == "replace" for e in edits)}))


if __name__ == "__main__":
    main()

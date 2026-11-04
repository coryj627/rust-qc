#!/usr/bin/env python3
"""Redact one audit report for blind grading.

Adapted from evals/results/method/prep_grading.py.

Usage: redact.py <report.md> <run_id> <redacted.md> <dropped-lines.txt>

Changes from prep_grading.py, as issue 30 asks:
- Field labels are removed, including `Cause:` and `Evidence (ran):` or `Evidence (read):`, so
  fields read as plain text.
- `## QA hygiene` and `## Unverified` both become `## Other notes`, and "QA hygiene" in the text
  becomes "other notes".
- Every dropped line is written to dropped-lines.txt with its reason, so a lost finding can be
  checked.

The rest is unchanged: mentions of the skill and its scripts, the verification section, severity
counts and fixes are removed, and severity words map to one scale.
"""

import pathlib
import re
import sys

MENTIONS = re.compile(
    r"rust-qc|\bskill\b|\bskill's\b|check\.sh|audit_scan|SKILL\.md|\bthe gate\b|\bgate\b|\bbrief\b",
    re.IGNORECASE,
)
FIX = re.compile(r"\s*(?:[-*]\s*)?(?:\*\*)?(?:Fix|Suggested fix)(?:\*\*)?:(?:\*\*)?.*$")
COUNTS = re.compile(
    r"(?:(?:Findings|Counts)(?: per severity)?:?\s*)?\(?\b\d+ Blockers?\b[^.\n]*\.?"
)
LABEL = re.compile(
    r"^(\s*(?:[-*]\s+)?)(?:\*\*)?"
    r"(?:Location|Problem|Cause|Established(?: by)?|Evidence(?: \((?:ran|read)\))?"
    r"|What a (?:user|caller) sees)"
    r"(?:\*\*)?:(?:\*\*)?\s*"
)
OTHER = re.compile(r"^(#{1,3}\s+)(?:QA hygiene|Unverified)\b.*$")


def redact(text, run_id):
    """Return the redacted report and the dropped lines, each as (reason, line)."""
    text = re.sub(r"/\S*?/audits/projects/" + run_id, "<workspace>", text)
    lines = text.splitlines()
    first_heading = next((i for i, line in enumerate(lines) if line.startswith(("# ", "## "))), 0)
    dropped = [("before first heading", line) for line in lines[:first_heading] if line.strip()]
    out, skipping = [], False
    for line in lines[first_heading:]:
        if re.match(r"^#{1,3}\s+Verification\b", line):
            skipping = True
            dropped.append(("verification section", line))
            continue
        if skipping and re.match(r"^#{1,3}\s", line):
            skipping = False
        if skipping:
            if line.strip():
                dropped.append(("verification section", line))
            continue
        if MENTIONS.search(line):
            dropped.append(("mentions skill, gate or brief", line))
            continue
        if FIX.match(line):
            dropped.append(("fix", line))
            continue
        line = COUNTS.sub("", line)
        line = LABEL.sub(r"\1", line)
        line = OTHER.sub(r"\1Other notes", line)
        line = re.sub(r"\bQA hygiene\b", "other notes", line)
        line = re.sub(r"^(#{1,3}\s+)Verdict\b", r"\1Summary", line)
        line = re.sub(r"^### +(.*)$", r"**\1**", line)
        line = re.sub(r"^(#{1,3}\s+)Note\s*$", r"\1Low", line)
        line = re.sub(r"\bBlockers?\b", "High", line)
        line = re.sub(r"\bMajor\b", "Medium", line)
        line = re.sub(r"\bMinor\b", "Low", line)
        if line.strip():
            out.append(line)
        elif out and out[-1].strip():
            out.append("")
    return "\n".join(out).rstrip() + "\n", dropped


def main():
    source, run_id, target, dropped_path = sys.argv[1:5]
    redacted, dropped = redact(pathlib.Path(source).read_text(), run_id)
    pathlib.Path(target).write_text(redacted)
    with open(dropped_path, "w") as out:
        out.writelines(f"{reason}\t{line}\n" for reason, line in dropped)


if __name__ == "__main__":
    main()

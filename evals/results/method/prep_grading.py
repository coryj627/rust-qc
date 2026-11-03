#!/usr/bin/env python3
"""Prepare blind grading. Run under tools/venv.

Usage: prep_grading.py <scratch>

For each project: redacts the 4 returned reports and gives them random letters (audits/grading/<project>/report-X.md),
writes a trimmed key for key graders (audits/keygrade/<project>/key.yaml), and a fresh workspace with only the crate for
the verifier (audits/projects/v-<project>/). The letter map stays in audits/private/letters.tsv.

Redaction removes what names the arm or follows only from the skill's report format: mentions of the skill and its
scripts, the verification section, severity counts and suggested fixes. Severity words are mapped to one scale.
"""
import pathlib
import random
import re
import subprocess
import sys

import yaml

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from prep_audits import CLOCK, PROJECTS, RUN  # noqa: E402

MENTIONS = re.compile(r"rust-qc|\bskill\b|\bskill's\b|check\.sh|audit_scan|SKILL\.md|\bthe gate\b|\bgate\b|\bbrief\b", re.I)
FIX = re.compile(r"\s*(?:[-*]\s*)?(?:\*\*)?(?:Fix|Suggested fix)(?:\*\*)?:(?:\*\*)?.*$")
COUNTS = re.compile(r"(?:(?:Findings|Counts)(?: per severity)?:?\s*)?\(?\b\d+ Blockers?\b[^.\n]*\.?")
TAGLINE = re.compile(r"^\s*(?:Location|Problem|Established(?: by)?|Evidence|What a (?:user|caller) sees):\s*")


def redact(text, run_id):
    text = re.sub(r"/\S*?/audits/projects/" + run_id, "<workspace>", text)
    lines = text.splitlines()
    first_heading = next((i for i, line in enumerate(lines) if line.startswith("# ")), 0)
    out, skipping = [], False
    for line in lines[first_heading:]:
        if re.match(r"^#{1,3}\s+Verification\b", line):
            skipping = True
            continue
        if skipping and re.match(r"^#{1,3}\s", line):
            skipping = False
        if skipping or MENTIONS.search(line):
            continue
        line = FIX.sub("", COUNTS.sub("", line))
        line = TAGLINE.sub("", line)
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
    return "\n".join(out).rstrip() + "\n"


def main():
    scratch = pathlib.Path(sys.argv[1])
    audits = scratch / "audits"
    rng = random.SystemRandom()
    runs = {}
    for row in (audits / "private" / "map.tsv").read_text().splitlines():
        run_id, project, arm = row.split("\t")
        runs.setdefault(project, []).append(run_id)
    rows = []
    for project in PROJECTS:
        folder = audits / "grading" / project
        folder.mkdir(parents=True)
        ids = runs[project][:]
        rng.shuffle(ids)
        for letter, run_id in zip("ABCD", ids):
            text = (audits / "private" / "returned" / f"{run_id}.md").read_text()
            (folder / f"report-{letter}.md").write_text(redact(text, run_id))
            rows.append(f"{project}\t{letter}\t{run_id}\n")
        key = yaml.safe_load((scratch / "keys" / f"{project}.yaml").read_text())
        keep = ("id", "kind", "summary", "location", "confirmed_at_baseline", "match_rule", "caveat")
        trimmed = {
            "project": project,
            "audited_path": key["audited_path"],
            "bugs": [{k: b.get(k) for k in keep} for b in key["bugs"]],
        }
        keyfolder = audits / "keygrade" / project
        keyfolder.mkdir(parents=True)
        (keyfolder / "key.yaml").write_text(yaml.safe_dump(trimmed, sort_keys=False, width=110))
        workspace = audits / "projects" / f"v-{project}"
        subprocess.run(["/bin/cp", "-cR", str(audits / "projects" / f"tmpl-{project}"), str(workspace)], check=True)
        (workspace / "run").write_text(RUN)
        (workspace / "clock").write_text(CLOCK)
        (workspace / "run").chmod(0o755)
        (workspace / "clock").chmod(0o755)
    (audits / "private" / "letters.tsv").write_text("".join(rows))


if __name__ == "__main__":
    main()

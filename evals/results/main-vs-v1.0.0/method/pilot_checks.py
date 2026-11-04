#!/usr/bin/env python3
"""Save the pilot reports and measure them, and evaluate the preflight checks that need a pilot run.

Usage: pilot_checks.py <stage1-folder>

Reads private/map.tsv, private/agents.tsv and private/streams/ under /srv/rqc/audits. Writes
pilot/<arm>-<run>.md (paths shortened), pilot/<arm>-<run>.redacted.md and .dropped-lines.txt, and
prints one JSON object with the pilot rows and the check results.
"""

import json
import pathlib
import re
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import AUDITS
from redact import redact
from streams import events, summary, tool_calls


def last_assistant_text(stream):
    text = None
    for event in events(stream):
        message = event.get("message")
        if event.get("type") != "assistant" or not isinstance(message, dict):
            continue
        parts = [b.get("text", "") for b in message.get("content") or [] if b.get("type") == "text"]
        if parts:
            text = "".join(parts)
    return text


def used_skill(stream):
    """Whether the agent read skill/SKILL.md (Read or a Bash cat that ran) and ran both scripts."""
    read_skill = ran_check = ran_scan = False
    for name, data, ran in tool_calls(stream):
        if not ran:
            continue
        command = data.get("command", "") if name == "Bash" else ""
        if name == "Read" and str(data.get("file_path", "")).endswith("/skill/SKILL.md"):
            read_skill = True
        if name == "Bash" and "cat skill/SKILL.md" in command:
            read_skill = True
        if name == "Bash" and command.lstrip().startswith("./run"):
            ran_check |= "check.sh" in command
            ran_scan |= "audit_scan.py" in command
    return read_skill, ran_check, ran_scan


def disk_mb(run_id):
    """Workspace size plus its build volume, in MB. A run that never built has no volume."""

    def du(path):
        return int(
            subprocess.run(
                ["du", "-sm", path], capture_output=True, text=True, check=True
            ).stdout.split()[0]
        )

    inspect = subprocess.run(
        ["docker", "volume", "inspect", f"rqc-target-{run_id}", "--format", "{{.Mountpoint}}"],
        capture_output=True,
        text=True,
        check=False,
    )
    mount = inspect.stdout.strip() if inspect.returncode == 0 else ""
    return du(str(AUDITS / "projects" / run_id)) + (du(mount) if mount else 0)


def minutes(run_id):
    log = AUDITS / "projects" / run_id / ".clock-log"
    rows = log.read_text().split() if log.exists() else []
    return int(rows[-1]) if rows else None


def main():
    out = pathlib.Path(sys.argv[1]) / "pilot"
    out.mkdir(parents=True, exist_ok=True)
    arms = dict(
        line.split("\t")[0::2] for line in (AUDITS / "private" / "map.tsv").read_text().splitlines()
    )
    rows, checks = [], {}
    for n, run_id in enumerate(sorted(arms, key=lambda r: arms[r] != "main"), 1):
        arm, stream = arms[run_id], AUDITS / "private" / "streams" / f"{run_id}.jsonl"
        s = summary(stream)
        report = (s["result"] or "").replace(str(AUDITS / "projects" / run_id), "<workspace>")
        (out / f"{arm}-{run_id}.md").write_text(report)
        redacted, dropped = redact(s["result"] or "", run_id)
        (out / f"{arm}-{run_id}.redacted.md").write_text(redacted)
        (out / f"{arm}-{run_id}.dropped-lines.txt").write_text(
            "".join(f"{r}\t{x}\n" for r, x in dropped)
        )
        headings = re.findall(r"^### +(.*)$", report, re.MULTILINE)
        read_skill, ran_check, ran_scan = used_skill(stream)
        rows.append(
            {
                "run": f"p{n}",
                "run_id": run_id,
                "arm": arm,
                "minutes": minutes(run_id),
                "report_words": len(report.split()),
                "disk_used_mb": disk_mb(run_id),
                "input_tokens": s["input_tokens"],
                "output_tokens": s["output_tokens"],
                "cache_read_tokens": s["cache_read_tokens"],
                "cache_write_tokens": s["cache_write_tokens"],
                "reported_cost_usd": s["reported_cost_usd"],
            }
        )
        checks[run_id] = {
            "arm": arm,
            "models": s["models"],
            "subtype": s["subtype"],
            "is_error": s["is_error"],
            "read_skill": read_skill,
            "ran_check_sh": ran_check,
            "ran_audit_scan": ran_scan,
            "cause_lines": len(re.findall(r"^\s*(?:\*\*)?Cause(?:\*\*)?:", report, re.MULTILINE)),
            "evidence_lines": len(
                re.findall(r"^\s*(?:\*\*)?Evidence \((?:ran|read)\)", report, re.MULTILINE)
            ),
            "finding_headings": len(headings),
            "headings_after_redaction": sum(f"**{h}**" in redacted for h in headings),
            "dropped_lines": len(dropped),
            "closing_summary": bool(re.search(r"^#{1,3}\s+Closing summary\b", report, re.M)),
            "closing_summary_removed": "Closing summary" not in redacted,
            "result_equals_last_text": (s["result"] or "") == (last_assistant_text(stream) or ""),
            "ends_with_clock": bool(
                re.search(r"\b\d+\s*(?:minutes?|min)\b[^\n]*$", (s["result"] or "").strip())
            ),
        }
    print(json.dumps({"pilot": rows, "checks": checks}, indent=2))


if __name__ == "__main__":
    main()

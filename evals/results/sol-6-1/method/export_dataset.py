#!/usr/bin/env python3
"""Export model-visible evidence, excluding profiles, sessions and reasoning.

This is an explicit allowlist export. Never copy private scratch directories into
the repository. Paths and credentials are scanned again after the export.
"""

import argparse
import filecmp
import json
import os
import re
from pathlib import Path

from check_runs import check, read_jsonl
from host_builds import violations


def scrub(text, scratch):
    text = text.replace(str(scratch), "<scratch>")
    text = re.sub(r"/Users/[^/\s\"'<>]+", "<host-user>", text)
    text = re.sub(r"/private/var/folders/[^\s\"'<>]+", "<host-temp>", text)
    text = re.sub(r"\b[\w.+-]+@[\w.-]+\.[A-Za-z]{2,}\b", "<email>", text)
    text = re.sub(r"\b(?:sk-|gh[pousr]_)[A-Za-z0-9_-]{16,}", "<token>", text)
    text = re.sub(r"\beyJ[A-Za-z0-9_-]{16,}\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
                  "<token>", text)
    return text


def publish(source, target, scratch):
    target.parent.mkdir(parents=True, exist_ok=True)
    text = source.read_text()
    clean = scrub(text, scratch)
    target.write_text(clean)
    return {"file": str(target.name), "scrubbed": clean != text}


def workspace_files(workspace):
    """Do not follow agent-created links out into the exporter's host filesystem."""
    excluded = {"target", ".cargo-home", ".git", "__pycache__", ".pytest_cache"}
    if workspace.is_symlink():
        return
    resolved = workspace.resolve()
    for directory, folders, files in os.walk(workspace, followlinks=False):
        parent = Path(directory)
        folders[:] = [name for name in folders
                      if name not in excluded and not (parent / name).is_symlink()]
        for name in files:
            path = parent / name
            if (not path.is_symlink() and path.is_file()
                    and path.resolve().is_relative_to(resolved)):
                yield path, path.relative_to(workspace)


def export_artifacts(scratch, run_id, output, template=None):
    workspace = scratch / "audits/projects" / run_id
    copied, excluded = [], []
    for path, relative in workspace_files(workspace):
        if relative.parts[0] in {"skill", ".agents"} or relative.name in {
            ".budget.json", "clock", "run", ".clock-start", ".clock-log", ".waits",
        }:
            continue
        if template is not None:
            original = template / relative
            if (original.is_file() and not original.is_symlink()
                    and filecmp.cmp(original, path, shallow=False)):
                continue
        target = output / "artifacts" / run_id / relative
        if path.stat().st_size > 10_000_000:
            excluded.append({"path": str(relative), "reason": "over_10MB_regenerate_from_probe"})
            continue
        data = path.read_bytes()
        if data.startswith((b"\x7fELF", b"\xcf\xfa\xed\xfe", b"MZ", b"!<arch>\n")):
            excluded.append({"path": str(relative), "reason": "compiled_binary_or_core"})
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        try:
            target.write_text(scrub(data.decode("utf-8"), scratch))
        except UnicodeDecodeError:
            target.write_bytes(data)
        copied.append(str(relative))
    deleted = []
    if template is not None:
        deleted = [str(relative) for _, relative in workspace_files(template)
                   if not (workspace / relative).exists()]
    return {"copied": copied, "excluded": excluded, "deleted_from_template": deleted}


def export(scratch, run_id, output, validity):
    private = scratch / "private" / run_id
    metadata = json.loads((private / "metadata.json").read_text())
    review_path = private / "input_review.json"
    if review_path.exists():
        review = json.loads(review_path.read_text())
        metadata["operator_input_review"] = review
        if not review["valid"]:
            validity = "invalid-other-source-skill"
    metadata["validity"] = validity
    metadata.pop("thread_id", None)
    folder = output / "raw" / run_id
    folder.mkdir(parents=True, exist_ok=True)
    (folder / "metadata.json").write_text(scrub(json.dumps(metadata, indent=2), scratch) + "\n")
    removed = {"reasoning": 0, "transport_events": 0}
    with (folder / "transcript.jsonl").open("w") as stream:
        for event in read_jsonl(private / "events.jsonl"):
            if event.get("method") != "item/completed":
                removed["transport_events"] += 1
                continue
            item = event["params"]["item"]
            if item["type"] == "reasoning":
                removed["reasoning"] += 1
                continue
            if item["type"] in {"userMessage", "agentMessage", "dynamicToolCall"}:
                item.pop("id", None)
                stream.write(scrub(json.dumps(item), scratch) + "\n")
    changes = []
    for name in ["prompt.txt", "developer.txt", "commands.jsonl", "input_review.json"]:
        if (private / name).exists():
            changes.append(publish(private / name, folder / name, scratch))
    for path in private.glob("command-*.log"):
        changes.append(publish(path, folder / path.name, scratch))
    changes.append(publish(private / "report.md", output / "reports" / f"{run_id}.md", scratch))
    checks = check(private)
    checks["host_execution_violations"] = violations(private)
    (folder / "checks.json").write_text(json.dumps(checks, indent=2) + "\n")
    return {"run_id": run_id, "removed_event_counts": removed,
            "files_changed_by_scrubbing": [c["file"] for c in changes if c["scrubbed"]]}


def scan(output):
    patterns = {
        "host_path": re.compile(r"/Users/|/private/(?:tmp|var/folders)/"),
        "email": re.compile(r"\b[\w.+-]+@[\w.-]+\.[A-Za-z]{2,}\b"),
        "token": re.compile(r"\b(?:sk-|gh[pousr]_)[A-Za-z0-9_-]{16,}"),
        "account_field": re.compile(r'"(?:access_token|refresh_token|account_id|id_token)"'),
        "path_environment": re.compile(r"(?:^|[\s\"'])PATH=[^\n]+", re.MULTILINE),
        "personal_name": re.compile(r"\bcoryj?\b", re.IGNORECASE),
    }
    hits = []
    for folder in [output / name for name in ("raw", "reports", "artifacts", "controls", "stage5")]:
        for path in folder.rglob("*"):
            if path.is_file():
                text = path.read_bytes().decode("utf-8", errors="replace")
                for name, pattern in patterns.items():
                    for match in pattern.finditer(text):
                        hits.append({"file": str(path.relative_to(output)), "kind": name,
                                     "line": text.count("\n", 0, match.start()) + 1})
    return hits


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scratch", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("run_ids", nargs="+")
    parser.add_argument("--validity", default="valid")
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    exports = [export(scratch, run, args.output, args.validity) for run in args.run_ids]
    hits = scan(args.output)
    record = {"exports": exports, "scan_hits": hits,
              "excluded": ["authentication", "profile", "raw session events", "reasoning",
                           "server stderr", "transport deltas", "thread identifiers"]}
    report = args.output / "export-scans"
    report.mkdir(exist_ok=True)
    (report / f"{args.run_ids[0]}.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps({"exported": len(exports), "scan_hits": hits}, indent=2))
    raise SystemExit(int(bool(hits)))


if __name__ == "__main__":
    main()

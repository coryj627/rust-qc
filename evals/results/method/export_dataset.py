#!/usr/bin/env python3
"""Export the evaluation's raw data into evals/results/raw/, with private context removed.

Usage: export_dataset.py <session folder holding scratchpad/ and tasks/> <evals/results/raw>

Transcripts keep only what each agent said and did: text, tool calls and tool results. Dropped: thinking, request records,
harness attachments and system reminders, which carry the host's user context and tool lists. Local paths, the host's
PATH and the session id are replaced by placeholders throughout.

Probes are the source and logs each agent left outside its crate, up to 256 KiB a file. Build output, whole-crate copies,
generated inputs and cargo homes are not exported. Every file an agent wrote by a tool call is also in its transcript.
"""
import collections
import gzip
import json
import os
import pathlib
import re
import sys

REMINDER = re.compile(r"<system-reminder>.*?</system-reminder>\s*", re.S)
PATH_LIST = re.compile(r"(?:/[^\s:\"'\\]+:){3,}/[^\s:\"'\\]+")
PATH_VALUE = re.compile(r"\b(\w*PATH)=([^\"\\\n]*)")
HOST_PATH = re.compile(r"<home>|/opt/homebrew")
SKIP_DIRS = {"crate", "skill", "target", ".cargo-home", ".git", "debug", "release", "deps", "incremental", ".fingerprint"}
PROBE_SUFFIXES = {".rs", ".toml", ".lock", ".sh", ".py", ".just", ".log", ".out"}
PROBE_NAMES = {"justfile", "Justfile"}
PROBE_LIMIT = 256 * 1024


def scrubber(session):
    """Return a function that replaces this machine's paths with placeholders."""
    scratch = str(session / "scratchpad")
    pairs = [(scratch, "<scratch>"), (scratch.replace("/private/tmp", "/tmp"), "<scratch>"), (str(session), "<session>")]
    home = str(pathlib.Path.home())
    pairs += [(home + "/Dev/rust-qc", "<repo>"), (home, "<home>"), (pathlib.Path.home().name, "<user>")]
    pairs.append((session.name, "<session-id>"))

    def scrub(text):
        for old, new in pairs:
            text = text.replace(old, new)
        text = PATH_VALUE.sub(lambda m: m[1] + "=<host-path>" if HOST_PATH.search(m[2]) else m[0], text)
        return PATH_LIST.sub(lambda m: "<host-path>" if HOST_PATH.search(m[0]) else m[0], text)

    return scrub


def clean_block(block, kinds):
    """Reduce one message block to its content, or None if it should go."""
    if isinstance(block, str):
        block = {"type": "text", "text": block}
    kind = block.get("type")
    kinds[kind] += 1
    if kind == "text":
        text = REMINDER.sub("", block.get("text", ""))
        return {"type": "text", "text": text} if text.strip() else None
    if kind == "tool_use":
        return {"type": "tool_use", "name": block.get("name"), "input": block.get("input")}
    if kind == "tool_result":
        content = block.get("content")
        if isinstance(content, list):
            content = [c for c in (clean_block(c, kinds) for c in content) if c]
        elif isinstance(content, str):
            content = REMINDER.sub("", content)
        return {"type": "tool_result", "is_error": bool(block.get("is_error")), "content": content}
    return None


def export_transcript(source, target, scrub, kinds):
    """Write one agent's cleaned transcript as gzipped JSON lines."""
    rows = []
    for line in source.read_text(errors="replace").splitlines():
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if record.get("type") not in ("user", "assistant"):
            continue
        content = record.get("message", {}).get("content")
        blocks = content if isinstance(content, list) else [content]
        cleaned = [b for b in (clean_block(b, kinds) for b in blocks if b) if b]
        if cleaned:
            rows.append({"time": record.get("timestamp"), "role": record["type"], "content": cleaned})
    body = "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in rows)
    target.parent.mkdir(parents=True, exist_ok=True)
    with open(target, "wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as out:
        out.write(scrub(body).encode())


def copy_text_or_bytes(source, target, scrub):
    """Copy one file, replacing local paths if it is text."""
    target.parent.mkdir(parents=True, exist_ok=True)
    data = source.read_bytes()
    try:
        target.write_text(scrub(data.decode()))
    except UnicodeDecodeError:
        target.write_bytes(data)


def main():
    session, raw = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    audits = session / "scratchpad" / "audits"
    private = audits / "private"
    scrub = scrubber(session)
    if raw.exists():
        sys.exit(f"{raw} already exists")

    for name in ("grades.json", "findings.json"):
        copy_text_or_bytes(private / name, raw / name, scrub)
    for folder, name in ((audits / "grading", "redacted-reports"), (audits / "keygrade", "grader-keys")):
        for source in sorted(folder.glob("*/*")):
            if source.is_file():
                copy_text_or_bytes(source, raw / name / source.relative_to(folder), scrub)

    kinds = collections.Counter()
    tasks = session / "tasks"
    roles = (
        ("agents.tsv", lambda row: f"audit/{row[0]}"),
        ("keygraders.tsv", lambda row: f"key-grader/{row[0]}-{row[1]}"),
        ("agents-verify.tsv", lambda row: f"verifier/{row[0]}"),
    )
    for table, name in roles:
        for line in (private / table).read_text().splitlines():
            row = line.split("\t")
            export_transcript(
                tasks / f"{row[-1]}.output", raw / "transcripts" / f"{name(row)}.jsonl.gz", scrub, kinds
            )

    skipped = 0
    for workspace in sorted((audits / "projects").iterdir()):
        if workspace.name.startswith("tmpl-"):
            continue
        for name in (".clock-log", ".waits"):
            if (workspace / name).is_file():
                copy_text_or_bytes(workspace / name, raw / "timing" / workspace.name / name.lstrip("."), scrub)
        for root, dirs, files in os.walk(workspace):
            if "CACHEDIR.TAG" in files or ("Cargo.toml" in files and any(f.startswith("LICENSE") for f in files)):
                dirs[:] = []
                continue
            dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
            for file in files:
                source = pathlib.Path(root) / file
                if source.suffix not in PROBE_SUFFIXES and file not in PROBE_NAMES:
                    continue
                if source.is_symlink() or source.stat().st_size > PROBE_LIMIT:
                    skipped += 1
                    continue
                copy_text_or_bytes(source, raw / "probes" / workspace.name / source.relative_to(workspace), scrub)

    print("block kinds seen:", dict(kinds))
    print("probe files skipped as links or over the limit:", skipped)
    print("exported bytes:", sum(p.stat().st_size for p in raw.rglob("*") if p.is_file()))


if __name__ == "__main__":
    main()

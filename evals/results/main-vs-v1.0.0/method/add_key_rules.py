#!/usr/bin/env python3
"""Copy each key to stage2/keys/ and add failure_rule and cause_rule to its reproduced entries.

Usage: add_key_rules.py <clone> <stage2-folder> <rules.json>

rules.json maps project -> entry id -> {"failure_rule": str, "cause_rule": str or null}, from the
key-rule agents. The two fields go on the lines right after the entry's `id:` line, as quoted
scalars, so the rest of the file is unchanged. After every insertion the file is parsed again and
checked: the new values read back exactly, and every entry's match_rule is unchanged.
"""

import json
import pathlib
import re
import sys

import yaml


def reproduced(stage2):
    rows = (stage2 / "reproducers.tsv").read_text().splitlines()[1:]
    return {(r.split("\t")[0], r.split("\t")[1]) for r in rows if r.split("\t")[4] == "reproduced"}


def scalar(value):
    return "null" if value is None else json.dumps(value, ensure_ascii=False)


def add(original, project, rules, done):
    text = original
    before = {b["id"]: b["match_rule"] for b in yaml.safe_load(original)["bugs"]}
    for bug_id, rule in rules.items():
        if (project, bug_id) not in done:
            continue
        match = re.search(rf"^(\s*)- id: {re.escape(bug_id)}\s*$", text, re.MULTILINE)
        if not match:
            raise SystemExit(f"{project}: no id line for {bug_id}")
        indent = match.group(1) + "  "
        insert = (
            f"\n{indent}failure_rule: {scalar(rule['failure_rule'])}"
            f"\n{indent}cause_rule: {scalar(rule['cause_rule'])}"
        )
        text = text[: match.end()] + insert + text[match.end() :]
        entries = {b["id"]: b for b in yaml.safe_load(text)["bugs"]}
        entry = entries[bug_id]
        if (
            entry["failure_rule"] != rule["failure_rule"]
            or entry["cause_rule"] != rule["cause_rule"]
        ):
            raise SystemExit(f"{project}: {bug_id} rules do not read back exactly")
        if any(entries[i]["match_rule"] != before[i] for i in before):
            raise SystemExit(f"{project}: a match_rule changed after inserting {bug_id}")
    return text


def main():
    clone, stage2, rules_path = map(pathlib.Path, sys.argv[1:4])
    rules = json.loads(rules_path.read_text())
    done = reproduced(stage2)
    (stage2 / "keys").mkdir(exist_ok=True)
    for project, project_rules in sorted(rules.items()):
        original = (clone / "evals" / "keys" / f"{project}.yaml").read_text()
        text = add(original, project, project_rules, done)
        (stage2 / "keys" / f"{project}.yaml").write_text(text)
        added = sum((project, i) in done for i in project_rules)
        print(f"{project}: rules added to {added} reproduced entries")


if __name__ == "__main__":
    main()

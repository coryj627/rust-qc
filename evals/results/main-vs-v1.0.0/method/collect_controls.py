#!/usr/bin/env python3
"""Validate each controls agent's answer and save it as stage2/controls/<project>.json.

Usage: collect_controls.py <stage2-folder> <project>...

An answer must be one controls/1 object with exactly 2 decoy bugs and 5 false findings, each with
every field filled. A sentence before or after the JSON is dropped, and the script says so.
"""

import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import AUDITS
from streams import summary

DECOY = ("id", "summary", "failure_rule", "cause_rule", "confirmed_false_by")
FALSE = ("id", "text", "confirmed_false_by")


def collect(stage2, project):
    text = summary(AUDITS / "private" / "streams" / f"c-{project}.jsonl")["result"] or ""
    body = text[text.index("{") : text.rindex("}") + 1]
    answer = json.loads(body)
    decoys, falses = answer["decoy_bugs"], answer["false_findings"]
    if answer.get("schema") != "controls/1" or len(decoys) != 2 or len(falses) != 5:
        raise SystemExit(f"{project}: wrong shape")
    for item, fields in [(d, DECOY) for d in decoys] + [(f, FALSE) for f in falses]:
        if not all(isinstance(item.get(k), str) and item[k].strip() for k in fields):
            raise SystemExit(f"{project}: {item.get('id')} has an empty or missing field")
    out = stage2 / "controls" / f"{project}.json"
    out.parent.mkdir(exist_ok=True)
    out.write_text(json.dumps(answer, indent=2, ensure_ascii=False) + "\n")
    extra = " (text around the JSON dropped)" if body != text.strip() else ""
    print(f"{project}: 2 decoys, 5 false findings{extra}")


def main():
    stage2 = pathlib.Path(sys.argv[1])
    for project in sys.argv[2:]:
        collect(stage2, project)


if __name__ == "__main__":
    main()

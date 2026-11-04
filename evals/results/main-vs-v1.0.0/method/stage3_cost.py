#!/usr/bin/env python3
"""Print the summed reported cost of Stage 3's agents so far, in USD.

Usage: stage3_cost.py <project>...

Counts each project's audits, graders, dispute agent and verifier that have a stream.
"""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from common import AUDITS
from stage3 import runs_of
from streams import summary

total = 0.0
for project in sys.argv[1:]:
    names = [*runs_of(project), f"g1-{project}", f"g2-{project}", f"d-{project}", f"v-{project}"]
    for name in names:
        path = AUDITS / "private" / "streams" / f"{name}.jsonl"
        if path.exists():
            total += summary(path)["reported_cost_usd"] or 0
print(f"{total:.2f}")

#!/usr/bin/env python3
"""Record one finished Stage 3 project in status.json.

Usage: stage3_status.py <results> <project> <agents-usd-so-far>
"""

import datetime
import json
import pathlib
import sys

results, project, spent = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
path = results / "status.json"
status = json.loads(path.read_text())
status["updated_at"] = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
if project not in status["done"]:
    status["done"].append(project)
status["remaining"] = [p for p in status["remaining"] if p != project]
status["usage"]["agents_reported_cost_usd"] = float(spent)
path.write_text(json.dumps(status, indent=2) + "\n")

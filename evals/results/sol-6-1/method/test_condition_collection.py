#!/usr/bin/env python3
"""A condition collector must not mix earlier runs or invent missing outcomes."""

import csv
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class ConditionCollectionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.scratch = self.root / "scratch"
        self.scratch.mkdir()
        self.output = self.root / "output"
        self.plan = self.root / "plan.json"
        self.plan.write_text(json.dumps([{
            "run_id": "selected", "condition": "review-v2", "role": "audit",
            "project": "example", "stage": 3, "arm": "baseline",
            "budget_minutes": 45, "skill": None,
        }]))

    def run_collector(self):
        result = subprocess.run([
            sys.executable, str(Path(__file__).with_name("collect_condition.py")),
            str(self.scratch), str(self.plan), "--output", str(self.output),
        ], capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        with (self.output / "runs.tsv").open() as stream:
            return list(csv.DictReader(stream, delimiter="\t"))

    def make_run(self, run, status):
        private = self.scratch / "private" / run
        private.mkdir(parents=True)
        (private / "metadata.json").write_text(json.dumps({
            "run_id": run, "status": status, "report_collected": status == "completed",
            "model": "gpt-6.1-sol", "effort": "high", "budget_seconds": 2700,
            "active_seconds": 60, "unexpected_requests": [], "command_timeouts": [],
        }))
        (private / "report.md").write_text("Candidate findings, not independently verified.\n")
        (private / "commands.jsonl").write_text("")
        (private / "events.jsonl").write_text("")
        return private

    def test_missing_run_has_no_measured_model_time_or_zero_score(self):
        rows = self.run_collector()
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["status"], "not_started")
        self.assertEqual(rows[0]["model"], "")
        self.assertEqual(rows[0]["minutes"], "")
        self.assertEqual(rows[0]["validity"], "missing-or-in-progress")

    def test_only_selected_completed_run_is_exported(self):
        self.make_run("selected", "completed")
        other = self.make_run("other-condition", "completed")
        (other / "auth.json").write_text("PRIVATE CANARY")
        rows = self.run_collector()
        self.assertEqual([r["run"] for r in rows], ["selected"])
        self.assertEqual(rows[0]["validity"], "complete")
        self.assertEqual(rows[0]["minutes"], "1.0")
        self.assertTrue((self.output / "reports/selected.md").exists())
        self.assertFalse((self.output / "raw/other-condition").exists())
        self.assertFalse(any("PRIVATE CANARY" in p.read_text() for p in self.output.rglob("*")
                             if p.is_file()))

    def test_live_partial_report_is_not_exported_as_completed(self):
        self.make_run("selected", "running")
        rows = self.run_collector()
        self.assertEqual(rows[0]["validity"], "missing-or-in-progress")
        self.assertFalse((self.output / "reports/selected.md").exists())


if __name__ == "__main__":
    unittest.main()

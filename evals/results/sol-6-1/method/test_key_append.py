#!/usr/bin/env python3
"""Protect original key bytes and YAML values when publishing new run rows."""

import csv
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import yaml
from append_key_runs import append


class KeyAppendTests(unittest.TestCase):
    def test_condition_rows_link_their_own_scores_and_state_eligible_bugs(self):
        with tempfile.TemporaryDirectory() as temporary:
            evaluation = Path(temporary) / "evals"
            method = evaluation / "results/sol-6-1/method"
            method.mkdir(parents=True)
            script = method / "append_key_runs.py"
            shutil.copyfile(Path(__file__).parent / script.name, script)
            key = evaluation / "keys/example.yaml"
            key.parent.mkdir()
            original = 'project: example\n# preserve prior rows\nruns:\n  - model: claude\n'
            key.write_text(original)
            condition = method.parent / "review-v2"
            condition.mkdir()
            row = {"run": "v2-run", "stage": "3", "role": "audit", "project": "example",
                   "validity": "complete", "status": "completed", "arm": "baseline",
                   "skill_version": "", "model": "gpt-6.1-sol", "effort": "high",
                   "budget_minutes": "45", "minutes": "12", "condition": "correctness-review-v2",
                   "report": "reports/v2-run.md"}
            with (condition / "runs.tsv").open("w") as stream:
                writer = csv.DictWriter(stream, fieldnames=list(row), delimiter="\t")
                writer.writeheader()
                writer.writerow(row)
            (condition / "key_scores.json").write_text(json.dumps([
                {"stage": 3, "project": "example", "bug": "eligible-1",
                 "runs": {"v2-run": "found"}},
            ]))
            result = subprocess.run([
                sys.executable, str(script), "--results-root", str(condition), "--apply",
            ], capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(key.read_text().startswith(original))
            added = yaml.safe_load(key.read_text())["runs"][-1]
            prefix = "results/sol-6-1/review-v2/"
            self.assertEqual(added["report"], prefix + row["report"])
            self.assertEqual(added["grading"], prefix + "key_scores.json")
            self.assertEqual(added["unredacted_recheck"], prefix + "key_recheck.json")
            self.assertEqual(added["condition"], row["condition"])
            self.assertEqual(added["eligible_bug_ids"], ["eligible-1"])

    def test_preserves_bytes_quotes_hashes_and_does_not_duplicate_runs(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "key.yaml"
            original = 'project: example\n# original comment\nruns:\n  - model: claude\n'
            path.write_text(original)
            entry = {"run_id": "sol-one", "model": "gpt-6.1-sol", "note": "value # intact",
                     "bugs_found": ["bug-1"], "bugs_partial": []}
            self.assertEqual(append(path, [entry], True), 1)
            self.assertTrue(path.read_text().startswith(original))
            self.assertEqual(yaml.safe_load(path.read_text())["runs"][-1], entry)
            self.assertEqual(append(path, [entry], True), 0)


if __name__ == "__main__":
    unittest.main()

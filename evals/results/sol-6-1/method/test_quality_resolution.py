#!/usr/bin/env python3
"""A disputed rubric score must remain unresolved until evidence adjudication."""

import json
import tempfile
import unittest
from pathlib import Path

from collect_grades import collect


class QualityResolutionTests(unittest.TestCase):
    def test_disagreement_is_not_averaged_into_a_final_grade(self):
        structures = json.loads((Path(__file__).resolve().parent.parent /
                                 "stage5/rubric-structure.json").read_text())
        criteria = [{"id": key, "max": value["max"], "score": value["max"], "evidence": key}
                    for key, value in structures["parser"]["quality_criteria"].items()]
        harm = {key: {"score": 0, "max": 2, "applicable": True, "evidence": "fixture"}
                for key in ("error_api", "dependencies", "structure")}
        first = {"candidate": "A", "criteria": criteria, "quality_total": 12,
                 "quality_interval": [12, 12], "harm": harm}
        second = json.loads(json.dumps(first))
        second["criteria"][0]["score"] = 0
        second.update(quality_total=11.5, quality_interval=[11.5, 11.5])
        with tempfile.TemporaryDirectory() as directory:
            scratch = Path(directory)
            private = scratch / "grading-private/s5-parser"
            private.mkdir(parents=True)
            (private / "map.json").write_text(json.dumps({"reports": {"A": "submission"}}))
            schedule = []
            for index, grade in enumerate((first, second), 1):
                run = f"grader-{index}"
                self.write_report(scratch, run, grade)
                schedule.append({"run_id": run, "grader": index, "candidates": ["A"]})
            (scratch / "quality-parser.json").write_text(json.dumps(schedule))
            output = scratch / "output"
            collect(scratch, output)
            row = json.loads((output / "quality_scores.json").read_text())[0]
            self.assertIsNone(row["final_quality"])
            self.assertEqual(row["resolution"], "unresolved")
            self.assertEqual(row["independent_mean_quality"], 11.75)
            self.write_report(scratch, "resolver", second)
            (scratch / "reconciliation-parser.json").write_text(json.dumps([
                {"run_id": "resolver", "candidates": ["A"]},
            ]))
            collect(scratch, output)
            row = json.loads((output / "quality_scores.json").read_text())[0]
            self.assertEqual(row["final_quality"], 11.5)
            self.assertEqual(row["resolution"], "adjudicated")
            self.assertEqual([g["quality_total"] for g in row["grades"]], [12, 11.5])

    @staticmethod
    def write_report(scratch, run, grade):
        private = scratch / "private" / run
        private.mkdir(parents=True)
        (private / "metadata.json").write_text('{"status":"completed"}\n')
        (private / "report.md").write_text(json.dumps(grade) + "\n")


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Ensure incomplete grading cannot become a score and controls cannot become defects."""

import tempfile
import unittest
from pathlib import Path

from collect_grades import decom_harm, key_sheet, objects, quality_sheet, split_verifier


class CollectionTests(unittest.TestCase):
    def test_categorical_harm_keeps_unknown_and_additional_checks_separate(self):
        states = {"error_api": "not_applicable", "runtime_dependencies": "absent",
                  "test_build_dependencies": "unverified", "modules": "absent",
                  "unrelated_edits": "absent", "suppressed_evidence_or_regressions": "present"}
        encoded = decom_harm([{"id": key, "state": state, "evidence": key}
                              for key, state in states.items()])
        self.assertFalse(encoded["error_api"]["applicable"])
        self.assertIsNone(encoded["dependencies"]["score"])
        self.assertEqual(encoded["structure"]["score"], 0)
        states["runtime_dependencies"] = "present"
        encoded = decom_harm([{"id": key, "state": state, "evidence": key}
                              for key, state in states.items()])
        self.assertEqual(encoded["dependencies"]["score"], 1)

    def test_multiline_json_and_final_clock_preserve_exact_result_objects(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.md"
            path.write_text('```json\n{\n  "candidate": "A",\n  "score": 12\n}\n'
                            '{"candidate":"B","score":11}\n```\nClock: 10 seconds\n')
            self.assertEqual(objects(path), [{"candidate": "A", "score": 12},
                                             {"candidate": "B", "score": 11}])

    def test_malformed_json_cannot_silently_drop_a_score(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.md"
            path.write_text('{"candidate": "A"\n')
            with self.assertRaisesRegex(ValueError, "malformed"):
                objects(path)

    def test_scores_for_absent_reports_are_rejected(self):
        row = {"bug": "K001", "A": {"v": "found"}, "B": {"v": "missed"}}
        with self.assertRaisesRegex(ValueError, "Unexpected or missing report labels"):
            key_sheet([row], {"K001": {}}, {"A": "run-a"})

    def test_missing_key_score_is_rejected_not_filled_as_missed(self):
        rows = [{"bug": "K001", "A": {"v": "found"}}]
        with self.assertRaisesRegex(ValueError, "Incomplete key sheet"):
            key_sheet(rows, {"K001": {}, "K002": {}}, {"A": "run-a"})

    def test_confirmed_false_control_is_excluded_even_if_verifier_believes_it(self):
        passages = {"P1": {"report": "A"}, "P2": {"report": "B"}}
        controls = {"P2": {"id": "false-1"}}
        rows = [{"item": "real claim", "sources": ["P1"], "status": "read", "severity": "medium"},
                {"item": "false claim", "sources": ["P2"], "status": "ran", "severity": "high"}]
        real, outcomes = split_verifier(rows, passages, controls)
        self.assertEqual([r["item"] for r in real], ["real claim"])
        self.assertFalse(outcomes[0]["refuted"])

    def test_mixed_control_is_flagged_and_cannot_add_report_membership(self):
        passages = {"P1": {"report": "A"}, "P2": {"report": "B"}}
        rows = [{"item": "mixed", "sources": ["P1", "P2"], "status": "read", "severity": "medium"}]
        real, _ = split_verifier(rows, passages, {"P2": {"id": "false-1"}})
        self.assertEqual(real[0]["reports"], "A")
        self.assertTrue(real[0]["control_mixed_with_real"])

    def test_quality_total_must_agree_with_criteria(self):
        row = {"candidate": "A", "criteria": [{"score": 5, "max": 12}],
               "quality_total": 12, "quality_interval": [12, 12]}
        with self.assertRaisesRegex(ValueError, "arithmetic"):
            quality_sheet([row], {"A": "run-a"})

    def test_duplicate_criteria_cannot_satisfy_total(self):
        row = {"candidate": "A", "criteria": [{"id": "same", "score": 6, "max": 6}] * 2,
               "quality_total": 12, "quality_interval": [12, 12]}
        with self.assertRaisesRegex(ValueError, "duplicate"):
            quality_sheet([row], {"A": "run-a"})

    def test_negative_criterion_cannot_cancel_overcredit(self):
        row = {"candidate": "A", "criteria": [
            {"id": "one", "score": -2, "max": 6}, {"id": "two", "score": 14, "max": 6}],
            "quality_total": 12, "quality_interval": [12, 12]}
        with self.assertRaisesRegex(ValueError, "criterion score"):
            quality_sheet([row], {"A": "run-a"})

    def test_unknown_harm_is_preserved_separately_from_not_applicable(self):
        row = {"candidate": "A", "criteria": [{"id": "all", "score": 12, "max": 12}],
               "quality_total": 12, "quality_interval": [12, 12],
               "harm": {"error_api": {"score": None, "max": None, "applicable": False},
                        "dependencies": {"score": None, "max": 3, "applicable": True},
                        "structure": {"score": 0, "max": 3, "applicable": True}}}
        result = quality_sheet([row], {"A": "run-a"})
        self.assertIsNone(result["A"]["harm"]["dependencies"]["score"])
        self.assertTrue(result["A"]["harm"]["dependencies"]["applicable"])


if __name__ == "__main__":
    unittest.main()

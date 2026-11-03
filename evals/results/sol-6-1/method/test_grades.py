#!/usr/bin/env python3
"""Ensure incomplete grading cannot become a score and controls cannot become defects."""

import unittest

from collect_grades import key_sheet, quality_sheet, split_verifier


class CollectionTests(unittest.TestCase):
    def test_missing_key_score_is_rejected_not_filled_as_missed(self):
        rows = [{"bug": "K001", "A": {"v": "found"}}]
        with self.assertRaisesRegex(ValueError, "Incomplete key sheet"):
            key_sheet(rows, {"K001": {}, "K002": {}}, {"A": "run-a"})

    def test_confirmed_false_control_is_excluded_even_if_verifier_believes_it(self):
        passages = {"P1": {"report": "A"}, "P2": {"report": "B"}}
        controls = {"P2": {"id": "false-1"}}
        rows = [{"item": "real claim", "sources": ["P1"], "status": "read"},
                {"item": "false claim", "sources": ["P2"], "status": "ran"}]
        real, outcomes = split_verifier(rows, passages, controls)
        self.assertEqual([r["item"] for r in real], ["real claim"])
        self.assertFalse(outcomes[0]["refuted"])

    def test_mixed_control_is_flagged_and_cannot_add_report_membership(self):
        passages = {"P1": {"report": "A"}, "P2": {"report": "B"}}
        rows = [{"item": "mixed", "sources": ["P1", "P2"], "status": "read"}]
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

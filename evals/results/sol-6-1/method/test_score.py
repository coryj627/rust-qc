#!/usr/bin/env python3
"""Check complete-cell inference and the unchanged exact sign-flip calculation."""

import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "sol_score", Path(__file__).resolve().parents[1] / "score.py",
)
score = importlib.util.module_from_spec(spec)
spec.loader.exec_module(score)


class ScoreTests(unittest.TestCase):
    def setUp(self):
        self.runs = [
            {"run": run, "project": "fixture", "stage": "3", "role": "audit", "arm": arm}
            for run, arm in [("a", "baseline"), ("b", "baseline"),
                             ("c", "with-skill"), ("d", "with-skill")]
        ]

    def test_missing_run_is_not_zero_or_a_one_replicate_comparison(self):
        result = score.compare({"a": 5, "b": 7, "c": 9}, self.runs, 3)
        self.assertEqual(result["projects"], [])
        self.assertIsNone(result["exact_sign_flip_p_uncorrected"])
        self.assertEqual(result["excluded_incomplete_projects"], ["fixture"])

    def test_gap_is_reported_with_both_replicate_differences(self):
        result = score.compare({"a": 5, "b": 7, "c": 8, "d": 12}, self.runs, 3)
        self.assertEqual(result["mean_project_difference"], 4)
        self.assertEqual(result["mean_within_arm_absolute_difference"], 3)
        self.assertEqual(result["projects"][0]["within_arm_absolute_differences"],
                         {"baseline": 2, "with-skill": 4})
        self.assertEqual(result["exact_sign_flip_p_uncorrected"], 1)

    def test_original_exact_two_project_result(self):
        self.assertEqual(score.exact_sign_flip([1, 1]), 0.5)
        self.assertEqual(score.exact_sign_flip([0, 0]), 1)

    def test_harm_uses_rubric_maximum_and_excludes_unknown(self):
        def grade(value, maximum, applicable=True):
            return {"harm": {"dependencies": {
                "score": value, "max": maximum, "applicable": applicable}}}

        quality = [{"run": "a", "grades": [grade(1, 2), grade(2, 2)]},
                   {"run": "b", "grades": [grade(1.5, 3), grade(3, 3)]},
                   {"run": "unknown", "grades": [grade(None, 3), grade(0, 3)]},
                   {"run": "na", "grades": [grade(None, None, False)] * 2}]
        values, raw = score.harm_values(quality, "dependencies")
        self.assertEqual(values, {"a": 0.75, "b": 0.75})
        self.assertEqual(raw["b"][1], {"score": 3, "max": 3})


if __name__ == "__main__":
    unittest.main()

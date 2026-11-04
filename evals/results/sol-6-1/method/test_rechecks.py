#!/usr/bin/env python3
"""Keep the random recheck sample reproducible without filling empty populations."""

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from prepare_rechecks import sample_scores


class RecheckTests(unittest.TestCase):
    def test_sample_is_ceiling_ten_percent_without_replacement(self):
        population = list(range(56))
        sample = sample_scores(population, 123)
        self.assertEqual(len(sample), 6)
        self.assertEqual(len(set(sample)), 6)
        self.assertEqual(sample, sample_scores(population, 123))

    def test_no_scores_produce_no_recheck_instead_of_a_fake_observation(self):
        self.assertEqual(sample_scores([], 123), [])

    def test_explicit_condition_output_does_not_overwrite_original_study(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            method = root / "method"
            method.mkdir()
            for name in ("collect_rechecks.py", "collect_grades.py"):
                shutil.copyfile(Path(__file__).parent / name, method / name)
            original = root / "key_recheck.json"
            original.write_text('{"prior_study":"preserve"}\n')
            scratch = root / "scratch"
            scratch.mkdir()
            (scratch / "recheck-selection.json").write_text(json.dumps({
                "population_size": 0, "sample_size": 0, "selected": [],
            }))
            output = root / "new-condition.json"
            result = subprocess.run([
                sys.executable, str(method / "collect_rechecks.py"), str(scratch),
                "--output", str(output),
            ], capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(original.read_text(), '{"prior_study":"preserve"}\n')
            self.assertEqual(json.loads(output.read_text())["judgments_rechecked"], 0)
            self.assertIsNone(json.loads(output.read_text())["disagreement_rate"])


if __name__ == "__main__":
    unittest.main()

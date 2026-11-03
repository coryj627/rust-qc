#!/usr/bin/env python3
"""A DECOM grader must not receive other submissions or their result records."""

import json
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from quality_inputs import provision


class QualityInputTests(unittest.TestCase):
    def test_selected_candidate_is_the_only_candidate_in_every_input_surface(self):
        with tempfile.TemporaryDirectory() as directory:
            scratch = Path(directory)
            source = scratch / "audits/projects/assess-s5-decom"
            (source / "prepared").mkdir(parents=True)
            (source / "prepared/case.md").write_text("frozen task")
            (source / "reports").mkdir()
            for label in "AB":
                (source / "candidates" / label).mkdir(parents=True)
                (source / "candidates" / label / "source.txt").write_text(label)
                (source / "reports" / f"{label}.md").write_text(label)
                (source / "results" / label).mkdir(parents=True)
                (source / "results" / label / "check.log").write_text(label)
            (source / "results/assessment.json").write_text(json.dumps({
                "case": "decom", "candidates": {"A": {"outcome": "a"}, "B": {"outcome": "b"}},
            }))

            def clone(command, **_kwargs):
                shutil.copytree(command[-2], command[-1])

            with patch("quality_inputs.subprocess.run", side_effect=clone):
                work = provision(scratch, "decom", "grade-only-a", ["A"])
            self.assertEqual({p.name for p in (work / "candidates").iterdir()}, {"A"})
            self.assertEqual({p.name for p in (work / "reports").iterdir()}, {"A.md"})
            self.assertFalse((work / "results/B").exists())
            result = json.loads((work / "results/assessment.json").read_text())
            self.assertEqual(set(result["candidates"]), {"A"})


if __name__ == "__main__":
    unittest.main()

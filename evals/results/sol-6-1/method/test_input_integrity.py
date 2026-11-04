#!/usr/bin/env python3
"""A changed runtime asset must fail the check against the frozen case commit."""

import shutil
import tempfile
import unittest
from pathlib import Path

from input_integrity import verify


class InputIntegrityTests(unittest.TestCase):
    def test_frozen_assets_pass_and_changed_spec_is_rejected(self):
        root = Path(__file__).resolve().parent.parent
        with tempfile.TemporaryDirectory() as temporary:
            scratch = Path(temporary)
            work = scratch / "audits/projects/assess-s5-parser"
            prepared = work / "prepared"
            prepared.mkdir(parents=True)
            for name in ("case.md", "rubric.md", "scoring.md"):
                shutil.copyfile(root / "stage5/cases/parser" / name, prepared / name)
            shutil.copytree(root / "stage5/cases/parser/hidden", prepared / "hidden")
            (work / "gate").mkdir()
            (work / "gate/check.sh").write_text("archived gate fixture")
            skill = scratch / "skill-tree/.agents/skills/rust-qc/scripts"
            skill.mkdir(parents=True)
            (skill / "check.sh").write_text("archived gate fixture")
            (scratch / "grading-private/s5-parser").mkdir(parents=True)
            self.assertTrue(verify(scratch, "parser")["runtime_assets_match_frozen_commit"])
            (prepared / "case.md").write_text("changed after assessment")
            with self.assertRaisesRegex(ValueError, "input changed"):
                verify(scratch, "parser")


if __name__ == "__main__":
    unittest.main()

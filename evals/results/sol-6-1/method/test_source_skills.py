#!/usr/bin/env python3
"""Source-bundled skills must not bypass a clean profile's catalog isolation."""

import tempfile
import unittest
from pathlib import Path

from source_skills import require_sole_skill


class SkillBoundaryTests(unittest.TestCase):
    def test_rejects_an_uncatalogued_source_skill(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            (work / "skill").mkdir()
            (work / "skill/SKILL.md").write_text("tested skill")
            require_sole_skill(work, "skill")
            other = work / "crate/skills/other"
            other.mkdir(parents=True)
            (other / "SKILL.md").write_text("not allowed")
            with self.assertRaisesRegex(ValueError, "Other skill files"):
                require_sole_skill(work, "skill")

    def test_no_skill_arm_rejects_even_a_tested_skill_copy(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            (work / "SKILL.md").write_text("unexpected")
            with self.assertRaises(ValueError):
                require_sole_skill(work)


if __name__ == "__main__":
    unittest.main()

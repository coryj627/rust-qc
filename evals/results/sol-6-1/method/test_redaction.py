#!/usr/bin/env python3
"""Guard against evidence loss from report format differences."""

import unittest

from prep_grading import redact


class RedactionTests(unittest.TestCase):
    def test_tool_mention_does_not_remove_a_finding(self):
        result, changes = redact("The gate calls check.sh, but parse() still overflows.\n")
        self.assertIn("parse() still overflows", result)
        self.assertEqual(changes[0]["action"], "replace")

    def test_verification_drops_are_all_logged_and_next_finding_survives(self):
        text = "# Findings\nPanic on empty input.\n## Verification\ncargo test: pass\n"
        text += "## Another defect\nA safe API corrupts data.\n"
        result, changes = redact(text)
        self.assertIn("A safe API corrupts data.", result)
        self.assertEqual([e["line"] for e in changes if e["action"] == "drop"], [3, 4])

    def test_hashes_inside_code_are_not_verification_headings(self):
        text = "# Finding\n```python\n# Verification\nraise Exception('bug')\n```\n"
        result, changes = redact(text)
        self.assertEqual(result, text)
        self.assertEqual(changes, [])

    def test_fix_evidence_is_preserved(self):
        result, _ = redact("Fix: preserve the probe chain after truncate.\n")
        self.assertIn("preserve the probe chain", result)

    def test_blank_lines_and_long_fence_are_preserved(self):
        text = "````markdown\n```\n# Verification\nevidence\n````\n\n\n"
        result, changes = redact(text)
        self.assertEqual(result, text)
        self.assertEqual(changes, [])

    def test_preserved_verification_keeps_regression_evidence_but_hides_tool_name(self):
        text = ("# Findings\nIncorrect boundary result.\n## Verification\n"
                "check.sh completed; the boundary probe returned 600 instead of 599.\n")
        clean, edits = redact(text, preserve_verification=True)
        self.assertIn("the boundary probe returned 600 instead of 599.", clean)
        self.assertNotIn("check.sh", clean)
        self.assertFalse(any(e["action"] == "drop" for e in edits))
        self.assertNotIn("the boundary probe", redact(text)[0])


if __name__ == "__main__":
    unittest.main()

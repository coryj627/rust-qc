#!/usr/bin/env python3
"""Verify that verifier passage packaging cannot erase report evidence."""

import unittest

from prepare_grading import passages, verifier_text


class PassageTests(unittest.TestCase):
    def test_all_original_bytes_reassemble_including_blank_lines(self):
        text = ("Intro.\n\n1. **Finding** details.\n\n\n```rust\nlet x = 1;\n\n"
                'println!("{x}");\n```\n\nTail')
        self.assertEqual("".join(passages(text)), text)
        self.assertIn('```rust\nlet x = 1;\n\nprintln!("{x}");\n```\n\n', passages(text))

    def test_inner_fence_does_not_end_longer_fence(self):
        text = "````text\n```\n\nevidence\n````\n\nAfter\n"
        self.assertEqual(passages(text), ["````text\n```\n\nevidence\n````\n\n", "After\n"])

    def test_normalizes_labels_and_logs_count_footer_without_erasing_claim(self):
        text = ('1. **High — Overflow.** Source evidence.\n\n'
                '**1 finding. Last `./clock`: 2 minutes.**\n')
        result, edits = verifier_text(text)
        self.assertEqual(result, 'Overflow. Source evidence.\n\n')
        self.assertEqual([e["action"] for e in edits], ["replace", "drop"])

    def test_verifier_formatting_preserves_code_exactly(self):
        code = '```text\n1. **High — literal code evidence**\n```\n'
        self.assertEqual(verifier_text(code), (code, []))
        inline = 'Evidence: `let a = **ptr;` is unchanged.\n'
        self.assertEqual(verifier_text(inline), (inline, []))

    def test_summary_count_does_not_reveal_injected_controls(self):
        text = ("The audit found 13 defects: 2 High, 8 Medium, and 3 Low. "
                "Nine runtime defects were reproduced, including a use-after-free.\n")
        clean, edits = verifier_text(text)
        self.assertEqual(clean,
                         "Nine runtime defects were reproduced, including a use-after-free.\n")
        self.assertEqual(edits[0]["reason"], "report_count_removed_before_control_insertion")

    def test_functional_summary_counts_are_logged_without_erasing_evidence(self):
        text = ("The existing tests pass, but targeted checks confirmed "
                "**15 Major findings and four Minor findings**. "
                "The parser emits 600 references instead of 599.\n\n"
                "**Finding count: 19 — 15 Major, four Minor.** "
                "Coverage used ordinary regressions. Last clock reading: 851 seconds.\n")
        clean, edits = verifier_text(text, functional=True)
        self.assertNotIn("15 Major", clean)
        self.assertNotIn("four Minor", clean)
        self.assertNotIn("Finding count: 19", clean)
        self.assertIn("The parser emits 600 references instead of 599.", clean)
        self.assertIn("Coverage used ordinary regressions.", clean)
        self.assertTrue(any(e["reason"] == "functional_report_count_normalization"
                            for e in edits))
        self.assertIn("15 Major findings and four Minor findings", verifier_text(text)[0])
        self.assertIn("Finding count: 19", verifier_text(text)[0])

    def test_functional_count_normalization_preserves_fenced_probe_bytes(self):
        text = '```text\nFinding count: 19 — 15 Major, four Minor.\n```\n'
        self.assertEqual(verifier_text(text, functional=True), (text, []))


if __name__ == "__main__":
    unittest.main()

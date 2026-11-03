#!/usr/bin/env python3
"""Verify that verifier passage packaging cannot erase report evidence."""

import unittest

from prepare_grading import passages


class PassageTests(unittest.TestCase):
    def test_all_original_bytes_reassemble_including_blank_lines(self):
        text = ("Intro.\n\n1. **Finding** details.\n\n\n```rust\nlet x = 1;\n\n"
                'println!("{x}");\n```\n\nTail')
        self.assertEqual("".join(passages(text)), text)
        self.assertIn('```rust\nlet x = 1;\n\nprintln!("{x}");\n```\n\n', passages(text))

    def test_inner_fence_does_not_end_longer_fence(self):
        text = "````text\n```\n\nevidence\n````\n\nAfter\n"
        self.assertEqual(passages(text), ["````text\n```\n\nevidence\n````\n\n", "After\n"])


if __name__ == "__main__":
    unittest.main()

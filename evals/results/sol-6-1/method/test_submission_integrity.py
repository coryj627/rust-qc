#!/usr/bin/env python3
"""Detect source and symlink changes without treating build caches as source."""

import tempfile
import unittest
from pathlib import Path

from verify_submissions import difference, inventory


class SubmissionIntegrityTests(unittest.TestCase):
    def test_source_and_link_tampering_are_detected_but_cache_changes_are_not(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "source.txt").write_text("original")
            (root / "link").symlink_to("source.txt")
            expected = inventory(root)
            (root / "target").mkdir()
            (root / "target/cache").write_text("generated")
            self.assertFalse(any(difference(expected, inventory(root)).values()))
            (root / "source.txt").write_text("changed")
            (root / "link").unlink()
            (root / "link").symlink_to("different-target")
            self.assertEqual(difference(expected, inventory(root)),
                             {"added": [], "removed": [], "changed": ["link", "source.txt"]})
            alias = root / "root-alias"
            alias.symlink_to(root, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "real workspace"):
                inventory(alias)


if __name__ == "__main__":
    unittest.main()

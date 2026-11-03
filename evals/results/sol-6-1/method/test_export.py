#!/usr/bin/env python3
"""Ensure exporting untrusted workspaces cannot follow links to host data."""

import tempfile
import unittest
from pathlib import Path

from export_dataset import scrub, workspace_files


class ExportTests(unittest.TestCase):
    def test_host_link_and_build_cache_are_excluded(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            work = root / "work"
            work.mkdir()
            (root / "private-canary").write_text("not for publication")
            (work / "escape").symlink_to(root / "private-canary")
            (work / "target").mkdir()
            (work / "target/binary").write_bytes(b"cached build")
            (work / "probe.rs").write_text("fn main() {}\n")
            self.assertEqual([str(p) for _, p in workspace_files(work)], ["probe.rs"])
            alias = root / "root-link"
            alias.symlink_to(root, target_is_directory=True)
            self.assertEqual(list(workspace_files(alias)), [])

    def test_scrubs_host_identity_and_synthetic_token(self):
        text = "/Users/example/project user@example.invalid sk-12345678901234567890"
        cleaned = scrub(text, Path("/private/tmp/evaluation"))
        self.assertNotIn("/Users/example", cleaned)
        self.assertNotIn("user@example.invalid", cleaned)
        self.assertNotIn("sk-12345678901234567890", cleaned)


if __name__ == "__main__":
    unittest.main()

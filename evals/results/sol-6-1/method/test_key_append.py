#!/usr/bin/env python3
"""Protect original key bytes and YAML values when publishing new run rows."""

import tempfile
import unittest
from pathlib import Path

import yaml
from append_key_runs import append


class KeyAppendTests(unittest.TestCase):
    def test_preserves_bytes_quotes_hashes_and_does_not_duplicate_runs(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "key.yaml"
            original = 'project: example\n# original comment\nruns:\n  - model: claude\n'
            path.write_text(original)
            entry = {"run_id": "sol-one", "model": "gpt-6.1-sol", "note": "value # intact",
                     "bugs_found": ["bug-1"], "bugs_partial": []}
            self.assertEqual(append(path, [entry], True), 1)
            self.assertTrue(path.read_text().startswith(original))
            self.assertEqual(yaml.safe_load(path.read_text())["runs"][-1], entry)
            self.assertEqual(append(path, [entry], True), 0)


if __name__ == "__main__":
    unittest.main()

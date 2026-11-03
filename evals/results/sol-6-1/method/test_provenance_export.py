#!/usr/bin/env python3
"""Provenance export must never sweep private profile/session files into output."""

import tempfile
import unittest
from pathlib import Path

from collect_provenance import collect
from export_dataset import scan


class ProvenanceExportTests(unittest.TestCase):
    def test_explicit_allowlist_and_scanner_cover_grading_and_schedules(self):
        with tempfile.TemporaryDirectory() as temporary:
            scratch = Path(temporary) / "scratch"
            output = Path(temporary) / "output"
            (scratch / "grading-private").mkdir(parents=True)
            output.mkdir()
            (scratch / "stage5-schedule.json").write_text('[]\n')
            (scratch / "auth.json").write_text('private canary')
            (scratch / "raw-session.jsonl").write_text('private canary')
            self.assertEqual(collect(scratch, output), 1)
            self.assertEqual(
                sorted(str(p.relative_to(output)) for p in output.rglob('*') if p.is_file()),
                ['grading/provenance.json', 'schedules/stage5-schedule.json'],
            )
            for folder in ("grading", "schedules"):
                (output / folder / "canary.txt").write_text('user@example.invalid')
            self.assertEqual({hit['file'] for hit in scan(output)},
                             {'grading/canary.txt', 'schedules/canary.txt'})


if __name__ == "__main__":
    unittest.main()

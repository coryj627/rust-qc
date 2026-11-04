#!/usr/bin/env python3
"""Challenge the transcript adapters with known violations and quoted commands."""

import json
import tempfile
import unittest
from pathlib import Path

from check_runs import check
from host_builds import violations


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.folder = Path(self.temporary.name)
        self.metadata = {"run_id": "test", "model": "gpt-6.1-sol", "effort": "high",
                         "status": "completed", "report_collected": True,
                         "active_seconds": 10, "budget_seconds": 900}
        (self.folder / "metadata.json").write_text(json.dumps(self.metadata))
        (self.folder / "report.md").write_text("Report.\n")

    def tearDown(self):
        self.temporary.cleanup()

    def record(self, command, execution="sb run", tool="sandbox_exec"):
        (self.folder / "commands.jsonl").write_text(json.dumps({
            "index": 1, "command": command, "execution": execution, "tool": tool,
            "timeout_seconds": 30,
        }) + "\n")
        kind = "dynamicToolCall" if tool == "sandbox_exec" else "commandExecution"
        (self.folder / "events.jsonl").write_text(json.dumps({
            "method": "item/completed", "params": {"item": {"type": kind, "tool": tool}},
        }) + "\n")

    def test_detects_known_host_build_without_executing_it(self):
        self.record("env RUSTUP_AUTO_INSTALL=0 cargo test", "host", "exec_command")
        self.assertIn("host_or_unknown_execution", check(self.folder)["flags"])
        self.assertTrue(violations(self.folder))

    def test_container_build_and_quoted_build_text_are_not_host_builds(self):
        self.record("cat <<'EOF'\ncargo test\nEOF\ncargo test --offline")
        self.assertEqual(check(self.folder)["flags"], [])
        self.assertEqual(violations(self.folder), [])

    def test_rejects_wrong_model_even_when_report_exists(self):
        self.record("cat src/lib.rs")
        self.metadata["model"] = "different-model"
        (self.folder / "metadata.json").write_text(json.dumps(self.metadata))
        self.assertIn("model_or_effort_mismatch", check(self.folder)["flags"])

    def test_unmatched_command_cannot_disappear_from_evidence(self):
        self.record("cargo test")
        (self.folder / "commands.jsonl").write_text("")
        self.assertIn("tool_command_count_mismatch", check(self.folder)["flags"])


if __name__ == "__main__":
    unittest.main()

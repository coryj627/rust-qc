#!/usr/bin/env python3
"""Exercise dispatch safety without starting models, containers or candidate code."""

import asyncio
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from run_schedule import dispatch


class ScheduleTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.schedule = self.root / "schedule.json"
        self.args = SimpleNamespace(scratch=self.root, schedule=self.schedule,
                                    workers=1, exclude_project=[], codex="never-executed",
                                    auth_file=self.root / "not-an-auth-file")
        self.calls = []
        self.failures = {}

    def tearDown(self):
        self.temporary.cleanup()

    def rows(self, projects):
        self.schedule.write_text(json.dumps([
            {"run_id": f"test-{i}", "project": project, "budget_minutes": 45}
            for i, project in enumerate(projects)
        ]))

    async def fake_process(self, *command, **_kwargs):
        run = command[3]
        self.calls.append(run)
        folder = self.root / "private" / run
        folder.mkdir(parents=True)
        failure = self.failures.get(run)
        await asyncio.sleep(0)

        async def wait():
            (folder / "metadata.json").write_text(json.dumps({
                "status": "failed" if failure else "completed",
                "error": {"codexErrorInfo": failure} if failure else None,
            }))
            return int(bool(failure))

        return SimpleNamespace(wait=wait)

    async def test_two_dispatchers_claim_each_run_once(self):
        self.rows(["one", "two", "three"])
        with patch("run_schedule.asyncio.create_subprocess_exec", self.fake_process):
            results = await asyncio.gather(dispatch(self.args), dispatch(self.args))
        self.assertEqual(results, [0, 0])
        self.assertCountEqual(self.calls, ["test-0", "test-1", "test-2"])

    async def test_provider_block_skips_project_without_retry(self):
        self.rows(["blocked", "blocked", "independent"])
        self.failures["test-0"] = "cyberPolicy"
        with patch("run_schedule.asyncio.create_subprocess_exec", self.fake_process):
            self.assertEqual(await dispatch(self.args), 0)
        self.assertEqual(self.calls, ["test-0", "test-2"])

    async def test_stop_on_policy_halts_the_whole_condition(self):
        self.rows(["blocked", "independent"])
        self.args.stop_on_policy = True
        self.failures["test-0"] = "cyberPolicy"
        with patch("run_schedule.asyncio.create_subprocess_exec", self.fake_process):
            self.assertEqual(await dispatch(self.args), 1)
        self.assertEqual(self.calls, ["test-0"])

    async def test_stop_on_policy_remains_stopped_when_resumed(self):
        self.rows(["blocked", "independent"])
        self.args.stop_on_policy = True
        failed = self.root / "private/test-0"
        failed.mkdir(parents=True)
        (failed / "metadata.json").write_text(json.dumps({
            "status": "failed", "error": {"codexErrorInfo": "cyberPolicy"},
        }))
        with patch("run_schedule.asyncio.create_subprocess_exec", self.fake_process):
            self.assertEqual(await dispatch(self.args), 1)
        self.assertEqual(self.calls, [])

    async def test_usage_limit_stops_dispatch_and_sets_global_marker(self):
        self.rows(["one", "two"])
        self.failures["test-0"] = "usageLimitExceeded"
        with patch("run_schedule.asyncio.create_subprocess_exec", self.fake_process):
            self.assertEqual(await dispatch(self.args), 1)
        self.assertEqual(self.calls, ["test-0"])
        self.assertTrue((self.root / "STOP-RESOURCE-LIMIT.json").is_file())


if __name__ == "__main__":
    unittest.main()

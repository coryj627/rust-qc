#!/usr/bin/env python3
"""Exercise transport framing with an escaped, model-visible tool result."""

import asyncio
import json
import sys
import unittest

from runner import EVENT_LINE_LIMIT


class TransportTests(unittest.IsolatedAsyncioTestCase):
    async def test_completed_event_over_default_readline_limit_is_not_lost(self):
        script = "import json; print(json.dumps({'output': '\\u2603' * 60000}))"
        process = await asyncio.create_subprocess_exec(
            sys.executable, "-c", script, stdout=asyncio.subprocess.PIPE,
            limit=EVENT_LINE_LIMIT,
        )
        event = await process.stdout.readline()
        self.assertGreater(len(event), 65536)
        self.assertEqual(json.loads(event)["output"], "\u2603" * 60000)
        self.assertEqual(await process.wait(), 0)


if __name__ == "__main__":
    unittest.main()

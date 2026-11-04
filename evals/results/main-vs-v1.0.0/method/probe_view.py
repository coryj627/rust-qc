#!/usr/bin/env python3
"""Print each tool call in a stream with its result, the permission denials and the final text.

Usage: probe_view.py <stream.jsonl>
"""

import json
import sys

calls = {}
with open(sys.argv[1]) as lines:
    for line in lines:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        message = event.get("message")
        content = message.get("content") if isinstance(message, dict) else None
        for block in content if isinstance(content, list) else []:
            if not isinstance(block, dict):
                continue
            if block.get("type") == "tool_use":
                calls[block["id"]] = (block["name"], json.dumps(block["input"])[:110])
            if block.get("type") == "tool_result":
                name, args = calls.get(block.get("tool_use_id"), ("?", ""))
                text = block.get("content")
                text = text if isinstance(text, str) else json.dumps(text)
                print(
                    f"{name:6} error={block.get('is_error')!s:5} {args}\n       -> {text[:170]!r}"
                )
        if event.get("type") == "result":
            print(
                "denials:",
                [
                    (d["tool_name"], json.dumps(d["tool_input"])[:80])
                    for d in event.get("permission_denials") or []
                ],
            )
            print("cost:", event.get("total_cost_usd"), "turns:", event.get("num_turns"))
            print(str(event.get("result"))[-900:])

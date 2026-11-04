"""Read a claude -p stream-json file: tool uses, models, usage and the final result."""

import json


def events(path):
    with open(path) as lines:
        for line in lines:
            try:
                yield json.loads(line)
            except json.JSONDecodeError:
                continue


def tool_uses(path):
    """Yield (name, input) for every tool call the agent made."""
    for event in events(path):
        message = event.get("message")
        if event.get("type") != "assistant" or not isinstance(message, dict):
            continue
        if not isinstance(message.get("content"), list):
            continue
        for block in message["content"]:
            if isinstance(block, dict) and block.get("type") == "tool_use":
                yield block.get("name"), block.get("input") or {}


def summary(path):
    """Models seen, the init event's tool and skill counts, and the result's text and usage."""
    models, init, result = set(), {}, {}
    for event in events(path):
        if event.get("type") == "assistant":
            message = event.get("message")
            model = message.get("model") if isinstance(message, dict) else None
            if model:
                models.add(model)
        elif event.get("subtype") == "init":
            init = event
        elif event.get("type") == "result":
            result = event
    usage = result.get("usage") or {}
    return {
        "models": sorted(models),
        "init_model": init.get("model"),
        "skills": len(init.get("skills") or []),
        "slash_commands": len(init.get("slash_commands") or []),
        "mcp_servers": len(init.get("mcp_servers") or []),
        "plugins": [p.get("name") for p in init.get("plugins") or []],
        "result": result.get("result"),
        "is_error": result.get("is_error"),
        "subtype": result.get("subtype"),
        "num_turns": result.get("num_turns"),
        "input_tokens": usage.get("input_tokens"),
        "output_tokens": usage.get("output_tokens"),
        "cache_read_tokens": usage.get("cache_read_input_tokens"),
        "cache_write_tokens": usage.get("cache_creation_input_tokens"),
        "reported_cost_usd": result.get("total_cost_usd"),
    }

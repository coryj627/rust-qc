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
    """Models seen, the init event's tool and skill counts, and the run's report and usage.

    A run can end more than once: when a background command finishes after the agent's final
    message, Claude Code wakes it for another turn, which ends with a second result. The report
    is the first result's text, the one the agent returned when it finished. Token usage is per
    result, so it is summed; total_cost_usd is cumulative, so the last result's is the run's cost.
    """
    models, init, results = set(), {}, []
    for event in events(path):
        if event.get("type") == "assistant":
            message = event.get("message")
            model = message.get("model") if isinstance(message, dict) else None
            if model:
                models.add(model)
        elif event.get("subtype") == "init":
            init = event
        elif event.get("type") == "result":
            results.append(event)
    first = results[0] if results else {}
    last = results[-1] if results else {}

    def total(field):
        values = [(r.get("usage") or {}).get(field) for r in results]
        return sum(v for v in values if v is not None) if results else None

    return {
        "models": sorted(models),
        "init_model": init.get("model"),
        "skills": len(init.get("skills") or []),
        "slash_commands": len(init.get("slash_commands") or []),
        "mcp_servers": len(init.get("mcp_servers") or []),
        "plugins": [p.get("name") for p in init.get("plugins") or []],
        "result": first.get("result"),
        "later_results": [r.get("result") for r in results[1:]],
        "is_error": first.get("is_error"),
        "subtype": first.get("subtype"),
        "num_turns": first.get("num_turns"),
        "input_tokens": total("input_tokens"),
        "output_tokens": total("output_tokens"),
        "cache_read_tokens": total("cache_read_input_tokens"),
        "cache_write_tokens": total("cache_creation_input_tokens"),
        "reported_cost_usd": last.get("total_cost_usd"),
    }


REFUSED = (
    "approval",
    "was blocked",
    "can't be checked",
    "haven't granted",
    "requested permissions",
    "Contains ",
    "multiple operations",
)


def tool_calls(path):
    """Yield (name, input, ran) for every tool call; ran is False when permissions refused it."""
    calls, order = {}, []
    for event in events(path):
        message = event.get("message")
        content = message.get("content") if isinstance(message, dict) else None
        for block in content if isinstance(content, list) else []:
            if not isinstance(block, dict):
                continue
            if block.get("type") == "tool_use":
                call_id = block.get("id") or f"call-{len(order)}"
                calls[call_id] = [block.get("name"), block.get("input") or {}, True]
                order.append(call_id)
            if block.get("type") == "tool_result" and block.get("tool_use_id") in calls:
                text = block.get("content")
                text = text if isinstance(text, str) else str(text)
                if block.get("is_error") and any(r in text for r in REFUSED):
                    calls[block["tool_use_id"]][2] = False
    for call_id in order:
        yield tuple(calls[call_id])

#!/usr/bin/env python3
"""Append one CSV row for a cloud thread's token usage (issue #1940).

Reads a `get_session` result as JSON on stdin and appends a row to
$USAGE_LOG_CSV (default /mnt/project-files/token-usage/threads.csv), writing the header first
when the file is new or empty. Columns match token-usage/2026-11-15-sessions.csv. Fields the
result lacks stay empty. Usage: mcp get_session (own id) | scripts/autopilot/usage-log.py
"""
import csv
import json
import os
import sys

COLUMNS = [
    "id", "title", "created_at", "updated_at", "model", "last_served_model", "effort_level",
    "cost_usd", "cache_read_tokens", "cache_write_tokens", "input_tokens", "output_tokens",
    "context_used_tokens", "claude_reply_count", "parent_session_id", "recent_action",
]
DEFAULT_PATH = "/mnt/project-files/token-usage/threads.csv"


def dig(obj, *path):
    for key in path:
        if not isinstance(obj, dict) or key not in obj:
            return None
        obj = obj[key]
    return obj


def first(obj, *paths):
    for path in paths:
        value = dig(obj, *path)
        if value is not None and not isinstance(value, (dict, list)):
            return value
    return ""


def row_of(data):
    ccr = data.get("ccr", data)
    meta = ccr.get("external_metadata") or {}
    usage = meta.get("usage") or {}
    ctx = ccr.get("session_context") or {}

    def use(*names):
        return first(usage, *[(n,) for n in names])

    row = {
        "id": first(ccr, ("id",)),
        "title": first(ccr, ("title",)),
        "created_at": first(ccr, ("created_at",)),
        "updated_at": first(ccr, ("updated_at",)),
        "model": first(ccr, ("configured_model",), ("model",)) or first(ctx, ("model",)),
        "last_served_model": first(meta, ("last_served_model",)),
        "effort_level": first(ctx, ("effort_level",)) or first(ccr, ("effort_level",)),
        "cost_usd": use("cost_usd", "total_cost_usd", "cost"),
        "cache_read_tokens": use("cache_read_tokens", "cache_read_input_tokens"),
        "cache_write_tokens": use(
            "cache_write_tokens", "cache_creation_tokens", "cache_creation_input_tokens"
        ),
        "input_tokens": use("input_tokens"),
        "output_tokens": use("output_tokens"),
        "context_used_tokens": first(meta, ("context_usage", "used_tokens")),
        "claude_reply_count": first(meta, ("claude_reply_count",))
        or use("claude_reply_count", "reply_count"),
        "parent_session_id": first(ccr, ("parent_session_id",), ("lineage", "parent_session_id")),
        "recent_action": first(meta, ("recent_action",)),
    }
    return [row[c] for c in COLUMNS]


def main():
    try:
        data = json.load(sys.stdin)
    except json.JSONDecodeError as err:
        sys.exit(f"usage-log: stdin is not JSON: {err}")
    if not isinstance(data, dict):
        sys.exit("usage-log: expected a JSON object")
    path = os.environ.get("USAGE_LOG_CSV", DEFAULT_PATH)
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    new = not os.path.exists(path) or os.path.getsize(path) == 0
    with open(path, "a", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        if new:
            w.writerow(COLUMNS)
        w.writerow(row_of(data))


if __name__ == "__main__":
    main()

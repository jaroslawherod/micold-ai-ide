#!/usr/bin/env python3
"""Print how large a running autopilot unit's context is, so it can hand over before it grows costly.

Usage:
  context.py "<unit description>"   the newest subagent of this worktree's sessions with exactly that
                                    description (the one the orchestrator dispatched it with)
  context.py                        the newest main session of this worktree (the orchestrator)

Prints `CONTEXT <tokens> OK <cap>` (exit 0) or `CONTEXT <tokens> OVER <cap>` (exit 3). Exit 2 when no
transcript matches. The cap is AUTOPILOT_CONTEXT_CAP, default 150000. The size is the context of the
last request in the transcript: input + cache write + cache read.
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path


def project_dir():
    top = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True)
    root = top.stdout.strip() or os.getcwd()
    return Path.home() / ".claude" / "projects" / re.sub(r"[^A-Za-z0-9]", "-", str(Path(root).resolve()))


def transcript(folder, description):
    if description is None:
        hits = list(folder.glob("*.jsonl"))
    else:
        hits = []
        for meta in folder.glob("*/subagents/agent-*.meta.json"):
            try:
                if json.loads(meta.read_text()).get("description") == description:
                    t = meta.with_name(meta.name.removesuffix(".meta.json") + ".jsonl")
                    if t.exists():
                        hits.append(t)
            except (OSError, json.JSONDecodeError):
                continue
    return max(hits, key=lambda p: p.stat().st_mtime, default=None)


def last_context(path):
    ctx = 0
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            msg = rec.get("message") or {}
            u = msg.get("usage")
            if rec.get("type") != "assistant" or not u or msg.get("model") == "<synthetic>":
                continue
            ctx = (u.get("input_tokens", 0) + u.get("cache_creation_input_tokens", 0)
                   + u.get("cache_read_input_tokens", 0))
    return ctx


def main(argv):
    if argv and argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    description = " ".join(argv) if argv else None
    cap = int(os.environ.get("AUTOPILOT_CONTEXT_CAP", "150000"))
    t = transcript(project_dir(), description)
    if t is None:
        print(f"context: no transcript for {description or 'the main session'!r}", file=sys.stderr)
        return 2
    ctx = last_context(t)
    over = ctx >= cap
    print(f"CONTEXT {ctx} {'OVER' if over else 'OK'} {cap}")
    return 3 if over else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

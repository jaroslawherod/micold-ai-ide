#!/usr/bin/env python3
"""PostToolUse hook: tell an autopilot unit when its context passed the cap, so it hands over.

Units are told to check their context at each checkpoint (unit.md, *Hand over at 150k*), but
measured runs showed units that never checked and grew to 266k. This hook does the check for them.

It reads the hook input on stdin, finds the transcript of the caller (the subagent's when the call
came from one, else the session's) and takes the context of its last request. Under the cap it
prints nothing. Over it, and only on a branch an autopilot ledger names, it prints a
PostToolUse `additionalContext` that tells the caller to hand over. It repeats only after the
context grew by another AUTOPILOT_CONTEXT_STEP tokens (default 20000), so it does not nag on every
call. The cap is AUTOPILOT_CONTEXT_CAP, default 150000, as in context.py.
"""

import glob
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

TAIL = 2_000_000  # bytes; the last request's usage sits at the end of the transcript


def last_context(path):
    ctx = 0
    with open(path, "rb") as f:
        f.seek(0, os.SEEK_END)
        size = f.tell()
        f.seek(max(0, size - TAIL))
        lines = f.read().decode("utf-8", errors="replace").splitlines()
    for line in lines[1 if size > TAIL else 0:]:  # the first line of a tail may be cut
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


def on_ledger_branch(cwd):
    def git(*args):
        return subprocess.run(["git", "-C", cwd, *args], capture_output=True, text=True).stdout.strip()

    top, branch = git("rev-parse", "--show-toplevel"), git("branch", "--show-current")
    if not top or not branch:
        return False
    line = f"- **Worktree branch**: {branch}"
    for pattern in ("specs/*/autopilot.md", "specs/*/bugs/*.autopilot.md", "specs/quick/*.autopilot.md"):
        for ledger in glob.glob(os.path.join(top, pattern)):
            try:
                if line in Path(ledger).read_text(errors="replace").splitlines():
                    return True
            except OSError:
                continue
    return False


def main():
    try:
        hook = json.load(sys.stdin)
    except json.JSONDecodeError:
        return 0
    session = hook.get("transcript_path") or ""
    agent = hook.get("agent_id")
    transcript = Path(session[: -len(".jsonl")]) / "subagents" / f"agent-{agent}.jsonl" if agent else Path(session)
    if not session or not transcript.is_file():
        return 0
    cap = int(os.environ.get("AUTOPILOT_CONTEXT_CAP", "150000"))
    step = int(os.environ.get("AUTOPILOT_CONTEXT_STEP", "20000"))
    ctx = last_context(transcript)
    if ctx < cap or not on_ledger_branch(hook.get("cwd") or "."):
        return 0

    state = Path(tempfile.gettempdir()) / f"autopilot-context-{agent or hook.get('session_id', 'session')}"
    try:
        if ctx < int(state.read_text()) + step:
            return 0
    except (OSError, ValueError):
        pass
    try:
        state.write_text(str(ctx))
    except OSError:
        pass

    if agent:
        msg = (f"autopilot context: {ctx} tokens, over the {cap} cap. Every further call re-reads all of it. "
               "If you are an autopilot unit: finish the step in hand, then follow unit.md *Hand over at 150k* now "
               "(run checkpoint.sh, write *Handover* in the ledger, commit, return `STATUS: HANDOVER`). "
               "Any other subagent: stop exploring and return your answer.")
    else:
        msg = (f"autopilot context: {ctx} tokens, over the {cap} cap. Tell the user in one line that `/clear` then "
               "`/speckit-autopilot resume` would restart the orchestrator small, and carry on.")
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": msg}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())

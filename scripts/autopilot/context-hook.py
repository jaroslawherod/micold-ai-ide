#!/usr/bin/env python3
"""PostToolUse hook: tell an autopilot unit when its context passed the cap, so it hands over, and
when it probes one read-only call at a time, so it batches.

Units are told to check their context at each checkpoint (rules/context.md, *Hand over at 150k*), but
measured runs showed units that never checked and grew to 266k. This hook does the check for them.

It reads the hook input on stdin, finds the transcript of the caller (the subagent's when the call
came from one, else the session's) and takes the context of its last request. Under the cap it
prints nothing. Over it, and only on a branch an autopilot ledger names, it prints a
PostToolUse `additionalContext` that tells the caller to hand over. It repeats only after the
context grew by another AUTOPILOT_CONTEXT_STEP tokens (default 20000), so it does not nag on every
call. The cap is AUTOPILOT_CONTEXT_CAP, default 150000, as in context.py.

Batching: in measured runs, a lone read-only call right after another cost 3.8M and 5.9M cost_eq a
run, a tenth of it, at about 100k of context each. When the caller's last AUTOPILOT_BATCH_RUN
requests (default 3) each made exactly one read-only tool call, and its context is at least
AUTOPILOT_BATCH_CTX (default 60000), the hook says so once, and not again for the next 8 requests.
What counts as read-only is autopilot-tokens.py's `unbatched` rule.
"""

import glob
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

TAIL = 2_000_000  # bytes; the last request's usage sits at the end of the transcript
BATCH_QUIET = 8  # requests after a batching message before the next one


def read_only_rule():
    """autopilot-tokens.py's read_only(), or None when it cannot be loaded."""
    try:
        path = Path(__file__).resolve().parent.parent / "autopilot-tokens.py"
        spec = importlib.util.spec_from_file_location("autopilot_tokens", path)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        return mod.read_only
    except Exception:  # the hook must never fail a tool call
        return None


def last_requests(path):
    """(context of the last request, [(message id, [tool_use blocks])] in order) from the
    transcript's tail."""
    ctx = 0
    order, tools = [], {}
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
        mid = msg.get("id")
        if mid not in tools:
            tools[mid] = []
            order.append(mid)
        tools[mid] += [b for b in msg.get("content") or [] if isinstance(b, dict) and b.get("type") == "tool_use"]
    return ctx, [(mid, tools[mid]) for mid in order]


def lone_reads(requests, read_only):
    """How many of the last requests in a row made exactly one tool call, a read-only one."""
    n = 0
    for _, blocks in reversed(requests):
        if len(blocks) != 1 or not read_only(blocks[0]):
            break
        n += 1
    return n


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
    ctx, requests = last_requests(transcript)
    who = agent or hook.get("session_id", "session")
    msgs = []

    run_min = int(os.environ.get("AUTOPILOT_BATCH_RUN", "3"))
    if ctx >= int(os.environ.get("AUTOPILOT_BATCH_CTX", "60000")) and len(requests) >= run_min:
        read_only = read_only_rule()
        run = lone_reads(requests, read_only) if read_only else 0
        state = Path(tempfile.gettempdir()) / f"autopilot-batch-{who}"
        ids = [mid for mid, _ in requests]
        try:
            last = state.read_text()
        except OSError:
            last = ""
        recent = last in ids and len(ids) - 1 - ids.index(last) < BATCH_QUIET
        if run >= run_min and not recent and on_ledger_branch(hook.get("cwd") or "."):
            try:
                state.write_text(ids[-1])
            except OSError:
                pass
            msgs.append(f"autopilot batching: your last {run} calls were one read-only call each, and each re-read "
                        f"{ctx} tokens. Put the probes you can foresee in one message, or in one Bash command "
                        "(`grep -n a f; grep -n b g`). A search whose end you cannot foresee goes to an `Explore` "
                        "subagent.")

    if ctx >= cap and on_ledger_branch(hook.get("cwd") or "."):
        msg = over_cap(ctx, cap, step, agent, who)
        if msg:
            msgs.append(msg)
    if msgs:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": " ".join(msgs)}}))
    return 0


def over_cap(ctx, cap, step, agent, who):
    state = Path(tempfile.gettempdir()) / f"autopilot-context-{who}"
    try:
        if ctx < int(state.read_text()) + step:
            return None
    except (OSError, ValueError):
        pass
    try:
        state.write_text(str(ctx))
    except OSError:
        pass

    if agent:
        return (f"autopilot context: {ctx} tokens, over the {cap} cap. Every further call re-reads all of it. "
                "If you are an autopilot unit: finish the step in hand, then follow rules/context.md *Hand over at 150k* now "
                "(run checkpoint.sh, write *Handover* in the ledger, commit, return `STATUS: HANDOVER`). "
                "Any other subagent: stop exploring and return your answer.")
    return (f"autopilot context: {ctx} tokens, over the {cap} cap. Tell the user in one line that `/clear` then "
            "`/speckit-autopilot resume` would restart the orchestrator small, and carry on.")


if __name__ == "__main__":
    sys.exit(main())

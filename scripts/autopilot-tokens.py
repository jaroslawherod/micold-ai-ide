#!/usr/bin/env python3
"""Report token usage of a Claude Code session, split by the subagents it ran.

Built to measure speckit-autopilot runs: the orchestrator is the main session, and every unit,
reviewer and forked skill is a subagent whose transcript sits under <session>/subagents/.

Usage:
  scripts/autopilot-tokens.py <session.jsonl | session-id> [...]   # one or more sessions
  scripts/autopilot-tokens.py --worktree [<dir>]                   # every session of a worktree
  add --json for machine-readable output

A session id is looked up under ~/.claude/projects/*/. --worktree defaults to the current
directory and maps it to its ~/.claude/projects/ folder the same way Claude Code does.

Columns:
  calls       API requests (one per assistant message id)
  input       uncached input tokens
  cache_w     tokens written to the prompt cache
  cache_r     tokens read from the prompt cache
  rebuilds    requests after a transcript's first that wrote over half of a 30k+ context to the
              cache: the cache had expired (5 min for subagents, 1 h for the main session) or the
              prefix changed (compaction, model switch)
  unbatched   requests that made one read-only tool call (Read, Grep, Glob, or a Bash read such as
              cat, grep, ls, git status/log/diff, gh … view) right after a request that did the
              same: a call that could have gone into the previous message
  output      LOWER BOUND: transcripts store usage from the start of the stream, so output is
              mostly undercounted. Input and cache columns are exact.
  peak_ctx    largest context sent in one request (input + cache_w + cache_r)
  cost_eq     input-token equivalent at API price ratios: input 1, cache write 1.25 (5m) or 2 (1h),
              cache read 0.1, output 5. Comparable across runs on the same model only.
"""

import json
import re
import sys
from collections import defaultdict
from pathlib import Path

PROJECTS = Path.home() / ".claude" / "projects"
FIELDS = ("calls", "input", "cache_w", "cache_r", "rebuilds", "unbatched", "output", "peak_ctx",
          "cost_eq")
# A request that writes more than half of a context this large re-caches the conversation.
REBUILD_MIN_CTX = 30_000


def read_jsonl(path):
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    yield json.loads(line)
                except json.JSONDecodeError:
                    continue


READ_ONLY_BASH = re.compile(
    r"^\s*(cat|sed -n|head|tail|grep|rg|ls|find|wc|git (status|log|diff|show|branch|rev-parse|cherry)"
    r"|gh (pr|run) (view|list|checks))\b")


def read_only(block):
    name, args = block.get("name"), block.get("input") or {}
    if name in ("Read", "Grep", "Glob"):
        return True
    return name == "Bash" and bool(READ_ONLY_BASH.match(args.get("command", "")))


def usage_of(path):
    """Sum usage per model for one transcript. Content blocks of one message repeat its usage, so
    each message id counts once."""
    seen = set()
    first = True
    per_model = defaultdict(lambda: dict.fromkeys(FIELDS, 0))
    tools = defaultdict(list)  # message id -> read_only() of each tool call, in order
    order = []  # (message id, model) of counted messages
    for rec in read_jsonl(path):
        if rec.get("type") != "assistant":
            continue
        msg = rec.get("message") or {}
        u = msg.get("usage")
        mid = msg.get("id") or rec.get("requestId")
        for block in msg.get("content") or []:
            if isinstance(block, dict) and block.get("type") == "tool_use":
                tools[mid].append(read_only(block))
        if not u or mid in seen or msg.get("model") == "<synthetic>":
            continue
        seen.add(mid)
        order.append((mid, msg.get("model", "?")))
        cc = u.get("cache_creation") or {}
        w1h = cc.get("ephemeral_1h_input_tokens", 0)
        cw = u.get("cache_creation_input_tokens", 0)
        w5m = cw - w1h
        inp, cr, out = u.get("input_tokens", 0), u.get("cache_read_input_tokens", 0), u.get("output_tokens", 0)
        m = per_model[msg.get("model", "?")]
        m["calls"] += 1
        m["input"] += inp
        m["cache_w"] += cw
        m["cache_r"] += cr
        ctx = inp + cw + cr
        if not first and ctx >= REBUILD_MIN_CTX and cw > ctx / 2:
            m["rebuilds"] += 1
        first = False
        m["output"] += out
        m["peak_ctx"] = max(m["peak_ctx"], inp + cw + cr)
        m["cost_eq"] += inp + 1.25 * w5m + 2 * w1h + 0.1 * cr + 5 * out
    prev_solo_read = False
    for mid, model in order:
        solo_read = tools[mid] == [True]
        if solo_read and prev_solo_read:
            per_model[model]["unbatched"] += 1
        prev_solo_read = solo_read
    return per_model


def add(a, b):
    for k in FIELDS:
        a[k] = max(a[k], b[k]) if k == "peak_ctx" else a[k] + b[k]


def total(per_model):
    t = dict.fromkeys(FIELDS, 0)
    for m in per_model.values():
        add(t, m)
    return t


def short_model(name):
    return re.sub(r"^claude-|-\d{8}$", "", name)


def session_report(path):
    path = Path(path)
    units = []  # (id, parent, label, per_model)
    units.append(("main", None, "orchestrator (main session)", usage_of(path)))
    sub = path.with_suffix("") / "subagents"
    if sub.is_dir():
        for t in sorted(sub.glob("agent-*.jsonl")):
            aid = t.stem.removeprefix("agent-")
            meta_p = t.with_suffix(".meta.json")
            skill_p = sub / f"agent-{aid}.forked-skill.json"
            meta = json.loads(meta_p.read_text()) if meta_p.exists() else {}
            if skill_p.exists():
                label = "skill:" + json.loads(skill_p.read_text()).get("skillName", "?")
            else:
                label = meta.get("description") or meta.get("agentType") or aid
            units.append((aid, meta.get("parentAgentId", "main"), label, usage_of(t)))
    return units


def tree_rows(units):
    """Rows in tree order, each with its own usage and its subtree roll-up."""
    children = defaultdict(list)
    by_id = {u[0]: u for u in units}
    for u in units[1:]:
        parent = u[1] if u[1] in by_id else "main"
        children[parent].append(u[0])
    rows = []

    def walk(uid, depth):
        _, _, label, pm = by_id[uid]
        own = total(pm)
        roll = dict(own)
        start = len(rows)
        rows.append(None)
        for c in children[uid]:
            add(roll, walk(c, depth + 1))
        models = ",".join(sorted(short_model(m) for m in pm)) or "-"
        rows[start] = dict(depth=depth, label=label, models=models, own=own, rollup=roll,
                           leaf=not children[uid])
        return roll

    walk("main", 0)
    return rows


def fmt(n):
    n = int(n)
    if n >= 1_000_000:
        return f"{n / 1e6:.1f}M"
    if n >= 1_000:
        return f"{n / 1e3:.0f}k"
    return str(n)


def print_markdown(path, units):
    rows = tree_rows(units)
    grand = rows[0]["rollup"]
    by_model = defaultdict(lambda: dict.fromkeys(FIELDS, 0))
    for _, _, _, pm in units:
        for m, v in pm.items():
            add(by_model[short_model(m)], v)
    print(f"### {Path(path).stem}\n")
    print("| Unit | Model | " + " | ".join(FIELDS) + " | share |")
    print("|---|---|" + "---:|" * (len(FIELDS) + 1))
    for r in rows:
        # Own numbers for every row; a parent's subtree is summed in its "+ children" line.
        name = "&nbsp;&nbsp;" * r["depth"] + r["label"].replace("|", "/")
        vals = r["own"]
        share = vals["cost_eq"] / grand["cost_eq"] * 100 if grand["cost_eq"] else 0
        print(f"| {name} | {r['models']} | " + " | ".join(fmt(vals[k]) for k in FIELDS)
              + f" | {share:.0f}% |")
        if not r["leaf"] and r["depth"] > 0:
            vals = r["rollup"]
            share = vals["cost_eq"] / grand["cost_eq"] * 100 if grand["cost_eq"] else 0
            print(f"| {'&nbsp;&nbsp;' * r['depth']}↳ with subagents | | "
                  + " | ".join(fmt(vals[k]) for k in FIELDS) + f" | {share:.0f}% |")
    print(f"| **Total** | | " + " | ".join(f"**{fmt(grand[k])}**" for k in FIELDS) + " | 100% |")
    print("\n| Model | " + " | ".join(FIELDS) + " |")
    print("|---|" + "---:|" * len(FIELDS))
    for m, v in sorted(by_model.items(), key=lambda kv: -kv[1]["cost_eq"]):
        print(f"| {m} | " + " | ".join(fmt(v[k]) for k in FIELDS) + " |")
    print()


def resolve(arg):
    p = Path(arg)
    if p.is_file():
        return [p]
    hits = sorted(PROJECTS.glob(f"*/{arg}.jsonl"))
    if not hits:
        sys.exit(f"no transcript for {arg!r}")
    return hits


def worktree_sessions(d):
    folder = PROJECTS / re.sub(r"[^A-Za-z0-9]", "-", str(Path(d).resolve()))
    hits = sorted(folder.glob("*.jsonl"), key=lambda p: p.stat().st_mtime)
    if not hits:
        sys.exit(f"no sessions in {folder}")
    return hits


def main(argv):
    as_json = "--json" in argv
    argv = [a for a in argv if a != "--json"]
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    if argv[0] == "--worktree":
        paths = worktree_sessions(argv[1] if len(argv) > 1 else ".")
    else:
        paths = [p for a in argv for p in resolve(a)]
    out = []
    for p in paths:
        units = session_report(p)
        if as_json:
            out.append({"session": str(p), "units": [
                dict(r, own={k: round(v) for k, v in r["own"].items()},
                     rollup={k: round(v) for k, v in r["rollup"].items()})
                for r in tree_rows(units)]})
        else:
            print_markdown(p, units)
    if as_json:
        json.dump(out, sys.stdout, indent=1)
        print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

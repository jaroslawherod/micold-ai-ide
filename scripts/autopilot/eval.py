#!/usr/bin/env python3
"""Metrics for past autopilot runs, so a change to the skill, agents or CLAUDE.md can be measured.

  eval.py [run]            print each corpus run's metrics, the totals and the delta to the baseline
  eval.py baseline         write the current totals to evals/autopilot/baseline.json
  eval.py add <pr> [--ledger FILE] [--tokens N] [--note TEXT]
                           add a run (a failed or escalated one is a regression case)
  eval.py changed [REF]    run only when an agent-shaping file differs from REF (default origin/main)

Per run (deterministic graders; `null` when the source is missing):
  review_rounds    rows of the ledger's "Review rounds" table
  first_pass       1 when every review's round 1 is not CHANGES, else 0 (ledger needed)
  ci_red           failed GitHub workflow runs on the PR's head branch (gh)
  tokens           cost_eq the case recorded from scripts/autopilot-tokens.py --json (optional)
Ledgers are gitignored, so a case keeps a snapshot under evals/autopilot/ledgers/.
Set GH to use another gh binary. Exit 1 when a metric got worse than the baseline.
"""
import json, os, re, subprocess, sys

ROOT = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True).stdout.strip() or "."
DIR = os.path.join(ROOT, "evals/autopilot")
CORPUS, BASELINE = os.path.join(DIR, "corpus.json"), os.path.join(DIR, "baseline.json")
SHAPING = [".claude/skills/speckit-autopilot", ".claude/agents", "CLAUDE.md"]
# metric -> True when a higher value is worse
METRICS = {"review_rounds": True, "first_pass": False, "ci_red": True, "tokens": True}


def load(path, default):
    try:
        with open(path) as f:
            return json.load(f)
    except FileNotFoundError:
        return default


def gh(*args):
    r = subprocess.run([os.environ.get("GH", "gh"), *args], capture_output=True, text=True)
    return json.loads(r.stdout) if r.returncode == 0 and r.stdout.strip() else None


def review_rows(text):
    """(review, round, verdict) rows of the ledger's Review rounds table."""
    m = re.search(r"^## Review rounds\n(.*?)(?=^## |\Z)", text, re.S | re.M)
    rows = []
    for line in (m.group(1) if m else "").splitlines():
        c = [x.strip() for x in line.strip().strip("|").split("|")]
        if len(c) >= 4 and c[1].isdigit():
            rows.append((c[0], int(c[1]), c[3]))
    return rows


def ledger_metrics(text):
    rows = review_rows(text)
    first = [v for _, r, v in rows if r == 1]
    return {
        "review_rounds": len(rows),
        "first_pass": int(all(not v.upper().startswith("CHANGES") for v in first)) if first else None,
    }


def run_metrics(case):
    out = {k: None for k in METRICS}
    led = case.get("ledger")
    if led and os.path.exists(os.path.join(ROOT, led)):
        with open(os.path.join(ROOT, led)) as f:
            out.update(ledger_metrics(f.read()))
    pr = gh("pr", "view", str(case["pr"]), "--json", "headRefName", "-q", "{h:.headRefName}")
    if pr:
        runs = gh("run", "list", "--branch", pr["h"], "--limit", "200", "--json", "conclusion")
        if runs is not None:
            out["ci_red"] = sum(1 for r in runs if r.get("conclusion") == "failure")
    out["tokens"] = case.get("tokens")
    return out


def totals(per_run):
    t = {}
    for k in METRICS:
        v = [m[k] for m in per_run.values() if m[k] is not None]
        t[k] = round(sum(v) / len(v), 2) if v else None
    return t


def collect():
    cases = load(CORPUS, [])
    per = {c["id"]: run_metrics(c) for c in cases}
    return per, totals(per)


def fmt(v):
    return "-" if v is None else str(v)


def cmd_run():
    per, tot = collect()
    base = load(BASELINE, None)
    print(f"{'run':<28}" + "".join(f"{k:>15}" for k in METRICS))
    for i, m in per.items():
        print(f"{i:<28}" + "".join(f"{fmt(m[k]):>15}" for k in METRICS))
    print(f"{'mean (' + str(len(per)) + ' runs)':<28}" + "".join(f"{fmt(tot[k]):>15}" for k in METRICS))
    if base is None:
        print("no baseline: run `eval.py baseline`")
        return 0
    worse = False
    print(f"{'baseline':<28}" + "".join(f"{fmt(base.get(k)):>15}" for k in METRICS))
    line = f"{'delta':<28}"
    for k, high_bad in METRICS.items():
        a, b = tot[k], base.get(k)
        if a is None or b is None:
            line += f"{'-':>15}"
            continue
        d = round(a - b, 2)
        worse |= d > 0 if high_bad else d < 0
        line += f"{d:>+15}"
    print(line)
    if worse:
        print("WORSE than the baseline")
    return 1 if worse else 0


def cmd_baseline():
    _, tot = collect()
    os.makedirs(DIR, exist_ok=True)
    with open(BASELINE, "w") as f:
        json.dump(tot, f, indent=2)
        f.write("\n")
    print(f"baseline written: {tot}")
    return 0


def cmd_add(args):
    if not args or not args[0].isdigit():
        print(__doc__, file=sys.stderr)
        return 2
    case = {"id": f"pr-{args[0]}", "pr": int(args[0])}
    opts = dict(zip(args[1::2], args[2::2]))
    if "--ledger" in opts:
        os.makedirs(os.path.join(DIR, "ledgers"), exist_ok=True)
        dest = f"evals/autopilot/ledgers/{case['id']}.md"
        with open(opts["--ledger"]) as s, open(os.path.join(ROOT, dest), "w") as d:
            d.write(s.read())
        case["ledger"] = dest
    if "--tokens" in opts:
        case["tokens"] = float(opts["--tokens"])
    if "--note" in opts:
        case["note"] = opts["--note"]
    cases = [c for c in load(CORPUS, []) if c["id"] != case["id"]] + [case]
    with open(CORPUS, "w") as f:
        json.dump(cases, f, indent=2)
        f.write("\n")
    print(f"added {case['id']}")
    return 0


def cmd_changed(args):
    ref = args[0] if args else "origin/main"
    r = subprocess.run(["git", "diff", "--name-only", ref, "--", *SHAPING], capture_output=True, text=True, cwd=ROOT)
    files = r.stdout.split()
    if not files:
        print(f"no agent-shaping file differs from {ref}")
        return 0
    print("agent-shaping files changed:", *files, sep="\n  ")
    return cmd_run()


def main(argv):
    cmd = argv[0] if argv else "run"
    if cmd == "run":
        return cmd_run()
    if cmd == "baseline":
        return cmd_baseline()
    if cmd == "add":
        return cmd_add(argv[1:])
    if cmd == "changed":
        return cmd_changed(argv[1:])
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

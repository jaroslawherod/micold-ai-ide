# Task: fix a bug with no spec change (bug unit)

When: the bug flow starts. The scope is the issue's text. No spec, plan, tasks or BUG record.

1. **Ledger.** Create it from [../templates/autopilot-ledger.md](../templates/autopilot-ledger.md)
   as `specs/quick/<YYYY-MM-DD>-<issue>-<slug>.autopilot.md`, **Kind** `bug`, **Issue** and
   **Input** from your prompt, one milestone row `B`. The directory is gitignored: the ledger is
   never committed.
2. **Reproduce on `origin/main`** with `systematic-debugging`. Try the report's steps and obvious
   variations: other OS arm, fresh profile, several sessions. No repro: escalate (category 5) and
   ask for the missing detail. Never guess-fix.
3. **Check the flow fits**, now and at every later step. Stop and return `NEXT:` when:
   - a `specs/<NNN>-*` requirement, scenario or ticked task describes the behaviour wrongly, or
     misses it: `NEXT: bugfix`, with the spec and the repro;
   - the fix adds behaviour nobody specified: `NEXT: feature`, with the repro and the correct
     behaviour;
   - nothing a user sees is broken: `NEXT: chore`;
   - the fix passes the cap (about 5 files, 200 changed lines) and none of the above holds:
     escalate (category 1) with what is done and what is left. The bugfix flow would send it back.
4. **Red first.** A test that fails on `origin/main` for the reported reason. A flaky failure: make
   it fail on demand (a loop under `scripts/build-lock.sh`, `--test-threads=1` or a shorter
   timeout) and note the rate.
5. **Fix** the root cause. The red test passes; a flaky one passes the same number of runs.
6. **Gate, with review A in its shadow.** Start the full gate detached ([gate.md](gate.md)).
   Meanwhile run the `code-review` skill at `medium` on `origin/main...HEAD` in a fresh subagent.
   Fix real findings and a red gate, and repeat. A second round of fixes that still leaves a real
   finding: escalate (category 5). No review B.
7. Open the PR per [pr.md](pr.md). The body says how the fix was proven ("failed on main at
   `<test>`, passes here"). Record the PR in the ledger.

Hands on: return `DONE` with the PR number, or `DONE` with `NEXT: …` and `PR: none`.

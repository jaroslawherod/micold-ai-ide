# Phase Q: quick unit

Small work that needs no spec: a flaky test, a typo, a log line, a dependency or config bump, a
small refactor. No spec, plan or tasks; the scope is the orchestrator's one-line task. **All** of
these hold, or it is not quick:

- No behaviour a user sees changes, and no spec requirement is touched.
- About 3 files and 100 changed lines at most, in one PR.
- No wire protocol, persistence format, concurrency, process or sandbox boundary, or security check
  changes.

1. **Ledger.** Create it from [../templates/autopilot-ledger.md](../templates/autopilot-ledger.md) as
   `specs/quick/<YYYY-MM-DD>-<slug>.autopilot.md`, **Kind** `quick`, one milestone row `Q`. The
   directory is gitignored: the ledger is never committed, and the run has no record PR.
2. **Check it is quick**, now and at every later step. It is not: stop and return `DONE` with
   `NEXT: bug` (behaviour a spec covers is wrong) or `NEXT: feature` (new behaviour), and what you
   found. No PR.
3. **Red first**, on `origin/main`:
   - A flaky test: make it fail on demand (run it in a loop under `scripts/build-lock.sh`, with
     `--test-threads=1` or a shorter timeout) and note the failure rate. It never fails: escalate
     (category 5) with what you tried.
   - Another code change: a test that fails for the reason of the change.
   - Docs or config only: no test.
4. **Fix.** The red command now passes (a flaky test: the same number of runs, zero failures).
5. **Gate** per [../references/pr-and-merge.md](../references/pr-and-merge.md) §2, and meanwhile
   **review A**: the `code-review` skill at `high` on `origin/main...HEAD` in a fresh subagent. Fix
   real findings and repeat, up to 3 counted rounds, then escalate (category 5). No review B.
6. Commit, push and open the PR per §3–4. Title `fix|test|chore|refactor|docs(<area>): …`; the body
   says how the change was proven (e.g. "failed 7/200 on main, 0/200 here"). Record the PR in the
   ledger and return `DONE` with its number.

**Continued with a red CI log:** as in [4-milestone.md](4-milestone.md).

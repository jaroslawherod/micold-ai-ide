# Task: a chore (chore unit)

When: the chore flow starts. The scope is the issue's text. No spec, plan or tasks.

1. **Ledger.** Create it from [../templates/autopilot-ledger.md](../templates/autopilot-ledger.md)
   as `specs/quick/<YYYY-MM-DD>-<issue>-<slug>.autopilot.md`, **Kind** `chore`, **Issue** and
   **Input** from your prompt, one milestone row `C`. The directory is gitignored: the ledger is
   never committed.
2. **Check it is a chore**, now and at every later step. **All** of these hold:
   - No behaviour a user sees changes, and no spec requirement is touched.
   - About 5 files and 200 changed lines at most, in one PR (more only at `effort:high`).
   - No wire protocol, persistence format, concurrency, process or sandbox boundary, or security
     check changes.

   It does not: stop and return `DONE` with `NEXT: bug` (behaviour is broken), `NEXT: bugfix` (a
   spec is wrong) or `NEXT: feature` (new behaviour), and what you found. No PR.
3. **Red first**, on `origin/main`:
   - A flaky test: make it fail on demand (run it in a loop under `scripts/build-lock.sh`, with
     `--test-threads=1` or a shorter timeout) and note the failure rate. It never fails: escalate
     (category 5) with what you tried.
   - Another code or script change: a test that fails for the reason of the change.
   - Docs or config only: no test.
4. **Do it.** The red command now passes (a flaky test: the same number of runs, zero failures).
5. **Gate, with review A in its shadow.** Start the gate that fits the diff ([gate.md](gate.md)).
   Meanwhile run the `code-review` skill at `low` on `origin/main...HEAD` in a fresh subagent. Fix
   real findings and a red gate, and repeat. A second round of fixes that still leaves a real
   finding: escalate (category 5). No review B.
6. Open the PR per [pr.md](pr.md). The body says how the change was proven (e.g. "failed 7/200 on
   main, 0/200 here"). Record the PR in the ledger.

Hands on: return `DONE` with the PR number, or `DONE` with `NEXT: …` and `PR: none`.

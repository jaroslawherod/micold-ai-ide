# Phase 0: bug unit

Scope: reproduce, report, patch, verify, review. Do not fix code; a milestone unit does that.

1. **Reproduce on `origin/main`** with `systematic-debugging`. Try the report's steps and obvious
   variations: other OS arm, fresh profile, several sessions. No repro: escalate (category 5) and ask
   for the missing detail. Never guess-fix.
2. **Find the owning spec**: the `specs/<NNN>-*` whose requirements cover the broken behaviour.
   - **None** (code predates specs, or behaviour never specified): return `DONE` with the line
     `SWITCH: feature`, the repro, and the correct behaviour. The orchestrator starts Phase 1.
   - **Owning feature still in flight** (`**Status**` not Closed, or its ledger not `done`): it
     belongs to another flow. Escalate as *blocked by work outside my flow*, with the repro.
3. **`speckit-bugfix-report`** writes `bugs/BUG-<k>.md` in the owning spec, with root cause and any
   false completions. Create the ledger beside it as `bugs/BUG-<k>.autopilot.md` from
   [../templates/autopilot-ledger.md](../templates/autopilot-ledger.md). Set **Worktree branch** to the exact output of
   `git branch --show-current`.
4. **`speckit-bugfix-patch`**, then **`speckit-bugfix-verify`**. A fresh reviewer checks the patch
   against the bug rubric.
5. **Size it.** If the fix adds behaviour the spec never intended, or the patch adds more than 10
   tasks: set the bug ledger's **Phase** to `done` with the note `promoted to a feature`, commit the
   BUG record and ledger, and return `DONE` with `SWITCH: feature`. The new spec cites `BUG-<k>` as
   input. Otherwise
   commit the BUG record, patch and ledger (do not push) and return `DONE` with the fix's task IDs.

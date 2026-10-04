# Task: report and patch a bug against its spec (bugfix unit)

When: the bugfix flow starts. Scope: reproduce, report, patch, verify, review. Do not fix code; the
milestone unit does that.

1. **Reproduce on `origin/main`** with `systematic-debugging`. Try the report's steps and obvious
   variations: other OS arm, fresh profile, several sessions. No repro: escalate (category 5) and
   ask for the missing detail. Never guess-fix.
2. **Find the owning spec**: the `specs/<NNN>-*` whose requirements cover the broken behaviour.
   - **None**, or the spec is right and stays as it is: return `DONE` with `NEXT: bug`, and the
     repro. Behaviour nobody specified is missing: `NEXT: feature`, with the correct behaviour.
   - **Owning feature still in flight** (`**Status**` not Closed, or its ledger not `done`): it
     belongs to another flow. Escalate as *blocked by work outside my flow*, with the repro.
3. **`speckit-bugfix-report`** writes the record in the owning spec, with root cause and any
   false completions. Name it `bugs/BUG-<issue>.md`, by the run's issue number, not the skill's
   next sequential number: two runs never take the same one. Create the ledger beside it as `bugs/BUG-<issue>.autopilot.md` from
   [../templates/autopilot-ledger.md](../templates/autopilot-ledger.md): **Kind** `bugfix`,
   **Issue** and **Input** from your prompt, **Worktree branch** the exact output of
   `git branch --show-current`.
4. **`speckit-bugfix-patch`**, then **`speckit-bugfix-verify`**. A fresh reviewer checks the patch:
   [review.md](review.md) with [../rubrics/bug.md](../rubrics/bug.md).
5. **Size it.** The fix adds behaviour the spec never intended, or the patch adds more than 10
   tasks: set the ledger's **Phase** to `done` with the note `switched to feature`, commit the BUG
   record and ledger, and return `DONE` with `NEXT: feature`. The new spec cites `BUG-<issue>`.
6. **Tier of the fix.** `light` (Sonnet ships it) when the BUG record names a confirmed root cause
   and the exact code to change, the fix is a few tasks besides the regression test, and it meets
   `light` in [milestone-format.md](milestone-format.md). Otherwise, or when unsure, `full`. Write
   it in the ledger's milestone row.
7. Commit the BUG record, patch and ledger. Do not push: they ship in the fix's PR.

Hands on: return `DONE`, `PR: none`, the fix's task IDs and `TIER: light` or `TIER: full`. Next
unit: milestone ([implement.md](implement.md)); its PR is the run's last.

---
name: speckit-autopilot
description: Use when the user hands over a feature idea or a bug report and wants the whole Spec Kit flow run end to end with as little of their involvement as possible — "autopilot", "run it autonomously", "take it all the way to main", "only ask me when you must" — or says to resume or continue an interrupted autopilot run.
argument-hint: "<feature description> | bug: <report> | resume"
user-invocable: true
---

# Spec Kit autopilot

One prompt in. You write the spec, clarify it, plan, cut tasks, and ship reviewed milestones to
`main`. Ask the human **only** for decisions that are theirs. Batch every ask and make it loud and
specific.

**You are the reviewer.** No human reviews a spec, plan or diff here. Every artifact and every diff
gets a review by a **fresh-context subagent**, never the context that wrote it. Green CI is not a
review.

Follow the rules to the letter. A rule that looks slow is what stops an unattended flow shipping the
wrong thing.

Overview and diagrams: [README.md](README.md). Load files in `references/` when a phase needs them.

## Entry

| Argument | Start at |
|---|---|
| `resume` | Find this worktree's ledger (see *Resuming*), check each recorded PR's `state` with `gh pr view`, and continue at the first unfinished step. **Never** rebuild progress from `gh pr list` or from memory. |
| `bug: …`, or text describing broken behaviour | Phase 0 (bug path) |
| anything else | Phase 1 |

First run `git fetch origin` and check the worktree is clean. As soon as the target directory
exists, copy [templates/autopilot-ledger.md](templates/autopilot-ledger.md) to `autopilot.md` in the
feature directory, or to `bugs/BUG-<k>.autopilot.md` beside the BUG record. Set **Worktree branch**
to the exact output of `git branch --show-current`; `resume` finds the ledger by it. Update the
ledger **before** every commit.

### Resuming

Your ledger records this worktree's branch and is not `done`:

```bash
b=$(git branch --show-current)
grep -lxF -- "- **Worktree branch**: $b" specs/*/autopilot.md specs/*/bugs/*.autopilot.md 2>/dev/null \
  | xargs -r grep -LxF -- '- **Phase**: done'
```

Search the working tree, not `origin/main`: an uncommitted or unpushed ledger is the newest copy.

- **One match**: resume it.
- **None**: `git fetch origin`, then search
  `git grep -lF -- "- **Worktree branch**: $b" origin/main -- 'specs/'`. Resume only a match whose
  Phase on `origin/main` is not `done`. Still nothing: say there is no run to resume here, and stop.
- **Several**: ask with one `AskUserQuestion`. Each option names a ledger's feature, phase and next
  step. Recommend the most recently committed one.

Put debug notes, probe scripts and logs in the session scratchpad, never the worktree.

**When GitHub and the ledger disagree, GitHub is right.** Fix the ledger. A milestone marked merged
whose PR is open, or whose changes are missing from `origin/main`, goes back into the milestone loop.
If a commit on `main` reverted it, escalate (category 6).

## Phases

| # | Phase | Skills | Ends with |
|---|---|---|---|
| 0 | **Bug**, see below | `systematic-debugging` → `speckit-bugfix-report` → `speckit-bugfix-patch` → `speckit-bugfix-verify` → review (bug rubric) | **One PR** with the BUG record, spec patch, regression test and fix, merged on green, then the **handoff**. Or a switch to Phase 1. |
| 1 | **Spec** | `speckit-specify` → review (spec rubric) | **PR 1**: the spec, merged on green |
| 2 | **Clarify**, in rounds until a scan finds nothing | `speckit-clarify` | Answers written to spec.md (ship in PR 2) |
| 3 | **Design** | `speckit-plan` → review · `speckit-tasks` → cut milestones · `speckit-analyze` → fix · review (tasks + milestone rubric) · close checklists | **PR 2**: clarified spec, plan, research, contracts, tasks with `## Milestones`, merged on green |
| 4 | **Milestones**, one at a time | the milestone loop below | One PR per milestone, each merged on green |
| 5 | **Close**, in this order | `speckit-converge` → `speckit-tdd-verify` → `speckit-docguard-guard` | Unbuilt behaviour found by any of the three becomes a new milestone (back to 4, then rerun all three). Fix test-strength and docs findings that add no behaviour in the close PR. That PR sets spec `**Status**` to `Closed <date> — shipped in PRs #…`, gets a fresh-subagent review, merges, then the **handoff** |

`systematic-debugging` is the superpowers skill. If that plugin is enabled instead of the personal
copy, invoke `superpowers:systematic-debugging`.

`speckit-specify` offers up to 3 clarification questions. Do not ask them in Phase 1. Leave them as
`[NEEDS CLARIFICATION]` markers for Phase 2.

Each artifact review loops fix → re-review for up to **3 rounds**. A fourth round is an escalation.
Dispatch and rubrics: [references/review-rubrics.md](references/review-rubrics.md).

## Context: the session orchestrates, subagents run units

One context for a whole run re-reads every earlier phase on every call; one such session made 5,246
calls. So the session that got the prompt is the **orchestrator**. It keeps the ledger, asks the
human, waits on CI and merges. It does not run phase skills.

Each **unit** runs in its own `general-purpose` subagent on the session model (omit `model`):

| Unit | Scope |
|---|---|
| Bug | Phase 0 steps 1–5: reproduce, report, patch, verify, bug-rubric review |
| Spec | Phase 1, through opening PR 1 |
| Clarify round | one `speckit-clarify` run and its triage |
| Design | Phase 3, through opening PR 2 |
| Milestone K | Phase 4 steps 1–5, through opening its PR |
| Close | Phase 5, through opening the close PR |

- **Prompt:** this file's path and section, the ledger path, the worktree path and branch, and the
  unit's scope (for a milestone, its task IDs). The unit dispatches its own reviewers and forked
  skills.
- **Return:** `STATUS: DONE | ESCALATE | FAILED`, the PR number if one was opened, and at most five
  lines of summary. The unit updates the ledger before it returns. The orchestrator reads the
  ledger, not the transcript.
- **Subagents have no `AskUserQuestion`.** At a user decision (clarify triage, escalation) the unit
  stops with `ESCALATE` and the questions in escalation format. The orchestrator asks them, then
  continues **the same** subagent with `SendMessage` and the answers.
- **Red CI** goes back to the unit that opened the PR, via `SendMessage` with the failing log. That
  subagent still holds the change's context.

### Phase 0: bug report

1. **Reproduce on `origin/main`** with `systematic-debugging`. Try the report's steps and obvious
   variations: other OS arm, fresh profile, several sessions. No repro: escalate (category 5) and ask
   for the missing detail. Never guess-fix.
2. **Find the owning spec**: the `specs/<NNN>-*` whose requirements cover the broken behaviour.
   - **None** (code predates specs, or behaviour never specified): switch to Phase 1 and specify the
     correct behaviour, citing the repro.
   - **Owning feature still in flight** (`**Status**` not Closed, or its ledger not `done`): it
     belongs to another flow. Escalate as *blocked by work outside my flow*, with the repro.
3. **`speckit-bugfix-report`** writes `bugs/BUG-<k>.md` in the owning spec, with root cause and any
   false completions. Start the ledger beside it.
4. **`speckit-bugfix-patch`**, then **`speckit-bugfix-verify`**. A fresh reviewer checks the patch
   against the bug rubric in [references/review-rubrics.md](references/review-rubrics.md).
5. **Size it.** Switch to Phase 1 if the fix adds behaviour the spec never intended, or the patch
   adds more than 10 tasks. The new spec cites `BUG-<k>` as input. Otherwise:
6. **Run one milestone** through the Phase 4 loop. Its first task is a regression test that fails on
   `origin/main` for the reported reason. Title the PR `fix(NNN): … (BUG-<k>)`. No spec PR, design
   PR or close phase: after the merge, go to the handoff.

### Phase 2: clarify, triaged

`speckit-clarify` asks at most 5 questions per run, so run it repeatedly. Triage each question:

- **The repo settles it** (constitution, earlier spec, contract, existing code): answer it and record
  `- Q: … → A: … _(agent-resolved: <path>#<section>)_`.
- **Otherwise the user decides.** Collect these for the round and ask them in **one**
  `AskUserQuestion` (up to 4 questions, each with a `(Recommended)` option and its evidence), under
  the escalation banner. Record answers as `_(decided by user)_`.

Done when a run reports no critical ambiguities. A fifth round is an escalation.

### Phase 3: milestones and checklists

Cut milestones by [references/milestones.md](references/milestones.md). Every milestone ships a
**deliverable**: something observable on `main`. "Setup" or "Foundational" is never a milestone on
its own.

`speckit-implement` stops on unchecked checklist items. Resolve them **here**. For each item:

- a reviewer subagent confirms the artifacts satisfy it, and you tick it; or
- you fix the spec or plan until they do; or
- it needs a decision, and you escalate.

If `speckit-implement` asks "proceed anyway?", never answer "yes". Go back to Phase 3.

### Phase 4: the milestone loop

For milestone K:

1. Once the previous PR reads `MERGED`: `git switch -C <worktree-branch> origin/main`.
2. `speckit-implement` with `Milestone MK only: tasks T0xx–T0yy. Do not start any other task.` Its
   mandatory `tdd.run` hook drives red → green → refactor.
3. `mise run gate`. If `cfg(target_os)` code changed, also `cargo check --target aarch64-apple-darwin`.
   If anything visible changed, run the `visual-pass` skill. It is a forked Sonnet subagent that sees
   only its arguments: pass the worktree path, the quickstart section or change to check, and what
   counts as a pass.
4. **Code review. Run both, in parallel, on every milestone:**
   - **A**: the `code-review` skill at `high` on `origin/main...HEAD`.
   - **B**: a fresh subagent checks the diff against the milestone's deliverable, its acceptance
     scenarios and the constitution (see the rubric).

   Verify each finding against the code first. Fix real ones and go back to step 3. Decline a
   finding that contradicts the spec, and record why in the ledger.
5. Tick the milestone's tasks, update the ledger, commit, `git push --force-with-lease`, and open the
   PR per [references/pr-and-merge.md](references/pr-and-merge.md).
6. The orchestrator waits for `ci complete`. Red: continue the milestone's subagent with the failing
   log; it runs `systematic-debugging`, fixes and pushes (at most 3 attempts). No checks: diagnose
   per the reference. Green: `gh pr merge <n> --rebase`, then confirm `state` is `MERGED`.

## Ownership: only this flow's work

Other sessions work in this repo at the same time. You own:

- this worktree and its branch
- the feature directory (or BUG record) this flow created
- the PRs in the ledger
- the subagents you spawned

Everything else is outside the flow:

- **Other PRs.** Never list, review, merge, rebase, approve runs on, comment on or close them, even
  when green.
- **Other specs.** Never edit another feature's `tasks.md` or spec. Only exception: the bug path's
  patch to a **Closed** owning spec, via `speckit-bugfix-patch`.
- **Other worktrees and branches.** Never touch them.
- **Red `main`, or a failure from code this flow did not write.** Do not fix it. Escalate as
  *blocked by work outside my flow*, and say what you checked.
- **A defect in someone else's code.** List it under *Follow-ups not done* in the handoff.
- **This worktree and its branch.** Never pass `--delete-branch`, never delete either. The user
  removes the worktree in micold IDE, which also cleans up the branch.

## Asking the human

Every ask uses the banner and categories in [references/escalation.md](references/escalation.md),
plus a `PushNotification`. **Escalate only for:**

- a product or scope decision the repo does not settle
- a constitution conflict
- an irreversible or outward action beyond merging this flow's own PRs
- missing access
- non-convergence
- the plan proving false in a way that changes requirements

**Never escalate** review findings, fmt or clippy results, merge conflicts, flaky reruns, checkless
PRs, visual checks, or a choice between equivalent implementations. Handle those yourself.

## Handoff: the last message

First verify all three:

- `git status --porcelain` is empty
- after `git fetch origin`, `git cherry origin/main HEAD | grep '^+'` prints nothing. Rebase-merge
  rewrites SHAs, so `git log origin/main..HEAD` still lists merged commits; `git cherry` matches by
  patch.
- every PR in the ledger reads `MERGED`

If any check fails, report exactly what remains. Otherwise send this, with a `PushNotification`:

```
✅ WORK COMPLETE — <NNN-feature>
Delivered (all merged to main):
  M1 #<pr> — <deliverable>        (a bug: BUG-<k> #<pr> — <what now works>)
  M2 #<pr> — <deliverable>
Decisions: <n> made by you, <m> resolved by me from repo evidence — see <ledger path>
Follow-ups not done: <none | list>
This worktree has no uncommitted work, no unpushed commits and no open PRs.
👉 You can remove this worktree in micold IDE now — that also cleans up its branch.
```

## Red flags: stop and re-read this skill

| Thought | Reality |
|---|---|
| "CI is green, so it's reviewed" | CI checks the code works. Reviews A and B check it is the right code. Run both. |
| "I wrote the plan, I'll re-read it myself" | Self-review sees what you meant. A fresh subagent sees what you wrote. |
| "These checklist items are formalities, proceed" | Close them in Phase 3 or escalate. Never "proceed anyway". |
| "Setup is phase 1, so it's milestone 1" | A milestone ships a deliverable. Fold Setup and Foundational into the first user story. |
| "I'll delete the merged branch to tidy up" | The IDE owns cleanup. No `--delete-branch`. |
| "That green PR from another session is ready, I'll merge it" | Not in the ledger, not yours. |
| "main is red, I'll wait for it to recover" | Silent waiting stalls the flow. Escalate as *blocked by work outside my flow*. |
| "Quick question for the user…" | Batch it with a recommendation under the banner, or resolve it from evidence. |
| "This milestone is small, I'll do it in the orchestrator" | Everything the orchestrator reads is re-read on every later call. Dispatch the unit. |
| "I remember where I was" | The ledger and `gh pr view` say where you are. Memory does not. |
| "Everything merged. Done!" | Run the three checks, then send the WORK COMPLETE handoff. |

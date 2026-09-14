---
name: speckit-autopilot
description: Use when the user hands over a feature idea or a bug report and wants the whole Spec Kit flow run end to end with as little of their involvement as possible — "autopilot", "run it autonomously", "take it all the way to main", "only ask me when you must" — or says to resume an autopilot run for a feature number.
argument-hint: "<feature description> | bug: <report> | resume <NNN> [BUG-<k>]"
user-invocable: true
---

# Spec Kit autopilot

One prompt in. The agent then writes the spec, clarifies it, plans and cuts tasks, and ships
reviewed milestones to `main`. The human is asked **only** for decisions that are theirs to make,
and every ask is loud, specific and batched.

**The agent is the reviewer.** No human reviews a spec, a plan or a diff in this flow, so every
artifact and every diff gets an agent review. That review is done by a **fresh-context subagent**,
never the context that wrote the artifact. A green CI run is not a review.

**Violating the letter of these rules is violating their spirit.** Where a rule looks like it slows
the flow down, it is the part that keeps an unattended flow from shipping the wrong thing.

The flow and its diagrams: [README.md](README.md). Details live in `references/` and are loaded when
the phase needs them.

## Entry

| Argument | Start at |
|---|---|
| `resume <NNN>` or `resume <NNN> BUG-<k>` | Read the ledger (`specs/<NNN>-*/autopilot.md`, or `specs/<NNN>-*/bugs/BUG-<k>.autopilot.md`), confirm each recorded PR's `state` with `gh pr view`, and continue at the first unfinished step. **Never** reconstruct progress from `gh pr list` or from memory. |
| `bug: …`, or wording describing broken behaviour | Phase 0 (bug path) |
| anything else | Phase 1 |

Before any phase, run `git fetch origin` and check the worktree is clean. Copy
[templates/autopilot-ledger.md](templates/autopilot-ledger.md) into place the moment its directory
exists: `autopilot.md` in the feature directory, or `bugs/BUG-<k>.autopilot.md` beside the BUG
record. Update the ledger **before** every commit.

Debug notes, probe scripts and logs go in the session scratchpad, never the worktree, so nothing
but deliverables is ever there to commit.

**When GitHub and the ledger disagree, GitHub is right.** Correct the ledger. A milestone recorded
as merged whose PR is open, or whose changes are missing from `origin/main`, goes back into the
milestone loop. If a commit on `main` reverted it, someone decided against it: escalate
(category 6).

## Phases

| # | Phase | Skills driven | Ends with |
|---|---|---|---|
| 0 | **Bug**, see below | `superpowers:systematic-debugging` → `speckit-bugfix-report` → `speckit-bugfix-patch` → `speckit-bugfix-verify` → review (bug rubric) | **One PR** carrying the BUG record, the spec patch, a regression test and the fix, merged on green, then the **handoff**. Or a switch to Phase 1. |
| 1 | **Spec** | `speckit-specify` → artifact review (spec rubric) | **PR 1**: the spec, merged on green |
| 2 | **Clarify** in rounds until a scan finds nothing | `speckit-clarify` | Clarifications written to spec.md (merged with PR 2) |
| 3 | **Design** | `speckit-plan` → review · `speckit-tasks` → cut milestones · `speckit-analyze` → fix · review (tasks + milestone rubric) · close checklists | **PR 2**: clarified spec, plan, research, contracts, tasks with `## Milestones`, merged on green |
| 4 | **Milestones**, one at a time | the milestone loop below | One PR per milestone, each merged on green |
| 5 | **Close**, in this order | `speckit-converge` → `speckit-tdd-verify` → `speckit-docguard-guard` | Unbuilt behaviour from any of the three becomes a new milestone (back to 4, then rerun all three). Test-strength and docs findings that add no behaviour are fixed in the close PR. That PR sets spec `**Status**` to `Closed <date> — shipped in PRs #…`, gets a fresh-subagent review like every diff, merges, then the **handoff** |

`speckit-specify` offers up to 3 clarification questions of its own. Do not ask the user them in
Phase 1. Leave them as `[NEEDS CLARIFICATION]` markers and triage them in Phase 2.

Each artifact review loops fix → re-review up to **3 rounds**. A fourth round is an escalation. How
to dispatch a reviewer and the rubrics: [references/review-rubrics.md](references/review-rubrics.md).

### Phase 0: a bug report

1. **Reproduce on `origin/main`** with `superpowers:systematic-debugging`. Try the report's steps
   and the obvious variations: another OS arm, a fresh profile, several sessions. If it still does
   not reproduce, escalate (category 5) and ask for the missing detail. Never guess-fix.
2. **Find the owning spec**, the `specs/<NNN>-*` whose requirements cover the broken behaviour.
   - **None does** (the code predates specs, or the behaviour was never specified): switch to
     Phase 1 and specify the correct behaviour, citing the reproduction.
   - **The owning feature is still in flight**: its `**Status**` is not Closed, or its autopilot
     ledger is not `done`. That spec belongs to another flow. Escalate as *blocked by work outside
     my flow*, and include the reproduction.
3. **`speckit-bugfix-report`** writes `bugs/BUG-<k>.md` in the owning spec, with the root cause
   and any false completions. Start the ledger beside it.
4. **`speckit-bugfix-patch`**, then **`speckit-bugfix-verify`**. A fresh reviewer then checks the
   patch against the bug rubric in [references/review-rubrics.md](references/review-rubrics.md).
5. **Decide the size.** Switch to Phase 1 when the fix adds behaviour the spec never intended, or
   the patch adds more than 10 tasks. The new spec cites `BUG-<k>` as its input. Otherwise:
6. **Run one milestone** through the Phase 4 loop. Its first task is a regression test that fails on
   `origin/main` for the reported reason. The PR is titled `fix(NNN): … (BUG-<k>)`. No spec PR,
   design PR or close phase follows: after the merge, go to the handoff.

### Phase 2: clarify, triaged

`speckit-clarify` asks at most 5 questions per run, so run it repeatedly. Triage every question:

- **The repo settles it** (constitution, an earlier spec, a contract, existing code). Answer it
  yourself and record `- Q: … → A: … _(agent-resolved: <path>#<section>)_`.
- **Otherwise it is the user's decision.** Collect these across the round and ask them in **one**
  `AskUserQuestion` call (up to 4 questions, each with a `(Recommended)` option and its evidence),
  under the escalation banner. Record the answers as `_(decided by user)_`.

The phase is done when a run reports no critical ambiguities. A fifth round is an escalation.

### Phase 3: milestones and checklists

Cut milestones by [references/milestones.md](references/milestones.md). Every milestone ships a
**deliverable**, something observable on `main`. A phase such as "Setup" or "Foundational" is never
a milestone on its own.

`speckit-implement` stops on unchecked checklist items. Resolve that **here**, never there. For each
unchecked item, either:

- a reviewer subagent confirms the artifacts satisfy it, and you tick it; or
- you fix the spec or plan until they do; or
- it needs a decision, and you escalate it.

When `speckit-implement` asks "proceed anyway?", the answer is never "yes". Go back to Phase 3.

### Phase 4: the milestone loop

For milestone K:

1. `git switch -C <worktree-branch> origin/main`, and only once the previous PR reads `MERGED`.
2. `speckit-implement` with the argument `Milestone MK only: tasks T0xx–T0yy. Do not start any other
   task.` Its mandatory `tdd.run` hook drives red → green → refactor.
3. `mise run gate`. When `cfg(target_os)` code changed, also
   `cargo check --target aarch64-apple-darwin`. When anything visible changed, run the `visual-pass`
   skill.
4. **Code review. Both reviews run on every milestone, in parallel:**
   - **A** — the `code-review` skill at `high` on `origin/main...HEAD`.
   - **B** — a fresh subagent checks the diff against the milestone's deliverable, its acceptance
     scenarios and the constitution (see the rubric).

   Weigh each finding with `superpowers:receiving-code-review`. Fix the real ones and go back to
   step 3. A finding that contradicts the spec is declined, with the reason recorded in the ledger.
5. Tick the milestone's tasks, update the ledger, commit, `git push --force-with-lease`, and open the
   PR from [references/pr-and-merge.md](references/pr-and-merge.md).
6. Wait for `ci complete`. When it goes red, run `superpowers:systematic-debugging`, fix, and push
   (at most 3 attempts). When the PR has no checks, diagnose it (see the reference). When it goes
   green, run `gh pr merge <n> --rebase` and confirm `state` is `MERGED`.

## Ownership: only this flow's work

Other sessions work in this repo at the same time. You are responsible for:

- this worktree and its branch
- the feature directory (or BUG record) this flow created
- the PRs listed in the ledger
- the subagents you spawned

Everything else is outside the flow:

- **Other PRs.** Never list them, review them, merge them, rebase them, approve their runs, comment
  on them or close them, even when they are green.
- **Other specs.** Never edit another feature's `tasks.md` or spec. The one exception is the bug
  path's patch to a **Closed** owning spec, made through `speckit-bugfix-patch`.
- **Other worktrees and branches.** Never touch them.
- **A red `main`, or a failure caused by code this flow did not write.** Don't fix it. Escalate it
  as *blocked by work outside my flow*, and state what you checked.
- **A defect found in someone else's code.** Record it under *Follow-ups not done* in the handoff.
- **The worktree and its branch.** Never pass `--delete-branch`, and never delete either one. The
  user removes the worktree in micold IDE, and that cleans up the branch too.

## Asking the human

Every ask uses the banner and categories in [references/escalation.md](references/escalation.md),
together with a `PushNotification`. **Escalate only for:**

- a product or scope decision the repo does not settle
- a constitution conflict
- an irreversible or outward action beyond merging this flow's own PRs
- missing access
- non-convergence
- the plan proving false in a way that changes requirements

**Never escalate** review findings, fmt or clippy results, merge conflicts, flaky reruns, checkless
PRs, visual checks, or a choice between equivalent implementations. Handle those yourself.

## Handoff: the last message

Before writing it, verify all three:

- `git status --porcelain` is empty
- after `git fetch origin`, `git cherry origin/main HEAD | grep '^+'` prints nothing. A rebase-merge
  rewrites SHAs, so `git log origin/main..HEAD` still lists merged commits; `git cherry` matches
  them by patch instead.
- every PR in the ledger reads `MERGED`

If any check fails, report exactly what remains instead. Otherwise send this, and send it with a
`PushNotification`:

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
| "CI is green, so it's reviewed" | CI checks that the code works. Reviews A and B check it is the right code. Run both. |
| "I wrote the plan, I'll re-read it myself" | A self-review confirms what you meant. A fresh subagent reads what you wrote. |
| "These checklist items are just formalities, proceed" | Close them in Phase 3 or escalate. Never answer "proceed anyway". |
| "Setup is phase 1, so it's milestone 1" | A milestone ships a deliverable. Fold Setup and Foundational into the first user story. |
| "I'll delete the merged branch to tidy up" | The IDE owns cleanup. No `--delete-branch`. |
| "That green PR from another session is ready, I'll merge it" | It isn't in the ledger, so it isn't yours. |
| "main is red, I'll wait until it recovers" | Waiting silently stalls the flow. Escalate as *blocked by work outside my flow*. |
| "Quick question for the user…" | Batch it, add a recommendation, use the banner, or resolve it from evidence. |
| "I remember where I was" | The ledger and `gh pr view` say where you are. Memory doesn't. |
| "Everything merged. Done!" | Verify the three checks, then send the WORK COMPLETE handoff with the worktree line. |

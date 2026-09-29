---
name: speckit-autopilot
description: Use when the user hands over a feature idea or a bug report and wants the whole Spec Kit flow run end to end with as little of their involvement as possible — "autopilot", "run it autonomously", "take it all the way to main", "only ask me when you must" — or says to resume or continue an interrupted autopilot run.
argument-hint: "<feature description> | bug: <report> | resume"
user-invocable: true
---

# Spec Kit autopilot

One prompt in. The flow writes the spec, clarifies it, plans, cuts tasks, and ships reviewed
milestones to `main`. Ask the human **only** for decisions that are theirs. Batch every ask and make
it loud and specific.

**No human reviews anything here.** Every artifact and every diff gets a review by a
**fresh-context subagent**, never the context that wrote it. Green CI is not a review.

Follow the rules to the letter. A rule that looks slow is what stops an unattended flow shipping the
wrong thing.

## You are the orchestrator

This session keeps the ledger, dispatches units, asks the human, waits on CI and merges. It never
runs a phase skill, and never reads a phase file or a unit's transcript. Everything it reads is
re-read on every later call of the run.

Each phase's work runs in a **unit**: a fresh subagent that reads only [unit.md](unit.md) and its own
phase file. Overview and diagrams for humans: [README.md](README.md).

## Entry

| Argument | Start at |
|---|---|
| `resume` | Find this worktree's ledger (see *Resuming*), check each recorded PR's `state` with `gh pr view`, and continue at the first unfinished step. **Never** rebuild progress from `gh pr list` or from memory. |
| `bug: …`, or text describing broken behaviour | Bug unit |
| anything else | Spec unit |

First run `git fetch origin` and check the worktree is clean. The first unit creates the ledger from
[templates/autopilot-ledger.md](templates/autopilot-ledger.md): `autopilot.md` in the feature
directory, or `bugs/BUG-<k>.autopilot.md` beside the BUG record. Its **Worktree branch** is the exact
output of `git branch --show-current`; `resume` finds the ledger by it.

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

**When GitHub and the ledger disagree, GitHub is right.** Fix the ledger. A milestone marked merged
whose PR is open, or whose changes are missing from `origin/main`, goes back to a milestone unit.
If a commit on `main` reverted it, escalate (category 6). A non-empty *Open escalation* means the
question was never answered: ask it again, then dispatch a fresh unit of that phase with the answer.

## Phases and units

| # | Phase | Unit and file | Ends with |
|---|---|---|---|
| 0 | **Bug** | Bug: [phases/0-bug.md](phases/0-bug.md) | Patched BUG record, reviewed. Then one milestone unit ships record, patch, regression test and fix in **one PR**, then the **handoff**. Or the unit returns `SWITCH: feature` and the flow goes to Phase 1. |
| 1 | **Spec** | Spec: [phases/1-spec.md](phases/1-spec.md) | **PR 1**: the spec |
| 2 | **Clarify**, in rounds | Clarify round: [phases/2-clarify.md](phases/2-clarify.md) | Dispatch rounds until one returns `CLEAN` as its first summary line. Answers ship in PR 2. A fifth round is an escalation (category 5). |
| 3 | **Design** | Design: [phases/3-design.md](phases/3-design.md) | **PR 2**: clarified spec, plan, research, contracts, tasks with `## Milestones` |
| 4 | **Milestones**, one at a time | Milestone K: [phases/4-milestone.md](phases/4-milestone.md) | One PR per milestone |
| 5 | **Close** | Close: [phases/5-close.md](phases/5-close.md) | New milestones (back to 4, then close again), or the close PR, then the **handoff** |

Every PR merges on green before the next unit starts.

### Dispatching a unit

Run each unit in its own `general-purpose` subagent on the session model (omit `model`).

- **Description:** name the unit (`Milestone M2 042`, `Clarify round 3 042`). Token reports group by
  it.
- **Prompt:** keep this opening fixed for every unit, so the prompt cache reuses it:
  `You are a speckit-autopilot unit. Read .claude/skills/speckit-autopilot/unit.md and follow it.`
  Then the phase file, ledger path (`none yet` for the first unit), worktree path and branch, and
  the scope: for the first unit, the user's prompt verbatim; for a spec unit after a bug switch, the
  repro and correct behaviour the bug unit returned; for a milestone, its ID and task IDs (and
  `BUG-<k>` for a bug).
- **Return:** the unit ends with `STATUS: DONE | ESCALATE | FAILED`, a PR number if it opened one,
  and at most five lines. Read the ledger, not the transcript.
- **`ESCALATE`:** subagents have no `AskUserQuestion`. Ask the returned questions yourself (see
  *Asking the human*), then continue **the same** subagent with `SendMessage` and the answers.
- **`FAILED`:** read the ledger and the five lines. Retry once with a fresh unit, or escalate.

### Waiting and merging

After a unit returns `DONE` with a PR, follow [references/pr-and-merge.md](references/pr-and-merge.md)
§5–6: wait for `ci complete` in the background, then `gh pr merge <n> --rebase` and confirm `state`
is `MERGED`.

- **Red in this flow's code:** continue the unit that opened the PR with `SendMessage` and the failing
  log (at most 3 attempts). That subagent still holds the change's context.
- **Red outside this flow, or no checks:** handle it per the reference.

After the merge, record the merge SHA in the ledger and dispatch the next unit.

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

Before the checks, close the ledger: set **Phase** to `done`, record the last merge SHA, and copy
the Total row and model table of `mise run autopilot-tokens` into *Token usage*. Ship that through
one more PR (`docs(NNN): record the autopilot run`), merged on green.

If any check fails, report exactly what remains. Otherwise send this with a `PushNotification`:

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
| "This step is small, I'll do it here" | Everything the orchestrator reads is re-read on every later call. Dispatch the unit. |
| "I'll check the unit's work in its transcript" | Read the ledger and the five-line return. |
| "CI is green, so it's reviewed" | Units run reviews A and B. CI checks only that the code works. |
| "I'll delete the merged branch to tidy up" | The IDE owns cleanup. No `--delete-branch`. |
| "That green PR from another session is ready, I'll merge it" | Not in the ledger, not yours. |
| "main is red, I'll wait for it to recover" | Silent waiting stalls the flow. Escalate as *blocked by work outside my flow*. |
| "Quick question for the user…" | Batch it with a recommendation under the banner, or resolve it from evidence. |
| "I remember where I was" | The ledger and `gh pr view` say where you are. Memory does not. |
| "Everything merged. Done!" | Run the three checks, then send the WORK COMPLETE handoff. |

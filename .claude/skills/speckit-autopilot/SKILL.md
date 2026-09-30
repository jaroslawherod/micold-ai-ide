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
| `resume` | Run `scripts/autopilot/resume.sh` (see *Resuming*) and continue at the first unfinished step. **Never** rebuild progress from `gh pr list` or from memory. |
| `bug: …`, or text describing broken behaviour | Bug unit |
| anything else | Spec unit |

First run `git fetch origin` and check the worktree is clean. The first unit creates the ledger from
[templates/autopilot-ledger.md](templates/autopilot-ledger.md): `autopilot.md` in the feature
directory, or `bugs/BUG-<k>.autopilot.md` beside the BUG record. Its **Worktree branch** is the exact
output of `git branch --show-current`; `resume` finds the ledger by it.

### Resuming

Run `scripts/autopilot/resume.sh`. It finds this worktree's ledger (working tree first, where an
unpushed ledger is newest; then `origin/main`) and prints its phase, next step, open escalation, and
GitHub's state for each recorded PR.

| Exit | Meaning | Do |
|---|---|---|
| 0 | `LEDGER …` | Resume at the first unfinished step. `HANDOVER` lines: dispatch a unit of that phase to continue from the ledger's *Handover*. |
| 0 | `LEDGER-ON-MAIN …` | Run `scripts/autopilot/branch-start.sh`, then `resume.sh` again. |
| 2 | `NONE` | Say there is no run to resume here, and stop. |
| 3 | several ledgers | Ask with one `AskUserQuestion`: each option names a ledger's feature, phase and next step. Recommend the most recently committed one. |
| 4 | `RECORD-PR-PENDING <ledger> <pr\|none>` | The run finished but its record PR never merged. Wait on and merge `<pr>`; on `none`, dispatch the record unit with the ledger's last PR and its merge SHA. Then run the handoff. |

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
| 6 | **Record** | Record: [phases/6-record.md](phases/6-record.md) | The record PR, opened at the **handoff** |

Every PR merges on green before the next unit starts.

### Dispatching a unit

Run each unit in its own `general-purpose` subagent. Pick the model by the work, not the phase
name. A unit keeps its model when continued with `SendMessage`.

| Unit | `model` |
|---|---|
| Spec, clarify, design, bug, close, and a milestone the ledger does not mark **Docs-only** `yes` | omit (session model) |
| A milestone the ledger marks **Docs-only** `yes` | `"sonnet"` |
| Record | `"haiku"` |

A cheaper unit that returns `FAILED` is retried on the session model.

- **Description:** name the unit (`Milestone M2 042`, `Clarify round 3 042`). Token reports group by
  it.
- **Prompt:** keep this opening fixed for every unit, so the prompt cache reuses it:
  `You are a speckit-autopilot unit. Read .claude/skills/speckit-autopilot/unit.md and follow it.`
  Then the phase file, ledger path (`none yet` for the first unit), worktree path and branch, the
  previous PR and its merge SHA (none for the first unit), and the scope: for the first unit, the user's prompt verbatim; for a spec unit after a bug switch, the
  repro and correct behaviour the bug unit returned; for a milestone, its ID and task IDs (and
  `BUG-<k>` for a bug).
- **Return:** the unit ends with `STATUS: DONE | ESCALATE | FAILED | HANDOVER`, a PR number if it opened one,
  and at most five lines. Read the ledger, not the transcript.
- **`ESCALATE`:** subagents have no `AskUserQuestion`. Ask the returned questions yourself (see
  *Asking the human*), then continue **the same** subagent with `SendMessage` and the answers.
- **`FAILED`:** read the ledger and the five lines. Retry once with a fresh unit, or escalate.
- **`HANDOVER`:** its context passed 150k. Dispatch a fresh unit of the same phase, model and scope,
  with `part <n>` added to the description and `Continue from the ledger's Handover.` in the
  prompt. A fourth part for one unit is an escalation (category 5).

### Waiting and merging

After a unit returns `DONE` with a PR, run `scripts/autopilot/wait-merge.sh <n>` detached, and wait
on its last line (it runs as long as CI does):

```bash
log="$SCRATCHPAD/pr-<n>.log"
# The wrapper writes its own pid: `setsid` may fork, so `$!` is not the process to watch.
AUTOPILOT_LOG_DIR="$SCRATCHPAD" setsid nohup \
  bash -c 'echo $$ >"$0.pid"; scripts/autopilot/wait-merge.sh <n>; echo "WAIT_EXIT=$?"' "$log" \
  >"$log" 2>&1 &
# then, with run_in_background (stops too if the script was killed without a result):
until grep -q '^WAIT_EXIT=' "$log" || ! kill -0 "$(cat "$log.pid")" 2>/dev/null; do sleep 30; done
tail -6 "$log"
```

| Last line | Do |
|---|---|
| `MERGED <n> <sha>` | Run `scripts/autopilot/context.py`; on `OVER`, tell the user in one line that `/clear` then `/speckit-autopilot resume` would restart you small, and carry on. Dispatch the next unit with the PR and SHA; it records them in the ledger. Never edit the ledger yourself between units: `branch-start.sh` refuses a dirty tree. |
| `RED <n> <run> <log>` | In this flow's code: continue the unit that opened the PR with `SendMessage` and the log path (at most 3 attempts). Outside it: handle it per [references/pr-and-merge.md](references/pr-and-merge.md) §5. |
| `CHECKLESS <n> <reason>` | Handle the reason per the reference's *A PR with no checks*, then run the script again. |
| `MERGE-FAILED <n> <message>` | Fix per the reference's §6 table, then run the script again. |
| `CLOSED <n>` | Someone closed the PR. Escalate (category 1); never reopen it unasked. |
| `TIMEOUT <n> <what>` or no result line | Check `gh auth status` and the PR by hand, then run the script again. |

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

First dispatch the record unit with the last PR and its merge SHA. It closes the ledger and opens
the record PR; wait on it and merge it as any other. It is not listed in the ledger, so pass it to
the checks.

Then run `scripts/autopilot/handoff-check.sh <ledger> <record-pr>`. It checks the tree is clean,
every commit is on `origin/main` (by patch, since rebase-merge rewrites SHAs), and every PR reads
`MERGED`. If it prints `NOT DONE`, report exactly what remains. Otherwise send this with a `PushNotification`:

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
| "Everything merged. Done!" | Run `handoff-check.sh`, then send the WORK COMPLETE handoff. |

---
name: speckit-autopilot
description: Use when the user hands over a feature idea, a bug report, a chore or a GitHub issue and wants it run end to end with as little of their involvement as possible — "autopilot", "run it autonomously", "take it all the way to main", "only ask me when you must" — or says to resume or continue an interrupted autopilot run.
argument-hint: "#<issue> | <flow> [high|low] #<issue> | <feature description> | bug: <report> | chore: <task> | resume"
user-invocable: true
---

# Spec Kit autopilot

One prompt or one issue in; reviewed work merged to `main` out. Ask the human **only** for
decisions that are theirs. **No human reviews anything here:** every artifact and diff is reviewed
by a fresh-context subagent, never the context that wrote it. Green CI is not a review.

This file only routes. Each step is a small file: read the one the table names when its moment
comes, and no other. Follow each to the letter; a rule that looks slow is what stops an
unattended flow shipping the wrong thing.

## You are the orchestrator

This session chooses the flow, keeps the issue, dispatches units, asks the human, waits on CI and
merges. It never runs a Spec Kit skill and never reads a unit's task files or transcript.
Everything it reads is re-read on every later call: batch independent probes into one message.

Each unit is a fresh subagent that reads [rules/unit.md](rules/unit.md) and the task files its
flow lists. Overview for humans: [README.md](README.md).

## Route

First `git fetch origin`, and check the worktree is clean.

| When | Read |
|---|---|
| The argument is `resume` | [tasks/resume.md](tasks/resume.md) |
| Any other argument: `#<n>` or an issue URL (`gh issue view <n> --json title,body,labels`), `<flow> [high\|low] #<n>`, or text. Choose the flow and the effort from the command, the labels and the text | [flows/choose.md](flows/choose.md), then only the chosen one of [flows/bug.md](flows/bug.md), [flows/bugfix.md](flows/bugfix.md), [flows/feature.md](flows/feature.md), [flows/chore.md](flows/chore.md) |
| The flow is chosen: claim or open the issue, write the labels back | [tasks/issue.md](tasks/issue.md) |
| Starting a unit, and when it returns | [tasks/dispatch.md](tasks/dispatch.md) |
| A unit returned a PR | [tasks/merge.md](tasks/merge.md) |
| CI red outside this flow, a PR with no checks, a refused merge | [tasks/ci.md](tasks/ci.md) |
| A unit returned `NEXT:`, or passed its flow's cap | [flows/switch.md](flows/switch.md) |
| A unit returned `ESCALATE`, or the flow is not plain | [tasks/ask.md](tasks/ask.md) |
| The run's last PR merged | [tasks/handoff.md](tasks/handoff.md) |
| Unsure whether something is this flow's to touch | [rules/ownership.md](rules/ownership.md) |

## Red flags: stop and re-read the file the table names

| Thought | Reality |
|---|---|
| "This step is small, I'll do it here" | Everything the orchestrator reads is re-read on every later call. Dispatch the unit. |
| "It's labelled `bug`, but I'd call it a feature" | Labels win over your reading. They disagree with the text: ask once. |
| "I'll run the full flow to be safe" | The lighter flows exist because the heavy steps found nothing on small work. Follow the flow chosen. |
| "I'll check the unit's work in its transcript" | Read the ledger and the five-line return. |
| "CI is green, so it's reviewed" | Units run the reviews. CI checks only that the code works. |
| "That green PR from another session is ready, I'll merge it" | Not in the ledger, not yours. No `--delete-branch` either. |
| "main is red, I'll wait for it to recover" | Silent waiting stalls the flow. Escalate as *blocked by work outside my flow*. |
| "Quick question for the user…" | Batch it with a recommendation under the banner, or resolve it from evidence. |
| "I remember where I was" | The ledger and `gh pr view` say where you are. Memory does not. |
| "Everything merged. Done!" | Run the handoff task: check, token report, close the issue, WORK COMPLETE. |

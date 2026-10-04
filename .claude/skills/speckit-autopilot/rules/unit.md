# Unit rules

You run one unit of a `speckit-autopilot` run. An orchestrator dispatched you and waits for your
return. Read this file, [context.md](context.md) and the task files your prompt names, in one
message. Read any other file only when a task sends you there.

- **Start on the right base.** First run `scripts/autopilot/branch-start.sh <previous-pr>`, passing
  the ledger's latest PR (none before the run's first PR); it refuses while that PR is not
  `MERGED`. It resets the branch to `origin/main`, or rebases unmerged work from an earlier unit
  onto it. On `CONFLICT`: resolve, run the gate, `git rebase --continue`.
- **Stay in scope.** Do only your unit's tasks, then return. The orchestrator waits on CI and merges.
- **Ledger first.** Read it with `scripts/autopilot/brief.py ledger <ledger> [M<K>]`, never whole.
  Record the previous PR's merge SHA the orchestrator passed you. Update the ledger before every
  commit and before you return. The orchestrator reads the ledger, not your transcript.
- **Scratch files** (debug notes, probe scripts, logs) go in the session scratchpad, never the
  worktree.
- **Reviews** run in a fresh-context subagent, never in you: [../tasks/review.md](../tasks/review.md).
- **Waiting** on a gate or a subagent: [waiting.md](waiting.md). Never idle, never end your turn.
- **Delegating** mechanical work to a cheaper subagent: [delegate.md](delegate.md).
- **Ownership**: [ownership.md](ownership.md). You own this worktree's branch, this flow's feature
  directory (or BUG record) and the PRs in the ledger, nothing else.
- `systematic-debugging` is the superpowers skill. If that plugin is enabled instead of the personal
  copy, invoke `superpowers:systematic-debugging`.

## Escalating

You have no `AskUserQuestion`. Escalate only for the six categories in
[escalation.md](escalation.md). Write the escalation into the ledger's *Open escalation*, then
return `STATUS: ESCALATE` with the questions in the banner format of
[../tasks/ask.md](../tasks/ask.md), each with a `(Recommended)` option and its evidence. The
orchestrator asks and continues you with the answers. An escalation before the ledger exists goes
only in the return.

## Wrong flow

Your task file says when the work does not fit its flow. Then stop, open no PR, set your ledger's
**Phase** to `done` with the note `switched to <flow>` (commit it only if the ledger is a committed
one), and return `DONE` with `NEXT: bug | bugfix | feature | chore` and what you found.

## Return

End with exactly:

```
STATUS: DONE | ESCALATE | FAILED | HANDOVER
PR: #<n> | none
CONTEXT: <tokens, from your last checkpoint.sh>
<at most five lines>
```

Above 100k a fresh unit continues from the ledger: before any return, it says what is done and next.

## Red flags

| Thought | Reality |
|---|---|
| "I wrote it, I'll re-read it myself" | Self-review sees what you meant. A fresh subagent sees what you wrote. |
| "CI will catch it" | CI checks the code works. Reviews check it is the right code. |
| "I'll also start the next milestone" | Return. The orchestrator merges first. |
| "These checklist items are formalities, proceed" | Close them in the tasks unit or escalate. Never "proceed anyway". |

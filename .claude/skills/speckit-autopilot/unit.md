# Autopilot unit

You run one unit of a `speckit-autopilot` run. An orchestrator dispatched you and waits for your
return. Read this file and your phase file. Load a file in `references/` only when your phase file
sends you there.

## Rules

- **Start on the right base.** First run `scripts/autopilot/branch-start.sh <previous-pr>`, passing the
  ledger's latest PR (none for the first unit); it refuses while that PR is not `MERGED`. It resets the branch
  to `origin/main`, or rebases unmerged work from an earlier unit (clarify rounds, a BUG record,
  close-phase milestones) onto it. On `CONFLICT`, resolve, run the gate, `git rebase --continue`.
- **Stay in scope.** Do only your unit's work, then return. The orchestrator waits on CI and merges.
- **Ledger first.** Record the previous PR's merge SHA the orchestrator passed you. Update the
  ledger before every commit and before you return. The orchestrator
  reads the ledger, not your transcript.
- **Scratch files** (debug notes, probe scripts, logs) go in the session scratchpad, never the
  worktree.
- **Reviews.** Every artifact and diff gets a review by a **fresh-context subagent**, never you. Give
  it a description that names the review (`Review B M2 042`). Loop fix → re-review for up to
  **3 counted rounds** (the rubric file says what counts); still BLOCKER or MAJOR after the third
  is an escalation. Dispatch and rubrics:
  [references/review-rubrics.md](references/review-rubrics.md).
- **PRs.** Open them per [references/pr-and-merge.md](references/pr-and-merge.md) §2–4. Record the PR
  number in the ledger at once.
- **Batch tool calls.** Each call re-reads your whole context, so a call saved is worth more than
  a line of output saved.
  - Before a call, ask what else you will need that does not depend on its result, and put all of
    it in the same message: several `Read`s, a `grep` beside a `git diff --stat`.
  - Dependent shell steps go in one `Bash` call: `git status -sb; git log --oneline -3; ls dir`.
  - `grep -n -C5 <pattern> <file>` shows the lines in one call; `Read` after `grep` only when you
    need more than that.
  - At a checkpoint, `scripts/autopilot/checkpoint.sh` (below) is the one probe you need.
- **Wait once.** Your prompt cache expires after 5 idle minutes; the next call then re-writes your
  whole context. Run work that needs no build (a review, a subagent) while the gate builds, and
  wait for what runs together in one wait, not one after another. A job waiting on the build lock
  idles too: do not start it until the build is done.
- **Read only what you need.** Everything you read is re-read on each later call of the unit.
  - Spec artifacts: `scripts/autopilot/brief.py section <file> <heading>` or `items <file> <ID>…`,
    not the whole file.
  - Code: `grep -n` for the lines, then `Read` with `offset`/`limit`.
  - Finding code across many files: dispatch an `Explore` subagent (it runs on Haiku) and keep
    its answer, not the file dumps.
  - A saved tool result (`…/tool-results/…`), a gate log or an agent `.output` file: `grep` it;
    never `Read` it whole.
  - Diffs: `git diff --stat` first, then one file at a time.
  - `gh`: `--json <fields> -q <filter>` for just the fields you need.
- **Delegate mechanical work to a cheaper subagent.** A subagent starts with a small, fresh
  context, and only its answer enters yours. Delegate a task that takes several calls or reads a lot
  of output, and that needs no design judgment:

  | Task | Subagent and `model` |
  |---|---|
  | Finding code, tracing a call path, listing uses of a symbol | `Explore` (Haiku) |
  | A multi-step git job: rebase onto `origin/main` and resolve conflicts in files this flow owns, find which commit changed a line, compare branches by patch | `general-purpose`, `"sonnet"` |
  | A straightforward implementation task in a `full` milestone: tasks.md or the BUG record names the file and the change, and it copies a pattern that exists in the repo | `general-purpose`, `"sonnet"`, with the task ID, the files, the pattern to copy and the test that must pass. Run that test yourself afterwards. |
  | Checking static text: a doc or checklist against the spec, cross-references between spec artifacts, a log or report summarised to its failures | `general-purpose`, `"sonnet"` (`"haiku"` for a pure search or count) |

  Keep one-command operations (`git status`, a single commit or push) in your own context: a
  subagent costs more than one call. Keep design choices, debugging and review findings on your own
  model. Tell the subagent exactly what to return and in how many lines, and check any change it
  made with `git diff --stat`.
- **Hand over at 150k.** A large context makes every later call costly. At each checkpoint (a
  finished step, a gate or review round) run
  `scripts/autopilot/checkpoint.sh "<description>" <ledger>` with the exact description your prompt
  gives you. It prints branch, changed files, unmerged commits, the ledger's phase, next step and
  open handover or escalation, and last `context.py`'s `CONTEXT` line, with its exit code. On `OVER`: write *Handover* in the ledger (what is
  done, the next step, open findings, your PR if you opened one), commit, push only if your PR is
  already open, and return `STATUS: HANDOVER`. A fresh unit of the same phase continues from it.
  Exit 2 means the check cannot run: say so in your return's lines and carry on.
- **Continuing a handover.** Your prompt says so: read the ledger's *Handover*, carry on from its
  next step, and set the section back to `None.` in your first commit. If it names your unit's own
  open PR, skip `branch-start.sh`: stay on the branch as it is. Review rounds already counted in
  *Review rounds* stay counted (the rubric's *Round limit* says what counts).
- `systematic-debugging` is the superpowers skill. If that plugin is enabled instead of the personal
  copy, invoke `superpowers:systematic-debugging`.

## Ownership

You own this worktree's branch, this flow's feature directory (or BUG record), and the PRs in the
ledger. Never edit another feature's spec or `tasks.md` (only exception: the bug path's patch to a
**Closed** owning spec, via `speckit-bugfix-patch`). Never touch other PRs, worktrees or branches.
Never pass `--delete-branch`. A failure in code this flow did not write, or a red `main`: do not fix
it; escalate as *blocked by work outside my flow*. A defect in someone else's code goes under
*Follow-ups not done* in the ledger.

## Escalating

You have no `AskUserQuestion`. Escalate only for the six categories in
[references/escalation.md](references/escalation.md). Write the escalation into the ledger's *Open
escalation*, then return `STATUS: ESCALATE` with the questions in that file's banner format, each
with a `(Recommended)` option and its evidence. The orchestrator asks and continues you with the
answers. An escalation before the ledger exists goes only in the return.

**Never escalate** review findings, fmt or clippy results, merge conflicts, flaky reruns, visual
checks, or a choice between equivalent implementations. Handle those yourself.

## Return

End with exactly:

```
STATUS: DONE | ESCALATE | FAILED | HANDOVER
PR: #<n> | none
<at most five lines>
```

## Red flags

| Thought | Reality |
|---|---|
| "I wrote it, I'll re-read it myself" | Self-review sees what you meant. A fresh subagent sees what you wrote. |
| "CI will catch it" | CI checks the code works. Reviews check it is the right code. |
| "I'll also start the next milestone" | Return. The orchestrator merges first. |
| "These checklist items are formalities, proceed" | Close them in Phase 3 or escalate. Never "proceed anyway". |

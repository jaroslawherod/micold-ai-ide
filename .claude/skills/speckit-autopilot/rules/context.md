# Keeping your context small

Every call re-reads your whole context, so a call saved is worth more than a line of output saved,
and everything you read is re-read on each later call.

## Batch tool calls

- Before a call, ask what else you will need that does not depend on its result, and put all of it
  in the same message: several `Read`s, a `grep` beside a `git diff --stat`.
- Dependent shell steps go in one `Bash` call: `git status -sb; git log --oneline -3; ls dir`.
- `grep -n -C5 <pattern> <file>` shows the lines in one call; `Read` after `grep` only when you
  need more than that.
- A hook message starting `autopilot batching:` means your last calls were one read each. Batch.

## Read only what you need

- Spec artifacts: `scripts/autopilot/brief.py section <file> <heading>` or `items <file> <ID>…`,
  not the whole file.
- Code: `grep -n` for the lines, then `Read` with `offset`/`limit`. A hook blocks a `Read` without
  `limit` of a file over 400 lines.
- Finding code across many files: dispatch an `Explore` subagent (it runs on Haiku) and keep its
  answer, not the file dumps.
- A saved tool result (`…/tool-results/…`), a gate log or an agent `.output` file: `grep` it;
  never `Read` it whole.
- Diffs: `git diff --stat` first, then one file at a time.
- `gh`: `--json <fields> -q <filter>` for just the fields you need.

## Hand over at 150k

At each checkpoint (a finished step, a gate or review round) run
`scripts/autopilot/checkpoint.sh "<description>" <ledger>` with the exact description your prompt
gives you. It prints branch, changed files, unmerged commits, the ledger's phase, next step and
open handover or escalation, and a `CONTEXT` line. It is the one probe you need at a checkpoint.

- `OVER`: write *Handover* in the ledger (what is done, the next step, open findings, your PR if
  you opened one), commit, push only if your PR is already open, and return `STATUS: HANDOVER`. A
  fresh unit of the same kind continues from it.
- Exit 2: the check cannot run. Say so in your return's lines and carry on.
- A hook message starting `autopilot context:` means you are over the cap. Finish the step in
  hand and hand over then; do not wait for the next checkpoint.

## Continuing a handover

Your prompt says so. Read the ledger's *Handover*, carry on from its next step, and set the section
back to `None.` in your first commit. If it names your unit's own open PR, skip `branch-start.sh`:
stay on the branch as it is. Review rounds already counted in *Review rounds* stay counted.

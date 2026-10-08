# Running a prompt in parallel

**Run in parallel** sends one prompt to several AI CLIs at once so you can compare how each tackles
it. You choose 2 to 8 runs, each with its own AI CLI. Every run gets its own
[worktree](worktrees-and-sessions.md) and branch, and a session in that worktree that receives your
prompt once.

## Starting a parallel run

Click **Run in parallel** in the sidebar header, right beside **Add a worktree**. A dialog opens
with these fields:

- **Prompt** — what every run is asked to do.
- **Type** — the kind of change (for example `feat`), used to name the branches.
- **Ticket** (optional) — a ticket reference to include in the names.
- **Name** — a short name for the group, for example `login-page`.
- **Base branch** — the branch each run starts from. It defaults to the project root's current
  branch.
- **Runs** — a numbered list (#1, #2, ...). Each row has a provider select, which offers only the
  AI CLIs installed on this computer, and a remove button. **Add run** adds another row.

Under the list the dialog shows the names it will create, for example
`feat/login-page-1 → .claude/worktrees/feat-login-page-1`: the branch, then the worktree folder.

**Start runs** stays disabled while something is wrong, and the dialog shows the first problem:

- the prompt is empty;
- no type is chosen;
- the name is empty;
- there are fewer than 2 or more than 8 runs;
- a run's provider is not available;
- a derived branch or worktree name is not valid.

## The group in the sidebar

The runs appear as one **group row** named after the group, with tags such as `3 runs` and
`1 failed`. Click it to collapse or expand the group; the counts stay visible when it is collapsed.

Each run row shows `#<n>` and its provider. The runs are listed only inside the group, not again
among the ordinary worktrees. A run row keeps the usual worktree menu (right-click), so you can
review its changes, open its sessions or delete it like any other worktree.

## When a run fails

The other runs carry on; one failure never stops the rest.

- A failed run is marked as failed, and its reason is the row's tooltip.
- A run that failed before its worktree existed is still listed, with a `failed` tag and no
  worktree menu, since there is nothing to manage.
- If a run's session started but the prompt could not be delivered, the run keeps its worktree and
  session and says "prompt not delivered" with the reason. You can open the session and paste the
  prompt yourself.

## Restarting the app

Groups are remembered. After a restart every group, its runs and the run order are as they were.

A run that was still being created when the app (or the service) stopped cannot finish by itself.
It reads **Failed** with the reason "interrupted", and the half-made folder or branch of that run is
removed. Runs that had finished are untouched. A folder is left alone, and the reason says so, when
it holds something the app did not make for that run (a session record names it, for example).

## Deleting a run

Delete a run's worktree from its own row, as for any worktree. The run leaves the group and the
group keeps the rest. A group with no runs left disappears. A group that lost run #2 shows `#1` and
`#3`; numbers are never reused.

## Dismiss group

Right-click a group row and choose **Dismiss group**. The app asks first. Dismissing forgets only the
grouping: the worktrees, branches and sessions of its runs stay, and appear as ordinary rows.
**Cancel** changes nothing.

## Coming later

Comparing the runs, **Pick this one**, and the offer to clean up the other runs come in later
releases of this feature.

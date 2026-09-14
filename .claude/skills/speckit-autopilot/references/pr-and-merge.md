# From a green local gate to merged on `main`

Every PR in the flow (spec, design, each milestone, close) goes through the same path.

## 1. Branch

Every PR comes from **this worktree's own branch**. The IDE cleans up exactly that branch when the
user removes the worktree, so the flow never creates other branches.

```bash
git fetch origin
gh pr view <previous-pr> --json state -q .state      # must print MERGED (skip for PR 1)
git switch -C "$(git branch --show-current)" origin/main
```

Starting from `origin/main` each time is deliberate. A rebase-merge rewrites SHAs, so stacking the
next change on the old local commits leaves GitHub trying to replay commits that are already on
`main`. It refuses with `This branch can't be rebased`.

## 2. Local gate

```bash
log="$SCRATCHPAD/gate-$(date +%s).log"
setsid nohup bash -c 'mise run gate; echo "GATE_EXIT=$?"' >"$log" 2>&1 &
```

- **Detach it.** The build lock can queue this behind another worktree's build for a long time, and
  a plain background task can be killed while it waits. Use `Monitor` with an until-loop on
  `grep -q GATE_EXIT= "$log"`, then read the exit code.
- **It runs CI's order:** fmt → clippy (core, then workspace) → `cargo test --workspace` →
  `scripts/tests/*.test.sh`. `mise run test` alone is not the gate: CI stops at fmt first.
- **Changed a `cfg(target_os = …)` arm?** Also run
  `scripts/build-lock.sh cargo check --workspace --target aarch64-apple-darwin`. Only this host's
  target is built locally.
- **Changed how something looks?** Run the `visual-pass` skill and save its evidence in the spec
  directory.
- **Docs- and specs-only PRs** (PR 1, PR 2, close) skip the cargo steps. Run
  `scripts/tests/*.test.sh` only.

## 3. Commit and push

- **Conventional commits, scoped by feature number:**
  - `feat(NNN): …` for new behaviour
  - `fix(NNN): … (BUG-NNN)` for a bug fix
  - `test(NNN): …` for tests
  - `docs(NNN): …` for spec artifacts
- End each message with the session attribution line from the system reminder.
- Update `autopilot.md` in the same commit that finishes the step.
- Push with `git push --force-with-lease -u origin HEAD`.

## 4. Open the PR

| PR | Title |
|---|---|
| Spec | `docs(NNN): specify <feature>` |
| Design | `docs(NNN): clarify, plan and cut milestones for <feature>` |
| Milestone | `feat(NNN): <deliverable, imperative>`, or `fix(NNN): …` for a bug |
| Close | `docs(NNN): close the spec` |

The title prefix matters:
- release-please builds the changelog from it
- CI's user-guide gate fires on `feat`

Create the PR from a body file:

```bash
gh pr create --base main --title "<title>" --body-file "$SCRATCHPAD/pr-body.md"
```

Milestone body:

```markdown
## Milestone M<K> of <total> — <feature> (specs/<NNN>-<slug>)

**Deliverable**: <from tasks.md>
**Verify**: <from tasks.md — the command/test/quickstart section>
**Tasks**: T0xx–T0yy · **Satisfies**: <scenarios, FRs>

## Agent review
- Review A (code-review, high): <n findings, n fixed, n declined — one line each>
- Review B (conformance): <verdict; Verify output excerpt>

## Local gate
`mise run gate` passed at <sha>. <macOS cross-check / visual-pass lines, if run>

<session attribution URL>
```

Record the PR number in the ledger right away.

## 5. Wait for `ci complete`

`ci complete` is the only required check. Watch it in the background instead of polling:

```bash
gh pr checks <n> --required --watch --fail-fast
```

Run it with `run_in_background`. You are notified when it exits.

| Result | Action |
|---|---|
| **Green** | Merge (step 6). No confirmation needed. |
| **Red, in this flow's code** | Read the failing job's log (`gh run view <run-id> --log-failed`). Run `superpowers:systematic-debugging`, fix, re-run the gate, push. After the third failed attempt, escalate (category 5). |
| **Red, outside this flow's code** (a test or file this flow never touched) | First rerun the failed jobs once: `gh run rerun <run-id> --failed`. If it passes, that was a flake, so carry on. If it fails again, look at `main`'s latest completed runs (`gh run list --branch main --status completed --limit 3`). Whether or not `main` shows the same failure, don't fix it. Escalate as *blocked by work outside my flow*, with the evidence. |
| **No checks at all** | See below. |

### A PR with no checks

Diagnose in this order:

1. `gh pr view <n> --json state,mergeable`
   - `state` is `MERGED`: already done, move on.
   - `mergeable` is `CONFLICTING`: GitHub cannot build a merge ref, so no workflow fires. Run
     `git fetch origin && git rebase origin/main`, resolve the conflicts, re-run the gate, then
     `git push --force-with-lease`.
2. `gh run list --branch <branch> --limit 5`: runs with conclusion `action_required` are waiting for
   approval. Approve **only runs on this flow's PR**:
   `gh api -X POST repos/{owner}/{repo}/actions/runs/<id>/approve`.
3. None of the above means an Actions outage dropped the event:
   `gh pr close <n> && gh pr reopen <n>`. The SHA stays the same and no force-push is needed.

## 6. Merge

```bash
gh pr merge <n> --rebase          # never --delete-branch; never --admin
gh pr view <n> --json state -q .state   # trust this, not the exit status
```

| Problem | Fix |
|---|---|
| `This branch can't be rebased` while the PR reads `MERGEABLE` | The branch carries commits already on `main`. Run `git rebase origin/main` (git skips them), then `git push --force-with-lease`. Wait for green again. |
| The branch has a merge commit from `main`, so it cannot be replayed | Squash the green head: `gh api -X PUT repos/{owner}/{repo}/pulls/<n>/merge -f merge_method=squash -f sha=<head>`. |
| "base branch policy prohibits the merge" | Usually a missing `workflow` token scope on a PR that touches `.github/workflows`. Escalate (category 4) with the command the user needs to run: `gh auth refresh -s workflow`. |

After merging, record the merge SHA in the ledger. That record is committed with the next PR.

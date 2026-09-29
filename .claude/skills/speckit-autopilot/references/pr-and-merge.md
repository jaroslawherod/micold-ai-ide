# From a green local gate to merged on `main`

Every PR (spec, design, each milestone, close) follows this path.

## 1. Branch

Use **this worktree's own branch** for every PR. Never create other branches.

```bash
scripts/autopilot/branch-start.sh    # prints RESET, REBASED <n>, DIRTY or CONFLICT
```

Unmerged work from an earlier unit (clarify rounds, a BUG record) is rebased, not dropped. Otherwise
start from `origin/main`. Rebase-merge rewrites SHAs, so stacking on old commits fails with
`This branch can't be rebased`.

## 2. Local gate

```bash
log="$SCRATCHPAD/gate-$(date +%s).log"
setsid nohup bash -c 'mise run gate; echo "GATE_EXIT=$?"' >"$log" 2>&1 &
```

- **Detach it.** A plain background task can be killed while it waits on the build lock. Use
  `Monitor` with an until-loop on `grep -q GATE_EXIT= "$log"`, then read the exit code.
- **Order is CI's:** fmt → clippy (core, then workspace) → `cargo test --workspace` →
  `scripts/tests/*.test.sh`. `mise run test` alone is not the gate.
- **Changed a `cfg(target_os = …)` arm?** Also run
  `scripts/build-lock.sh cargo check --workspace --target aarch64-apple-darwin`.
- **Changed how something looks?** Run the `visual-pass` skill. Save its evidence in the spec
  directory.
- **Docs- and specs-only PRs** (PR 1, PR 2, and a close PR that touches no code or tests): run `scripts/tests/*.test.sh` only.

## 3. Commit and push

- Use conventional commits, scoped by feature number: `feat(NNN): …` (behaviour),
  `fix(NNN): … (BUG-NNN)` (bug), `test(NNN): …`, `docs(NNN): …` (spec artifacts).
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

Keep the prefix exact: release-please builds the changelog from it, and CI's user-guide gate fires
on `feat`.

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

Record the PR number in the ledger at once.

## 5. Wait for `ci complete`

The orchestrator runs `scripts/autopilot/wait-merge.sh <n>`, which does this section and §6 and
prints one result line. The rules it follows, and what to do by hand when it stops:

`ci complete` is the only required check. Watch it in the background. Do not poll:

```bash
until gh pr checks <n> --required 2>/dev/null | grep -q 'ci complete'; do sleep 15; done
gh pr checks <n> --required --watch --fail-fast; echo "CHECKS_EXIT=$?"
```

Run it with `run_in_background`.

- **Wait for the check to appear first.** Right after a push, `--watch` prints
  `no required checks reported` and exits **0**. That is not green.
- **Merge only when `ci complete` reads `pass`.** No checks after about 5 minutes means the PR is
  checkless (see below).

| Result | Action |
|---|---|
| **Green** | Merge (step 6). No confirmation needed. |
| **Red, in this flow's code** | Read the log (`gh run view <run-id> --log-failed`). Run `systematic-debugging`, fix, re-run the gate, push. After the third failed attempt, escalate (category 5). |
| **Red, outside this flow's code** | Rerun once: `gh run rerun <run-id> --failed`. Passes: flake, carry on. Fails again: check `main` (`gh run list --branch main --status completed --limit 3`). Either way, don't fix it. Escalate as *blocked by work outside my flow*, with the evidence. |
| **No checks at all** | See below. |

### A PR with no checks

Diagnose in this order:

1. `gh pr view <n> --json state,mergeable`
   - `state` is `MERGED`: done.
   - `mergeable` is `CONFLICTING`: no workflow can fire. Run `git fetch origin && git rebase origin/main`, resolve
     conflicts, re-run the gate, `git push --force-with-lease`.
2. `gh run list --branch <branch> --limit 5`: runs with conclusion `action_required` await approval.
   Approve **only runs on this flow's PR**:
   `gh api -X POST repos/{owner}/{repo}/actions/runs/<id>/approve`.
3. Otherwise an Actions outage dropped the event: `gh pr close <n> && gh pr reopen <n>`.

## 6. Merge

```bash
gh pr merge <n> --rebase          # never --delete-branch; never --admin
gh pr view <n> --json state -q .state   # trust this, not the exit status
```

| Problem | Fix |
|---|---|
| `This branch can't be rebased` while the PR reads `MERGEABLE` | Commits already on `main`. Run `git rebase origin/main` (git skips them), `git push --force-with-lease`, wait for green. |
| Branch has a merge commit from `main` | Squash the green head: `gh api -X PUT repos/{owner}/{repo}/pulls/<n>/merge -f merge_method=squash -f sha=<head>`. |
| "base branch policy prohibits the merge" | Usually a missing `workflow` token scope on a PR touching `.github/workflows`. Escalate (category 4) with the user's command: `gh auth refresh -s workflow`. |

After merging, record the merge SHA in the ledger. It is committed with the next PR.

# Task: commit, push and open the PR

When: the gate for this diff is green ([gate.md](gate.md)) and its reviews are done. Use **this
worktree's own branch** for every PR. Never create other branches.

1. **Commit.** Conventional commits: `feat(NNN): …` (behaviour), `fix(NNN): … (BUG-<k>)`,
   `test(NNN): …`, `docs(NNN): …` (spec artifacts); a bug or chore flow scopes by area
   (`fix(terminal): …`). End each message with the session attribution line from the system
   reminder. Update the ledger in the same commit that finishes the step.
2. **Push:** `git push --force-with-lease -u origin HEAD`.
3. **Open it:** `gh pr create --base main --title "<title>" --body-file "$SCRATCHPAD/pr-body.md"`.
   The body ends with `Refs #<issue>`, never `Closes`: the first merge would close the issue.
4. Record the PR number in the ledger at once.

| PR | Title |
|---|---|
| Design | `docs(NNN): specify and plan <feature>` |
| Milestone | `feat(NNN): <deliverable, imperative>` |
| Bugfix | `fix(NNN): <what now works> (BUG-<k>)` |
| Bug | `fix(<area>): <what now works>` |
| Chore | `ci\|build\|test\|chore\|refactor\|docs(<area>): …` |
| Close | `docs(NNN): close the spec` |

Keep the prefix exact: release-please builds the changelog from it, and CI's user-guide gate fires
on `feat`. `NNN` is the spec directory's number.

Milestone and bugfix body (bug and chore: *Why* and how it was proven, then the last two sections):

```markdown
## Milestone M<K> of <total> — <feature> (specs/<NNN>-<slug>)

**Deliverable**: <from tasks.md>
**Verify**: <from tasks.md — the command/test/quickstart section>
**Tasks**: T0xx–T0yy · **Satisfies**: <scenarios, FRs>

## Agent review
- Review A (code-review, <level>): <n findings, n fixed, n declined — one line each>
- Review B (conformance, sonnet): <verdict; Verify output excerpt, or "not run in this flow">

## Local gate
`mise run gate` passed at <sha>. <macOS cross-check / sandbox / visual-pass lines, if run>

Refs #<issue>
<session attribution URL>
```

Hands on: return `DONE` with the PR number. The orchestrator waits on CI and merges.

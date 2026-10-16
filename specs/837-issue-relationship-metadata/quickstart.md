# Quickstart: validating issue relationship metadata (837)

## Part A: automated

Prerequisites: the workspace builds (`mise run gate`, or `scripts/gate.sh` in the cloud). No `gh`
and no network are needed, because every test stubs GitHub with `FakeIssueSource` or a recorded
fixture.

| Run | Proves |
|---|---|
| `mise run test-core` (`cargo test -p micold-core --test issue_details`) | The parse, counts, caps, hidden items, cross-repository references, PR dedupe, per-kind not readable, neutralising, `to_json` omission and display lines. Contracts [details-read.md](contracts/details-read.md), [tool-result.md](contracts/tool-result.md), [form-details.md](contracts/form-details.md) §3 |
| `cargo test -p micold-daemon --test mcp_create_worktree` | US1 1-6, US3-1, US3-2 and US3-4 through the real tool path. A hanging fake under a short bound returns in time with `details_error`. A refused call has no `issue`. 550/532 cases unchanged (SC-004) |
| `cargo test -p micold-client` (form and shell tests) | US2-1 to US2-5 and US3-3 at the reducer level. The read starts only on a GitHub pick (SC-006). A stale answer is dropped (FR-012). Create does not wait (US2-4) |

Expected: all green. `mise run duplication` stays within budget (the shared invisible-character rule
is extracted, not copied).

## Part B: manual / visual pass (GUI glue)

Prerequisites: a signed-in `gh` (`gh auth status`), and a github.com repository you can write to
that has these issues:

- **I-blocked**: blocked by one open and one closed issue. It has a parent, two sub-issues (one
  closed), an assignee, a milestone, a project with a Status value, and a draft PR that closes it.
- **I-plain**: an issue with none of that.

Run the client (`mise run run`, or the `visual-pass` skill on the showcase or client).

1. Open the project, then New worktree, then **GitHub issue**, and pick I-blocked. Within a few
   seconds, rows appear below the picker. "Blocked by" has a red label and reads "Blocked: 1 open of
   2: #… (open), #… (closed)". Parent, Sub-issues ("1 of 2 closed: …"), Closing pull requests
   ("(draft)"), Assignees, Milestone and Projects follow, in that order. **Create** is enabled.
2. Pick I-plain. The I-blocked rows disappear and none appear for I-plain.
3. Pick I-blocked and press **Create** at once. The worktree is created without waiting. The
   sidebar row looks as it does for any issue worktree (US2-6).
4. Run `gh auth refresh -r` without `read:project` (or use a token without it), then pick
   I-blocked. Projects reads "Not readable: …" and the other rows show.
5. Disconnect the network, then pick an issue. One caption reads "Couldn't read this issue's
   details. …". **Create** still works.
6. Check light and dark themes: the warning label is legible in both.

Agent path: in a session, ask the agent to call `create_worktree` with `github_issue: <I-blocked>`.
The result has `issue.blocked: true` and the lists above. With I-plain, it has `"issue": {"number": N}`
only.

Record the pass (screenshots, or `visual-pass` notes) in the ledger of the PR that ships the view.

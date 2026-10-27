# Contract: location title

The behaviour each test pins. IDs are cited by tasks and tests.

## L1 Order (FR-001, US2.1–2.5)

`choose` returns the first source with a usable text, in this order: GitHub issue title (ticket
`Github`), Jira summary (ticket `Jira`), open/draft pull request title, spec heading, main session
label, own commit subject, branch, display name. A source whose text normalises to nothing (L4) is
skipped. Only one issue source can apply: the title ticket is either a number or a key.

## L2 Title ticket (FR-002, US1.1, US1.3, edge cases)

| dir_name | branch | ticket |
|---|---|---|
| `feat-813_overview` | `feat/900_x` | `Github(813)` (folder wins) |
| `project-thread-t2a600` | `feat/1504-readable-titles` | `Github(1504)` (short-branch rule) |
| `project-thread-t2a600` | `feat/1504_readable-titles` | `Github(1504)` |
| `feat-ABC-42_login` | any | `Jira("ABC-42")` |
| `wt` | `feat/abc-42_login` | `Jira("ABC-42")` |
| `wt` | `fix/some_bug` | none (`some` is neither) |
| `claude-project-thread-7041o6` | `claude/project-thread-7041o6` | none |
| `wt` | none (detached) | none |

The linked ticket (sidebar issue tag, Jira chip, Jira status keys) is unchanged: folder only.

## L3 Tooltip words (FR-008, US2.6)

| Source | Words |
|---|---|
| GitHub issue #n | `From GitHub issue #n` |
| Jira KEY | `From Jira KEY` |
| Pull request #n | `From pull request #n` |
| Spec | `From the feature spec` |
| Session | `From the session title` |
| Commit | `From the latest commit` |
| Branch | `Branch name` |
| Folder | none: today's tooltip |

Surfaces (research R6): sidebar row tooltip (line after the title), session-area header title
tooltip (line after the title), expanded Overview card (line under the title, beside the display
name). The folder name stays on each (row `Name:`/`Folder:` lines; header and card display name).

## L4 Normalisation (FR-012, edge cases)

- BOM stripped; first line only; trailing `\r` stripped; trimmed; empty → no title.
- Spec: first `# ` line; `Feature Specification:` prefix removed; `[FEATURE NAME]` or any
  `[...]`-only text → no title; missing/unreadable/huge (`task_progress` size limit) file → none.

## L5 GitHub title reads and retry (FR-003, FR-009, US3)

- Asked on the existing occasions only (`request_titles` callers). Seeds from the New-worktree
  form's held issues first (unchanged).
- With switch off or no route: nothing asked, nothing recorded (unchanged).
- A number is asked when it has no entry, or is `Failed { at }` with `now - at >= 300 s`. Never
  while `Pending`; never again after `Known` or `NoIssue`.
- While `Pending` or `Failed`, the card shows the next source (L1), never a loading text.

## L6 Jira summaries (FR-004)

- One request per read, on the existing triggers (`Open`, `Focus`), behind `jira_request`.
- Asked keys = linked keys (status + summary) ∪ branch-only Jira title tickets (summary only),
  validated `<PROJECT>-<digits>` of the connection's project.
- Status stored for linked keys only; summary for all asked keys; failure keeps the last summary
  until the board retires its entries; `clear` drops all.

## L7 Spec and commit facts (FR-006, SC-005, edge cases)

- Read once per worktree listing (`features::worktree::State.listing` bump), per valid worktree,
  on a blocking thread; never on the UI thread.
- Commit: merge-base with the default branch (`review_base`), `log -1 --format=%s base..HEAD`;
  none without a branch, without a base, with an empty range, on error or timeout.
- Not read without local git (`caps.shared_git()` none).
- An answer is dropped when its project is not active, its dir is not listed, or a newer
  `listing` is stored for its dir.

## L8 No new reading (FR-005, SC-004)

Pull request, session and branch inputs are read from state at render. The only added network
reads are GitHub titles for branch tickets, their retries, and the summary field / branch-only keys
in the existing Jira request. `tests/issues_are_requested_only_on_named_events.rs` and
`tests/jira_is_requested_only_on_named_events.rs` keep passing unchanged in their caller counts.

## L9 Privacy (FR-011)

No title text in any `Debug`, log line or outgoing request other than the reads themselves.

# Contract: key rows, ticket and type for a Jira pick

**Feature**: [spec.md](../spec.md) | Research [R1](../research.md#r1--reuse-the-034038-pipeline-with-a-second-provider), [R8](../research.md#r8--matching-order-between-issue-type-and-labels)

## 1. Row and ticket

| | GitHub | Jira |
|---|---|---|
| `title_line` | `#12 Add login` | `ABC-123 Add login rate limit` |
| `row_text` (ranked) | `#12 Add login  ·  reporter  ·  l1, l2` | `ABC-123 Add login rate limit  ·  reporter  ·  l1, l2` |
| ticket (`Issue::ticket`) | `12` | `ABC-123` |
| name | `name_from_title(title)` (50 chars) | `name_from_title(summary)` |
| tooltip | description, redacted in `Debug` | same |

`naming::naming_for_issue` calls `issue.ticket()`. The branch is `${type}/${KEY-123}_${name}`; the
sidebar tag comes from `parse_tags`/`issue_label`, which already upper-case a Jira key (FR-015).
A key `naming.rs` does not accept as a tag yields the ticket text anyway and the worktree is
created (Edge "Key that naming cannot read"): covered by a test with a 1-letter project key.

## 2. Type

`type_for_issue(mapping, type_name, labels) = type_for_labels(mapping, [type_name] ++ labels)`.
First mapping entry in list order whose trimmed, lower-cased label equals any of them wins (FR-013);
none -> type left to the user. `Issue::type_name()` is `None` for GitHub, so GitHub picks are
unchanged. The Settings **GitHub issues** section intro gains: "Also applies to Jira issues:
an issue's type name and labels are matched against this list." (FR-014); the default entries
are unchanged, so `Bug` -> `fix`.

## 3. Pick

`IssuePicked { number, mapping }` is unchanged: the reducer finds the issue by `number` in
loaded/searched, sets `ticket = issue.ticket()`, `name = name_from_title(title)`, `type_ =
type_for_issue(...)`, clears `error`, closes the list (034 FR-009 to FR-014, FR-010a).

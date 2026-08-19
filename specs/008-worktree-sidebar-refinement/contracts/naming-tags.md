# Contract: Naming — Friendly Name & Tag Derivation

**Module**: `src/naming.rs` (pure, no I/O). Consumers: `src/app.rs` (`worktree_tree`),
`src/ui/sidebar.rs`, filter predicate.

## Functions

```rust
/// The boundary between a ticket and the descriptive name. `slugify` maps every
/// non-alphanumeric character to '-', so a derived name can never contain one.
pub const TICKET_SEP: char = '_';

/// Human-friendly display name derived from a worktree directory name.
/// Strips the leading ConventionalType token and the ticket, turns separators into
/// spaces, and sentence-cases. Falls back to a readable form of `dir_name` if the
/// descriptive remainder is empty.
pub fn display_name(dir_name: &str) -> String;

/// Tags parsed from a worktree directory name: at most one Type, at most one Issue.
/// Order: Type first (if the leading token is a known ConventionalType), then Issue.
/// A Status tag is NOT added here (it is injected at render time from WorktreeStatus).
pub fn parse_tags(dir_name: &str) -> Vec<Tag>;
```

`ConventionalType` and its `as_str()`/parse already exist and are reused unchanged.

## Rules

- **Type token**: the segment before the first `-` is a Type tag iff it equals a known
  `ConventionalType` token (`feat|fix|chore|docs|refactor|test|build|ci|perf|style`). Otherwise
  no Type tag (worktree is "untyped").
- **Ticket**: everything between the Type token and the FIRST `TICKET_SEP`, verbatim. A name
  with no `TICKET_SEP`, or nothing before it, has no ticket — nothing infers one from the
  *shape* of a segment (BUG-003). Rendered as `#123` when it is all digits (GitHub/GitLab
  issue number) and upper-cased otherwise (`abc-123` → `ABC-123`).
- **Friendly name**: everything after the first `TICKET_SEP`, or — when there is none —
  everything after the Type token. Join the segments with spaces, sentence-case (first letter
  upper, rest as-is).

### Why the boundary rather than a pattern (BUG-003)

The earlier rule matched any lowercase word followed by any all-digit word. `feat-abc-123`
(a bare Jira ticket) and `feat-reporting-2` (a name with a disambiguator) are the same string
pattern, so the rule read `REPORTING-2` out of the second, emptied its descriptive remainder,
and fell back to a label with the type token still in it. It also could not see a GitHub-style
`#123` at all, because `slugify` leaves `123`, which fails "starts with a letter" — the ticket
was silently discarded and its digits leaked into the name.

No sharper pattern exists; the shapes are identical. So the name carries the answer instead.

### What a branch says

Branches carry `TICKET_SEP` too, so `dir_name_from_branch` recovers a ticket exactly rather than
guessing at it: a branch this app derived comes back through the existing-branch picker with its
ticket intact. The inverse never *invents* a boundary — a branch written without one yields a
directory without one.

The cost is that a `snake_case` branch from outside this app reads as ticketed (`fix/some_bug` →
chip `SOME`, name "Bug"). `_` means one thing everywhere and nothing can tell the two apart.

## Examples (become `tests/naming.rs` cases)

| `dir_name` | `display_name` | `parse_tags` |
|------------|----------------|--------------|
| `feat-abc-123_login-page` | `Login page` | `[Type(Feat), Issue("ABC-123")]` |
| `feat-123_login-page` | `Login page` | `[Type(Feat), Issue("#123")]` |
| `fix-crash-on-open` | `Crash on open` | `[Type(Fix)]` |
| `chore-bump-deps` | `Bump deps` | `[Type(Chore)]` |
| `feat-reporting-2` | `Reporting 2` | `[Type(Feat)]` |
| `feat-abc-123` | `Abc 123` | `[Type(Feat)]` (no boundary, no ticket) |
| `my-experiment` | `My experiment` | `[]` (untyped) |
| `main` | `Main` | `[]` (untyped) |
| `abc-123_login-page` | `Login page` | `[Issue("ABC-123")]` (untyped, ticketed) |
| `feat_login-page` | `Login page` | `[Type(Feat)]` (boundary, nothing before it) |
| `feat-abc-123_` | `Feat abc 123`* | `[Type(Feat), Issue("ABC-123")]` |

\* When nothing descriptive remains, fall back to a readable form of `dir_name`; exact fallback
text is asserted in tests.

## Invariants

- Pure and deterministic; identical input ⇒ identical output.
- Never mutates or reads the branch, folder, or git state.
- `display_name` output is never empty.

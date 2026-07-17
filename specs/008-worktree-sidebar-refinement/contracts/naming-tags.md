# Contract: Naming — Friendly Name & Tag Derivation

**Module**: `src/naming.rs` (pure, no I/O). Consumers: `src/app.rs` (`worktree_tree`),
`src/ui/sidebar.rs`, filter predicate.

## Functions

```rust
/// Human-friendly display name derived from a worktree directory name.
/// Strips the leading ConventionalType token and any Jira-style issue key,
/// turns '-' into spaces, and sentence-cases. Falls back to `dir_name` if the
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
- **Issue key**: first match of `\b[A-Z][A-Z0-9]+-\d+\b` (case-insensitive match, value
  upper-cased). At most one.
- **Friendly name**: remove the Type token (if any) and the Issue key (if any) from the
  segments, join remaining segments with spaces, sentence-case (first letter upper, rest as-is).

## Examples (become `tests/naming.rs` cases)

| `dir_name` | `display_name` | `parse_tags` |
|------------|----------------|--------------|
| `feat-abc-123-login-page` | `Login page` | `[Type(Feat), Issue("ABC-123")]` |
| `fix-crash-on-open` | `Crash on open` | `[Type(Fix)]` |
| `chore-bump-deps` | `Bump deps` | `[Type(Chore)]` |
| `my-experiment` | `My experiment` | `[]` (untyped) |
| `main` | `Main` | `[]` (untyped) |
| `feat-ABC-123` | `Feat ABC 123`* | `[Type(Feat), Issue("ABC-123")]` |

\* When the only descriptive content is the type/issue, the remainder may be empty → fall back
to a readable form of `dir_name`; exact fallback text is asserted in tests.

## Invariants

- Pure and deterministic; identical input ⇒ identical output.
- Never mutates or reads the branch, folder, or git state.
- `display_name` output is never empty.

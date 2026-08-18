# Contract: Worktree Naming Derivation

**Feature**: 005-worktree-session-terminal | Pure core (`src/naming.rs`). No I/O.

Single source of truth for mapping add-worktree form inputs to a directory name and a git
branch name (FR-006, FR-006a). Kept in one place so it can become user-configurable later
without touching the creation flow.

## Inputs

- `type_`: one of the Conventional-Commits vocabulary (FR-005a). Required.
- `ticket`: optional free text (FR-005b).
- `name`: required free text.

## Slugify (pure)

`slugify(&str) -> String`:
1. Lowercase.
2. Replace every char not in `[a-z0-9]` with `-`.
3. Collapse consecutive `-`; trim leading/trailing `-`.
4. Guard git/OS tails: strip trailing `.lock`; reject results that are empty, `..`, `@`, or a
   Windows reserved device name (`con`, `prn`, `aux`, `nul`, `com1..9`, `lpt1..9`).

Output alphabet `[a-z0-9-]` is valid as BOTH a git ref component (git check-ref-format) and a
cross-OS directory name.

**`slugify` can never emit `_`.** That is what makes `_` usable as the ticket boundary below
(BUG-003): a character the derivation cannot produce is free to mean exactly one thing.

## Derivation

```
type_str = lowercase Conventional type (e.g. "feat")
t = slugify(ticket)   // when ticket provided and non-empty after slug
n = slugify(name)

with ticket:     dir_name = "{type_str}-{t}_{n}"   branch = "{type_str}/{t}-{n}"
without ticket:  dir_name = "{type_str}-{n}"        branch = "{type_str}/{n}"
```

The directory carries the `_` boundary and the branch does not. The directory is this app's own
name for the worktree and is what the sidebar re-reads to recover the ticket
(`008-worktree-sidebar-refinement/contracts/naming-tags.md`), so it is worth making unambiguous.
The branch is pushed, reviewed and matched by CI branch filters, so it keeps the shape everyone
already expects.

The consequence is an asymmetry: `dir_name_from_branch` (feature 016, FR-014) cannot recover a
ticket, because the branch never carried the boundary. A worktree created from a selected branch
is therefore ticketless. See the naming-tags contract for why guessing is the worse option.

### Examples

| type | ticket | name | dir_name | branch |
|------|--------|------|----------|--------|
| feat | ABC-123 | Login page | `feat-abc-123_login-page` | `feat/abc-123-login-page` |
| chore | (none) | cleanup | `chore-cleanup` | `chore/cleanup` |
| fix | #42! | Race/cond | `fix-42_race-cond` | `fix/42-race-cond` |

## Validation (`Result<DerivedNames, NamingError>`)

- `NoType` — no type selected.
- `EmptyNameAfterSlug` — `name` slugifies to empty (FR-008).
- `InvalidBranchRef` — assembled `branch` fails git check-ref-format (defense in depth;
  slug output should never hit this).

Collision errors (`DuplicateDir`, `DuplicateBranch`) are NOT decided here — they require live
git state and are returned by the create orchestration (see git-trait.md). This module only
derives + validates shape.

## Guarantees (test targets — SC-003b)

- Deterministic: same inputs → same `DerivedNames`.
- Ticket omitted ⟺ no empty separator in either output, and no `_` in `dir_name`.
- Ticket present ⟺ exactly one `_` in `dir_name`, and none in `branch`.
- Derived `dir_name` never contains `/`; derived `branch` contains exactly one `/` (after type).

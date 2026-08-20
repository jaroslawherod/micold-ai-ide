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

with ticket:     dir_name = "{type_str}-{t}_{n}"   branch = "{type_str}/{t}_{n}"
without ticket:  dir_name = "{type_str}-{n}"        branch = "{type_str}/{n}"
```

Both carry the `_` boundary, so `dir_name_from_branch` (feature 016, FR-014) recovers the ticket
exactly instead of guessing where it ended. The branch is the durable artifact — it outlives the
directory, gets pushed, and comes back through the existing-branch picker — so the boundary has to
be on it for a re-picked branch to keep its ticket.

`dir_name_from_branch` therefore slugifies *around* the boundary rather than through it: each `/`
segment is split on `_`, each part slugified, and the parts rejoined with `_`. A part that
slugifies to nothing is dropped along with its boundary, so a directory never begins or ends on
one.

The cost is that a `snake_case` branch from outside this app reads as ticketed — `fix/some_bug`
becomes `fix-some_bug`, chip `SOME`, name "Bug". `_` means one thing everywhere and nothing can
tell the two apart. One wrong chip on a foreign branch, against every app-made branch losing its
ticket the moment it is re-picked.

### Examples

| type | ticket | name | dir_name | branch |
|------|--------|------|----------|--------|
| feat | ABC-123 | Login page | `feat-abc-123_login-page` | `feat/abc-123_login-page` |
| chore | (none) | cleanup | `chore-cleanup` | `chore/cleanup` |
| fix | #42! | Race/cond | `fix-42_race-cond` | `fix/42_race-cond` |

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
- Ticket omitted ⟺ no empty separator in either output, and no `_` in either.
- Ticket present ⟺ exactly one `_` in each of `dir_name` and `branch`.
- `dir_name_from_branch(derive(x).branch) == derive(x).dir_name` for every `x` — exact, ticket
  included.
- Derived `dir_name` never contains `/`; derived `branch` contains exactly one `/` (after type).

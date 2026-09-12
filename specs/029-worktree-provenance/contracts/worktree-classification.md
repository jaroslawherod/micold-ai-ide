# Contract: Worktree Classification from Provenance

Governs the pure core function that decides whether a discovered worktree belongs to the user or to
an AI assistant. **This contract supersedes
[`014-hide-agent-worktrees/contracts/agent-worktree-classification.md`](../../014-hide-agent-worktrees/contracts/agent-worktree-classification.md)**
in its entirety for classification; that contract's `State` visible-set section survives with one
substitution, restated in §5 below.

Normative statement of FR-004, FR-005, FR-007, FR-007a, FR-007b, FR-011, FR-012 and FR-017
(research R4, R5).

## 1. API (`micold-core/src/worktree.rs`)

```rust
/// Who a worktree belongs to. Unchanged from 014, including its rationale: an enum, not a bool,
/// so a future third owner is an added variant rather than a refactor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeOwner {
    /// The app holds a provenance record for it — created here, migrated, or claimed. Always listed.
    User,
    /// The app holds no record for it. Hidden unless the reveal control is on (014 FR-002).
    Agent,
}

/// Everything classification needs about one project. Built once per refresh, not per worktree.
pub struct ProvenanceView<'a> {
    pub records: &'a BTreeSet<String>,
    pub state_unreadable: bool,
}

impl ProvenanceView<'static> {
    /// A readable project the app has created nothing in.
    pub fn none() -> Self;
    /// A project whose records could not be read (FR-011).
    pub fn unreadable() -> Self;
}

/// Classify one worktree against one project's provenance (FR-004). Derived, never stored.
pub fn classify_owner(worktree: &Worktree, provenance: &ProvenanceView<'_>) -> WorktreeOwner;
```

**Amended during implementation: no `root` field.** The draft carried
`pub root: &'a Path` so rule 2 below could compare `worktree.path.parent()` against it. It is
redundant and strictly more dangerous than the flag already on the worktree: `reconcile()` sets
`included` for every worktree outside the managed root and refuses to set it for one inside, so
`!worktree.included` **is** "directly under the root" — the same fact, already computed, and not
re-derivable wrongly by a caller that passes the wrong path. Rule 2 is stated in terms of `included`
accordingly. The classification table in §5 is unchanged: its "location" column now describes which
value `included` carries rather than which path was passed in.

`Worktree::owner()` and `Worktree::is_agent_owned()` are **removed**. Not deprecated, not kept as
shims: every call site must be made to supply provenance, and a surviving name-based method is the
one way FR-007 could silently fail to take effect. Removal is what turns "the naming rule no longer
hides anything" into a compile error rather than a code review.

## 2. Normative rule

A worktree is `Agent` **iff all three** hold:

1. `provenance.state_unreadable` is `false`, **and**
2. `worktree.included` is `false` — the worktree sits *directly* under the project's managed
   worktrees directory, which is exactly what `reconcile()` withholds that flag for, **and**
3. `provenance.records` does **not** contain `worktree.dir_name`.

Otherwise it is `User`.

Consequences, stated because each is a requirement in its own right:

- **FR-005** — a worktree anywhere else is `User` unconditionally. Location is checked by the
  function, not assumed as a precondition of the caller; 014 could document it as a precondition
  because `reconcile()` was the only source, and `included` worktrees (016 BUG-002) since made that
  untrue.
- The blocked-branch sentence (016 FR-032) classifies a `WorktreeRecord` git reported, which
  `reconcile()` never saw and which therefore carries no `included` flag. It reaches the same rule
  through a private helper taking `(dir_name, under_managed_root, provenance)`, so the list and the
  branch picker cannot disagree about one directory — BUG-001 by the other door.
- **FR-011** — an unreadable project classifies every one of its worktrees `User`, whatever
  `records` holds. This is checked **first**, and an empty `records` is never read as "hide
  everything".
- **FR-007** — no name is consulted. `AGENT_DIR_PREFIX`, `AGENT_BRANCH_PREFIX` and `is_agent_id()`
  survive in the file but have exactly one caller, `matches_reserved_convention()` (§3), reachable
  only from the migration.
- **FR-007b** — a record outranks everything: a worktree the app created is `User` even when its
  name imitates the reserved convention, because the name is not an input.
- **FR-017** — the project root ("Default") is not a worktree and never reaches this function.

## 3. The naming convention's one remaining caller

```rust
/// 014's reserved naming convention, surviving only as the FR-007a veto on the FR-006 backfill.
///
/// `true` iff `dir_name` is `"agent-"` + an agent id, or `branch` is `Some("worktree-agent-" +
/// an agent id)`. An **agent id** is >= 16 characters, all ASCII hex. Prefixes match
/// case-sensitively; the id accepts either hex case.
///
/// NOT reachable from `classify_owner`. Calling it from a hiding decision reintroduces FR-007.
pub fn matches_reserved_convention(dir_name: &str, branch: Option<&str>) -> bool;
```

014's truth table is carried over **unchanged** as this function's test corpus — all fourteen rows,
including the 16-vs-15 boundary pair and the case-sensitivity pair. They pinned the rule against a
"simplification" back to bare prefix matching under 014 and they pin it here for the same reason;
only the column heading changes, from `Owner` to `Vetoed`.

## 4. Invariants

1. **Pure**: no I/O, no `fs`, no git, no clock, no global state. Same inputs → same output.
2. **Total**: defined for every `Worktree`, including `branch: None`, every `WorktreeStatus`, and
   `included: true`.
3. **Health-blind**: `Valid`, `Missing` and `Invalid` classify identically (014 FR-007, spec edge
   case "registered but missing").
4. **Non-mutating**: takes `&Worktree`; nothing on disk is read or written (FR-003, FR-018, SC-005).
5. **Stateless**: no caching, no memoisation, no persisted classification (FR-012 — re-evaluated on
   every refresh).
6. **Name-blind**: the function body contains no reference to `AGENT_DIR_PREFIX`,
   `AGENT_BRANCH_PREFIX`, or `is_agent_id`. A test asserts the classification of two worktrees
   identical but for a reserved-convention name, one recorded and one not, and requires the *record*
   to decide both.

## 5. Truth table (normative test corpus)

"root" in the location column means directly under `<project>/.claude/worktrees`, i.e.
`included: false`; anything else is `included: true`. `records` is the project's set.

| `dir_name` | path parent | `records` | `state_unreadable` | Owner | Why |
|---|---|---|---|---|---|
| `feat-login` | root | `{feat-login}` | false | `User` | Recorded — the ordinary created case |
| `feat-login` | root | `{}` | false | `Agent` | Unrecorded under the root — US1, the whole feature |
| `feat-login-refactor` | root | `{other}` | false | `Agent` | An assistant session worktree is a name like any other |
| `agent-deadbeefdeadbeef` | root | `{agent-deadbeefdeadbeef}` | false | `User` | FR-007b — a record outranks the convention |
| `agent-deadbeefdeadbeef` | root | `{}` | false | `Agent` | Unrecorded, as 014 also concluded — by a different route |
| `agent-foo` | root | `{}` | false | `Agent` | 014 called this `User`; the name no longer matters (FR-007) |
| `elsewhere` | *outside* root | `{}` | false | `User` | FR-005 — never hidden on provenance grounds |
| `nested` | root`/sub` | `{}` | false | `User` | Not *directly* under the root |
| `feat-login` | root | `{}` | **true** | `User` | FR-011 — fail visible |
| `feat-login` | root | `{feat-login}` | **true** | `User` | Unreadable short-circuits before records are read |
| `missing-wt` (status `Missing`) | root | `{}` | false | `Agent` | Health-blind |
| `included-wt` (`included: true`) | *outside* root | `{}` | false | `User` | Inclusion is an explicit wish (016 BUG-002) |

The rows that must not be "simplified" away: the two `agent-*` rows (they are what proves FR-007 and
FR-007b took effect), both `state_unreadable` rows (FR-011 in both record states), and the two
location rows (FR-005 checked, not assumed).

## 6. `State` visible-set API (`micold-client/src/features/worktree.rs`)

014's contract for this section stands, with one substitution:

```rust
impl State {
    /// All worktrees while the reveal control is on; only user-owned ones while it is off
    /// (014 FR-002/FR-003).
    pub fn visible_worktrees(&self) -> impl Iterator<Item = &Worktree>;
}
```

- The filter predicate changes from `!w.is_agent_owned()` to
  `classify_owner(w, &self.provenance_view()) == WorktreeOwner::User`.
- All three of 014's invariants are re-asserted verbatim: **superset ordering preserved**,
  **identity when revealed**, **non-destructive**.
- All of 014's **required consumers** still read from it: `worktree_tree()`,
  `available_tag_filters()`, the sidebar's empty-state hint.
- All of 014's **required non-consumers** still do not: `set_worktrees()`'s pruning and
  `sessions_in_worktree()` reason about existence, not visibility. A hidden worktree still exists,
  and neither its rename override **nor its provenance record** may be pruned by them.

`ProvenanceView` is built by a private `State::provenance_view()` for the active project. Building it
per call is deliberate and cheap: it borrows three things the state already owns and it must reflect
the current refresh (FR-012).

## 7. Out of scope for this contract

Discovery is unchanged, exactly as under 014: `parse_worktrees()`, `reconcile()` and the daemon's
`refresh_worktrees()` gain no filtering step and no provenance parameter. Any design that drops
unrecorded worktrees before they reach `State::worktree.worktrees` violates FR-016 and makes both
the reveal control and the FR-025 count unimplementable.

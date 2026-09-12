# Phase 1 Data Model: Worktree Provenance

**Feature**: 029-worktree-provenance | **Date**: 2026-09-03 | **Plan**: [plan.md](./plan.md)

Three pieces of state are added and one existing type is re-shaped. Nothing is added to
`micold_core::worktree::Worktree` — see [research R4](./research.md#r4).

---

## 1. The provenance record

```rust
// micold-core/src/workspace.rs — beside `worktree_names`
pub struct Workspace {
    // …
    /// Worktrees this app created for the user, or the user claimed, per project (029 FR-001,
    /// FR-021). Keyed like `worktree_names`: project path → the worktree's `dir_name`.
    ///
    /// Presence *is* the classification (FR-004). Absence means only "not known to be the
    /// user's" — never "known to be the assistant's", which is why an unreadable project
    /// (§4) must not be read as an empty set.
    pub worktree_provenance: BTreeMap<PathBuf, BTreeSet<String>>,
}
```

| Property | Value | Requirement |
|---|---|---|
| Key | project path (lexically canonicalised, as every other per-project map) | FR-002 |
| Value | `dir_name` of a worktree directly under `worktrees_root(project)` | FR-002, FR-005 |
| Written by | worktree creation, the FR-006 migration, a claim | FR-001, FR-006, FR-021 |
| Removed by | worktree delete; project forget | FR-009 |
| Never written by | rename, session start, discovery, hiding, reveal | FR-008, FR-006d, FR-018 |

### Accessors (mirroring `worktree_name` / `set_worktree_name` / `clear_worktree_name`)

```rust
impl Workspace {
    /// Whether the app holds a provenance record for `dir_name` in the active project.
    pub fn is_user_created(&self, dir_name: &str) -> bool;
    /// Record a worktree as the user's (idempotent).
    pub fn record_user_created(&mut self, project: &Path, dir_name: &str);
    /// Drop a worktree's record (idempotent); removes the project's entry when it empties.
    pub fn forget_user_created(&mut self, project: &Path, dir_name: &str);
}
```

`Workspace::forget()` gains `self.worktree_provenance.remove(&path);` alongside its existing
`worktree_names` / `included_worktrees` removals — the second half of FR-009.

### Persistence

```rust
// micold-core/src/store.rs — StoredProjectState
#[serde(default)]
created_worktrees: Vec<String>,     // sorted; the BTreeSet flattened
#[serde(default)]
provenance_migrated: bool,          // §3
```

Both are `#[serde(default)]`, so the file schema is compatible in both directions and no version
bump is required. `projects.json` itself is untouched: provenance is per-project state and has no
legacy-catalog seed to migrate from, unlike `worktree_display_names`.

---

## 2. Ownership classification (derived, never persisted)

```rust
// micold-core/src/worktree.rs
pub enum WorktreeOwner { User, Agent }   // unchanged from 014

/// Everything classification needs about one project, gathered once per refresh.
pub struct ProvenanceView<'a> {
    /// The project's provenance records; empty is a legitimate value meaning "none yet".
    pub records: &'a BTreeSet<String>,
    /// The project's stored state could not be read this run (§4). When true, `records` is
    /// meaningless and every worktree classifies as `User` (FR-011).
    pub state_unreadable: bool,
}

pub fn classify_owner(worktree: &Worktree, provenance: &ProvenanceView<'_>) -> WorktreeOwner;
```

Normative rules and the truth table live in
[`contracts/worktree-classification.md`](./contracts/worktree-classification.md). In short:
`Agent` iff the worktree sits directly under the project's managed root, the project's state was
readable, and `records` does not contain its `dir_name`.

**Amended during implementation:** the drafted `root: &'a Path` field is gone. `reconcile()` already
computes "is this directly under the managed root" and stores the answer as `Worktree::included`
(inverted), so a `root` field would be a second copy of that fact that a caller could get wrong —
the location test is `!worktree.included`. The contract carries the full reasoning.

`Worktree::owner()` and `Worktree::is_agent_owned()` are **removed**, not deprecated: every call site
must be forced to supply provenance, and a method that silently keeps classifying from names is the
one way FR-007 could quietly fail to take effect.

### State transitions

```text
                 create (FR-001)  ┌──────────────┐  delete (FR-009)
        ─────────────────────────▶│  user-owned  │──────────────────▶ (no record)
                 claim  (FR-020)  │  (recorded)  │  forget (FR-009)
        ─────────────────────────▶└──────────────┘──────────────────▶ (no record)
              migration (FR-006)  ▲
        ─────────────────────────┘   once per project, evidence-based, veto-gated

  (no record) ──── reveal / session start / rename ────▶ (no record)   ← FR-006d, FR-008
```

There is no transition *out of* user-owned other than deletion or forget: FR-024 adds no un-claim.

---

## 3. The migration marker

```rust
// micold-core/src/workspace.rs
/// Projects whose one-time FR-006 evidence backfill has run (029 FR-006c).
pub provenance_migrated: BTreeSet<PathBuf>,
```

| Property | Value |
|---|---|
| Set | once, in the same write that persists the backfilled records (FR-006c) |
| Cleared | never, except by forgetting the project |
| Read | once per `refresh_worktrees()` for the project, to decide whether to run |
| Not set when | the project's state was unreadable that run (FR-011), or the project has no records file yet *and* the backfill has not run — the marker records that the pass **happened**, not that it found anything |

A project whose backfill found nothing still gets the marker: "ran and found no evidence" and "never
ran" must not be the same state, or every launch would re-collect evidence and FR-006d would fail.

---

## 4. The read-failure flag (derived, never persisted)

```rust
// micold-core/src/workspace.rs
/// Projects whose per-project state file failed to load this run (029 FR-011). Derived at
/// load time, never persisted, never sent on the wire as durable state.
pub unreadable_projects: BTreeSet<PathBuf>,
```

Populated where `load_project_state()` returns `ProjectStateLoad::Corrupt` (`store.rs:633`), which
today drops the project's records and continues silently. That silence is safe under 014 and unsafe
here: an empty record set would mean *hide everything*. See [research R5](./research.md#r5).

Effects, all of them FR-011's own words:

1. `ProvenanceView::state_unreadable` is true ⇒ every worktree in the project classifies `User`.
2. The FR-006 migration does not run for that project.
3. `provenance_migrated` is not set for that project, so the next successful read still gets its one
   backfill.

A create or claim the user performs during that run writes its record normally — the prohibition is
on the *failed read* causing writes, not on the user acting.

---

## 5. Wire projection

```rust
// micold-core/src/protocol/messages.rs — WorktreeSnapshot
/// Whether the app holds a provenance record for this worktree (029 FR-004). The daemon owns
/// the durable answer; the client reconstitutes its own map from these, as it does display names.
pub user_created: bool,
```

```rust
// ClientMsg
WorktreeClaim { req: u64, project: PathBuf, dir_name: String },
```

Both are specified in [`contracts/provenance-store.md`](./contracts/provenance-store.md) §5 and
[`contracts/claim-and-reveal.md`](./contracts/claim-and-reveal.md) §1.

---

## 6. Client-side derived values

```rust
impl crate::app::State {
    /// The worktrees currently shown (014 FR-002/FR-003, unchanged in meaning). Now filters on
    /// provenance rather than on names.
    pub fn visible_worktrees(&self) -> impl Iterator<Item = &Worktree>;

    /// How many of the active project's worktrees the reveal control is currently withholding
    /// (029 FR-025). Zero while the control is on, and zero when nothing is hidden.
    pub fn hidden_worktree_count(&self) -> usize;
}
```

`hidden_worktree_count()` is defined as `worktrees.len() - visible_worktrees().count()` so FR-025b —
"switching the control on reveals exactly that many rows" — holds by construction rather than by two
filters agreeing. That is the same reasoning 014's contract gives for making `visible_worktrees()`
the single source every surface reads from.

---

## 7. What deliberately does not change

| Type / value | Why it is untouched |
|---|---|
| `micold_core::worktree::Worktree` | A discovery fact. Provenance is app memory (research R4). |
| `WorktreeStatus`, `reconcile()`, `parse_worktrees()` | Discovery is unchanged; every worktree still reaches `State::worktree.worktrees` (FR-016). |
| `Workspace.included_worktrees` | Out-of-root worktrees are never classified (FR-005); their record already states the user's wish. |
| `SidebarState.show_agent_worktrees` | 014's toggle, its default-off, and its project-switch reset (`features/sidebar.rs:461`) are unchanged (FR-014). |
| `Tag::Agent` | Same variant, same label, same non-filterability (FR-015a). Only its *source* changes. |
| Session records | Read as migration evidence; never written by this feature. |

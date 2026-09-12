# Phase 0 Research: Worktree Provenance

**Feature**: 029-worktree-provenance | **Date**: 2026-09-03 | **Plan**: [plan.md](./plan.md)

The spec left no `NEEDS CLARIFICATION` markers — two clarification passes resolved seven questions
before planning. What follows resolves the *technical* unknowns those answers created: where the
record lives, who writes it, where the migration runs, and how a read failure reaches classification
without pretending to be an empty record set.

---

## R1 — Where the provenance record lives

**Decision**: A new `Workspace.worktree_provenance: BTreeMap<PathBuf, BTreeSet<String>>` — project
path → the set of worktree directory names this app created or the user claimed — persisted in the
per-project state file as `created_worktrees: Vec<String>` with `#[serde(default)]`.

**Rationale**: `Workspace` already carries `worktree_names: BTreeMap<PathBuf, BTreeMap<String,
String>>` with exactly the shape this feature needs, and the spec's own Assumptions name it as the
home. Following it costs nothing and inherits four behaviours for free:

- `Workspace::forget()` (`workspace.rs:119`) already removes `sessions`, `worktree_names`,
  `included_worktrees` and `foreground_by_project` for the forgotten project — adding one line there
  satisfies the second half of FR-009 outright.
- `StoredProjectState` (`store.rs:358`) already splits per-project records into their own file, so a
  new `#[serde(default)]` field is a non-breaking schema addition in both directions: an old build
  ignores it, a new build reading an old file sees an unmigrated project, which *is* the FR-006 case.
- The daemon is already the single writer of that file, and the client already mirrors the map for
  the active project from the catalog snapshot (`catalog_sync.rs:202-212`).
- `remove_project_state()` already deletes the whole file on forget.

**Alternatives considered**:

- *A separate `provenance.json` store.* Rejected: a second store with its own load path, its own
  corruption story, and its own forget hook, all to hold a set of strings the existing per-project
  file is already carrying keys for. The spec's Assumptions rule this out explicitly.
- *A marker file inside each worktree.* Rejected by FR-003 before it could be weighed: establishing
  provenance may not create anything on disk. It also fails the case it exists for — an assistant
  worktree cloned or copied from a user one would carry the marker.
- *Reusing `worktree_names` itself*, treating "has a display-name override" as provenance. Rejected:
  it conflates two facts with different lifetimes. Renaming would then create provenance (FR-008
  forbids it), and every app-created worktree would need a spurious override. The *migration* reads
  it as evidence (R2) — that is a one-time read, not a redefinition.

---

## R2 — The evidence set already exists in the codebase

**Decision**: FR-006's evidence rule — "a stored displayed-label override, or a persisted session
bound to it" — is implemented by reusing the expression `Catalog::snapshot()` already computes.

**Rationale**: This is the research finding that made the migration cheap. `catalog.rs:155-168`
builds a project's *durable-only* worktree list like this:

```rust
// "The durable knowledge of a worktree is its display-name override plus any session
//  bound to it"
let mut dirs: BTreeSet<String> = BTreeSet::new();
if let Some(map) = overrides { dirs.extend(map.keys().cloned()); }
dirs.extend(sessions.iter().filter_map(|s| s.worktree_dir.clone()));
```

That is FR-006's evidence set, verbatim, written for a different purpose eighteen months of features
ago. The clarification chose the rule on its merits; finding it already expressed here is
corroboration that "what the app durably knows about a worktree" has exactly one honest definition
in this codebase. The migration extracts it into a named function both callers use, rather than
copying it — two definitions of "evidence" that can drift is precisely how a migration comes to
hide something it should not have.

One correction the extraction must carry: `sessions_for()` filters out archived sessions, and FR-006
says "archived or not". The extracted function therefore reads the unfiltered session list; the
snapshot caller keeps its own filter. An archived session is still proof the user worked there.

**Alternatives considered**: *Filesystem evidence* (mtime, a `.git` file's age, whether the branch
has user commits). Rejected: FR-006a forbids inspecting disk, and every such signal is an assistant
worktree's signal too.

---

## R3 — Where the one-time migration runs, and what "once" is anchored to

**Decision**: In the daemon, inside `DaemonState::refresh_worktrees()` (`state.rs:799`), immediately
after discovery populates `Inner::worktrees` for a project. Guarded by a persisted per-project
boolean `provenance_migrated`, stored beside the records in the same file, and set in the same write
as the backfilled records.

**Rationale**: The migration needs three inputs simultaneously — the discovered worktree list, the
project's durable records, and the marker. `refresh_worktrees()` is the single place all three are in
hand, it already runs off the async runtime because discovery shells out to git, and every path that
could reveal a worktree to a user funnels through it (attach, create, delete, include, exclude,
project add — `server.rs:498/924/1205/1340/1377/1400`). Running it there means no new trigger, no new
lifecycle, and no "first open" concept to define separately from what the app already does on open.

Anchoring "once" to a *persisted per-project marker* rather than to a process lifetime is what
FR-006c actually requires and what makes FR-006d hold: a second client window, a daemon restart, or
a crash halfway through must not re-run evidence collection, because between the two runs the user
may have started a session in a revealed assistant worktree — 014 permits exactly that — and a
second evidence pass would then promote it permanently. The marker is the difference between a
migration and a standing rule, and the clarification's second pass exists because a standing rule
was found to be unsafe.

**Alternatives considered**:

- *Client-side, at startup.* Rejected: the client is not the writer of durable state, and two
  windows would race to run it.
- *A store-load-time migration* (in `JsonFileStore::load`). Rejected: the worktree list is not known
  there — the store has no git — so the backfill would have to record evidence for directory names
  that may no longer exist, writing records for worktrees that are gone.
- *A global "app has upgraded" marker* instead of per-project. Rejected by FR-006c's wording and by
  the plain case it protects: a project first opened a month after the upgrade has never had its
  evidence read, and a global marker would deny it the backfill and hide everything in it.

---

## R4 — How classification is expressed, now that it is not a property of the worktree

**Decision**: Replace `Worktree::owner()` / `Worktree::is_agent_owned()` with a free function in
`micold-core`:

```rust
pub fn classify_owner(worktree: &Worktree, provenance: &ProvenanceView<'_>) -> WorktreeOwner
```

where `ProvenanceView` bundles the project's record set, its managed root, and whether its state was
readable. `WorktreeOwner` keeps 014's two variants and its enum-not-bool rationale.

**Rationale**: Under 014 ownership genuinely *was* a property of the worktree — it was computed from
two of its own fields, which is why a method was right. Under provenance it is a relation between a
worktree and what the app remembers about its project, and a method on `Worktree` cannot express
that without either a hidden global or a new field.

The new-field version was considered seriously and rejected on Principle V grounds: `Worktree` is
what *discovery found* — `parse_worktrees()` builds it from `git worktree list --porcelain` — and
adding `user_created` to it would mean every construction site has to supply an app-state answer it
does not have. `FakeGit` would be inventing provenance; `reconcile()` would need the workspace
passed in. A discovery value that carries app memory is a value that can be constructed wrong.

Making the record set an explicit parameter has the property Principle V asks for: a caller
*cannot* classify without stating which project's records they are classifying against. The 014
contract's preconditions survive as invariants on the new function (pure, total, health-blind,
non-mutating, stateless), with location scoping (FR-005) promoted from a documented precondition to
an argument the function actually checks, since the record set is per project and the root is the
thing that decides eligibility.

**Alternatives considered**: *A `&State` method on the client only.* Rejected: it would put the rule
in the client crate, out of reach of core tests and of the daemon, which needs the same rule for the
snapshot flag.

---

## R5 — Making the read-failure case unrepresentable rather than guarded

**Decision**: A non-persisted `Workspace.unreadable_projects: BTreeSet<PathBuf>`, set when
`load_project_state()` returns `ProjectStateLoad::Corrupt`, carried into `ProvenanceView`, and
short-circuiting classification to `User` for every worktree in that project.

**Rationale**: This is the sharpest hazard in the feature. Today `ProjectStateLoad::Corrupt`
(`store.rs:633-637`) silently drops the project's sessions, names and inclusions and continues with
empty maps. Under 014 that was survivable — nothing was hidden by an empty map. Under provenance an
empty record set means *hide everything*, so the existing behaviour would turn a corrupt file into
"all your worktrees vanished", which is precisely the failure direction User Story 2 ranks as the
worse one.

An empty set therefore cannot be allowed to mean two things — "nothing recorded yet" (the FR-006
migration case) and "records lost" (the FR-011 fail-visible case). One value, two required and
opposite behaviours, is the overload that produces the bug. A distinct marker separates them.

Three consequences follow, and all three are FR-011's own words:

1. Classification treats every worktree in an unreadable project as user-owned for that run.
2. The migration must not run for it — its evidence is exactly the records that failed to load, so
   it would conclude "no evidence" and record nothing, then set the marker and consume the project's
   one chance at a correct backfill.
3. The done-marker must not be written for it, for the same reason.

Writes the user *initiates* during that run (a create, a claim) still record normally: FR-011
forbids the failed read from causing a write, not the user from acting.

**Alternatives considered**: *`Option<BTreeSet<String>>` with `None` meaning unreadable.* Rejected:
`None` and `Some(empty)` differing in meaning is the same overload one indirection down, and the map
is keyed per project while the flag has to survive being cloned into a `ProvenanceView`. *Refusing to
save an unreadable project's state at all.* Rejected as a larger behavioural change than this feature
should make to sessions and inclusions; the flag is scoped to provenance.

---

## R6 — The claim as a protocol message

**Decision**: `ClientMsg::WorktreeClaim { req, project, dir_name }`, answered with
`OperationResult::Ack` and a catalog broadcast — modelled directly on `WorktreeRename`
(`server.rs:1250`).

**Rationale**: `WorktreeRename`'s handler comment already states the shape: *"A display-name override
is durable catalog state — no git involved."* A claim is the same kind of act on the same kind of
state: validate nothing about disk, write one durable record, broadcast, ack. Its error surface is
one case (`IoFailed` on persistence), against rename's two, because there is no user-supplied string
to validate.

Reusing the rename shape also gets the multi-window story right for free: the broadcast is what makes
a second window's sidebar drop the `agent` chip from the row, and it is the same mechanism that
already carries a rename between windows.

**Alternatives considered**:

- *A client-local write.* Rejected: the client is not the writer of durable state, and a claim that
  did not reach the daemon would be lost on the next catalog push, which reads as the claim silently
  undoing itself.
- *Overloading `WorktreeInclude`.* Rejected: inclusion answers "show me a worktree from outside the
  managed root" and writes `included_worktrees`. A claim answers "this one inside the root is mine".
  They apply to disjoint sets of worktrees (FR-005), and merging them would make the out-of-root
  guarantee harder to state, not easier.

---

## R7 — The assistant's transient lock, and why nothing reads it

**Decision**: Not read, not surfaced, not used as a signal. Recorded here so a later reader does not
re-derive it.

**Rationale**: Claude Code marks a worktree it is working in via `git worktree lock --reason "claude
session <name> (pid <n> start <t>)"`, which `git worktree list --porcelain` reports as a `locked
<reason>` line. The mark is released at cleanup and cleared by any later session whose pid check
fails. It therefore answers *"is a session live in here right now"* and never *"who created this"* —
none of this repository's own five worktrees carries it, and several of them were created by
assistant sessions. It is not provenance and cannot be made into provenance.

The clarification put the live-session badge in its own feature, and Out of Scope explains why:
different question, different source, different staleness semantics, and under this feature every row
it would decorate is hidden by default.

The creation-time hook route (`WorktreeCreate` / `WorktreeRemove` in the assistant's `settings.json`)
was rejected before specification: those hooks are *delegation, not notification* — a configured
`WorktreeCreate` hook replaces `git worktree add` entirely and must itself create the directory and
echo its path. Using them for provenance would mean this app owning the assistant's worktree
creation, which is a different and much larger feature.

---

## R8 — Carrying provenance to the client

**Decision**: `WorktreeSnapshot` gains `user_created: bool`, filled by the daemon in
`snapshot_locked()` (`state.rs:436`); `catalog_sync.rs` rebuilds the client's
`worktree_provenance[active]` set from it, exactly as it rebuilds `worktree_names` from
`display_name` today (`catalog_sync.rs:202-212`).

**Rationale**: The pattern exists and is understood: the daemon owns the durable answer, projects it
per worktree onto the wire, and the client reconstitutes its own map for the active project. No new
message, no new request/response round trip, and a second window learns of a claim through the
`CatalogChanged` it is already receiving.

The client also loads the store itself at boot (`shell/startup.rs:129`), so the map is populated
before the daemon connects and the first frame is classified correctly rather than showing everything
and then hiding rows a moment later.

**Alternatives considered**: *A dedicated `ProvenanceChanged` push.* Rejected: strictly more
machinery for a fact that fits in a bool on a struct already sent for every worktree on every
change.

---

## R9 — The hidden count without a second widget

**Decision**: A `.count(usize)` builder step on the shared `ToggleChip`, rendering as a trailing
`· N` when non-zero and rendering nothing when zero. The value comes from a
`State::hidden_worktree_count()` unit-tested in the client's feature slice.

**Rationale**: Principle VIII's Component-reuse gate is the whole argument. 014 promoted
`ToggleChip` out of the sidebar's private `filter_chip()` precisely so the reveal chip would not
fork a second implementation; adding a count by building a bespoke labelled chip beside it would undo
that in the same file. A builder step is chainable, terminates in `.into()`, is opt-in for the tag
chips that do not want it, and is the shape the existing `.active()` / `.accent()` steps establish.

Zero rendering as nothing is FR-025a's requirement and also the right default: a project with nothing
hidden shows 014's control unchanged, which keeps SC-003's "every 014 scenario still passes" honest.

**Alternatives considered**: *Folding the count into the label string* (`"Show agent worktrees (3)"`).
Rejected by FR-015a: no user-visible string is renamed by this feature, and the count is stated to sit
*beside* the label rather than be folded into it — a rendering concern the widget should own, not a
different label.

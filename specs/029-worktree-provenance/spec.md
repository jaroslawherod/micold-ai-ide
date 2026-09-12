# Feature Specification: Worktree Provenance

**Feature Branch**: `feat/worktree-provenance`

**Created**: 2026-08-31

**Status**: Draft — clarified 2026-08-31 (7 questions across two passes, see [Clarifications](#clarifications)); ready for `/speckit-plan`.

**Input**: User description: "Extend feature 014 (hide-agent-worktrees) so worktree classification stops guessing from names. 014 marks a worktree Agent-owned only when its directory is `agent-<16+ hex>` or its branch is `worktree-agent-<16+ hex>`. That catches Claude Code's subagent-isolation worktrees and nothing else. Claude Code also creates *session* worktrees — `claude --worktree <name>` and the in-session EnterWorktree tool — under the same `.claude/worktrees/` root, named by whoever typed the name. Those are indistinguishable from a worktree the user made in the app, so they still land in the sidebar, the project switcher and the tag filters: exactly the pollution 014 set out to remove. […] So invert the rule. The app already creates every user worktree itself, so record that at creation and let provenance be known-user vs not-known-user instead of inferring agent-ness from a name pattern."

## Context

Feature 014 hides worktrees an AI assistant created for its own throwaway sub-tasks, and it decides
which ones those are by reading their names: a reserved prefix followed by a long machine-generated
hexadecimal identifier. That rule was correct for the only assistant-made worktrees that existed
when 014 shipped, and it is still correct for them — but it has turned out to cover only one of the
two kinds the assistant makes.

The assistant also creates **session** worktrees: one per working session, placed in the *same*
managed worktrees directory, but named by whoever typed the name — ordinary words, a feature name,
a branch name. Nothing in the name distinguishes such a worktree from one the user made in the app,
so 014's rule cannot see them and they land in the sidebar, the project switcher, and the tag
filters. That is precisely the pollution 014 set out to remove, arriving through the door 014's
naming rule leaves open. This project's own managed worktrees directory currently holds several of
them.

No durable marker for "an assistant created this" exists to read. The assistant marks a worktree
only for as long as it is *using* it, and that mark is removed at cleanup and cleared by any later
run that finds the owning process gone — so it answers "is a session live in here right now", never
"who created this". Delegating creation to the assistant's own extension points was considered and
rejected: those points replace worktree creation rather than report it, so using them would mean
handing the assistant ownership of a step this app performs itself.

What *is* knowable, exactly and durably, is the other half: **this app creates every worktree the
user makes.** So the rule inverts. Instead of guessing which worktrees an assistant made, the app
records the ones it made itself, at the moment it makes them, and classification becomes *known to
be the user's* versus *not known to be the user's*. Everything not known to be the user's is treated
the way 014 already treats an assistant-owned worktree: hidden by default, revealable, badged. The
app already keeps exactly this shape of record — a persisted, per-project, per-worktree map, synced
through the session daemon, pruned when its worktree goes — for worktree display names, so there is
both precedent and a home for it.

This feature changes *which worktrees are classified as the assistant's*. It does not change what
being so classified does. Every behavior 014 defined — hiding by default, the reveal control and its
reset on every project switch, the `agent` chip, full row actions on a revealed entry, and the
invariant that discovery keeps every worktree in the app's state with hiding a view concern only —
survives unchanged and applies to the larger set this feature classifies.

## Clarifications

### Session 2026-08-31

- Q: When provenance records are introduced, how should the app classify worktrees already on disk
  with no record? → A: Grandfather selectively from evidence the app already holds — backfill a
  record only for a worktree with a stored displayed-label override or a persisted session bound to
  it; hide the rest.
- Q: Should feature 014's reserved naming rule survive in the new classification, and where? → A:
  Deleted as a hiding rule — provenance is the sole hiding signal — and kept in exactly one place, as
  a veto on the FR-006 backfill, so a machine-named worktree is never grandfathered.
- Q: Should this feature surface "an assistant session is live in this worktree right now" from the
  assistant's transient mark? → A: Out of scope — it answers a different question from provenance and
  belongs to its own feature.
- Q: Should a genuinely-user worktree with no provenance record be claimable permanently, or is
  revealing it each time the only path? → A: Add a claim action on a revealed row that writes a
  provenance record, so the worktree is listed normally from then on.
- Q: Should the user-visible wording change now that the hidden set means "not created by this app"?
  → A: No — keep 014's strings exactly (chip `agent`, control "Show agent worktrees"); the
  documentation carries the wider meaning.
- Q: Should the evidence rule that grandfathers unrecorded worktrees run once, or apply continuously?
  → A: Once per project — run at the first open after the upgrade, record that it ran, never again.
  Afterwards only creation and claims write records.
- Q: Should the app say anything when the migration hides worktrees that were visible the day before?
  → A: No notice, but the reveal control permanently shows how many worktrees are currently hidden in
  this project.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Assistant session worktrees stop polluting the list (Priority: P1)

A developer has been running assistant sessions in a project for weeks. Each of those sessions left
a worktree in the project's managed worktrees directory, named in ordinary words by whoever started
it. The developer opens the project in the app and the sidebar lists only the worktrees they created
in the app themselves — none of the session leftovers, whatever they happen to be called.

**Why this priority**: This is the gap 014 left, and the reason the feature exists. Session
worktrees are the common case — one per session, accumulating daily — while the machine-named
sub-agent worktrees 014 already catches are the rare one. Delivered alone, this restores the clean
worktree list 014 promised.

**Independent Test**: In a project whose managed worktrees directory holds a mix of worktrees created
through the app and worktrees created outside it under ordinary names, open the project and confirm
the sidebar lists exactly the app-created set.

**Acceptance Scenarios**:

1. **Given** a project with 2 worktrees created through the app and 3 assistant session worktrees
   with ordinary word-based names, **When** the user opens the project, **Then** the sidebar lists
   exactly the 2 app-created worktrees.
2. **Given** an assistant session worktree whose name is indistinguishable in form from a
   user-created one (for example `feat-login-refactor`), **When** the user views the sidebar,
   **Then** it is not listed.
3. **Given** an assistant creates a new session worktree while the project is open, **When** the app
   next refreshes its view of the project's worktrees, **Then** no new row appears.
4. **Given** assistant-owned worktrees exist, **When** the app classifies and lists them, **Then**
   nothing on disk has been created, renamed, pruned, checked out, or deleted in order to establish
   or verify provenance.

---

### User Story 2 - My own worktrees are never hidden, whatever they are called (Priority: P1)

A developer creates a worktree in the app. It stays visible — this run, after a restart, after the
session daemon restarts, after they rename its label — no matter what name they gave it, including a
name that imitates the assistant's machine-generated naming convention exactly.

**Why this priority**: Co-equal with User Story 1, because the inverted rule moves the failure mode.
Under 014 a mistake made the user's work *visible when it should have been hidden*; under provenance
a mistake makes the user's work *vanish*. That is the worse direction, so the guarantee that an
app-created worktree is always visible carries the same priority as the hiding itself.

**Independent Test**: Create worktrees through the app under a range of names — ordinary, and one
deliberately imitating the reserved machine convention — then restart the app and confirm every one
is still listed.

**Acceptance Scenarios**:

1. **Given** a worktree created through the app, **When** the app is restarted, **Then** it is still
   listed without the user revealing anything.
2. **Given** a worktree created through the app whose name exactly imitates the assistant's reserved
   naming convention, **When** the user views the sidebar, **Then** it is listed as the user's own
   and carries no `agent` chip.
3. **Given** a worktree created through the app, **When** the user renames its displayed label,
   **Then** it remains classified as the user's own.
4. **Given** a worktree created through the app, **When** the user switches away to another project
   and back, **Then** it is still listed as the user's own.
5. **Given** the app cannot read a project's stored state at all — the record set is damaged or
   unreadable — **When** the project's worktree list is presented, **Then** no worktree is hidden on
   provenance grounds, so a storage failure never makes the user's work disappear.
6. **Given** a worktree that lives outside the project's managed worktrees directory, **When** the
   app builds its worktree list, **Then** it is listed exactly as it is today and is never hidden on
   provenance grounds.

---

### User Story 3 - Everything 014 established still works (Priority: P2)

A developer who has learned 014's behavior finds it unchanged: hidden worktrees are absent from
counts, filters, empty states, and session targets; the filter panel's reveal control brings them
back, marked with the `agent` chip and offering every ordinary row action; and the control is off
again the moment they switch projects or restart.

**Why this priority**: This feature widens the hidden set rather than redefining what hiding means.
If any part of 014 regresses, the wider set makes the regression worse — more rows silently
unreachable, with the same escape hatch broken.

**Independent Test**: With assistant-owned worktrees present, run 014's own acceptance scenarios for
hiding, reveal, the chip, row actions, and project-switch reset, and confirm each still holds.

**Acceptance Scenarios**:

1. **Given** worktrees classified as assistant-owned, **When** the user applies a sidebar filter,
   **Then** counts and results reflect only the user's own worktrees.
2. **Given** the reveal control is switched on, **When** the list is presented, **Then** the
   assistant-owned entries appear, each carrying the `agent` chip, each offering the full set of row
   actions with none disabled or given an extra confirmation step.
3. **Given** the reveal control is on, **When** the user switches to another project, **Then** it is
   off there, and switching back does not restore it.
4. **Given** a project with hidden worktrees, **When** the user opens the filter panel, **Then** the
   reveal control shows how many are hidden, and switching it on reveals exactly that many rows.
5. **Given** any worktree at all, **When** the app discovers the project's worktrees, **Then** every
   one of them is present in the app's own worktree state; hiding affects only what is presented.
6. **Given** the project's only location is its root ("Default"), **When** the list is presented,
   **Then** the root location is unaffected by provenance classification and is never hidden.

---

### User Story 4 - A deleted worktree's name can be reused safely (Priority: P3)

A developer deletes a worktree in the app. Later an assistant session creates a new worktree that
happens to reuse that directory name. It does not inherit the deleted worktree's provenance — it is
hidden like any other assistant session worktree.

**Why this priority**: A stale record is the one way the inverted rule can produce a false *user*
classification, which reopens the pollution the feature closes. It is a narrow case, so it ranks
below the guarantees above, but leaving it unhandled means the record set slowly accumulates lies.

**Independent Test**: Create a worktree in the app, delete it in the app, recreate a directory of the
same name outside the app, and confirm it is hidden.

**Acceptance Scenarios**:

1. **Given** a worktree created and then deleted through the app, **When** a worktree of the same
   directory name later appears without being created through the app, **Then** it is classified as
   assistant-owned and hidden.
2. **Given** a worktree created through the app is deleted through the app, **When** the project's
   stored state is next written, **Then** it carries no provenance record for the removed worktree.
3. **Given** a project is forgotten in the app, **When** its stored per-project state is discarded,
   **Then** its provenance records go with it, exactly as its other per-project records do.

---

### User Story 5 - Claim a worktree the app did not create (Priority: P3)

A developer notices one of their own worktrees is missing from the sidebar — they made it by hand in
a terminal, or it predates this feature and the app had no record of them ever using it. They switch
the reveal control on, find the row, and claim it. From then on it is listed as their own, reveal
control or not, across restarts.

**Why this priority**: This is the escape hatch that makes inverting the rule safe. The feature works
without it, but with it every misclassification — including any the migration gets wrong — is
recoverable in a single action, in-app, without touching anything on disk. Without it, the only
permanent fix is to delete the worktree and recreate it through the app.

**Independent Test**: Create a worktree outside the app under the managed directory, confirm it is
hidden, claim it from a revealed row, and confirm it is listed normally after switching reveal off and
restarting the app.

**Acceptance Scenarios**:

1. **Given** a revealed assistant-owned worktree, **When** the user claims it, **Then** it is listed
   as the user's own, loses the `agent` chip, and remains listed after the reveal control is switched
   off.
2. **Given** a claimed worktree, **When** the app is restarted, **Then** it is still listed as the
   user's own.
3. **Given** a revealed worktree whose name matches the reserved machine convention, **When** the user
   claims it, **Then** the claim is honoured — the convention vetoes only the automatic migration, not
   an explicit user action.
4. **Given** a worktree is claimed, **When** the claim takes effect, **Then** nothing on disk has been
   created, renamed, moved, checked out, or deleted.

---

### Edge Cases

- **Worktree created by hand in a terminal, under the managed directory**: it was not created through
  the app, so it is not known to be the user's and is hidden by default. This is an accepted
  consequence of inverting the rule — see [Assumptions](#assumptions) — and the reveal control is the
  supported way to reach it, with the claim action (FR-020) the way to keep it listed.
- **Worktree outside the managed directory**: never hidden on provenance grounds (FR-005). A worktree
  the user explicitly asked this app to show from elsewhere is by definition user-requested, and that
  wish is already recorded.
- **Session started in a revealed assistant worktree before the upgrade**: the app holds a session
  record for it, which is normally grandfathering evidence — but its machine-generated name matches
  the reserved convention, so the FR-007a veto keeps it from being backfilled and it stays hidden.
- **Record set damaged or unreadable**: nothing is hidden on provenance grounds (FR-011). The failure
  mode of losing records must be a visible list, never a vanished one.
- **Record written but the app closes before persisting**: the worktree is visible for the rest of the
  run regardless (FR-010). On the next launch it is an unrecorded worktree: backfilled only if the
  project has not yet migrated and the user had named it or run a session in it — otherwise hidden,
  and claimable in one action either way.
- **Session started in a revealed assistant worktree after the migration**: it stays hidden. Evidence
  no longer creates records once a project has migrated (FR-006d), so the only thing that makes such a
  worktree the user's is claiming it.
- **Directory name reused after deletion**: the old record is gone with the deletion (FR-009), so the
  new worktree is classified on its own merits. This holds for a claimed worktree exactly as for a
  created one — a claim writes the same record, and deletion removes it.
- **Claiming an assistant's worktree by mistake**: accepted. The claim is an explicit user action on a
  row the user had to reveal first, and its effect is a listing, not a change to the worktree. There is
  no inverse action in this feature; deleting the worktree removes its record.
- **Rename**: renaming changes only a worktree's displayed label, never its directory, so provenance
  is unaffected (FR-008).
- **Registered but missing / present but unregistered**: classification applies regardless of health
  state, exactly as under 014 — a broken assistant-owned entry is hidden rather than surfaced.
- **Project root ("Default")**: not a worktree; never classified, never hidden.
- **All worktrees hidden**: the project presents its normal empty state, not an error.
- **A session recorded against a now-hidden worktree**: handled by the app's existing behavior for a
  session whose worktree is unavailable, with no dedicated path added — unchanged from 014.

## Requirements *(mandatory)*

### Functional Requirements

#### Recording provenance

- **FR-001**: When the app creates a worktree on the user's behalf, it MUST record that fact as part
  of that creation, for every route by which the app creates one.
- **FR-002**: The provenance record MUST be durable — surviving app restart, session-daemon restart,
  and project switching — and MUST be stored per project and per worktree alongside the app's other
  per-project worktree records.
- **FR-003**: Establishing, reading, or maintaining a provenance record MUST NOT create, rename,
  prune, check out, move, or delete anything on disk — not the worktree, not its directory, not its
  branch, not any marker file inside it.

#### Classifying from provenance

- **FR-004**: A worktree MUST be classified as user-owned when the app holds a provenance record for
  it, and as assistant-owned when it does not — subject to FR-005, FR-006, FR-007, and FR-011.
- **FR-005**: Only a worktree located directly under the project's managed worktrees directory is
  eligible to be classified as assistant-owned. Any worktree elsewhere MUST remain visible, exactly
  as it is today — this preserves 014's location scoping unchanged.
- **FR-006**: The first time the app opens a project after this feature ships, it MUST run a one-time
  migration that backfills a provenance record for each worktree under the managed worktrees directory
  for which the app already holds evidence that the user worked in it through the app — namely a
  stored displayed-label override for that worktree, or a persisted session bound to it (archived or
  not). A worktree with neither MUST NOT be backfilled and is classified by FR-004 like any other
  unrecorded worktree.
- **FR-006a**: The migration MUST read only records the app already holds; it MUST NOT inspect,
  create, or modify anything on disk, and MUST NOT ask the user anything.
- **FR-006b**: The migration MUST be idempotent and MUST NOT overwrite or remove a provenance record
  that already exists, so a repeat run — from a crash mid-migration, or from two clients opening the
  same project — changes nothing.
- **FR-006c**: The migration MUST run at most once per project. Having run, it MUST be recorded as
  done and MUST NOT run again, so evidence never creates a provenance record after that point. From
  then on the only things that write a record are worktree creation (FR-001) and a claim (FR-020).
- **FR-006d**: A project that has migrated MUST classify an unrecorded worktree as assistant-owned
  whatever evidence accrues to it later. In particular, starting a session in a revealed
  assistant-owned worktree — which 014 permits — MUST NOT make it user-owned; claiming it (FR-020) is
  the only way to do that.
- **FR-007**: The reserved naming convention defined by 014 MUST NOT hide anything any longer.
  Provenance is the sole hiding signal: an unrecorded worktree under the managed directory is hidden
  whatever it is called, so the convention can no longer change any hiding outcome.
- **FR-007a**: The convention MUST survive in exactly one place — as a veto on the FR-006 backfill. A
  worktree whose directory name or bound branch name follows the reserved convention MUST NOT be
  grandfathered, whatever evidence the app holds for it. This closes the hole 014 opens by allowing a
  session to be started in a revealed assistant-owned worktree.
- **FR-007b**: A provenance record written at creation MUST outrank the convention: a worktree the app
  created is user-owned even when its name imitates the reserved convention.
- **FR-008**: Provenance MUST be unaffected by a worktree's displayed-label rename, since renaming
  changes only the label and never the worktree's identity on disk.
- **FR-009**: When the app deletes a worktree, its provenance record MUST be removed, so a later
  worktree that reuses the same directory name does not inherit it. When a project is forgotten, its
  provenance records MUST be discarded with its other per-project records.
- **FR-010**: A worktree just created through the app MUST be visible immediately, in the same run,
  without waiting on any persistence step and regardless of whether that step succeeded.
- **FR-011**: If the app *fails to read* a project's stored state — unreadable, damaged, or discarded
  as corrupt — classification MUST treat every one of that project's worktrees as user-owned for that
  run: loss of records fails visible, never hidden. A failed read MUST NOT trigger the FR-006 backfill
  and MUST NOT cause any provenance record — or the migration's done-marker — to be written, so a
  transient failure cannot overwrite the true record set or consume the one-time migration. This is distinct from a project read successfully that simply has no records yet,
  which is the migration case FR-006 governs.
- **FR-012**: Classification MUST be re-evaluated whenever the app refreshes its view of a project's
  worktrees, so worktrees appearing or disappearing during a run are classified correctly without a
  restart.

#### Preserving feature 014

- **FR-013**: Assistant-owned worktrees MUST NOT appear in the sidebar's worktree list while the
  reveal control is off, and MUST be excluded from every quantity and state derived from that list —
  filter results, counts, the project switcher, the tag filters, and the decision to show an empty
  state.
- **FR-014**: The filter panel's reveal control MUST continue to work exactly as 014 defined it:
  labelled "Show agent worktrees", off at every app start, reset to off on every project switch,
  discoverable even when the project has nothing to reveal, and leaving active tag filters untouched.
- **FR-015**: A revealed assistant-owned worktree MUST continue to carry the `agent` chip and to offer
  exactly the same row actions as a user-created one, with none disabled, hidden, or given an extra
  confirmation step — plus the claim action FR-020 adds.
- **FR-015a**: The user-visible wording 014 established MUST be unchanged: the chip reads `agent` and
  the reveal control reads "Show agent worktrees", even though the set they now describe is "not
  created by this app" rather than "named like an agent's". The documentation (FR-019) carries that
  wider meaning; no user-visible string is renamed by this feature. The only addition is the count
  required by FR-025, shown beside that label rather than folded into it.
- **FR-016**: Discovery MUST continue to place every worktree it finds into the app's worktree state
  regardless of classification; hiding MUST remain a presentation concern only.
- **FR-017**: The project's root location ("Default") MUST NOT be subject to provenance classification
  and MUST never be hidden by it.
- **FR-018**: Hiding MUST remain presentation-only: the app MUST NOT delete, prune, rename, check out,
  or otherwise modify an assistant-owned worktree, its directory, or its branch as a consequence of
  classifying or hiding it.
- **FR-019**: The user-facing documentation MUST be updated to describe the new rule: that the app
  hides worktrees it did not create itself, that assistant session worktrees are therefore hidden too,
  how to reveal them, what the count beside the reveal control means, what a user should expect for a
  worktree they created by hand outside the app, and how to claim such a worktree so it stays listed.

#### Claiming an unrecorded worktree

- **FR-020**: While the reveal control is on, every assistant-owned worktree row MUST offer a claim
  action that records the worktree as the user's own.
- **FR-021**: A claim MUST write the same provenance record FR-001 writes at creation, with the same
  durability (FR-002), the same removal on delete and on forget (FR-009), and the same read-only
  relationship to disk (FR-003).
- **FR-022**: A claim MUST take effect immediately: the worktree is listed as the user's own, without
  the `agent` chip, and remains listed once the reveal control is switched off.
- **FR-023**: A claim MUST be honoured for any revealed worktree, including one whose name follows the
  reserved convention. The FR-007a veto constrains only the automatic backfill, never an explicit user
  action.
- **FR-024**: No inverse action is in scope: this feature adds no way to un-claim a worktree or to hide
  one the app created. A worktree leaves the user-owned set only by being deleted, which removes its
  record (FR-009).

#### Making the hidden set visible as a quantity

- **FR-025**: The reveal control MUST show how many of the current project's worktrees are hidden by
  classification, so a user can see at a glance that something is being withheld and where to turn it
  on. No notification, prompt, banner, or badge about hidden or leftover worktrees is added — the
  count on the control is the whole of it.
- **FR-025a**: The count MUST track the project it is shown for and update whenever the app refreshes
  its view of that project's worktrees (FR-012), reaching zero — displayed as 014's plain control,
  with no count — when nothing is hidden.
- **FR-025b**: The count MUST reflect the same set the reveal control governs, so switching the
  control on reveals exactly that many rows, subject to any active tag filters (FR-014).

### Key Entities

- **Worktree entry**: an isolated working copy the app presents, identified by its directory name
  under the project's managed worktrees directory and its bound branch, carrying a health state.
- **Provenance record**: a durable, per-project, per-worktree note that *this worktree is the user's*.
  Written when the app creates a worktree, when the migration backfills one from existing evidence, or
  when the user claims one; removed when the app deletes the worktree or forgets the project. It
  records only that fact — everything else about a worktree is still derived from git and the
  filesystem at read time.
- **Ownership classification**: a derived property of a worktree entry — user-owned or
  assistant-owned — determined by whether a provenance record exists for it, scoped to the managed
  worktrees directory (FR-005), with records for pre-existing worktrees supplied once by the migration
  (FR-006) and 014's naming rule reduced to a veto on that migration (FR-007a).
- **Migration marker**: a per-project note that the one-time evidence backfill has run. Stored with
  the project's other records; its only effect is to stop the backfill running again.
- **Claim**: an explicit user action on a revealed assistant-owned row that writes the same provenance
  record creation writes. It changes only what the app records about a worktree, never the worktree.
- **Reveal control**: as 014 defined it — an on/off filter-panel option deciding whether
  assistant-owned worktrees are listed, off at every app start and again on every project switch — now
  also carrying a count of how many worktrees are currently hidden in this project (FR-025).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a project whose managed worktrees directory contains assistant session worktrees
  under ordinary, human-chosen names, zero of them appear in any of the app's worktree surfaces in a
  default run — the case 014's naming rule catches none of.
- **SC-002**: 100% of worktrees created through the app remain visible across an app restart, a
  session-daemon restart, a project switch, and a label rename — across a naming corpus that
  deliberately includes a name imitating the reserved machine convention, producing zero
  false hides.
- **SC-003**: Every acceptance scenario feature 014 defined for hiding, reveal, the `agent` chip, row
  actions on a revealed row, project-switch reset, and the discovery invariant still passes.
- **SC-004**: A user scanning the worktree list can identify their own worktrees without skipping over
  any entry they did not create, in a project where assistant sessions have run.
- **SC-005**: After the app has run with assistant-owned worktrees present, the on-disk state of those
  worktrees and their branches is byte-for-byte unchanged by the app.
- **SC-006**: Opening a project and rendering its worktree list is no slower than before the feature,
  with no user-perceptible delay introduced by the classification step.
- **SC-007**: No storage failure results in a hidden user worktree: with a project's records made
  unreadable, every worktree is still listed.
- **SC-008**: A user can restore a wrongly-hidden worktree to their list in a single action from the
  sidebar, without leaving the app or editing any configuration, and it is still listed after a
  restart.
- **SC-009**: On first launch after the upgrade, in a project where the user has named worktrees or
  run sessions in them, every such worktree is still listed, and every assistant worktree with neither
  is gone.
- **SC-010**: A user whose worktree disappeared at the upgrade can tell from the sidebar alone that
  worktrees are being hidden and how many, without opening the documentation or any settings.

## Out of Scope

- **A "session live here" badge.** The assistant does leave a readable mark on a worktree while a
  session is running in it, and that mark is a genuine — if transient — signal. It is deliberately not
  used here, and not surfaced here. It answers a different question from provenance (who is using this
  right now, not who made it), it is read from a different source with its own staleness and refresh
  semantics, and under this feature every row it would decorate is hidden by default. If it is wanted,
  it is its own feature.
- **Cleaning up leftovers.** The assistant owns the lifecycle of the worktrees it creates. This
  feature does not add pruning, cleanup prompts, or leftover notifications; unchanged from 014. The
  hidden count (FR-025) is a quantity on an existing control, not a notification.
- **User-configurable hide rules.** Classification stays built in: no user-defined name patterns, and
  no settings for it. The per-worktree claim (FR-020) is a record about one worktree, not a rule.
- **Owning worktree creation on the assistant's behalf.** The assistant's creation-time extension
  points replace worktree creation rather than report it; adopting them was considered and rejected.

## Assumptions

- The app creates every worktree the user makes through it, by a small and enumerable set of routes,
  so recording provenance at creation covers all of them.
- A worktree the user creates by hand outside the app, under the managed worktrees directory, will be
  hidden by default. This is the accepted cost of inverting the rule: there is no signal that
  separates such a worktree from an assistant's. Revealing it finds it and claiming it (FR-020) keeps
  it listed, and the documentation (FR-019) says so.
- Worktrees outside the managed worktrees directory are already governed by an explicit record of the
  user's wish to see them, so provenance adds nothing there and must not subtract anything.
- The record belongs with the app's existing per-project worktree records — same key, same
  persistence, same syncing through the session daemon, same disposal when the project is forgotten —
  rather than in a new store.
- No marker is written into the worktree on disk. Provenance is the app's own record about a worktree,
  never a change to it.
- The assistant's transient in-use mark is not a provenance signal and is not used as one: it is
  removed at cleanup and cleared by later runs, and it answers "is a session live here" rather than
  "who created this".
- Classification remains recomputed on every refresh from records plus the filesystem; no derived
  classification is itself persisted.
- The prose term **assistant-owned** is inherited from 014 and now means, precisely, "not known to have
  been created by this app". It is an internal term; the user-visible word stays **agent** (FR-015a).

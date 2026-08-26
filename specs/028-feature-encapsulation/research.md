# Phase 0 Research: Feature Encapsulation

**Feature**: 028-feature-encapsulation | **Date**: 2026-08-25 | **Plan**: [plan.md](./plan.md)

All measurements in this document were taken from the working tree at `d4e45b4`
(`feat/feature-encapsulation`, docs-only ahead of `b43c11c`), so they are the spec's own pinned
baseline. Every count is reproducible by the commands in [quickstart.md](./quickstart.md) §A.

---

## R0. Corrections to the spec's baseline table

Three of the spec's numbers did not survive contact with the tree. All three were corrected in the
spec by the clarification of 2026-08-26 — the first mattered because SC-004 is stated as a ratio,
and was unmeetable as written. The measurements are kept here as the record.

| Spec said | Tree says | Consequence |
|---|---|---|
| "11 feature modules", "1 of 11" | **10** — `connection`, `help`, `notifications`, `project`, `session`, `settings`, `sidebar`, `window`, `worktree`, `worktree_form` (`mod.rs` is not a feature) | SC-004 now reads **ten of ten, up from one of ten** |
| "seven `Outcome` variants" | **12** in `features/mod.rs` | Does not change scope; this feature extends the vocabulary either way |
| "18 `Widget` impls in `src/ui/`" | **13 files** carrying a `Widget` impl, **13** `tree::State::new` sites | Does not change scope; see R4 |

Everything else checks out: 119 root variants (`app.rs:43–504`), 44 public `State` fields
(`app.rs:506–718`), `State::update` 300 lines (`app.rs:866–1165`), `update_inner` 52 arms
(`main.rs:520–707`), 51 `OWNERS` entries in `tests/feature_write_isolation.rs`.

---

## R1. Attribution of the 119 root variants — the work the spec deferred to planning

**Decision**: every one of the 119 root variants is assigned to exactly one owner. 114 go to a
feature; 5 stay at the root. No variant needed a "shared between two features" verdict.

**Method**: each arm of `State::update` and of `main.rs::update_inner` was resolved to the
`features::<name>::` calls it makes, then to the `shell::<module>::` and
`overlay::registry::` calls where no feature call exists. Arms resolving to neither were
hand-classified against their single emit site in `src/ui/`. Script and raw output:
[quickstart.md](./quickstart.md) §A.3.

| Owner | Variants | Root arms after nesting |
|---|---:|---:|
| `session` | 37 | 1 |
| `project` | 19 | 1 |
| `worktree` | 18 | 1 |
| `connection` | 12 | 1 |
| `settings` | 10 | 1 |
| `sidebar` | 10 | 1 |
| `help` | 3 | 1 |
| `window` | 2 | 1 |
| `notifications` | 2 | 1 |
| `worktree_form` | 22 (already nested) | 1 |
| **root, cross-cutting** | **5** | 5 |

**Root message vocabulary: 119 → 15** (10 feature wrappers + 5 cross-cutting). The full
variant-by-variant table is in [data-model.md](./data-model.md) §2.

**The five that stay, and why each is genuinely cross-cutting (FR-003):**

- `EscapePressed` — one emit site (`ui/mod.rs:655`), but its consumer is
  `overlay::registry::escape`, which dispatches to whichever surface is topmost across *every*
  feature. Pushing it into a feature would make one feature the arbiter of another's dismissal.
- `ScrolledBeneathOverlay` — same shape (`registry`-dispatched), **and it has no producer at all
  today**. Grepping `src/` finds it only in its own declaration and its own arm. Recorded as a
  finding: the FR-013 guard must decide what to do with a variant nobody emits (see R6).
- `OverlayTransitionFinished` — emitted by any overlay's `on_hidden` (`ui/mod.rs:422`), consumed by
  the shell to release the closing-overlay snapshot. Cross-surface by construction.
- `WindowFocusChanged` — produced by the iced runtime, not by a feature. Consumed by
  `shell::os_theme` to re-probe the desktop preference.
- `NoOp` — the frame subscription's carrier (`shell/subscriptions.rs:110`). Belongs to no feature
  by definition.

**Alternatives considered:**
- *Push `EscapePressed`/`ScrolledBeneathOverlay` into an `overlay` feature module.* Rejected: the
  overlay registry is not a feature — it is the mechanism features register *with* — and feature
  017's `one_overlay_implementation.rs` guard already fixes its shape. Adding an eleventh feature
  module to hold two variants would satisfy a count, not a maintainer.
- *Split the daemon cluster between `connection` and `session`.* Rejected: `DaemonEvent` and
  `DaemonGridFrame` carry session payloads, but their *consumer* is `shell::daemon_sync`, which
  then calls `features::session`. The message is the connection's; what it causes is the session's,
  and that is what `Outcome` is for.

---

## R2. `TextCopyRequested` is not cross-cutting, despite its name

**Decision**: it belongs to `worktree`.

Its name reads as a generic clipboard request and its arm reaches `shell::clipboard`, which is why
the first pass classified it as root. It has exactly one emit site: `ui/mod.rs:470`, the "Copy name"
entry of the **worktree row menu**. One producer, one consumer, one feature — FR-013's rule names it
as a violation the day the guard lands. Recorded because it is the clearest example of why the
guard's rule has to be mechanical (spec Assumptions, "not editorial"): the editorial read was wrong
here, and the guard would have caught it.

---

## R3. What "the feature's own reducer entry point" must mean (FR-002, FR-005)

**Decision**: a feature satisfies FR-002 by exposing **at least one** of two entry points, and the
guard accepts either:

1. `pub fn update(state: &mut State, msg: Msg) -> Vec<Outcome>` — the pure reducer, in
   `src/features/<name>.rs`. This is `worktree_form`'s shape (`features/worktree_form.rs:657`).
2. `pub fn update(app: &mut App, msg: Msg) -> Task<Message>` — the effectful entry, in
   `src/shell/<name>.rs`, for a vocabulary whose arms are all effects.

**Rationale**: `features/connection.rs` today is 96 lines holding one type and one pure derivation
function. It writes no state at all — it is the only feature absent from the `OWNERS` map. Forcing
it to declare a pure reducer that returns `Vec::new()` for all twelve of its variants is precisely
the "empty vocabulary as ceremony" FR-005 forbids. `worktree_form` already demonstrates the split
in the other direction: its `Msg` has 22 variants, of which four are matched *in the shell*
(`main.rs` has four `Message::WorktreeForm` arms) and the rest in the pure reducer.

**A feature that declares no `Msg` at all** is the third accepted shape, for a derivation-only
module. No feature is in that state after this work — all ten own variants — but the guard states
the rule rather than the census.

**Alternative rejected**: a `Feature` trait with an associated `Msg` type and a required `update`.
Rust's trait dispatch would need the state and the outcome types uniform across features, which is
true today, but it buys nothing the free-function convention does not — and
`feature_registration_cost.rs` deliberately observes that there is no `FeatureId` and no central
match over one. A trait would create the central match this codebase spent 021 removing.

---

## R4. Story 2's literal reading is empty, and an existing guard is why

**This is the finding that shapes the whole of Story 2.**

FR-007's rule was applied mechanically to all 44 root fields: owner from `OWNERS`, readers by
scanning every `.rs` under `src/` for `self|state|core|app.core` followed by the field name, with a
feature's own module and its view files counted as inside (spec Edge Cases). Result:

| Class | Fields | Meaning |
|---|---:|---|
| **Qualifies under FR-007** | **5** | `about_open`, `expanded`, `default_expanded`, `sidebar_filters`, `sidebar_filter_open` |
| Read by another **feature** → FR-008 keeps it | 4 | `active_session`, `worktrees`, `reveal_suppressed_for`, `show_agent_worktrees` |
| Read by the **composition** (`ui/mod.rs`, another feature's view) | 12 | the popover/menu flags, `focused_field`, `notify`, `theme_pref`, `window_size` |
| Read by the **shell** (`main.rs`, `shell/*`) | 20 | every draft, every confirmation target, every viewport measurement |
| Read only by the **root reducer** (`app.rs`) | 2 | `sidebar_width`, `terminal_released` |

So FR-007 moves **5 of 44 fields**, not 44 — and the blocker is overwhelmingly the shell and the
composition, not other features. That alone would be worth recording. It gets sharper:

**All five are individually pinned to the application by `tests/logical_state_ownership.rs`**, a
feature-017 guard whose entire subject is that logical state must NOT move into a component:

| Field | Pinning test |
|---|---|
| `about_open` | `open_overlay_identity_is_application_owned` |
| `expanded`, `default_expanded` | `expanded_nodes_are_application_owned` |
| `sidebar_filters` | `tag_filters_are_application_owned` |
| `sidebar_filter_open` | `open_menu_identity_is_application_owned` (panel openness, same class) |

FR-021 forbids removing an existing assertion to accommodate this restructuring. **The intersection
of "qualifies under FR-007" and "a widget may hold it under 017" is therefore empty.**

**Decision (D-R4)**: Story 2 is delivered in two tracks, and the split is stated up front rather
than discovered at task 40.

- **Track 2A — feature-owned state structs (the substance).** Each feature's fields collapse into
  one struct owned by that feature's module, held as one field of the root `State`:
  `state.sidebar: sidebar::State`, `state.session: session::State`, and so on. Root `State` goes
  from **44 flat public fields to 10 feature structs** plus `workspace` and the shared members.
  This is exactly the shape `worktree_form` already has — `state.worktree_form` is a
  `worktree_form`-owned type — and it is what SC-007 actually asks for: everything a feature
  remembers is named in that feature's module, in one place, without consulting a test file. It
  satisfies FR-008 by construction (a shared path is one nobody's struct claims) and makes the
  `OWNERS` map derivable from the type system instead of hand-maintained.
- **Track 2B — component-owned state (FR-007 literally).** The rule is implemented as a guard, and
  the guard's allowlist carries the 017 test name as each entry's written reason. Today it moves
  nothing. It is not decoration: a field added tomorrow that is genuinely presentational and has one
  writer will be reported by it, and FR-012's builder requirement governs whatever moves.

**Alternatives considered:**
- *Relax `logical_state_ownership.rs` so the five can move.* Rejected on FR-021, and on merit: the
  guard's own reasoning ("would this value still mean something with the screen switched off")
  is right, and `expanded`/`sidebar_filters` plainly would.
- *Widen "the component that owns it" to mean the iced widget anyway, accepting the deletions.*
  Rejected: `expanded` is what the sidebar draws *and* what a reveal targets; a widget owning it
  would put the reveal's target inside the renderer, which is the over-reach 017's header names.
- *Do Track 2A only and mark FR-007 unimplementable.* Rejected: the rule is worth having even
  while it moves nothing, because it is the thing that stops the next `SelectState`-shaped field
  from landing in the root by default. `SelectState { open, highlight }` in
  `ui/material/select.rs:213` is the precedent that transient interaction state *does* belong in a
  widget here.

---

## R5. Lifetime is the risk in Track 2A, and it is not hypothetical (FR-009)

Grouping fields into a feature struct is behaviour-neutral **only if the struct is never replaced
wholesale where the fields were previously written one at a time**. Two concrete cases found:

- `Workspace::forget` (core) clears four `workspace` members as one invariant — already inventoried
  in `feature_write_isolation.rs`'s `CORE_MEDIATED` and unaffected, because `workspace` stays a
  shared member and is not folded into any feature struct.
- `State::set_worktrees` reconciles expansion, hover, menu, delete-confirmation and rename state
  across **three** features on one call. Under Track 2A its writes cross three structs. It already
  emits `Outcome::WorktreesReplaced` for the sidebar's share; the remaining cross-struct writes must
  not become "assign a fresh `worktree::State`", which would discard the menu and hover state that
  survives a re-discovery today.

**Decision**: no task may introduce `state.<feature> = <feature>::State::default()` or
`..Default::default()` on a feature struct. A struct-replacement is a lifetime change wearing a
refactor's clothes. This is stated as contract S3 and checked by the Track 2A guard.

---

## R6. Guard design (FR-013 – FR-018)

Three new rules. All three extend existing machinery rather than re-deriving it, per the spec's
Assumptions.

**G1 — no single-feature variant in the root vocabulary (FR-013).**
Reuses the arm-resolution scan built for R1: for each root variant, the set of `features::<n>::`,
`shell::<n>::` and `registry` calls its arms reach. Fails when that set is exactly one feature, and
names it. Allowlist entries carry a written reason. `ScrolledBeneathOverlay` (R1) resolves to zero
producers and zero features — the rule must therefore be stated over *the resolved owner set*, with
"empty" treated as cross-cutting and flagged in the report rather than failing.

**G2 — no single-owner path in the root state (FR-014).**
Under Track 2A this becomes near-trivial to state and is why 2A is worth doing: a *flat* public
field on `State` that is not a shared member is, by definition, a path no feature struct claims.
The rule is "every public field of `State` is either a declared shared member or a feature struct",
with the reader analysis of R4 as the allowlist's justification input.

**G3 — every feature module has a reducer entry point (FR-015).**
Extends `feature_registration_cost.rs`, which already enumerates feature modules from the filesystem
and already parses `fn` signatures taking the state mutably. Accepts the three shapes of R3.

**FR-017 (non-vacuity)**: each rule gets a recorded probe — the forbidden violation injected, the
suite observed failing, the injection reverted. This repository has an established practice for it
(021's tasks record probe counts and their distinct failure sets), and 021's own record shows why it
matters: T041's first two probes did not compile, so nothing ran and both looked like passes.

**FR-018 (runs without a window)**: `.github/workflows/ci.yml:153` is the authoritative list of
client tests that run on all three platforms — **11 tests, and not one of 021's guards is among
them**. 021's T058/T077 recorded this as an open decision and it is still open. G1/G2/G3 read source
text and open no window, so this feature adds them to that step, together with the four 021 guards
they extend (`feature_write_isolation`, `feature_registration_cost`, `root_is_routing_only`,
`logical_state_ownership`). That is a CI change, not a code change, and it is what makes FR-018
true rather than assumed.

---

## R7. The assertion freeze is out of scope for 028, and FR-021 wants it in

`scripts/check-assertions-frozen.sh` decides scope from the change: in scope iff it touches
`specs/021-mvu-slice-architecture/` or its branch names 021 (line 145–160). Feature 028 is
out of scope, so the check **reports and exits 0** rather than blocking.

FR-021 restates 021's FR-027 for this feature's duration, and Track 2A renames on the order of a
thousand assertion *spellings* (`state.expanded` → `state.sidebar.expanded`) without changing a
single expectation. That is exactly the case the adjudication file was built for.

**Decision**: extend `scope_reason()` to recognise 028 the same way it recognises 021, and add
`specs/028-feature-encapsulation/assertion-adjudications.md`. `FREEZE_ADJUDICATIONS` is already an
environment override, so the file's location needs no new mechanism. Without this, FR-021 is a
sentence in a spec with nothing enforcing it.

---

## R8. Frames while idle (FR-011, SC-008)

`tests/idle_requests_no_frames.rs` is already in the all-platform CI list. Neither track requests a
frame: Track 2A changes field paths only, and Track 2B moves nothing. The criterion is verified by
the existing test rather than by a new one, and `micold-core`'s `frame_probe` scene
(`frame_probe.rs:169`) is available if a measurement is wanted. No new work.

---

## R9. Sequencing

Story 1 is a precondition for Story 2 (the spec says so, and R4's reader analysis confirms it: a
field cannot leave the root while the root arm that writes it still names it). Within Story 1,
features are ordered smallest-first so the pattern is proven cheap before it is applied to the
37-variant one:

`help` (3) → `window` (2) → `notifications` (2) → `settings` (10) → `sidebar` (10) →
`connection` (12) → `worktree` (18) → `project` (19) → `session` (37)

`worktree_form` needs no conversion. `connection` is placed after `settings` deliberately: it is the
first feature whose entry point is the shell's rather than the reducer's (R3), so the two-shape rule
is exercised on a 12-variant feature rather than discovered on the 37-variant one.

FR-006 requires each conversion to leave the tree buildable and green, so each feature is one
commit and the guards land **after** the conversions they describe — a guard that has to be
relaxed to let its own migration through is not holding anything (`feature_registration_cost.rs`
makes the same argument about `is_shell` knowing about `shell/` before it existed).

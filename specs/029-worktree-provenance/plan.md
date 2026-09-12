# Implementation Plan: Worktree Provenance

**Branch**: `feat/worktree-provenance` | **Date**: 2026-09-03 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/029-worktree-provenance/spec.md`

## Summary

Feature 014 decides whether a worktree belongs to an AI assistant by reading its *name*
(`agent-<16+ hex>` / `worktree-agent-<16+ hex>`). That catches the assistant's sub-agent isolation
worktrees and nothing else — its **session** worktrees land in the same `.claude/worktrees/` root
under ordinary human-chosen names and are indistinguishable from the user's own.

This feature inverts the rule. The daemon creates every worktree the user makes, so it records that
at creation, and classification becomes *the app has a record for this* (user-owned) versus *it does
not* (assistant-owned, hidden exactly as 014 hides). Nothing 014 defined about what hiding *does*
changes; only the set it applies to.

Technically that is five moves:

1. **A record**: `Workspace.worktree_provenance` — a per-project set of directory names, keyed and
   persisted exactly like the existing `worktree_names` overrides, in the same per-project state
   file, disposed of by the same `forget()` (FR-002, FR-009).
2. **A classifier**: 014's name-only `Worktree::owner()` is replaced by a pure core function taking
   the worktree plus that record set plus the managed root. It is the *only* hiding signal; the 014
   name rule survives solely as the FR-007a veto inside the migration (FR-004…FR-007b).
3. **A one-time migration**, daemon-side, at the first discovery of a project after the upgrade:
   backfill a record for every discovered worktree the app already has evidence for — a display-name
   override, or a session bound to it — minus the name-veto, then set a per-project done marker
   (FR-006…FR-006d). The daemon's own catalog snapshot already computes precisely this evidence set,
   so the rule is not new machinery, it is an existing expression given a second reader (research R2).
4. **A claim**: a new `WorktreeClaim` client message writing the same record from a revealed row,
   modelled on `WorktreeRename`, which is already "durable catalog state, no git involved"
   (FR-020…FR-024).
5. **A count** on 014's reveal chip: `ToggleChip` gains a `count` builder step so "Show agent
   worktrees" can read `Show agent worktrees · 3` without a second widget (FR-025…FR-025b,
   Principle VIII).

The two guarantees that shape every decision below: **nothing on disk is touched to establish
provenance** (FR-003, FR-018), and **a storage failure fails visible** — an unreadable project state
classifies every worktree as the user's and writes nothing, rather than hiding the user's work
(FR-011).

## Clarifications incorporated

| # | Question | Resolution | Where it lands |
|---|---|---|---|
| 1 | What happens to worktrees already on disk with no record? | Selective backfill from evidence the app already holds | FR-006, research R2, `contracts/provenance-store.md` §4 |
| 2 | Does 014's naming rule survive? | Only as a veto on that backfill | FR-007a, `contracts/worktree-classification.md` §3 |
| 3 | Surface the assistant's live in-use lock? | Out of scope, its own feature | research R7 (rejected) |
| 4 | Is a misclassified worktree permanently recoverable? | Yes — a claim action on a revealed row | FR-020, `contracts/claim-and-reveal.md` §1 |
| 5 | Does the user-visible wording change? | No — 014's `agent` strings stand | FR-015a, `contracts/claim-and-reveal.md` §3 |
| 6 | Does the evidence rule run once or continuously? | Once per project, recorded done | FR-006c/006d, research R3 |
| 7 | Is the user told when the migration hides rows? | No notice; a count on the reveal control | FR-025, `contracts/claim-and-reveal.md` §2 |

## Technical Context

**Language/Version**: Rust, `stable` pinned by `rust-toolchain.toml` (both `mise run` and bare
`cargo` resolve to it).

**Primary Dependencies**: `iced` 0.14 (client only), `serde`/`serde_json` (persistence),
`tokio` (daemon). No new dependency is introduced by this feature.

**Storage**: The existing local-first JSON store — `projects.json` plus one per-project state file
each (`micold-core/src/store.rs`). Two new `#[serde(default)]` fields on `StoredProjectState`
(`created_worktrees`, `provenance_migrated`); the catalog file's own schema is untouched, so an
older build reading a newer store simply ignores them and a newer build reading an older store sees
an unmigrated project — which is exactly the FR-006 case.

**Testing**: `cargo test --workspace` (`mise run test`); logic-only iterations via
`mise run test-core`. Classification, the migration rule and the record's lifecycle are all
render-free and belong in `crates/micold-core/tests/`; state-level visibility and the claim reducer
in `crates/micold-client/tests/`; the RPC and the migration's trigger point in
`crates/micold-daemon/tests/`.

**Target Platform**: Linux, macOS, Windows desktop — identical behaviour on all three. Nothing here
is platform-specific: it is a `BTreeMap` in a JSON file and a comparison against a path prefix the
app already computes (`worktrees_root`).

**Project Type**: Desktop application — three-crate Rust workspace (render-free core, iced client,
session daemon).

**Performance Goals**: Classification stays O(1) per worktree per refresh — a `BTreeSet<String>`
lookup replacing a string-prefix scan (SC-006). The migration is one pass over a single project's
discovered worktrees, once, at a point that has already run a git subprocess.

**Constraints**: No filesystem or git operation may be performed to establish, verify, or maintain
provenance (FR-003, FR-018, SC-005). The daemon remains the single writer of durable state. Hiding
remains presentation-only: discovery still puts every worktree into `State::worktree.worktrees`
(FR-016).

**Scale/Scope**: Tens of worktrees per project, a handful of projects. One new protocol message, one
new `Workspace` field plus one marker set, one classifier replacing another, one builder step on an
existing shared widget, one new row action.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)** — PASS. Everything decisive here is render-free and
  testable without a window: the classifier is a pure function of (worktree, record set, root); the
  migration is a pure function of (discovered worktrees, evidence, marker) → records to write; the
  claim is a reducer. Each gets its failing test first, in `micold-core` or `micold-client` tests.
  The constitution's GUI-wiring exception is claimed for exactly one thing — passing the count into
  `ToggleChip` at the `sidebar.rs` call site — and the count *value* is computed by a `State` method
  that is unit-tested (mirroring 014, which put `has_visible_worktrees` in `State` for this reason).

- [x] **II. Multi-Session Support** — PASS. Provenance is per *worktree*, not per session, and no
  session state is added, moved, or made to depend on it. Sessions remain independently addressable
  in a hidden worktree exactly as under 014 (spec edge case: "a session recorded against a now-hidden
  worktree" is explicitly unchanged). The migration *reads* session records as evidence and never
  writes them.

- [x] **III. Worktree Integration** — PASS, and strengthened. The app still owns create/switch/
  cleanup with no manual git; this feature adds a record to the create path and removes it on the
  delete path, so the record's lifetime is the app-managed worktree's lifetime. The "Default" root
  is explicitly outside classification (FR-017) — the sanctioned non-worktree location is
  untouched. FR-003 goes further than the principle requires: no git operation at all is added.

- [x] **IV. Local-First Storage (NON-NEGOTIABLE)** — PASS. The record is two new fields in the
  per-project JSON state file already on the local disk. Nothing leaves the device; the feature works
  with no network at all; the daemon remains the single writer.

- [x] **V. Rust + iced Stack** — PASS. Rust and iced only. The type system carries the invariant:
  `WorktreeOwner` stays an enum (014's reasoning holds), the classifier takes the record set as an
  explicit parameter so a caller *cannot* classify without supplying provenance, and the
  fail-visible state is a distinct value rather than an empty-set-means-two-things overload
  (research R5) — the mis-classification FR-011 forbids is made unrepresentable rather than
  guarded against at each call.

- [x] **VI. Cross-Platform Parity** — PASS. No `cfg`, no platform path handling beyond the existing
  `worktrees_root`. CI runs the full suite on all three, and this change can affect what is built, so
  no documentation-only exemption is claimed.

- [x] **VII. Documentation First-Class** — PASS. FR-019 requires
  `docs/user-guide/worktrees-and-sessions.md` to be updated in this same change: the rule is now
  "worktrees the app did not create are hidden", assistant session worktrees are therefore hidden
  too, the count beside the reveal control is what it means, and a hand-made worktree is reached by
  revealing and kept by claiming.

- [x] **VIII. Reusable UI Component Foundation** — PASS. The only widget change is a `count(usize)`
  builder step on the *existing* shared `ToggleChip` (`ui/material/toggle_chip.rs`, promoted by 014
  precisely so the reveal chip would not fork one), chainable and terminating in `.into()`. The claim
  action reuses the existing row-menu entry shape; no forked widget, no feature-local one-off.

### Post-design re-check (after Phase 1)

Re-evaluated against `data-model.md` and the three contracts: still PASS on all eight, with two
things worth recording because the design made them sharper rather than looser.

**Principle I** got stricter, not laxer. `contracts/worktree-classification.md` states the classifier
as a total function over an explicit input triple, and `contracts/provenance-store.md` §4 states the
migration as a pure `plan_backfill()` that *returns* the records to write rather than writing them —
so the daemon's I/O is a separate, trivially-mockable step and the whole of FR-006…FR-006d is
covered by core tests with no daemon in the loop. That was not forced by the constitution; it is what
made the migration testable at all.

**Principle V** is where the design earned its keep. The first shape considered put a
`user_created: bool` on `micold_core::worktree::Worktree` itself. It types fine and it is wrong:
`Worktree` is what discovery *found*, and provenance is what the app *remembers* — fusing them means
every construction site of a discovery value has to invent an app-state answer, and `FakeGit` would
be inventing provenance. The contract instead keeps `Worktree` a discovery fact and passes the record
set alongside it (research R4). Related: `ProjectStateLoad::Corrupt` currently vanishes into a
silent drop, and FR-011 needs it to survive as far as classification — `data-model.md` §4 gives it a
non-persisted `unreadable` marker rather than letting "no records" mean both "nothing recorded yet"
and "records lost", which is the single most dangerous overload in this feature.

No entry is required in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/029-worktree-provenance/
├── plan.md                              # This file
├── spec.md                              # Feature specification (clarified, 7 answers)
├── research.md                          # Phase 0 — 9 decisions
├── data-model.md                        # Phase 1 — the record, the marker, the read-failure flag
├── quickstart.md                        # Phase 1 — how to prove it works
├── checklists/requirements.md           # Spec-quality checklist (16/16)
└── contracts/                           # Phase 1
    ├── provenance-store.md              # Storage, wire, daemon writes, the migration
    ├── worktree-classification.md       # The classifier that replaces 014's name rule
    └── claim-and-reveal.md              # The claim action, the row, the hidden count
```

### Source Code (repository root)

```text
crates/micold-core/                      # Render-free core: no iced, no PTY
├── src/
│   ├── worktree.rs                      # CHANGED: classify_owner() takes provenance, not names;
│   │                                    #   AGENT_* constants survive only as the FR-007a veto;
│   │                                    #   Worktree::owner()/is_agent_owned() are removed
│   ├── workspace.rs                     # CHANGED: worktree_provenance + provenance_migrated +
│   │                                    #   unreadable_projects; forget() disposes of all three
│   ├── store.rs                         # CHANGED: StoredProjectState gains two serde(default)
│   │                                    #   fields; ProjectStateLoad::Corrupt marks unreadable
│   └── protocol/messages.rs             # CHANGED: ClientMsg::WorktreeClaim;
│                                        #   WorktreeSnapshot gains user_created
└── tests/
    ├── worktree_provenance.rs           # NEW: classification truth table + the veto
    ├── provenance_migration.rs          # NEW: plan_backfill() — evidence, veto, idempotence, once
    ├── workspace.rs                      # CHANGED: forget() drops provenance
    ├── store_roundtrip.rs               # CHANGED: the two new fields; corrupt ⇒ unreadable
    └── protocol_roundtrip.rs            # CHANGED: the new message + snapshot field

crates/micold-daemon/                    # Single writer of durable state
├── src/
│   ├── catalog.rs                       # CHANGED: record/forget provenance; run + mark the
│   │                                    #   migration; project the flag into the snapshot
│   ├── state.rs                         # CHANGED: refresh_worktrees() is the migration's trigger;
│   │                                    #   snapshot_locked() overlays user_created
│   └── server.rs                        # CHANGED: WorktreeCreate records; WorktreeDelete forgets;
│                                        #   WorktreeClaim (new arm, modelled on WorktreeRename)
└── tests/
    ├── worktree_claim.rs                # NEW: the RPC, its errors, its broadcast
    └── worktree_provenance_rpc.rs       # NEW: create records, delete forgets, migration runs once

crates/micold-client/                    # iced GUI + render-free feature slices
├── src/
│   ├── features/worktree.rs             # CHANGED: visible_worktrees()/worktree_tags() consult
│   │                                    #   provenance; Msg::ClaimRequested; hidden_worktree_count()
│   ├── features/sidebar.rs              # UNCHANGED behaviour: the reveal toggle and its
│   │                                    #   project-switch reset are 014's, untouched
│   ├── catalog_sync.rs                  # CHANGED: mirror user_created into worktree_provenance,
│   │                                    #   exactly as display names are mirrored today
│   ├── ui/material/toggle_chip.rs       # CHANGED: .count(usize) builder step (Principle VIII)
│   └── ui/sidebar.rs                    # CHANGED: reveal chip passes the count; row menu gains
│                                        #   the claim entry on an assistant-owned row
└── tests/
    ├── worktree_visibility.rs           # NEW: hidden set, the count, out-of-root, Default
    ├── worktree_claim.rs                # NEW: the claim reducer and its immediacy
    ├── app_state.rs                     # CHANGED: 014's reveal assertions still hold
    └── sidebar_tree.rs                  # CHANGED: agent chip now driven by provenance

docs/user-guide/worktrees-and-sessions.md  # CHANGED: FR-019
```

**Structure Decision**: The existing three-crate workspace, unchanged. The split is the one the
constitution's Principle I implies and this repo already enforces: every decision — what a worktree
is classified as, what the migration would write, what the count is — lives in `micold-core` or in a
`micold-client` feature slice where it is testable without a renderer; `ui/` only passes values it
was handed. The daemon is where the record is *written*, because it is already the single writer of
durable state and the place `create_worktree` is called from.

## Complexity Tracking

> No Constitution Check violations. This section is intentionally empty.

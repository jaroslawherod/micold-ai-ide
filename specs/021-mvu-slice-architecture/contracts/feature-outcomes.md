# Contract: Feature Outcomes (Tier 3)

**Feature**: 021 | **Satisfies**: FR-020 – FR-024a, SC-007

The contract governing cross-feature effects. It exists to keep Tier 1's boundaries from eroding
back into a monolith one convenient reach-across at a time.

## The rule

| Direction | Permitted? | Mechanism |
|---|---|---|
| Feature **reads** another feature's data, for display | **Yes** — must stay possible and cheap | Direct read from shared state (FR-003a) |
| Feature **writes** another feature's data | **No** | Return an outcome; the root applies it (FR-020, FR-021) |

The asymmetry is deliberate and load-bearing. The spec's Edge Cases require a view to render from
two features' data — the sidebar reads session data today — which is why the shared state struct is
**not** partitioned into mutually invisible halves. Isolation is enforced on writes by guard test,
not on reads by type (FR-024a). See plan.md §Complexity Tracking for the recorded deviation from
Principle V this represents.

## Shape

```rust
// A feature reducer with no cross-feature consequence returns nothing (FR-021).
fn update(state: &mut State, msg: SidebarMsg) { … }

// One with a consequence returns it, and does not apply it.
fn update(state: &mut State, msg: WorktreeMsg) -> Vec<Outcome> { … }
```

Carrying an outcome vocabulary is **not** blanket plumbing — a reducer that touches only its own
data must not be forced to declare one (FR-021).

## Obligations

| # | Obligation | Requirement |
|---|---|---|
| O1 | A feature reducer mutates only its own feature's data | FR-020 |
| O2 | Cross-feature consequences are returned, never applied in place | FR-021 |
| O3 | The root reducer is the only interpreter of outcomes | FR-022 |
| O4 | Interpretation terminates | FR-024 |
| O5 | Interpretation does not depend on the order feature modules are composed in | FR-024 |
| O6 | O1 is enforced by a guard test that **names the offending path** on failure | FR-024a, SC-007 |

**O6's naming requirement is not cosmetic.** A guard that reports only "a cross-feature write
exists" sends the next maintainer hunting through nine feature modules. The failure message must
identify the writing path.

## Termination (O4)

Interpreting one outcome may produce another — the spec's Edge Cases name this case explicitly.

**Contract**: the root drains a work queue with a fixed iteration bound. Exceeding it panics in
debug and logs a no-op in release, so a cycle fails loudly under test rather than hanging the UI.

**Order independence (O5)**: outcomes are applied in emission order, and the set of outcomes a
feature emits must not depend on which other features exist or where they sit in the composition.

## Known outcomes at plan time

| Outcome | Emitted by | Interpreted as |
|---|---|---|
| `SessionsClosed(Vec<SessionId>)` | Worktree delete | Session feature closes them |
| `OverlayDismissed(SurfaceId)` | Worktree delete | Registry dismisses the surface |
| `ClipboardWrite(String)` | Any feature | Shell issues `iced::clipboard::write` (see service-capabilities.md) |
| `NotificationRaised(Notification)` | Any feature | Notification queue push |

The list is expected to grow during step 19 as the guard test (step 20) surfaces existing
cross-feature writes. That is the intended discovery mechanism, not a planning gap.

## The named anti-pattern: worktree delete

Deleting a worktree today reaches directly into session state and overlay state from inside the
worktree code path (FR-023, spec User Story 4).

**Required end state**: the delete path mutates only worktree data and returns
`SessionsClosed` + `OverlayDismissed`. The root applies both. The end-to-end observable result is
identical.

**Arbiter**: `crates/micold-client/tests/worktree_delete.rs` must pass **unchanged** (FR-027). It is
the proof that the refactor is behavior-preserving; if it needs an edit, the refactor is wrong.

## Verification

| Test | Holds | Status |
|---|---|---|
| `worktree_delete.rs` | Behavior preservation through the outcome conversion | Exists — frozen |
| `logical_state_ownership.rs` | Existing ownership assertions | Exists — frozen |
| `component_state_isolation.rs` | Existing isolation assertions | Exists — frozen |
| `feature_write_isolation.rs` | O1, O6 | **New** |
| `outcome_termination.rs` | O4, O5 | **New** |

`showcase_isolation.rs` is the precedent for the new guards' mechanism — the spec names it as a
worked example of enforcing an architectural line by test rather than by type.

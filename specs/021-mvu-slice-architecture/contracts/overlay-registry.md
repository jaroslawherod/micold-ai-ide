# Contract: Overlay Registry (Tier 2)

**Feature**: 021 | **Satisfies**: FR-007 – FR-014, SC-001

The internal contract between a floating surface and the generic dispatch that renders, dismisses
and stacks it. This is the interface a maintainer programs against when adding a dropdown or dialog.

## Registration

Adding a surface MUST cost its own module plus **at most one** registration line, and MUST require
editing **zero** central match statements (SC-001, down from six).

```rust
// features/my_thing.rs — everything about the surface lives here
impl FloatingSurface for MyThing {
    fn id(&self) -> SurfaceId;
    fn band(&self) -> StackBand;              // which stacking band
    fn dismissal(&self) -> DismissalRules;    // what closes it
    fn view(&self, scheme: &Roles) -> Element<'_, Message>;
    fn snapshot(&self) -> Self;               // cleared-state copy for the exit animation
}

// overlay/registry.rs — the single registration point
register!(MyThing);
```

### Obligations

| # | Obligation | Requirement |
|---|---|---|
| R1 | A registered surface is reachable by generic dispatch with no change to any enum or match | FR-008, FR-009 |
| R2 | An unregistered surface fails the build or a guard test — never silently at runtime | FR-010 |
| R3 | Registration order does not affect behavior | FR-024 |
| R4 | The builder chain terminates in `.into()` | FR-030, Principle VIII |

**R2 is the load-bearing one.** Today, forgetting one of eight edit sites produces an overlay that
opens but will not close, discovered by hand. The guard test must convert that into a build failure.

## Dismissal

`DismissalRules` declares which triggers close a surface. The generic dispatch consults it; no
per-surface branch exists anywhere else.

| Trigger | Applies to | Preserved from |
|---|---|---|
| Escape key | All | `on_escape` + the `ui/mod.rs` keyboard mirror |
| Outside click | Popovers, and modals that permit it | Existing per-surface behavior |
| Scroll beneath | Popovers | `dismiss_on_scroll_beneath` |
| Explicit message | All | Per-surface cancel messages |

### Ordering obligations

| # | Obligation | Requirement |
|---|---|---|
| D1 | When a popover and a modal are both open, Escape closes the **popover** first | FR-012 |
| D2 | Opening a modal closes all lightweight popovers | FR-012 |
| D3 | Dismissal MUST NOT alter state the dismissal does not own — closing the sidebar filter panel leaves active filters intact | FR-013 |

D1 is currently implemented as a hand-written check placed ahead of the `Overlay` match
(`ui/mod.rs:554`). It must survive as an explicit, tested rule of the generic dispatch, not
disappear with the special-case match.

## Exit-animation snapshot

The application renders a **copy** of a surface whose live state has already been cleared, so it can
animate out. The unified representation MUST preserve this (FR-011).

| # | Obligation | Requirement |
|---|---|---|
| A1 | A dismissed surface renders from its snapshot until its transition completes | FR-011 |
| A2 | Reopening the same surface while it animates out produces the pre-change behavior exactly | FR-011 |
| A3 | The snapshot holds no live reference to feature state | FR-011 |

**This is the riskiest obligation in the feature** (research.md §9). The `ClosingOverlay` enum
exists solely to serve it, and collapsing it into the surface type is where behavior is most likely
to shift unnoticed.

## Verification

Held by existing tests, none of which may be modified (FR-027):

| Test | Holds |
|---|---|
| `one_overlay_implementation.rs` | One implementation, not two parallel ones (FR-014) |
| `overlay_dismissal_delta.rs` | D1, D2, D3 |
| `overlay_stacking.rs` | Band ordering |
| `overlay_transition_identity.rs` | A1, A2, A3 |
| `about_open.rs`, `about_dismiss.rs`, `project_switcher.rs`, `switcher_forget_menu.rs`, `local_interactions.rs` | Per-surface behavior |

New:

| Test | Holds |
|---|---|
| `overlay_registration.rs` | R2 — an unregistered surface fails |

## Migration checkpoint

The six central sites that must reach zero (SC-001):

1. `Overlay` enum — `app.rs:55`
2. `on_escape` match — `app.rs:2322`
3. Keyboard-subscription escape mirror — `ui/mod.rs:519`
4. View match — `ui/mod.rs:337`
5. `capture_overlay` snapshot match — `main.rs:727`
6. `ClosingOverlay` enum — `app.rs:2387`

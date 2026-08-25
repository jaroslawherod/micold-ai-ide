# Behaviour Delta: Material Component Architecture

**Feature**: `specs/017-material-component-architecture` | **Task**: T015 | **Date**: 2026-07-27

This feature is defined by changing nothing the user can see. It makes **one** exception, sanctioned
by FR-024 and scoped to dismissal alone: the five floating-surface implementations each answered
"when does this close?" for themselves, and consolidating them onto one primitive means adopting one
answer.

**This document is the complete list.** Anything not on it is a defect, not a change.

Rows 1–4 follow from that consolidation. **Row 5 does not**: it is a pre-existing defect this
feature's branch happened to fix, listed because the list promises completeness, not because
FR-024 sanctions it.

The executable version is `crates/micold-client/tests/overlay_dismissal_delta.rs` — every dismissal
row below has a test there, including the rows that assert nothing moved. Row 5's tests live with
the component that fixed it, in `crates/micold-client/src/ui/material/ellipsized.rs`.

---

## What changed

### 1. A dialog now dismisses on a scrim click

| | Before | After |
|---|---|---|
| Escape | closes | closes |
| Click the scrim | swallowed, ignored | **closes** |
| Scroll behind it | ignored | ignored |

**Why**: every other floating surface in the application closed on an outside click. The dialog was
the one that did not, and the reason was not a decision — it was that `Modal` had never been given a
cancellation message to emit.

**How it cannot drift**: the scrim emits whatever `app::on_escape` would produce for the open
overlay. Escape and the scrim are now two gestures reading one rule, rather than two implementations
of the same intent.

**Surfaces affected**: About, project selector, rename project, add worktree, Settings, confirm
worktree delete, rename worktree, confirm session remove, confirm forget project.

**Considered and rejected**: marking the drafting dialogs (add worktree, Settings, the renames)
`NonDismissibleDialog` so a stray click could not discard input. Rejected because Escape already
discards those drafts today and always has — treating a scrim click as more dangerous than Escape
would be an inconsistency invented to justify an exception. The variant stays in the core, unused,
for a dialog that genuinely needs it.

### 2. Non-modal surfaces now dismiss when content scrolls beneath them

| | Before | After |
|---|---|---|
| Click outside | closes | closes |
| Escape | closes | closes |
| Scroll the list beneath | nothing happened | **closes** |

**Why**: a menu opened from a worktree row is stale the moment the rows move. Before this change no
widget reported a scroll at all, so no surface *could* react — the behaviour was absent rather than
chosen.

**Surfaces affected**: overflow menu, project switcher, sidebar filter panel, project context menu,
worktree context menu, session context menu.

**How it is wired**: the sidebar's scrollable reports every scroll unconditionally
(`Message::SidebarScrolled(offset)` — this named `Message::ScrolledBeneathOverlay` until feature
021's T081, which found that variant had no producer and deleted it; the reducer arm and the rule
below are the same ones); the reducer asks `micold_core::overlay::dismisses` whether anything
closes. With nothing open the message is inert.

### 3. The folder browser's scrollbar is now the design system's

| | Before | After |
|---|---|---|
| Sidebar list scrollbar | themed, 4px, 1px margin | unchanged |
| Folder-browser list scrollbar | the rendering stack's default | **themed, 4px, 1px margin** |

**Why**: nobody chose the difference. The sidebar's scrollable was written with an explicit style
and the folder browser's without one, and they have looked different ever since. Wrapping the
scrollable meant picking one, and the design system already specifies which.

**This is a visual change**, and therefore the one exception to this feature's zero-visual-change
property that is not about dismissal. It is listed here rather than in the parity walkthrough
because "the app looks identical" must not be quietly true-except-for-this.

**Considered and rejected**: giving the wrapper an `unthemed()` step so the folder browser could
keep its default scrollbar. That preserves an accident by making it a supported option, which is the
opposite of what wrapping is for.

### 4. A click during the overflow menu's fade-out no longer reopens it

| | Before | After |
|---|---|---|
| Click while the menu is open | closes it | closes it |
| Click during the ~90ms fade-out | **swallowed, and reopened the menu** | passes through to what is beneath |
| Click once the fade has finished | passes through | passes through |

**Why**: the menu's backdrop — the layer that turns an outside click into a dismissal — used to
exist for as long as the panel was drawn, including the 90ms it spent fading out. A click landing in
that window was read as "outside the open menu" and emitted `HelpMenuToggled`, which, with the menu
already closed, opened it again.

Making the panel self-animating (T039a) separated the two lifetimes: the panel is still on screen
because it is finishing its fade, but the surface only carries a dismissal while the menu is
actually open. There is no longer a window in which a click means something the user did not intend.

**Surfaces affected**: the toolbar overflow menu — the only fading one. Context menus and the
project switcher appear and disappear without a transition, so they never had the window.

**This is a fix, not a preserved behaviour**, and it is listed here because it is user-noticeable:
double-clicking the overflow-menu button used to be able to leave the menu open.

---

### 5. An over-long sidebar name now ends in an ellipsis

A session or worktree name too long for its row used to run past the end of the row and collide
with the close button, drawing over it. It now stops short and ends in `…`.

**Surfaces affected**: every row in the sidebar tree — sessions, worktrees and the project header.

**This one is different from the four above**, and the difference is worth stating rather than
smoothing over.

The other four follow from consolidating the overlays: this feature caused them, and FR-024
sanctions them. This one is a **pre-existing defect that this feature's branch happened to fix**.
The cause dates to 2026-07-16: the label asked for `Wrapping::None`, which stops text wrapping but
does not clip it — the rendering stack draws a paragraph past its layout node quite happily. The
feature's only edits to `tree_view.rs` before the fix were import lines.

So it is not a refactor regression. It is on this list anyway, because the list promises to be
complete and a user comparing the two builds sees a difference. "Nothing visible changed except
these things" is the single claim this feature asks a reviewer to trust, and a claim with a silent
exception is worth less than no claim.

**How it was fixed**: `material/ellipsized.rs`, a component that measures the text at layout time
and binary-searches the longest prefix that fits, rather than hard-clipping. A hard clip would cut
a glyph in half and give the user no signal that a name had been shortened.

`Ellipsized` is a component no requirement, plan entry or task called for — unplanned work that
landed mid-feature. It is nonetheless inside the boundary and covered by the gates, because
`material_builder_api` and `component_api_opacity` both scan the component directory rather than an
enumerated list. That is the difference between a boundary and a checklist: a checklist would have
missed it.

---

## What did not change

Listed because "we only changed dismissal" is a claim, and a claim needs the boundary drawn.

- **Escape reaches exactly what it used to.** Every overlay's Escape message is unchanged.
- **Outside-click dismissal of a menu** is not new; it is the behaviour the others were unified
  *onto*.
- **A dialog still survives a scroll behind it.** Gaining a scrim click must not turn a dialog into
  a menu.
- **Nothing about appearance.** Every scrim colour, panel width, offset and padding is byte-identical
  — `crates/micold-client/tests/style_snapshot.rs` asserts the resolved styles and would fail
  otherwise.
- **Stacking order.** Today's z-order is preserved exactly (popovers, then context menus, then
  dialogs). What changed is *why* it holds: it is now a property of each surface's layer rather than
  of the order `ui::view` composes them in (FR-010). No pair of surfaces reversed.

---

## Deviations from the task text

**T014 lists five surfaces to migrate; four moved onto the primitive.**

The **select dropdown** (`ui/material/select.rs`) did not, and should not. It is built on iced's
`pick_list`, which implements the rendering stack's own `Widget::overlay()` — its dropdown is
positioned from the trigger's on-screen bounds by the stack's overlay system. That is what makes it
work at all: the trigger lives inside a content-sized dialog, where a window-level floating surface
has no fill-sized window to anchor against. This is precisely the failure that made the hand-rolled
`SelectOverlay` reveal its list inline, and moving it back onto a stack-based primitive would
reintroduce it.

The honest framing: there is one primitive for **window-level** floating surfaces, and
widget-attached dropdowns delegate to the rendering stack's own overlay system, which is itself a
single shared implementation. Four hand-rolled implementations became zero; `select.rs` never had
one to remove.

**The terminal context menu is mounted on the terminal pane, not the window.** Its anchor point is
pane-local and the pane's origin is not known at render time, so there is nothing to translate the
point by. Same primitive (`cdk::overlay::Overlay`), mounted one level down. Its dismissal follows the
same rule.

---

## T039b/T041: the resize drag no longer needs a full-window capture layer

The sidebar's resize handle is 6px wide, and a pointer leaves a 6px target almost immediately once
it starts moving. The previous implementation handled this by mounting a **transparent capture layer
over the entire window** for the duration of the drag: `ui::view` stacked a full-size `mouse_area`
whenever `state.sidebar_dragging` was set, and that layer — not the handle — tracked the cursor and
ended the drag.

The handle now owns the drag itself. A widget's `update` receives every mouse event, not only the
ones over its own bounds, so a handle that remembers it is being dragged can follow the pointer
anywhere on screen without anything being laid over the window.

**Two user-visible consequences**, both improvements, both worth stating rather than discovering:

- **Events during a drag no longer pass through a capture layer.** The old layer sat above
  everything and swallowed what it did not use. Anything under the pointer mid-drag now sees events
  normally, apart from the cursor movement the handle captures.
- **The hover highlight stays lit while dragging away from the edge.** Previously the highlight was
  driven by `on_enter`/`on_exit` on the handle's own `mouse_area`, so pulling the pointer away
  during a drag unlit the edge while it was still being moved. It now stays lit until release.

## T041: a reported width is no longer gated on a drag flag

`Message::SidebarDragMoved` used to be ignored unless `state.sidebar_dragging` was set, because a
full-window layer was emitting it and the reducer needed to know whether a drag was genuinely in
progress. `SidebarDragStarted` and `SidebarDragEnded` existed only to maintain that flag.

With the handle owning the drag, it emits a width **only** while being dragged, so the flag has no
remaining purpose and both messages are gone. The reducer now adopts any width it is given.

This is a real weakening of a reducer-level invariant, and it is deliberate: the guard was
compensating for an emitter that could not be trusted to speak only when it meant to. Clamping to
`SIDEBAR_MIN_WIDTH`/`SIDEBAR_MAX_WIDTH` stays in the reducer, where it belongs — how wide the
sidebar may be is a decision about the application's layout, not about the edge being dragged.

## T042: the animation clock is gone

The application no longer runs a 60fps `AnimationTick` subscription. Every transition is played by
the component that owns it, and a self-animating widget asks the runtime for its next frame only
while it is actually moving.

The previous arrangement already gated the clock on `motion_animating(app)`, so an idle window was
not ticking — the difference is that there is now no central clock to gate, and no global
enumeration of what might be animating. `MotionKey`, `Animator`, and the whole
`micold_client::motion` module are deleted.

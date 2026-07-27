# Behaviour Delta: Material Component Architecture

**Feature**: `specs/017-material-component-architecture` | **Task**: T015 | **Date**: 2026-07-27

This feature is defined by changing nothing the user can see. It makes **one** exception, sanctioned
by FR-024 and scoped to dismissal alone: the five floating-surface implementations each answered
"when does this close?" for themselves, and consolidating them onto one primitive means adopting one
answer.

**This document is the complete list.** Anything not on it is a defect, not a change.

The executable version is `crates/micold-client/tests/overlay_dismissal_delta.rs` — every row below
has a test, including the rows that assert nothing moved.

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
(`Message::ScrolledBeneathOverlay`); the reducer asks `micold_core::overlay::dismisses` whether
anything closes. With nothing open the message is inert.

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

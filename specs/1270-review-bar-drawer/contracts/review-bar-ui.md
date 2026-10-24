# Contract: review bar (UI)

**Spec**: FR-001–FR-009, SC-001 | **Data model**: `review::bar`, client `review_bar`

Each item is testable; the column says at which layer.

| ID | Behaviour | Test layer |
|---|---|---|
| B1 | Shown only while an entry's Review tab is shown, as the last row of that tab's body, directly above the status bar, full tab width, `anatomy::review_bar::HEIGHT` (44) high; absent while a session tab is shown (FR-001). | client state (`review_bar` is `None` for a session tab); layout snapshot |
| B2 | Leading part: the counts text of the shown scope, in the longest `CountsForm` that fits (Full → DraftsOnly → Hidden), then the error message when set, with a × that dismisses it (FR-002, FR-006, R3). | core unit (`counts_text`); component test (form choice at widths); layout snapshot |
| B3 | Trailing part, right to left from the bar's edge: **Send review (n)** (fixed `SEND_WIDTH`), the overflow trigger (⋮), **Comments** toggle. Their bounds do not depend on drawer state, counts, labels or the error (FR-003, SC-001, R4). | geometry gate `tests/gates/review_bar_fixed.rs` |
| B4 | **Send review (n)**: `SendButton` from core — filled and enabled when n > 0 and sendable; "Sending…" disabled while any window sends the entry; disabled when the shown pull request is not in the list (FR-003, FR-005). | core unit; client state |
| B5 | Pressing it emits today's `Effect::ReviewSend` unchanged in content (FR-004), clears the bar's error, and starts the drawer's send watch with the sent drafts' ids. | client state |
| B6 | A failed send of this view sets the error "Couldn't send the review: *why*" (and today's snackbar); it stays until the next send of this view or the ×. A delivered send keeps today's success snackbar, wording unchanged (FR-006). | client state; shell glue via quickstart §B |
| B7 | Overflow menu items **Clear sent** and **Discard drafts…**, enabled per `Overflow` (FR-007); Clear sent emits today's clear for the shown scope; Discard drafts… opens today's confirmation. The menu is the app's own `MenuOverlay`, registered as a popover: Esc or an outside click closes it, changing nothing else (Story 2 #4). | core unit (`overflow`); client state; overlay registry test |
| B8 | **Comments** toggle shows the drawer state (active when open) and applies `DrawerEvent::Toggle` (FR-008). | client state |
| B9 | The Changes header no longer holds **Review (n)** or **Send review**; the panel's buttons and counts are gone (FR-009). | layout snapshot; quickstart §B |
| B10 | Light and dark: the bar uses existing roles only (surface container, outline divider on top); identical geometry in both schemes. | layout snapshot (other scheme identical); visual pass |

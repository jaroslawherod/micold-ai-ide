# Quickstart & Validation: Forget from the Switcher's Right-Click Menu

The pure surface (menu toggle/replace, cursor anchoring, clamping, popover exclusion, hand-off
into feature 014's forget flow) is covered by `tests/switcher_forget_menu.rs` and the
event-mapping test in the binary. The GUI wiring below is validated by this recorded manual
procedure, per Constitution Principle I's GUI-wiring exception.

## Prerequisites

- `mise trust` once per fresh worktree.
- Automated suites green: `mise run test` and `cargo test --features gui --bin micold-ai-ide`.
- Launch: `mise run run`.
- Have at least two projects in the list; give one a running session to exercise the
  session-stopping path that feature 014 owns.

## Scenario A — Forget from the switcher (US1)

1. Open the **project switcher** in the top bar.
2. **Right-click** a project row. → A context menu with **Forget project** (trash icon)
   appears. ✅ (US1 AS1)
3. Note the switcher panel is **still visible behind** the menu, so you can see which row you
   right-clicked. ✅ (FR-009)
4. Choose **Forget project**. → The context menu closes and feature 014's confirmation opens,
   naming that project. ✅ (US1 AS2)
5. Click **Forget**. → The project disappears from the switcher **and** the Known projects list;
   the removal survives a restart. ✅ (US1 AS3)
6. Check the filesystem: the project's folder and git repository are **unchanged**. ✅

## Scenario B — Dismissing forgets nothing (US1 AS4)

1. Right-click a row to open the menu, then click elsewhere. → The menu closes; no confirmation
   appears and nothing is forgotten. ✅
2. Right-click, choose **Forget project**, then **Cancel** (or press **Esc**) on the
   confirmation. → The project remains. ✅

## Scenario C — Placement and clamping (US2)

1. Right-click a row near the **middle** of the panel. → The menu's top-left corner is at the
   pointer, so it opens below-right of the cursor. ✅ (US2 AS1)
2. With it open, right-click a **different row**. → The menu moves to the new click point. ✅
   (US2 AS2)
3. Make the window **small**, then right-click a row near the **bottom-right corner**. → The menu
   slides back inside and stays **fully visible**; it never hangs off the right or bottom edge. ✅
   (US2 AS3) Note this is clamping, not flipping — near an edge it sits under the cursor.
4. With the menu open near an edge, **resize** the window. → It re-clamps and stays fully
   visible. ✅ (US2 AS4)

## Scenario D — Edge cases

1. Right-click the **"Add project…"** row. → **No** menu appears. ✅ (FR-007)
2. Right-click a project marked **unavailable** (folder deleted on disk). → The menu appears and
   Forget project works. ✅ (FR-008)
3. With a project menu open, open the **overflow menu**, the **sidebar filter**, or a **worktree**
   context menu. → The project menu closes; only one popover is ever open. ✅
4. Close the switcher while a row menu is open. → Both close together. ✅

## Scenario E — Idle cost (SC-004)

With the switcher **closed**, move the pointer around the window. → No repaint churn or CPU
increase versus before the change; the cursor subscription is only active while the switcher is
open. ✅

## Success criteria mapping

| Criterion | Validated by |
|-----------|--------------|
| SC-001 (three interactions) | Scenario A steps 2, 4, 5 |
| SC-002 (always fully on-screen) | Scenario C steps 3–4 |
| SC-003 (identical to the list route) | Scenario A steps 5–6 vs the Known projects Forget button |
| SC-004 (no idle cost) | Scenario E |

# Quickstart: Notify When a Session Needs Attention, and Track Unread Sessions

**Feature**: 039 | **Plan**: [plan.md](./plan.md) | Contracts: [wire](./contracts/wire.md),
[desktop notification](./contracts/desktop-notification.md), [unread mark](./contracts/unread-mark.md)

## Prerequisites

- `mise trust` once in a fresh worktree.
- For §B: the `visual-pass` skill's private display, pinned binaries and private data directory.
  A notification service on that display's session bus (`dunst` or `mako`); without one, B1
  checks the FR-010 path instead.
- For §C: a macOS bundle from `mise run app` (ad-hoc signed) and an installed Windows build.

## §A — Automated

```sh
mise run test-core     # in_view, AttentionTracker, notification_text, resolve_reveal, counts, wire
mise run gate          # daemon Views and integration, client reducers, component gates, fmt, clippy
```

Expected: both green. The tests named in [plan.md](./plan.md#test-strategy-by-layer) exist and
fail without the change they cover.

## §B — Recorded pass on Linux (visual-pass)

Two projects, P with sessions A and B, Q with session C. Record a screenshot for each step that
names a look, in the light and the dark scheme.

| # | Do | Expect | Covers |
|---|---|---|---|
| B1 | Select A. Let B finish a turn. | One notification: title `<B> is waiting for input`, body `<P> — <worktree>`. | US1.1, FR-004 |
| B2 | Let A finish a turn with the window focused. | No notification, no mark on A. | US1.2, US2.2 |
| B3 | Focus another application. Let A finish a turn. Return. | One notification; A marked until the window regains focus. | US1.3, US2.6 |
| B4 | Open Settings. Let A finish a turn. Leave Settings. | One notification; A marked, then not. | US1.9, US2.11 |
| B5 | Let C finish a turn. | Notification naming Q. Button shows `● 1`. Panel: Q `● 1 unread`. | US1.4, US2.4, US2.15 |
| B6 | Let B finish a turn. | B's row: mark at the trailing edge, label emphasised, activity indicator unchanged. Panel: P `● 1 unread` beside its running count. Button still `● 1`. | US2.1, US2.5, US2.16, FR-018, FR-032 |
| B7 | Select B. | Mark and P's count gone within 1 s. | US2.3, SC-006 |
| B8 | Close the window. Let A finish a turn. Open the application. | No notification, then or now. A is marked unless it is the session shown. | US1.13, US2.18, SC-009 |
| B9 | Click C's notification from another application (X11). | Window in front and focused, Q active, C shown, C's mark gone. | US3.1 to US3.3, SC-004 |
| B10 | Remove C, then click an older notification for it. | Window in front, selection unchanged, notice `That session is no longer available.` | US3.4 |
| B11 | Open a second window on Q. Click a notification for a session of Q from the first. | The second window comes forward and shows it. | US3.6 |
| B12 | Settings → Environment: the **Desktop notifications** switch is on. Turn it off. Let B finish a turn while not in view. | No notification; B marked. Turn it on: the next change notifies. | US4.1 to US4.5 |
| B13 | Stop the notification service. Let B finish a turn. | No notification, no in-app notice, B marked, one `warn` line in the log for the run. | FR-010 |
| B14 | Showcase: the three `UnreadMark` entries, both schemes. | As in the contract. | FR-030 |

## §C — By hand

| # | Where | Do | Expect |
|---|---|---|---|
| C1 | macOS bundle | B1, B2, B9. Deny notifications in System Settings, repeat B1. | Shown and clicked as on Linux; when denied, B13's outcome. |
| C2 | Windows, installed build | B1, B2, B9; click a toast from the notification centre. | Shown under the application's name; the click shows the session. Record whether the notification-centre click is reported. |
| C3 | Linux, session service in a container | B1, B6, B9. | Same as with the service on the host (SC-007). |
| C4 | Linux, Wayland session | B9. | Focus given, or the window marked as needing attention; record which. |

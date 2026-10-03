# M5 visual pass (2026-10-03)

Xvfb `:131` + lavapipe (not a real display), private HOME, XDG dirs and pin dir (`~/vp/m5bin`), built from HEAD d98f1a71 (client, showcase, daemon from one build; strings checked on all three). Sessions are a fake `claude` (`sleep`); turns driven by POSTing `UserPromptSubmit` / `Stop` to each session's hook URL. No notification service ran, so B5's notification itself is not covered.

## A. Showcase, light and dark: PASS
- `showcase-unreadmark-{light,dark}.png`: UnreadMark entry. Switcher button `Rebuild index ● 3` beside the plain button; no glyph collision, dot and count aligned with the label, both schemes.
- `showcase-switcher-panel-{light,dark}.png`: MenuOverlay "project switcher" panel. Row 1 `2 running` over `● 1 unread` (stacked, right-aligned), row 2 `● 2 unread`, row 3 unavailable with its warning icon. Finding (cosmetic): `session-daemon-notes` wraps onto two lines at the panel's width in both schemes (the count column keeps its width; the name does not truncate).

## B. Client (dark scheme only): PASS
Setup: projP with sessions A, B (A selected, window focused), projQ with C.
- B5 `B5-button-dark.png`, `B5-panel-tooltip-dark.png`: C stops -> button `projP ● 1`; panel projQ `1 running` / `● 1 unread`. Tooltip text `1 unread session in other projects` shown.
- B6 `B6-rows-dark.png`, `B6-panel-dark.png`: B stops -> mark at B's trailing edge between `claude` and the close X; activity indicator (coral, awaiting input) unchanged; label heavier than A's but subtle. Panel projP `2 running` / `● 1 unread`; projQ unchanged; button still `● 1`.
- B7 `B7-before-after-dark.png` (top before, bottom after), `B7-panel-dark.png`: selecting B (window focused) clears the mark within the 1 s capture; panel projP back to `2 running` only; button still `● 1` (C).

## Tooltip vs open panel
The tooltip showed while the panel was open (hover persisted after the click that opened it). It sits on the panel's top padding, overlapping the panel's top edge by about half its height; it covers no row text (nearest row, projP, is clear of it). Whether it should be suppressed while the panel is open is a judgement call: it overlaps the panel but hides nothing. See `B5-panel-tooltip-dark.png`.

## Observations
- Without input focus (no window manager; `xdotool windowfocus` not yet called) selecting B did NOT clear B's mark, as the "in view" rule needs focus. After `windowfocus` the mark cleared. Not a defect; the earlier first B7 attempt is explained by this.
- The row label's unread emphasis is hard to see at 3x (same as M4's note).

## Not covered
Light scheme for B5-B7 (client has no scheme toggle reached; M9 should), the notification itself (no service), the tooltip in the light scheme, mid-flight animation, perceived smoothness.

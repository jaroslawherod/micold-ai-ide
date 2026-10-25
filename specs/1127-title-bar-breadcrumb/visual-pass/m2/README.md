# Visual pass, milestone M2 (quickstart §B 1–3, 6 — M2 parts)

Run 2026-10-25 on Xvfb (tiny-skia), the pinned M2 build (b5961ecd), 1280 × 800 and 640 × 480, a
fixture project `micold-ai-ide` with worktrees on `fix/1117-x` and on a 60-character branch, and
three unread sessions in `other-project`.

| Check | Result | Evidence |
|---|---|---|
| §B1 Session on `fix/1117-x`: `micold-ai-ide ▾ ● 3 / fix/1117`, project in `on_surface`, `/` and place in the quieter `on_surface_variant`, then Overview | pass | `b1-bar-session.png` |
| §B2 Press the project segment: panel top at y 44 (divider row 43), leading edge at x 48, the segment's own; segment drawn selected | pass (pixel probe) | `b2-switcher-panel.png` |
| §B2 Keyboard open | not driven: under Xvfb the terminal holds the keyboard and Tab traversal from the Overview did not land reliably; covered by `switcher_anchor::opened_from_the_keyboard_the_panel_hangs_from_the_segments_edge`, the `title-bar-switcher-open` covered state (`panel_placement`) and M1's `title_bar_keyboard` | — |
| §B3 Overview (full screen) → `Overview`; Review of `fix-1117-x` → `fix/1117`; Settings → `Settings`, Overview inert, gear selected; bar height unchanged | pass | `b3-overview-changes-settings.png` |
| §B3 No project → `Select project` alone | not driven (no in-app way to close a project); covered by the `title-bar-no-project` state and `toolbar::tests` | — |
| §B6 640 × 480, 60-character branch: the place shortens with `…` first, the project keeps its name, every control stays in the bar | pass | `b6-640-place-shortens.png` |
| §B6 Tooltips: the shortened place shows the full branch; the project segment `Switch project` with the unread words under it | pass | `b6-tooltips.png` |

# BUG-1298 visual pass (T058, SC-006)

Run 2026-10-24 on the fix branch (after T057), the way *Reproduction* in `../BUG-1298.md` describes,
at two card widths:

- **Design**: `design.html` in Playwright Chromium, with `.grid.five` forced to five columns of
  `<width>` px so every `.card` is that wide, `data-theme` light then dark.
- **App**: `render_probe.rs` appended to `overview_card.rs`'s tests (not committed there),
  `PROBE_WIDTH=<width>`, headless tiny-skia, light and dark roles. The probe's Jira card now passes
  the label core makes (`MIC-87`, FR-024), not the old `Issue MIC-87`.
- `side.py` puts each design card (top) above the app's card (bottom).

Images: `fixed-vs-design-{light,dark}-{a,b}-{235,280}.png`. `a` is the "Proposed" row, `b` is
"The other states" plus the pinned full-text card. Sizes come from the probe's painted-text dumps.

| # | Verdict | Evidence (both themes, both widths) |
|---|---|---|
| D1 | matched | 1 px outline in the outline colour round every card |
| D2 | matched | body on the theme's `surface`; the state colour only in the header (the light surface's slight tint is the theme role, D21) |
| D3 | matched | state word 13 px weight 600, type word 13 px weight 500 at 85 %, raised where 85 % misses 4.5:1 (ledger D7); no tracking (D25) |
| D4 | matched | header bands the same height as the mockup's; header and body text start at x = 15 (1 px outline + 14 px inset) |
| D5 | matched | type word only, no type icon |
| D6 | matched | body inset 12/14/14 px, 8 px between blocks |
| D7 | matched | title 17 px weight 600, 22 px lines |
| D8 | matched | one wrapping meta line; "running" drops to the next line where the mockup's does |
| D9 | matched | the link leads the line, 13 px weight 600, underlined, link colour |
| D10 | matched | `MIC-87` on the Done card |
| D11 | matched | branch in monospace 13 px, body text colour |
| D12 | matched | `1 session running`, `1 session`, `2 sessions, 1 running`, `no sessions`, after "·" at 14 px muted |
| D13 | matched | "NOW" 12 px weight 600 muted, then the activity in the body colour; continuation lines start at the left edge |
| D14 | matched | 6 px rounded bar first, then `Implement · 4/10 tasks`, `PR phase · 21/25 tasks` at 13 px muted |
| D15 | matched | no counts line under the rows |
| D16 | matched | `#1118` etc. monospace 13 px, underlined, link colour |
| D17 | matched | Open on `primary_container`, Merged on `success_container`, Draft on `surface_variant`, 12 px weight 600. The shared `Tag` keeps its 4 px side inset where the mockup's pill has 8 px, so the app's pills are a little narrower; FR-026 does not fix the inset |
| D18 | matched | merge conflict / checks failing in error weight 600, pending in warning, passed in success, "no checks" muted |
| D19 | matched | `+1 more, hover to see`, 13 px muted |

Differences that remain are the recorded exceptions: D20 (the mockup's "checks running", the
second phrase "changes requested" and the ages "2 days ago" / "yesterday"), D21 (header colours:
the theme's container roles, stronger in dark Needs action and light Needs attention and Done),
D23 (the pinned card's full text: the app shows display name, full branch and directory, not the
labelled Branch/Worktree block), D24 (`main` in monospace) and D25 (no letter spacing). Line
breaks differ in places because the mockup's `system-ui` face is not Roboto. The mockup's second
card title ends in "on…" because its HTML says so, not by clamping.

Result: D1–D19 all matched.

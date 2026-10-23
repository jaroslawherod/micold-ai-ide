# Overview card redesign: proposal

Status: proposal. Goal: a card readable at a glance. Builds on 813, 1024 (five per row), 970.

## What the screenshot shows

At five per row (about 380 px wide, 172 px tall) a card has 9 items in 7 text rows, all in one
small size except the name:

1. **Hierarchy is flat.** Name, branch, summary, counts, activity, progress and PRs share one body
   size and weight. Nothing says what to read first.
2. **The name is the least readable thing.** It is the largest text yet ellipsized (*Ci red on main dae…*),
   while the small branch line under it repeats the same words in full.
3. **Branch, summary and name say the same thing** for issue worktrees: `fix/1117_ci-red-on-main-daemon…`,
   *CI red on main: daemon tests…*, *Ci red on main dae…*. Three rows, one fact.
4. **Activity line is noisy.** It shows `/speckit-autopilot #1117` or a raw
   `<agent-message from="ab56e2…">` where a human sentence belongs.
5. **Low contrast.** Small light-grey text on the Working purple and on the dark Idle fill.
6. **Counts are low value.** *1 sessions, 1 running* (also ungrammatical), *0 open · 0 merged* and
   *No task progress* take three rows to say "nothing".
7. **State is a small chip top right**, though state is the reason the Overview exists.

## Proposal

Card = four tiers, each one size/weight, empty tiers removed (so the card is shorter when it has less).

```
┌─────────────────────────────────────┐
│ ▌ !  Needs action              Bug  │  tier 1: state word + icon (label-large, state colour), type right
│ ▌  CI red on main: daemon tests    │  tier 2: title (title-medium, up to 2 lines, no ellipsis on 1st)
│ ▌  fix/1117 · 1 session running    │  tier 3: id/branch short, counts (body-medium, on-surface-variant)
│ ▌  ▸ Writing tests · 2/5           │  tier 4: activity (only if reported) / progress bar + PR chip
│ ▌  ███████░░░  Implement · 6/10    │
└─────────────────────────────────────┘
```

Changes, in order of impact:

1. **Title = issue title, else branch, else display name.** Drop the separate name row; the worktree
   dir name moves to the tooltip. The title may wrap to two lines.
2. **Branch shortened** to `type/number` (full branch in tooltip); merged with the session count:
   *1 session · 1 running*, omitted at 0 running when 1 session: *1 session*. Fix pluralisation.
3. **Activity sanitised in core:** strip `<agent-message …>` wrappers and a leading slash command
   to its argument-free name; show nothing rather than markup.
4. **Hide empty rows:** no *No task progress*, no *0 open · 0 merged*. PR shown as a chip only when
   ≥1 open/merged or when unreadable (dash + tooltip, as today).
5. **Type sizes:** at least 14 px body, 16 px title, 12 px floor for labels (today's branch/summary
   rows are below this).
6. **Contrast:** use the `on-*-container` roles for text on a state fill and tone the Working fill
   down (a left 4 px state-colour stripe plus a neutral `surface_container` body would also pass
   contrast on Idle, and reads as state without painting the whole card purple). Verify ≥ 4.5:1 with
   the existing palette tests.
7. **Fixed height becomes minimum height.** Cards keep one height per row (the grid already aligns
   rows) but the row takes the tallest card, so wrapping never clips.
8. **No truncation of what identifies a card.** Today the name, branch, summary and activity all
   end in `…` and the full text is only in a tooltip, which is invisible at a glance and on
   keyboard. Title wraps to up to 3 lines; the activity line wraps to 2. Only after that does it
   ellipsize, and the card's menu gets a **Details** entry (or hover/focus expands the card in
   place) showing the full title, full branch, dir name and full activity text. The branch is
   shortened by dropping the slug (`fix/1117`), so nothing long is left to cut.

### Option B (not recommended): group by state

Section headers (*Needs action 2*, *Working 3*, *Idle 5*) with a collapsed *Idle* and *Done*.
Strong for scale, but the user chose density/readability as the goal; keep as a follow-up.

## Scope and files

- `crates/micold-core/src/overview.rs`: title choice, short branch, counts text, activity sanitiser
  (pure, unit-tested, no GUI).
- `crates/micold-client/src/ui/material/overview_card.rs`: layout tiers, `HEIGHT` → min height,
  palette text roles; geometry tests there already assert no overlap at 280 px.
- `docs/user-guide/overview.md`, layout snapshot, visual pass at 280 / 380 / 480 px.
- Unchanged: ordering, keyboard, menus, ripple, five-per-row grid.

## Open questions

1. Whole-card state fill (today) or stripe on a neutral card (proposed)?
2. Keep the project-root card's "Project root" text, or show the project name as its title?
3. Is wrapping to two title lines acceptable, or should cards stay single-line?

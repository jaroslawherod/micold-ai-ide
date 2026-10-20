# Quickstart: Overview cards, readable layout and content (#1120)

## Part A: automated

| Command | Proves |
|---|---|
| `mise run test-core` | contracts C1–C10: title, short branch, branch line, sessions line, progress row, pull request row, sanitiser, slash sentence, hook prompt (FR-006–FR-013, FR-018) |
| `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card card_grid composition_contrast type_role_mapping` (several filters go after `--`) | K1–K4: header and body, 4.5:1 in 10 state-theme pairs, 17/14/12 px, no overlap at 280/380/480, equal row height and the 172 px minimum (FR-001–FR-005, FR-014–FR-016) |
| `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot` | the `overview-open` covered state matches the regenerated fixture; text-overflow and containment gates pass on it (FR-015, FR-019) |
| `scripts/build-lock.sh cargo test -p micold-client --test showcase_completeness` | the card is still shown in the showcase (FR-017) |
| `! grep -nE 'tracing::\|log::\|println!\|eprintln!\|dbg!' crates/micold-core/src/{markup,overview_text,hook_activity}.rs` | no prompt or label is logged (FR-020) |
| `scripts/gate.sh` | the whole merge gate |

Regenerate the fixture only once the card is done:
`UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`,
then read the fixture diff: only the Overview state may be new.

## Part B: visual pass (the `visual-pass` skill)

Prerequisite: `mise run showcase` (or the showcase binary) on a private display, light then dark.

| # | Do | Expect |
|---|---|---|
| B1 | Open the showcase's Overview card entry | five cards, each with a header band in its state colour (red-ish, amber, primary, green, grey), state icon and word left, type icon and word right; bodies one neutral colour (US1.1, SC-005) |
| B2 | Switch to dark | the same, all text legible (US1.2) |
| B3 | Compare the Idle card with the others | grey header, visibly quieter; text still crisp (US1.3) |
| B4 | Hover a state word, then a type word | reason tooltip; type word tooltip (US1.4) |
| B5 | Right-click a card; Tab to a card; click a card | outline, focus ring and ripple cover the header too (US1.5) |
| B6 | Read the sample cards | title is a sentence, branch `feat/813`, `3 sessions · 2 running`, activity `Continue M2` with no `<`, no *No task progress*, no `0 open · 0 merged` (US2, US3, SC-003) |
| B7 | Resize the showcase window so cards are about 280, 380 and 480 px wide | nothing overlaps; long lines end in `…`; cards in a row share one height (US4, SC-002) |
| B8 | Run the client on a project with worktrees, open the Overview | the same layout with real data; an issue title appears as the title once read |

Record screenshots under `specs/1120-overview-card-readability/visual-pass/` and the results in
`visual-pass.md`.

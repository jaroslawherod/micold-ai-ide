# Implementation Plan: Overview cards, a pull request list with state, linked

**Branch**: `claude/project-thread-7041o6` | **Date**: 2026-10-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1123-overview-card-pr-list/spec.md`

## Summary

Feature 040's status reading already asks GitHub for every field a row needs and drops all but the
current pull request. It now keeps every own pull request of the branch as a `BranchPrList`
(render-free core `micold_core::pr_list`), and the counts come from that list: the two 100-node
count pages give way to a newest-10 merged page of six scalar fields, so each reading costs no more
GitHub rate-limit points than today's (43 against 44 requests per branch) with the same requests
(R1, R2). The list replaces the counts map end to end, so rows
keep and clear exactly when counts do today (R4). A card draws up to two rows collapsed (all when
expanded) and "+N more"; each number is a feature 1122 `TextLink`, so every link rule holds per
number. Card links are named by a `CardLink` slot in the expansion core (R6). Opening goes through
the issue link's opener path (R8). Design decisions: [research.md](research.md) R1–R10.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14; existing `serde_json`; feature 1122's `TextLink`, feature
031's `LinkOpener`. No new crate.

**Storage**: N/A. Everything in memory (feature 040 FR-032).

**Testing**: core: `pr_list` K1–K4, K6, `overview_expansion` K5a–K5g, parse/query Q1–Q2, P1–P7 over
recorded answers (`crates/micold-core/tests/pull_request_*.rs`); client in-crate: `OverviewCard`
U1–U3, `CardGrid` U4, hand-off and menu return U5–U6 and opener U7 (`main_tests.rs`), reducer tests for `State.lists`; `composition_contrast.rs`;
`tests/layout_snapshot.rs` with covered state `overview-pr-rows`. Visual pass:
[quickstart.md](quickstart.md) §B. Whole gate: `scripts/gate.sh`.

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: rows change with the reading that changes them (spec Story 1 sc 7): the
existing reading interval; the card is derived per render from ≤ 30 entries per branch.

**Constraints**: no new request, timer or request kind; GitHub points per reading not above today's
(R2: 44 → 43 connection requests per branch, `m<i>` without `commits`); same `ReadingFailure` keep/clear rules (FR-007); no address in a log or
`Debug` (FR-014); no card activation from a number (SC-003); root `Message` vocabulary unchanged
(`root_vocabulary_is_cross_cutting`); no new `motion::wake_at` caller; the daemon and wire are
untouched (the reading runs in the client).

**Scale/Scope**: core: 1 new module (`pr_list.rs`), `pull_request.rs` (query, parse,
`StatusReading.lists`), `overview.rs` (`CardModel.pull_requests`, `PrCounts` doc),
`overview_expansion.rs` (`CardLink`, last-gain focus model); client: `OverviewCard` (rows, per-number links, one link
helper shared with the issue link), `CardGrid` (link ids per card), `ui/overview.rs`
(`card_link_id`, rows, `pr_display`), `features/pr_status.rs`, `shell/pr_status.rs`,
`shell/daemon_sync.rs`, `features/overview.rs` (`PullRequestLinkActivated`, `FocusReturn`, `MenuOrigin`), `main.rs` (hand-offs and menu return per `CardLink`), `ui/material/pull_request_indicator.rs` (`PrMark::word`), showcase,
1 covered state; docs: `docs/user-guide/overview.md`.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Order, phrase, rows, counts and link focus are red-first core tests (K1–K6, Q1–Q2, P1–P7); widgets and hand-offs red-first (U1–U7). Only the opener's OS call is glue, checked by quickstart §B. |
| II. Multi-Session | PASS | Rows are per branch, whatever sessions the location has; opening touches no session; each window keeps its own reading as today. |
| III. Worktree Integration | PASS | Branch names as today; no VCS read or write added. |
| IV. Local-First | PASS | Same GitHub reading through the user's `gh`, fewer points; addresses in memory only; opening is an explicit user action through the OS. |
| V. Rust + iced | PASS | `BranchPrList` sorts on construction; `StatusPhrase::of` returns `None` for merged/closed, so a row with words for a merged pull request cannot be built; `CardLink` names links by number, not position. |
| VI. Cross-Platform | PASS | Opener per platform (feature 031); core has no OS branch; logical px; CI on three platforms. |
| VII. Documentation | PASS | `docs/user-guide/overview.md` (Pull requests, Keyboard) and covered state `overview-pr-rows` in the same PR (FR-017). |
| VIII. Components | PASS | `TextLink`, `Tag` (pill, tinted by `PrMark::role`), `Ellipsized` reused; `StatusChip`, `PullRequestRow` considered and rejected (R9); `OverviewCard` and `CardGrid` extended by builders; colours from roles, contrast tested; showcase card with rows (FR-015). |

Post-design re-check: unchanged, all PASS. No Complexity Tracking entries.

## Project Structure

### Documentation (this feature)

```text
specs/1123-overview-card-pr-list/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md
├── contracts/reading.md     # query pages and parse
├── contracts/card-rows.md   # core rules K1–K6, widgets and hand-offs U1–U7, wiring
└── checklists/requirements.md
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── pr_list.rs               # new: ListedPr, BranchPrList, StatusPhrase, PrRow
├── pull_request.rs          # m<i> page, oc/mc gone, parse → lists, shared node mapping
├── overview.rs              # CardModel.pull_requests
└── overview_expansion.rs    # CardLink
crates/micold-core/tests/pull_request_{query,parse,counts,select,is_never_stored}.rs
crates/micold-client/src/
├── ui/material/overview_card.rs, card_grid.rs
├── ui/overview.rs
├── features/pr_status.rs, features/overview.rs
├── shell/pr_status.rs, shell/daemon_sync.rs
├── main.rs
└── showcase/sections/surfaces.rs
crates/micold-client/tests/support/covered_states.rs
docs/user-guide/overview.md
```

**Structure Decision**: rules in `micold-core` (Principle I), widgets and wiring in
`micold-client`; no daemon or protocol change.

## Complexity Tracking

None.

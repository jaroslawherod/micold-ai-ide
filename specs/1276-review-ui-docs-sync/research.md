# Research: Review UI — docs, layout snapshot, visual pass and mockup sync

No item in the plan's Technical Context was left NEEDS CLARIFICATION; the spec's three
clarifications (FR-004 is the core-logic clause, 1280×800 only, follow-ups labelled) are taken as
given. Each decision below records what was chosen and what was rejected.

## R1 — Where the guide-equals-bindings test gets the keys

- **Decision**: The test reads the review key table in `micold-core` that #1274 (R5) delivers
  (its acceptance: "pure logic in micold-core with unit tests") and the client binds; the plan calls
  it `micold_core::review::keys::REVIEW_KEYS` until #1274 merges. If #1274 merges with its keys
  only in the client, this feature moves the table to `micold-core` unchanged and has the client
  bind from it.
- **Rationale**: One table that both the key handler and the `?` sheet read is the only source the
  guide can be held to; it is the pattern `structured::commands::TERMINAL_ONLY` already follows for
  `terminal_only_doc.rs`. A table in `micold-core` is testable without iced (`mise run test-core`).
- **Alternatives rejected**: (a) A client test that scrapes the key handler's `match` arms — breaks
  on refactors and cannot see the sheet's wording. (b) A hand-kept list in the test — would pass
  after a key change that skipped both guide and test, the failure FR-004 exists to catch. (c) A
  shell script grepping the guide — the docs-only gate does not build Rust, so it cannot see the
  binding.

## R2 — Format of the guide's shortcut table

- **Decision**: Two columns, `Key` and `Does`; one row per table entry in the table's order; the key
  cell renders the Linux / Windows chord and adds ``(`Cmd+…` on macOS)`` when the table's modifier is
  the platform's primary one; the `Does` cell is the sheet's text for that entry. Contract:
  [contracts/guide-shortcut-table.md](contracts/guide-shortcut-table.md).
- **Rationale**: The guide must name each platform difference (FR-001); writing both chords in one
  cell keeps one row per binding and lets the test render both. The sheet text as the description
  means the guide, the sheet and the binding cannot disagree.
- **Alternatives rejected**: (a) One table per platform — triples the rows and hides that the
  bindings are the same. (b) Free-text descriptions not compared — a renamed action would leave the
  guide describing the old one.

## R3 — Sharing the marker-table helpers

- **Decision**: Move `marked_section` and `rows` from `terminal_only_doc.rs` into
  `crates/micold-core/tests/support/marked_table.rs` (parametrised by page, markers and cell count),
  used by both tests.
- **Rationale**: CLAUDE.md asks to extend a match rather than copy; `mise run duplication` budgets
  copies.
- **Alternatives rejected**: Copy the two functions into the new test — a ~30-line clone the
  duplication check would charge.

## R4 — Gating review states at the smallest window

- **Decision**: A new gate module `tests/gates/review_tab_fits.rs` lays out every `review-*` covered
  state at `MIN_WINDOW_SIZE` and 1280×800 in both themes and runs the existing
  `support::layout::{text_overflows, escapes}` checks plus two review-specific assertions (bar above
  the status bar; drawer inside the diff).
- **Rationale**: `layout_text_overflow.rs` and `gates/containment.rs` run only at the canonical
  1280×800 window (`cached_records` over `covered_states()`); FR-007 also asks for the smallest
  window. `session_area_fits.rs` already loops `[MIN_WINDOW_SIZE, WINDOW]` × themes for one state;
  the new gate follows that shape for the review family.
- **Alternatives rejected**: (a) Make the overflow and containment tests run every state at two
  windows — doubles the whole suite's layout cost for states that do not need it. (b) Record the
  smallest window in the fixture too — the fixture is defined at one size (`layout-snapshot.md`
  § *What is covered*); a second size changes its format for every feature.

## R5 — `session-area-changes-open`

- **Decision**: Redefine it: the Review tab selected in the strip, header above, review body below;
  anchors `session.header`, `session.header.changes`, `session.strip`, `review.tab`. Update
  `session_area_fits.rs::the_header_stands_over_the_open_changes_view` to assert the strip is
  present.
- **Rationale**: FR-006 forbids leaving it recording "header over the view, no strip", which R0
  makes impossible. Keeping the name keeps the history of the header-button gate (feature 1126 T053)
  readable; if R0 already redefined or renamed it, follow R0 and only update the doc.
- **Alternatives rejected**: Delete it and add `review-tab-shown` — loses the header-button
  width gate unless it is moved too; the coverage-narrowing check would flag the drop.

## R6 — When to re-record the fixture

- **Decision**: Once, in M2, after #1269–#1275 and #1128 are all on `origin/main`; never before
  and never for another issue's layout.
- **Rationale**: Spec FR-005 and Edge Cases; the orchestrator's standing rule. Each slice records its
  own states; R7's re-record is the reconciliation.
- **Alternatives rejected**: Re-record per merged slice — repeats the slices' own work and churns
  the fixture while siblings are still landing.

## R7 — Visual pass route

- **Decision**: The `visual-pass` skill (Xvfb + lavapipe, private data directory, xdotool) on the
  real client and daemon, seeded with an invented repository and review store; crops committed, not
  full frames.
- **Rationale**: It is the verified route in this repository (`.claude/skills/visual-pass/SKILL.md`)
  and needs no human display; the skill forbids `mise run screenshot` for this.
- **Alternatives rejected**: (a) The component showcase (`sections/review.rs`) — it shows components,
  not the tab in the strip with the bar above the status bar, which is what FR-009 checks.
  (b) `mise run screenshot` — the skill's §1 says why not.

## R8 — Mockup sync route

- **Decision**: Read the canvas with the Artifact tool; with *writer* access, edit and republish to
  the same URL; otherwise write the difference list for the owner. Either way record the comparison
  in `visual-pass/mockup-sync.md`. Runs after the visual pass so each board is compared with a crop.
- **Rationale**: The canvas is a claude.ai artifact owned by the user (spec Assumptions); a new URL
  would orphan the link in the issue and the redesign doc.
- **Alternatives rejected**: Publish a new canvas — breaks the link in #1269–#1276 and
  `review-redesign.md`. Export screenshots into the canvas — loses the shared components the
  redesign doc asks to keep in step.

## R9 — Inbound links

- **Decision**: Keep the heading text (and so the id) of every section whose behaviour did not
  change; re-point the four inbound links found today (`settings.md:660`,
  `worktrees-and-sessions.md:240`, `agent-tools.md:300`, `agent-tools.md:311`) where their target
  moves; `site/checks/links.sh --sources` is the check.
- **Alternatives rejected**: HTML anchors for old ids — mdBook generates ids from headings and the
  links check would not see a hand-made anchor in rendered HTML.

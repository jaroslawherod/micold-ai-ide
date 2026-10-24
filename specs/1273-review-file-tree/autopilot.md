# Autopilot ledger — #1273 review-file-tree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1273 (issue #1273 "Review UI: file tree, filter and sort"); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1273
- **Worktree branch**: claude/project-thread-p2tb7i
- **Started**: 2026-10-24
- **Phase**: implement (M1 PR)
- **Next step**: orchestrator opens the M1 PR (title/body in the unit's scratchpad), waits on CI and merges; then milestone M2 (core-only, need not wait for #1272). M3 waits for #1272. If the user's card on 4.3 picks "disabled with a reason", set `UNREVIEWED_UNTIL_MARKS` to `Offer::Disabled` per research R3 (in M2) and update spec 4.3 and FR-012

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1354 | design (spec, plan, tasks) | merged (rebase) | 9c8473afbb8e49aaa8853892a6df818df3fdd138 |
| — | M1 core tree layout | gated (scripts/gate.sh green at ddc64264), pushed; orchestrator opens it | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T006 | full | core `review::tree::layout` (grouping, chaining, both openness modes, Path order) + `TreeChoices` toggle/reveal; Files parity in core tests | — | implemented, gated; PR to open |
| M2 | T007–T018 | full | core totals, bar maths, indent cap, hover text, filter + spans, Size / Unreviewed-first sorts, sort offer, SC-003 timing | — | pending (mergeable before #1272) |
| M3 | T019–T023 | full | Changes reducer keeps folds per view; reveal on select; Files `tree_rows` on core | — | blocked: start after #1272 merges |
| M4 | T024–T030 | full | tree rows + `ChangeBar` drawn; user guide; snapshot; visual pass | — | blocked: after M3 (#1272) |
| M5 | T031–T039 | full | filter box (Esc, clear, no-match) + sort chips; user guide; snapshot states; visual pass | — | blocked: after M4 (#1272) |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify r1 | 4.2: do Size / Unreviewed first keep folders or flatten? | Keep the tree; order within each level | agent-resolved | docs/development/review-redesign.md#Layout item 2 |
| 2 | clarify r1 | 4.3: Unreviewed first before #1271's marks exist? | Hidden; core implements and unit-tests the ordering against a reviewed set | recommended, pending user's decision card | no `reviewed` state in crates; review-redesign.md#Tasks. If the user picks otherwise, update spec 4.3 and FR-012 |
| 3 | clarify r2 | (scan) | No critical ambiguities; clarify done | agent | spec.md |
| 4 | plan | Sort control: `Select` or `ToggleChip` row? | `ToggleChip` row; `Select` has no disabled options, so the 4.3 hide/disable switch stays one constant | agent | research.md R11, R3 |
| 5 | tasks | Plan's M3 (all reducer msgs) / M4 (all drawing) / M5 (docs, snapshot, visual) layering? | Split client work by story instead: M3 tree state, M4 tree+bar drawing with its guide and snapshot, M5 filter+sort end to end; plan's milestone table updated | agent | milestones.md rules 4 (guide in the feat PR), 6 (no half-wired UI); a drawing change re-records the snapshot in the same PR |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | cd4bfb8960277ca0f26816848ec501822343f1ec:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN (3 MINOR, fixed, prose only) |
| Plan | 1 | 27619aca6068cb5bfbd487391e1db782f959f496:f7905e9c249e711053db1639dcc416db7e1b114b | CHANGES (1 MAJOR, 3 MINOR; all fixed) |
| Plan | 2 | 2edca85acb541dbe1c6af23daa58aac7d52dde64:f7905e9c249e711053db1639dcc416db7e1b114b | CLEAN (sonnet; every fix cited) |
| Tasks | 1 | bb169f2093970d1919ca520ac5765b3094415eaa:b18235fb3e0c3b1e03cf6278542ddad08555ff7e | CHANGES (1 MAJOR: `[P]` on T034 sharing a file; fixed, prose only — task metadata, no behaviour claim; MINOR F2, F3 fixed; F4 declined) |
| Review A M1 (code-review high) | 1 | 5696a22e0cae23f54d87d5a8792580eec1767ed5:9c8473afbb8e49aaa8853892a6df818df3fdd138 | 5 MINOR (F1, F3, F4, F5 fixed in 88f3f377; F2 deferred) |
| Review B M1 (conformance, sonnet) | 1 | 5696a22e…:9c8473af | CLEAN uncited — not counted; re-run |
| Review B M1 (conformance, sonnet) | 1 | bce2c567f7b1bed1d8b308b7a0b8a6704cea53c6:88f3f377cea4adfd8323022041b81e37fc170188 | CLEAN, cited (2 MINOR: cycle-log count fixed; test-first commits noted) |

## Declined review findings

- tasks r1 F4 (checklist's FR-017 implementation-detail waiver): FR-017's "render-free logic in the shared core, covered by unit tests" is the issue's stated acceptance (spec Input), kept deliberately since the specify unit; not a self-granted waiver.
- analyze P2 (drop `Offer::Disabled`): kept on purpose; it is the one-constant switch for the pending decision card on 4.3 (Decision 2, research R3).
- analyze C4 (T040 in no milestone): T040 changes no code; milestones.md rule 4 leaves it to the close unit, as tasks.md says.
- analyze S1 (M2 has 12 tasks): kept whole; one file's rules, ~600 lines, a split by story edits the same `layout` (noted in M2).
- analyze T1 (part): "scrolling stays smooth" stays a Part B check (SC-003 wording is the spec's); the timing threshold is release 100 ms / debug 500 ms (T018).

Analyze fixes (spec, plan, data-model, research, tasks): I1 sibling order by name, chained folder by its first folder (spec 4.1, data-model §3, R3, T002); U1 totals over listed files (FR-007); U2 bar scale over the whole list (Assumptions, T022, T033); C1 `list_offset` kept on re-read (T020); C2 reveal via `OpenFileDiff` and `ReferenceActivated` (data-model §6, T020, T022); C3 choices made while reading/failed, no-match keeps diff (T020, T033); P1 render-free `tree_pane` (data-model §6, T020, T022, T027, T036); T2 `[P]` only on disjoint files; T3 cache renamed `change_rows`; T4 24-char boundary test at #1272's width (T019, T027); T5 fuzzy matching stated (Assumptions).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F2 (`walk` clones and sorts a level's files) | `(&str, usize)` pairs, small; M2 replaces the ordering with Size / Unreviewed-first sorts, so the per-level sort is reworked there |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- #1272 (source dropdown, two-column layout) is not merged; the spec defers left-column chrome to it.

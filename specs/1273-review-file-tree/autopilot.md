# Autopilot ledger — #1273 review-file-tree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1273 (issue #1273 "Review UI: file tree, filter and sort"); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1273
- **Worktree branch**: claude/project-thread-p2tb7i
- **Started**: 2026-10-24
- **Phase**: implement (M5)
- **Next step**: M5 T036 (draw filter box and sort chips in `ui/changes.rs`); see Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1354 | design (spec, plan, tasks) | merged (rebase) | 9c8473afbb8e49aaa8853892a6df818df3fdd138 |
| #1367 | M1 core tree layout | merged (rebase) | 78c2fc228b7d3e1c88160385cea8ef5a6d402bd4 |
| #1375 | M2 totals, bars, filter, sort | merged (rebase) | c592cb89 |
| #1426 | M3 tree state in the Changes reducer; Files on core | merged (rebase) | d84e94860 |
| #1430 | M4 tree rows + ChangeBar drawn; guide; snapshot; visual pass | open (CI) | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T006 | full | core `review::tree::layout` (grouping, chaining, both openness modes, Path order) + `TreeChoices` toggle/reveal; Files parity in core tests | #1367 | merged |
| M2 | T007–T018 | full | core totals, bar maths, indent cap, hover text, filter + spans, Size / Unreviewed-first sorts, sort offer, SC-003 timing | #1375 | merged |
| M3 | T019–T023 | full | Changes reducer keeps folds per view; reveal on select; Files `tree_rows` on core | #1426 | merged |
| M4 | T024–T030 | full | tree rows + `ChangeBar` drawn; user guide; snapshot; visual pass | #1430 | open |
| M5 | T031–T039 | full | filter box (Esc, clear, no-match) + sort chips; user guide; snapshot states; visual pass | — | T031–T035 done (local, unpushed) |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify r1 | 4.2: do Size / Unreviewed first keep folders or flatten? | Keep the tree; order within each level | agent-resolved | docs/development/review-redesign.md#Layout item 2 |
| 2 | clarify r1 | 4.3: Unreviewed first before #1271's marks exist? | Hidden; core implements and unit-tests the ordering against a reviewed set | recommended, pending user's decision card | no `reviewed` state in crates; review-redesign.md#Tasks. If the user picks otherwise, update spec 4.3 and FR-012 |
| 3 | clarify r2 | (scan) | No critical ambiguities; clarify done | agent | spec.md |
| 4 | plan | Sort control: `Select` or `ToggleChip` row? | `ToggleChip` row; `Select` has no disabled options, so the 4.3 hide/disable switch stays one constant | agent | research.md R11, R3 |
| 5 | tasks | Plan's M3 (all reducer msgs) / M4 (all drawing) / M5 (docs, snapshot, visual) layering? | Split client work by story instead: M3 tree state, M4 tree+bar drawing with its guide and snapshot, M5 filter+sort end to end; plan's milestone table updated | agent | milestones.md rules 4 (guide in the feat PR), 6 (no half-wired UI); a drawing change re-records the snapshot in the same PR |
| 6 | M3 | #1271 merged (marks in `micold_core::review::marks`): does Unreviewed first stay hidden? | No: M5 offers `sort_options(marks)` with `marks` = marks known (connected), reviewed predicate from the marks `file_reviews` judges; `UNREVIEWED_UNTIL_MARKS` stays the offer while marks are unknown. M3 has no sort message, so its layout keeps an always-false predicate (T022). T034, T036 updated | agent (orchestrator's brief) | #1271 closed; `features::changes::file_reviews` takes `connected` |
| 7 | M3 (T019) | Did #1272 move the list pane or rename its types? | List pane = the 260 px file column left of the diff (`ui::changes::FILE_COLUMN_WIDTH` 260.0); row height `ui::changes::ROW_HEIGHT` 36.0; `TreeView` indent step `spacing::MD`. `OpenView`, `ListBody`, `Msg::FileSelected`/`SourceChosen`/`ReferenceActivated`/`OpenFileDiff` unchanged; plan.md and contract §2 ("inside the left column #1272 provides") need no change | agent | `crates/micold-client/src/ui/changes.rs:57,69` |
| 8 | M3 (T020) | M2 A F1: should a select while filtering reveal only in the active set? | No: `reveal` clears the ancestors from both sets. A file selected by any route must stay listed once the filter clears (scenario 3.4 "selected again when the filter is cleared", edge case "Selection inside a collapsed folder") | agent | spec.md scenario 3.4, data-model §2 |
| 9 | M3 (T022) | `ReferenceActivated` browses rather than selects; reveal there? | Yes, when the reference names a file the shown source lists (edge case names "a reference followed from a diff") | agent | `reference_activated` in features/changes.rs |
| 11 | M4 (T027) | A 24-character name, the kind tag, the bar and `+a −r` do not fit one line of the 260 px column (name ≈ 170 px in the emphasised role; trailing cluster ≥ 100 px) | Two-line tree rows, `ROW_HEIGHT` 36 → 48: a file's name (review glyph first) over its tags, bar, counts and comment tag; a folder's label over its totals. Name keeps the row's width less padding and the glyph slot (SC-006 test). The visual pass showed the list's rows are 236 px (the column's 24 px inset), not 260: the cap now uses `FILE_LIST_WIDTH`, the tree indents 12 px a level (not `TreeView`'s 16, decision 7) and rows lead with 4 px, so a 24-character name (143 px) keeps its room four levels deep | agent | spec SC-006, edge case "Long file names"; `ui::changes::tests`; visual pass light s4 |
| 12 | M4 (T025) | Bar colours (R7 left the family open) | added `success`, removed `error`: the scheme's green and red roles, both themes; `diff_added` / `diff_removed` are pale row fills | agent | research R7; tokens `success`, `error` |
| 10 | M3 (T022) | Where does `refresh_change_rows` run? | Once in `update` beside `reshape`, after every non-scroll message on the shown view, skipped when a digest of the leaves and choices is unchanged: no arm can miss it, and typing elsewhere lays nothing out | agent | research R9 (never in `view`) |
| 13 | M5 (T034) | The reducer cannot see the connection: how does it refuse Unreviewed first and get the reviewed set? | Reducer: `sort_options(review_of(state, view).is_some())` (marks pushed for the entry); layout's reviewed set = listed files whose mark (entry's last push, shown source) equals the current id (`file_review(.., 0) == Reviewed`), hashed into `change_key` so a mark change re-lays out. UI: `changes::sort_offers(state, view, connected)` = `sort_options(connected && pushed)`, the chosen sort always Shown so the active chip never vanishes while disconnected | agent | `main.rs` `connection_status` is shell-only; `ui::changes::view` takes `connected` |

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
| Review A M2 (code-review high) | 1 | da02017a64ead8f45b762267d2848db413cea704:2bad8b6cb7f52c5bd212d0a49655fe71af1b88d3 (base 6d0475a5) | 10 MINOR (F2, F3, F4, F5, F6, F7, F8, F9 fixed; F1, F10 declined) |
| Review B M2 (conformance, sonnet) | 1 | da02017a…:2bad8b6c (base 6d0475a5) | CLEAN, cited (tree.rs, tasks.md:70-91, cycle-log.md:15-19) |
| Review A M3 (code-review high) | 1 | f22196b4e09cd9e1d7f351b7b611662b7d134cb0:0db79c00204b5edec9f374c8343473ee7fed851b (base bb931624) | 5 MINOR (F1 fixed in 7b228493: reference to an unchanged file tested; F2–F5 declined) |
| Review B M3 (conformance, sonnet) | 1 | f22196b4…:0db79c00 | CLEAN, cited (changes.rs arms, tree_state / tree_source tests, tasks T019–T023); 2 MINOR (F2 cycle-log note fixed, prose only; F1 declined) |
| Review B M4 (conformance, sonnet) | 1 | 8f8c6c26a30918e69bb32051d5b1305ee780612b:23fe1317d03e3500752bc65f76f3d734d8b4c124 (base 5c9daf4c) | CHANGES, cited: F1 MAJOR (no red for T024/T027) fixed by mutation red, cycle-log M4; F2 MINOR fixed (`kind_tag`, `counted` tests); F3, F4 MINOR fixed (prose) |
| Review B M4 (conformance, sonnet) | 2 (full: rebase made round 1's snapshot stale) | abd9218d5ce05c39cb42b65bfc539592eb54cd6c:16aa4cb36024ee74d40f3fca9f718029b2e28c0b | CLEAN, cited (changes.rs, change_bar.rs, glyph.rs, guide, cycle-log M4); all four round-1 fixes hold |
| Review A M4 (code-review high) | 1 | 91fc5a5c20ea7c55ab33ba07c7f36ef62ae056d1:bbfebda252276fda85ce54f86693fd8fc106ecfb (base 5c9daf4c) | CLEAN, cited (changes.rs, change_bar.rs, glyph.rs, tree.rs); 2 MINOR (F1 fixed: indent cap shaped once; F2 declined). A first dispatch read a stale M3 prompt and reviewed M3's diff: not counted; its one M4 point (`files[*leaf]` unguarded) fixed in bbfebda2 |

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
| M2 | A | F1 (`reveal` while filtering clears `collapsed`, so clearing the filter does not restore a fold above the selected file) | follows data-model §2 and R2 as written ("removes every ancestor folder of `path` from both sets"); whether a select while filtering should keep the pre-filter fold is M3's reducer wiring (T020, scenario 3.4) — M3 to decide and, if needed, reveal only in the active set |
| M2 | A | F10 (wall-clock thresholds; untested Unreviewed-first + filter + chain) | the timed test is SC-003's chosen measure (R9), slowest of runs, 5x headroom in debug; sort keys of a chained row are its outer node's, equal to the inner's by construction, and L11 covers sort + filter. Fixed the part that held: the Size step now times the full unfiltered list |
| M3 | A | F2 (`FileSelected` reveal while filtering clears `collapsed`) | decision 8: a file selected by any route stays listed once the filter clears (scenario 3.4, edge case "Selection inside a collapsed folder") |
| M3 | A | F3 (`refresh_change_rows` builds leaves and a digest on every non-scroll message) | decision 10 / R9: the digest is linear and skips the layout; a generation counter needs every `list`/`diff`/`tree` writer to bump it, the miss decision 10 avoids; SC-003 timing covers the large list |
| M3 | A | F4 (non-shown views keep stale rows) | only the shown view is drawn or read by `tree_pane`; a tab switch is a message through `update`, which refreshes the newly shown view |
| M3 | A | F5 (`tree_rows` and `change_leaves` map leaves separately; Files lays out per draw) | different count rules by design (Files has none); Files' per-draw layout predates M3, its cost unchanged |
| M3 | B | F1 (T034/T036 edited in M3) | decision 6 records why; prose in tasks only |
| M4 | A | F2 (the SC-006 test restates the cap formula) | the depth-0 assertion fails when the column cannot hold a 24-character name at all, and `cap >= 2` fails when the tree could not indent; the drawn row is the visual pass's check (T030, Part B step 6) |

## Handover

M5 unit 1 (context cap). Done, committed locally on `claude/project-thread-p2tb7i` (NOT pushed: #1430 open from this branch):
T031–T032 `TextField::on_escape` (micold-material `text_field.rs`, wrapper `EscapeWhileFocused`; 2 harness tests green);
T033–T035 `Msg::TreeFilterChanged/TreeFilterCleared/TreeSortChosen`, `changes::sort_offers`, `reviewed_paths` feeding `refresh_change_rows` (decision 13);
tests in `tests/features_changes.rs` (tree_state: 6 new; reviewed_marks: `unreviewed_first_follows_the_marks`; source_menu: `a_source_switch_keeps_the_filter_and_the_sort`), 34 passed.
Next: T036 draw (filter TextField with `.on_escape(Msg::TreeFilterCleared)`, leading search icon, trailing clear; `ToggleChip` per `sort_offers(.., connected)`, Disabled in Tooltip, Hidden skipped; spans highlighted in labels — check whether M4 rows already draw `row.spans`; NoMatch line `No changed file matches the filter` + **Clear filter** text button; only when `tree_pane != Hidden`), T037 snapshot states, T038 guide, T039 visual pass; then review A/B, gate, and the rebase/push ordering in the orchestrator's M5 brief (push only after #1430 merges).
Cycle log: M5 rows added for T031–T035.

## Open escalation

None.

## Follow-ups not done

- None.

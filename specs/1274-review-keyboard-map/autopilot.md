# Autopilot ledger — #1274 review-keyboard-map

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1274 "Review UI: keyboard map for reviewing": Blocked by: #1273, #1271, #1127. j/k file, n/p hunk, ]/[ comment, v reviewed, g next unreviewed, c comment, Ctrl+Shift+Enter send, ? shortcut sheet; the jump-to field finds files of the open review and comments. Part of the review UI redesign: `docs/development/review-redesign.md`. Mockup: https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu (Review tab redesign). Acceptance also: pure logic in micold-core with unit tests; docs, layout snapshot and visual pass (1280x800, both themes) updated. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1274
- **Worktree branch**: claude/project-thread-m7gq7k
- **Started**: 2026-10-24
- **Phase**: milestones
- **Next step**: M5 (T052–T057) in progress on cc2e4548a.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1366 | design (spec, plan, tasks) | merged | 76c1335e705aec8ebf02d278273bcc578d91112f |
| #1404 | M1 core key map | merged | 8f38e9ca85309f92e24376c870038cd481eff913 |
| #1415 | M2 single-key switch core (+ D4 design amendment) | merged | 83f693ff7c3b14e115a2ae17dc67fa5085546242 |
| #1451 | M3 move and mark (j k n p v g, Settings switch) | merged | 6cbd52f0ed4581036bec4dd0f6527022dcd4a046 |
| #1458 | M4 comments, send chord, shortcut sheet | merged | cc2e4548a0f9b835009b6e250e39416ba2222b8c |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011, T022–T023, T028–T032, T048–T051 | full | core REVIEW_KEYS + resolve, navigate rules, header_rows, jump candidates + detail ranking, unit tests (no blocker) | #1404 | merged |
| M2 | T059–T062 | full | core single-key switch: `Settings.review_single_keys` (default on) + `resolve(.., single_keys)`, `sheet_rows`, `key_hint`, unit tests (no blocker) | #1415 | merged |
| M3 | T012–T021, T063–T065, T024–T027 | full | j/k/n/p/v/g in the Review tab, hunk mark, Settings → Appearance switch row, docs (blockers merged) | #1451 | merged |
| M4 | T033–T047 | full | ]/[, c, send chord, shortcut sheet (filtered by the switch), full docs — after M3 (same blockers) | #1458 | merged |
| M5 | T052–T057 | full | jump-to review entries — #1127 merged, after M4 | — | in progress |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | `g` single key or `g f` prefix (redesign table) for jump-to? | `g` = next unreviewed (issue wording); jump-to keeps Ctrl/Cmd+Shift+P | agent-resolved | issue #1274 body; spec.md#assumptions |
| D2 | spec | Do next/prev wrap? | j/k, ]/[, g wrap (803 step rule); n/p continue into adjacent file and wrap | agent-resolved | crates/micold-core/src/navigation.rs; spec.md FR-003, FR-004 |
| D3 | clarify 1 | Send chord with unsaved composer text: save+send, or saved drafts only? | Saved drafts only; composer stays open with its text and the keyboard | agent-resolved | spec.md#FR-010 ("exactly" Send review); crates/micold-client/src/features/changes.rs Msg::SendPressed; review-redesign.md proposal 8 |
| D4 | clarify 1 | FR-001: single-letter keys on by default or behind a Settings switch (redesign open question 5)? | On by default, with a Settings switch to turn them off ("On, with a switch to turn off") | decided by user (2026-10-25, decision card) | spec.md FR-001, FR-022 and clarify rounds 2–3 (Session 2026-10-25, all agent-resolved: switch scope, Settings → Appearance and client-held storage like `reduce_motion`, off behaviour, sheet/tooltips, other windows, jump-to unchanged) |
| D5 | tasks | Plan's M2 (US1–US5 client, ~29 tasks) as one milestone? | Split by story into M2 (US1, US2) and M3 (US3–US5); jump-to becomes M4 (renumbered M3–M5 by D7). Plan, quickstart, contracts, research updated to match | agent-resolved | milestones.md rules 2–3; tasks.md#milestones |
| D6 | tasks | M1 is core-only (22 tasks, no user-visible deliverable)? | Kept whole: every client task needs the blockers (rule 6: core and tests ship, UI waits); splitting core gains nothing observable | agent-resolved | plan.md#milestones; research R1 |
| D7 | tasks 2 | Where does the switch's work go? | New **M2** (core: setting field + `resolve`/sheet/tooltip filters, T059–T062, no blocker); the Settings row and its save/load (T063–T065) join the keys' client milestone, renumbered **M3** (old M2), so the switch never ships without keys (rule 6); old M3/M4 → M4/M5. M3 now 17 tasks: kept whole, the switch row has no deliverable without `j`…`g` | agent-resolved | milestones.md rules 1–6; plan.md D10; research R4 |
| D8 | plan 2 | Settings page and storage for the switch | Appearance page, `Checkbox` + `field_note` after **Reduce motion**; `Settings.review_single_keys`, client-held like `reduce_motion` (daemon write keeps it; no wire change) | agent-resolved | crates/micold-core/src/settings.rs `reduce_motion`; crates/micold-daemon/src/catalog.rs settings write (BUG-025); research R4 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 2f7c3dc307d7f818f4c2995e0ffd0af13213bfa9:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN: 3 MINOR, all fixed (FR-002 scope, FR-004 p/n with no current hunk, checklist note) |
| Review plan | 1 | 209eba49689ebf46a4457c2a5dd880637479b3d2:5ca86ffe05655dc3708ecbaa3676d58bdfc4f439 | CLEAN: 3 MINOR, all fixed (R7 why not `.key_binding`; timing bound 50 ms; pending edge keyed to its path) |
| Review tasks | 1 | 7aa12f7058288de6231b9f39a7ed2953013b393a:dbe223deca1c5b2b34bb6d6d28a4e81151b49cd2 | CLEAN: 2 MINOR, fixed, prose only (T019/T020 test coverage note; T018 wording) |
| Review A M1 (code-review high) | 1 | a103038eb262768a10b41d932d46c9ca30051309:624a8eef02ba1008484c115d155023c9f0081051 | CLEAN: 8 findings, no BLOCKER/MAJOR; 1 fixed (`p` from a stale hunk past a shrunken diff now takes the last hunk, test added), 7 declined |
| Review B M1 (conformance, sonnet) | 1 | 570a8d0e8d03a8b98d1152198e00110d247a8c22:624a8eef02ba1008484c115d155023c9f0081051 | CLEAN: Verify (`cargo test -p micold-core --all-targets`) green, core lib 1707 passed; 1 MINOR (commit the review-A fix) — done in the M1 commit |
| Review tasks 2 (D4 amendment: spec FR-022, plan D10, tasks M2) | 1 | 7a6cb58e29f637cb27fdf83d51760327c475361f:8ee6afde8eab56a73be917e1be7eb1c9f0b0f08d | CHANGES: 2 MAJOR (T060 missed `ValidSettings::into_settings`; T063 tested `v`/tooltips built later in T025), 2 MINOR — all fixed |
| Review tasks 2 | 2 (sonnet) | a33bf9eb7499aa3ee3a28b49446066c80161726e:f0555a548473e311d58e81708fa47c8b2c8cdf6a | CLEAN |
| speckit-analyze (amendment) | — | — | 0 CRITICAL/HIGH; 5 MEDIUM + 5 LOW: C1–C8, C10 fixed in tasks/spec/plan/quickstart; C9 declined |
| Review tasks 2 | 3 (sonnet, scoped to analyze fixes; not counted) | f0555a548 + working tree | CLEAN: 2 MINOR fixed, prose only (`?` closes the sheet only while single keys are on in research R8/contract K2; Scale/Scope M2/M3 lines) |
| Review A M2 (code-review high) | 1 | 5d0dfa4de1f0e3e6bbdbfb83edf55e23bc661685:110db9b34069c52a9f73af2e6e6df4c0d4524846 | CLEAN: 7 findings, no BLOCKER/MAJOR; 2 fixed (one `ReviewKey::is_on` predicate shared by `resolve`/`sheet_rows`/`key_hint`; persist test: a form save keeps a stored `false`), 5 declined |
| Review B M2 (conformance, sonnet) | 1 | 5d0dfa4de1f0e3e6bbdbfb83edf55e23bc661685:110db9b34069c52a9f73af2e6e6df4c0d4524846 | CHANGES: BLOCKER Verify not run (gate died, disk full); MINOR TDD red wording — fixed |
| Review B M2 | 2 (sonnet) | tree of c78577d96 | CLEAN: gate GATE_EXIT=0, review::keys 18 ok, settings tests ok, settings_sections ok |
| Review A M3 (code review high, headless reviewer) | 1 | adde9b38f77bb64399106bfd29f0aa938e49c034:0956edfa59b3efca63cd24a2ac226930fec0e927 | CLEAN: 3 MINOR; F1 (`reset_diff_pane` keeps `pending_edge`) and F3 (`select` of the selected file keeps the current hunk) fixed, F2 declined |
| Review B M3 (conformance, sonnet, headless) | 1 | a8df1179a0a164f1635bb3278ad9b68849fefdc6:af6b01c9f13795709c5e62d92e749c3f375f0366 | CLEAN: 2 MINOR declined (F1 = review A's F2; F2 a re-read keeps `current_hunk`: an index past the new count marks nothing and `step_hunk` clamps it). Client suite TEST_EXIT=0 (all client tests) |
| Review A M4 (code review high, headless reviewer) | 1 | ed5be9d8e88d85fc6fd992076485015db62ff00d:8ab608743e35d9b25ac8218c9a017e6b5ec3d323 | CLEAN: 2 MINOR; F1 (`composer_chord` duplicated the listener's key_input → review_press step) fixed with `ui::composer::review_press`; F2 (composer chord has no repeat flag) declined, see below. A first run returned a bare CLEAN; re-run with JSON output (15 turns) gave this |
| Review B M4 (conformance, sonnet, headless) | 1 | 4128587d1fe3d3ebf9f6c615f96914f7c478b947:eb55db847cf5dac44c9c683d98cdb24abfa07d4b | CLEAN: 2 MINOR; F1 (T037 has no red) fixed, prose only (cycle log: a guard); F2 (T047 open) — the visual pass runs next. Bash refused in the reviewer, so Verify is the unit's targeted runs + scoped gate |

## Declined review findings

- Review A M4 F2: the comment box's send chord passes `repeat: false` — iced's editor `KeyPress` has no repeat flag, so it cannot be known there; a repeat reaches `SendPressed`, which sends nothing once the service reports the send under way (as a double click on **Send review (n)** does). MINOR.

- Review A M1: declined (1) AltGr heuristic lets Ctrl+Alt+key act — contract K3 / research R7 say Ctrl+Alt without logo counts as none, on purpose (AltGr layouts); (2) comment `name` is whole text — contract J1 says text with lines joined by spaces; (3) jump-to uses the stored `range.start()`, not the located line — jump-to lists comments of unloaded files, whose located line is unknown; revisit in M4 when it is wired; (4) `header_rows` builds all rows — research R9's choice, once per `n`/`p`, MINOR; (5) `Press.primary` derivable from `ctrl`/`logo` — contract K5's shape, taste; (6) placeholder `has_hunks`/`in_diff` in `review_candidates` — `comment_order` reads neither, MINOR; (7) client `resolve` returns `None` for review keys — M4 (T052–T057) wires it.
- Review A M2: declined (1) `?` dead under `Holder::Sheet` with the switch off — spec FR-012/FR-022 as clarified: `?` is a single key and off; Escape and outside click close the sheet (T041 tests it); (3) `SINGLE_KEYS_OFF_NOTE` literal assert — pins contract K4a's text, as the K1 sheet-text tests do; (6) `"jknp][vgc?"` restates the table — it pins FR-001's ten keys in the public, stable order (#1276); (7) no daemon-write test for the field — same client-held path as `reduce_motion`, kept by `update_reporting`'s re-read (BUG-025 tests); (4) placeholder `true` — covered now by the new persist test, replaced in T064.
- speckit-analyze (amendment, 2026-10-25): declined C9 (split T064) — T064 is the one green step for T063's tests; splitting it gains no observable step.
- speckit-analyze (tasks): 0 CRITICAL/HIGH. Fixed C1 (SC-005 rank test in T050), C2 (SC-003 reducer timing in T017/T033/T041), C3 (SC-001 walk in T024/T038). Declined I1 (SC-004 counts FR-001/FR-010 keys; the sheet listing the save-comment chord too is a superset, not a conflict), U2 (T058 left to the close unit per milestones.md rule 4), S1 (M1 size: see D6), I2/C4/I3/U1/S2/P1 (LOW, no change).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M3 | A | F2: a pending drawer jump returns before `resolve_pending_edge` | They cannot both be pending for one read: `DrawerRowSelected` selects through `Msg::FileSelected`, which clears `pending_edge` for another file; for the same file the jump is the newer request and wins |
| M3 | B | F2: a re-read of the selected file keeps `current_hunk` | The mark is drawn from `header_rows(..).get(i)`, so an index past a shorter diff marks nothing; `step_hunk` clamps it (M1 review A fix); a file change clears it |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M5 T052 (#1127 re-read, 2026-10-26): the merged title bar's jump-to field (`ui/toolbar.rs::jump_press`) only sends the chord's `QuickSwitch(Msg::Toggled)`, so entries still go in `features/quick_switch.rs::candidates` (plan D8 kept). Divergences from the plan: the row view is `ui/quick_switcher.rs::view_of` (not `ui/quick_switch.rs`); a choice is a `navigation::Step` applied by `shell::navigation::apply_step` → `State::apply_navigation`, so the Review-tab step is a new `Step::Review(ReviewJump)` that dispatches `Sidebar(ReviewChangesRequested(shown location))` then `changes::Msg::Jumped`; `step_comment` split into core `navigate::comment_move` for one id; `Entry` gained `label` (a comment row shows its first line). The Overview left the title bar (sidebar row / Ctrl+Shift+O): no effect here — `ReviewChangesRequested` closes the Overview as the Changes button does. "Current entry" = `shown_location()` in the active project. Files listed in `j`/`k` order (tree sort and filter); comments those of the shown scope, at their stored first line (M1 decline 3 kept: hidden tabs have no located lines).
- M4 full gate: `scripts/gate.sh` GATE_EXIT=0 at f2c78e5aa. Gate fixes after the reviews: the sheet's key column moved to a new material `Keycap` (`material_boundary`: no `style` in `ui/`), Keycap posed in the showcase, dialog counts in `popover_displacement.rs` (28). The visual pass predates the Keycap swap (cap outline instead of fill; layout snapshot re-recorded).

- T047 visual pass M4 (2026-10-26, this unit, by hand per `.claude/skills/visual-pass/SKILL.md`): **PASS**. Release build of the M4 tree (client + its own daemon) on Xvfb :77, tiny-skia, XTest via python-xlib, private `MICOLD_DATA_DIR`, 1280×800, light and dark; fixture `a.rs` `b.rs` `c.rs` (two hunks each) + `d.bin`, Default entry. Evidence `specs/1274-review-keyboard-map/visual-pass/m4-*.png`. B8: `j n c` opens the composer on hunk 1's added line with the keyboard in it (typed text landed), three drafts made with `c` + Ctrl+Enter; on `c.rs` with no hunk `c` draws *Pick lines or move to a hunk (n) to comment* — visible only in the first ~30 ms frame, then cleared (the known changes_watch re-read bug: each `ListRead` clears `view.notice`; not fixed here). B7: `]`×4 `[` → a.rs:3, a.rs:45, b.rs:3 (selects b.rs), wraps to a.rs:3, `[` → b.rs:3, each card highlighted (the outdated → drawer case is unit-tested only: no outdated comment in the fixture). B9: in the composer `j` is typed, Ctrl+Shift+Enter sends (a session starts; the send then fails in this container with Claude Code's folder-trust message), composer stays open with `j`, nothing saved (still 3 drafts). B10: the chord and **Send review (3)** give the same result and bar error. B11: `?` opens the sheet (12 rows, Linux labels), `j` changes no pixel, Escape / outside click / `?` close it, overflow → **Keyboard shortcuts** opens the same sheet. B12: **Send review** tooltip *Send the draft comments (Ctrl+Shift+Enter)* (light and dark). B16 (M4 part, `review_single_keys: false` seeded): `?` does nothing, overflow item opens the sheet with only the two chords and the note, the chord still sends. B15: layout snapshot re-recorded with both sheet states (`eb55db847`). xdotool absent; `?` needs Shift_L+slash through XTest.

- M4 (2026-10-26): T036's `text_area.rs` is `crates/micold-material/src/text_area.rs` now (the client re-exports it); `.on_press` takes iced's `KeyPress`, and the composer turns it into a core `Press` with `ui::composer::key_input` + `keymap::review_press`. iced's `KeyPress` has no repeat flag, so the composer passes `repeat: false`: a held chord there could resend (the service's `sending` flag then refuses it).
- M4: the shortcut sheet's open state is per view (`OpenView::sheet_open`, like `overflow_open`); `ShortcutsOpened`/`ShortcutsClosed` are routed as pure state in `app::State::update` (as the overflow menu's), so the registry's cancel reaches them.

- M3 handover (moved here when M3 merged): M3 unit part 2 (2026-10-26): T012–T026, T063–T065 done; reviews A and B CLEAN; `scripts/gate.sh` GATE_EXIT=0 at f8ebb7f79; PR opened (see *Pull requests*). **Open: T027** (visual pass, quickstart §B B1–B6, B12, B13, B16 at 1280×800 light/dark): not run — the `visual-pass` skill is not loadable in this session (`Unknown skill: visual-pass`, the workspace is not trusted) and this unit has no Agent tool; a headless worker would need unrestricted Bash. Next: run T027 through an `autopilot-worker` (debug build with `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`; ~4.5 GB disk free; xdotool not installed), save evidence to `specs/1274-review-keyboard-map/visual-pass/`, tick T027, push to the PR.
  
  T027 visual pass (2026-10-26, autopilot-worker): **PASS** for the M3 checks. Ran the real client + daemon (release, pinned pair, `client attached to daemon` confirmed) on Xvfb + tiny-skia software rendering in the cloud container, not a real display; XTest input via python-xlib; private `MICOLD_DATA_DIR`; window 1280×800; light and dark (theme seeded in the private `settings.json`). Fixture: a git repo, Default entry (uncommitted), `a.rs` `b.rs` `c.rs` with two hunks each and a binary `d.bin`; no comments (B7–B11 are M4). Evidence: `specs/1274-review-keyboard-map/visual-pass/`.
  - B1 `j` `j` `k`: a → b → c → b, diff from the top (`b1-jk-light.png`, `dark-b1-b6.png`). B2/B3 `n` ×3 then `p`: b hunk 1, b hunk 2, c hunk 1; `p` → b's last hunk (`b2-b3-np-light.png`). The second hunk is not scrolled to the very top only because the short fixture's diff is already at its scroll end. B4: mark = header in primary container roles; dark is strong, light is a pale lavender against the grey unmarked header (≈1.1:1 luminance between the two fills; told apart by hue) — visible, noted, not changed (`b4-current-hunk-light-dark.png`); side-by-side also marks (`side-by-side-current-hunk-dark.png`). B5 `v` ×2: 0 → 1 of 4 (**Reviewed** active) → 0 (`b5-v-light.png`). B6 `g`: c, d.bin, a (wrap), b; all marked → "All files reviewed" is drawn (`b6-all-reviewed-note-light.png`, ~50 ms frame) but gone within ~150 ms — see finding below. B12: "Previous change (p)", "Next change (n)", "Mark this file reviewed on its current change (v)" / "Reviewed on its current change; press to unmark (v)", both themes; hunk buttons hidden on `d.bin`; placement (↑ ↓ before **Mark reviewed**) matches the mockup's `DiffView` header. B13: `jk` typed in the tree filter, selection unchanged. B16 (M3 part): row and note under **Suggestion chips**, same style; cleared + Save → `review_single_keys: false` stored, `j` and `v` do nothing, tooltip names no key; still off after a restart (`b16-switch-light.png`, `b13-b16-dark.png`). Not covered: B12's **Send review** chord tooltip (M4), animation/frame pacing.
  - Finding, not M3 and not fixed (out of scope): on Linux the Changes watch re-reads the list about every 0.5 s with no file change. notify 8.2's inotify backend watches `OPEN`, and `shell/changes_watch.rs` `watch_loop` forwards every event's paths regardless of `event.kind`, so git's own reads of the work tree (`Access(Open)`) wake the next read — a self-sustaining loop (seen from attach, before the Review tab opened). Each `ListRead` clears `view.notice`, so M3's "All files reviewed" note (and any other notice) vanishes within one cycle. Likely fix: drop `EventKind::Access(_)` events in the watcher callback; needs its own issue (482/1269 watcher, idle-CPU impact).

- T012 (M3, 2026-10-26): merged code re-read. Changes recorded in plan D5 and tasks T016/T018/T020/T025: the Review tab state stayed in `features/changes.rs` (no #1269 module); select path `Msg::FileSelected`; **Mark reviewed** = `Msg::MarkToggled`; nav order = new core `review::tree::file_order` (layout without folds; `change_rows` holds only visible rows); new `Effect::ScrollList` + `LIST_SCROLL_ID`; new `FieldId::ReviewComposer` (composer focus was not tracked). The key press is resolved in `app::State::update_changes` (holder + switch at the press), not in the listener.
- M3: the switch's startup load (`shell/startup.rs`, one line) has no unit test — no startup harness exists; covered by quickstart B16 in the visual pass.
- M2 rebase (2026-10-25): PR #1415 conflicted with main (no CI ran). Rebased onto origin/main 35ac6de2d; only conflict `crates/micold-client/tests/settings_sections.rs` `DEFERRED`: kept main's `suggestion_chips` (#1275 M5) and this branch's `review_single_keys` (1274 M3) entries, both sides' rows intact. settings.rs, persist.rs, core settings.rs, settings_issue_mapping.rs auto-merged.

- M2: container disk (~29 GB free) ran out twice in the full gate; target-shared wiped and the gate run with `CARGO_PROFILE_{DEV,TEST}_DEBUG=0`, `STRIP=symbols`. `gh` unauthenticated: branch-start.sh run without the PR argument after confirming M1's merge SHA is on origin/main; PR via GitHub MCP.
- Plan review ran via headless `claude -p --agent autopilot-reviewer`: Bash is denied there (workspace untrusted), so the reviewer used Read/Grep/Glob; a first run that only had denied Bash calls returned a vacuous CLEAN and was rerun. Symbols it could not confirm (`UnifiedIndex`, `SideIndex`, `jump_offset`, `settings::DiffLayout`, `is_primary_shift`, `Msg::SendPressed`, `window.focused_field`, `overlay::registry::topmost`) were confirmed by the plan unit with grep.
- Coordinator requirement from #1276 folded into the plan: public `micold_core::review::keys::REVIEW_KEYS` in M1; client binds only from it.

- Spec review ran via headless `claude -p --agent autopilot-reviewer` (read-only tools): no Agent tool in this unit.

- Blockers #1269/#1270/#1271/#1273/#1127 not merged at spec time; spec states the dependency (spec.md#dependencies).

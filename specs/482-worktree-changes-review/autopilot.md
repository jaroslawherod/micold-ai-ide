# Autopilot ledger — #482 worktree-changes-review

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #482 "Review a worktree's changes with inline comments and send them to the session" (scope verbatim from the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #482
- **Worktree branch**: claude/project-thread-v1va8z
- **Started**: 2026-10-06
- **Phase**: implement
- **Next step**: milestone M5: continue from *Handover* (composer focus fix, T075 record, verify.md)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #624 | whole branch (design + every milestone), opened by the orchestrator | draft | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T026 | full | Changes view with the changed-file list (MVP) | #624 | done |
| M2 | T027–T040 | full | Unified diff of the selected file | #624 | done |
| M3 | T041–T049 | full | Side-by-side layout, syntax colouring, kept layout | #624 | done |
| M4 | T050–T063 | full | Comments on lines and ranges | #624 | done |
| M5 | T064–T075 | full | Send comments to the running session | — | todo |
| M6 | T076–T080 | full | Send when no session is running | — | todo |
| M7 | T081–T089 | full | Live refresh and outdated comments | — | todo |
| M8 | T090–T097 | full | Clear, discard, and removal with the worktree | — | todo |

## Decisions

- M4 T054/T058: slot rows. `VirtualRows` takes sorted `(row, extra height)` pairs (`visible_range_with`/`spacers_with`); `DiffView::slot(side, line, element)` anchors a slot by its line rather than a row index (the caller knows lines, `DiffView` maps them in one pass over the rows), wraps each in an iced `Sensor` whose `on_resize` reports the measured height (`.on_slot_measured`), and takes the measured heights back (`.slot_heights`), `SLOT_ESTIMATE` until measured. Gutter shift-clicks: a small gutter widget tracks `ModifiersChanged` itself. `TextArea` is a thin widget around `text_editor` drawing `FilledField`'s container, hover layer and indicator (focus read from the editor's tree state).
- M4 T059: `features::changes::State.reviews` is keyed by `(project, wire worktree dir)` rather than by `SessionLocation` (not `Ord`), and keeps every attached project's pushes so a project switch shows each entry's own comments without a re-push; `Msg::Opened` now carries the project (the root passes `workspace.active`). The client sends the composer text untrimmed; the service trims (W1). Selecting another file drops the pick and a new-comment composer (an edit composer stays).
- Clarify round 1: US3 scenario 4 and FR-003 answered by the orchestrator as defaults (user away); recorded in spec.md Clarifications. No further ambiguities, no second round.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

- Tasks: US1 split in three (list / unified diff / layouts+colour), US2 in two (comments / send), US4 in two (refresh+outdated / tidy), along acceptance scenarios. M1 stays at 26 tasks because Setup + Foundational (core value types, protocol v30 in one edit — a second bump would fail schema_hash.rs) must ride with it. Polish T098–T100 changes no code: close unit.
- Gate (tasks unit): specs-only, `scripts/check-criteria-observables.sh` passed (spec has no criteria table). Design PR: none of its own — this run ships one PR for the whole branch (orchestrator opens it); PR body written to the session scratchpad pr-body-482.md.
- speckit-analyze (0 CRITICAL, 2 HIGH, 6 MEDIUM, 5 LOW): fixed in tasks.md (D1 watch::Debouncer + cap_spans as tested pure functions, D2 two-session test moved to T066, D3–D7, D12); spec.md edits (D8 no-bracketed-paste edge, D3 worktree removed outside the app, D10 FR-016 wording, D13 toggles reset on open) and plan.md Constitution row I (D1). D9, D11 left: wording only.
- M1 L1 deviation: the Committed/Uncommitted toggles are `ToggleChip`s (`.active().disabled()`), not `LabelledToggle` as contracts/changes-view.md L1 says — `LabelledToggle` has no on/off state to show. Same behaviour, the library's stateful control.
- M1 V1 deviation: the Default row has no menu of its own; it reuses the worktree `WorktreeMenu` with `dir_name == ""` (the wire's name for the project root), right-press added in `ui/sidebar.rs`, and `ui/mod.rs` offers only **Review changes** for `""`. Avoids a second context-menu surface.
- M1 part 5: the review reads go through the `Git` seam (`Git::review_base`/`change_list`, delegating to `GitCli`'s inherent methods; `FakeGit` answers no base / no files) and `Capabilities::shared_git()`, because `no_concrete_implementations` allows `GitCli` to be chosen only in `shell/capabilities.rs`. With no local git (daemon elsewhere) the view says the files cannot be read here.
- M1 part 5: `settings_sections` DEFERRED gains `("diff_layout", "482 T041")`; the root vocabulary guard moves to 15 feature wrappers (`Changes`); the layout snapshot's worktree-menu state grows by the **Review changes** row (regenerated deliberately).
- M1 part 4: the orchestrator waived the part-4 escalation limit for M1 (steady progress on a 26-task milestone) and continued.
- M3 Windows CI (`review_git` LF → CRLF test red on windows-latest): the fix is in the test fixture (repo-local `core.autocrlf=false`), not in `GitCli::file_diff`. With `autocrlf=true` git normalises the edit away and would not commit it; forcing `autocrlf=false` in the diff would show every line of every touched file changed on a Windows checkout. Quickstart §C B6 now says to run with `core.autocrlf=false` and why; the user guide notes it.

- M4 T057: the daemon's `Reviews` keys entries by worktree dir `String` (`""` = Default, as on the wire; `SessionLocation` is not `Hash`) and reads a project's review file on first use rather than at catalog adoption — same observable behaviour (attach and edits read it), no read for projects nobody opens. `ClearSent`/`DiscardPending` answer `Refused` ("not available in this build") until M8 (W4); `Busy` for a comment in a send arrives with M5.
- M5 T070: MCP `send_session_input`/`deliver_first_prompt` keep their pre-confirm trust check and write without requiring bracketed paste (unchanged behaviour, `write_submission(.., false)`); the review send requires bracketed paste (W9) and refuses without it.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c5e15968c86ce0c17544cde68a31ff74e2fb1e8b:107ab7ed3aa9b067a4dca5c02aca6e6709d4d158 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | c960571fb7bed1fa440f89fe3b9720fd30c8d897:c74e2d4a9e09412fc66dd4915e3c7e105b785552 | CLEAN: 2 MINOR (both fixed) |
| Plan | 1 | 76b23838a33fc28d5c9f6db389cc15f46acd39a1:3a5ac98c80a82e8c74fb02606f5bee99309eaeeb | CLEAN: 3 MINOR (all fixed, prose only) |
| Review A M1 (code-review high) | 1 | c30e1678e86420d958d5891c993fbec650892723:1ed086452e5a2581dc19a1453b11c73730c68936 | CLEAN: 2 MINOR (untracked files read whole to count lines; lossy UTF-8 of non-UTF-8 paths), not fixed |
| Review B M1 (conformance, sonnet) | 1 | 6f1dd0f1b8ccac63f75133430d4aa2f34f23e4c6:e3ab664fc93e6c9de5e7223181127788b4024731 | CHANGES: 1 MAJOR (cycle-log greens missing) fixed by running the tests and recording them; 2 MINOR (T009 test-after kept as recorded; Verify not runnable in reviewer sandbox, unit re-ran it: all pass) |
| Gate M1 | full | c30e1678e86420d958d5891c993fbec650892723:1ed086452e5a2581dc19a1453b11c73730c68936 | green except the 6 root-only permission tests (container runs as root; pass in CI) |
| Review A M2 (code-review high, scoped 24009c2d..HEAD) | 1 | 0141f0f550c385a1179f01e688868725e0526001:142cf0fa764870556e1cbadf46a52186aab5b9c6 | CHANGES: 5 MAJOR, 3 MINOR — F1–F3 fixed (literal pathspecs, symlink target, untracked line count); F4 fixed (`UnifiedIndex`), F5 fixed (failed list read drops selection), F6, F7 MINOR fixed; F8 declined |
| Gate M2 (pre-fix push) | full | 1a705ff567ac2a882e87654e35d62ea5d00fa450:5d37a3e5fa473db25fc799e1e5b6128001fc4741 | green except the 6 root-only permission tests; pushed 5d37a3e5 |
| Review A M2 (code-review high, scoped 142cf0fa..HEAD, sonnet) | 2 | b004815caf3de89ca8132c68b3565f11535f0752:3d697c274f0b993433e7d3a0311ae71c659c034c | CLEAN; 2 MINOR not fixed (git.rs `count_lines` reads an oversize untracked file whole; list row shows 0 lines for it) |
| Gate M2 (after A fixes) | full | b004815caf3de89ca8132c68b3565f11535f0752:3d697c274f0b993433e7d3a0311ae71c659c034c | green except the 6 root-only permission tests |
| Review B M2 (conformance, sonnet) | 1 | d3f26dbd66e4483c1eabd9e84926affe4230bd21:3d697c274f0b993433e7d3a0311ae71c659c034c | CLEAN: 3 MINOR (Verify not runnable in its sandbox — gate6 log: review_git 17 passed, features_changes 22 passed; ledger staleness, since updated; T040 open) |
| Tasks | 1 | bdc7eb7285b7355f190266e00dd32362649c017c:a284e9b2ea58bc2c731608a1600a913ad9b91dec | CLEAN: 3 MINOR (all fixed: checklist ticked, T067 outdated list deferred to M7, M1 Verify + SC-001/SC-005 mapped) |
| Gate M3 | full | 54d7e5783b500fcf5ba1d76cc2cee853433d3b67:a04451e3567b124ee02c64019d01feff6a2f9689 | green except the 6 root-only permission tests; push refused by the gate hook (no `mise run gate` record: mise absent here), commits left local |
| Review A M3 (code-review high, scoped 4aafd1bf..HEAD) | 1 | 54d7e5783b500fcf5ba1d76cc2cee853433d3b67:a04451e3567b124ee02c64019d01feff6a2f9689 | CLEAN; 1 MINOR not fixed (an in-flight `LayoutInForce` echo can briefly show the previous layout after two quick toggles) |
| Review B M3 (conformance, sonnet) | 1 | 54d7e5783b500fcf5ba1d76cc2cee853433d3b67:a04451e3567b124ee02c64019d01feff6a2f9689 | CLEAN; 1 MINOR (Verify not runnable in its sandbox; the unit's full gate ran it: all green but the 6 root-only tests) |
| Visual pass M3 (T049) | 1 | a04451e3 | B3 pass; B18 found the showcase diff literal missing its context indent (colours 4 chars off), fixed in showcase/sections/review.rs, re-run pass |
| Gate M3 (after the showcase fix) | full | c28ea9d173beadd20e96bdc75bf0abdf4c47a656:a04451e3567b124ee02c64019d01feff6a2f9689 | green except the 6 root-only permission tests; push refused by the gate hook, commits left local |
| Review A M4 (code-review high, scoped 9dbcf9e9..HEAD) | 1 | e64ae19d83f1642c7bdf02e3f2843609f3dc188e:2ada2a4a7c1723fd4dc39e3a3cbd344b1bfc35cf | CHANGES: 1 MAJOR, 3 MINOR — F1 fixed (Sensor key carries whether the height is known, so a dropped height re-measures); F4 MINOR fixed (edit composer closes when its comment is gone); F2, F3 MINOR declined |
| Review A M4 (code-review high, scoped fix diff, sonnet) | 2 | a6006bfc6d7c4efa1b155d6c2a023138d3f1209b:2d659f8afe3ae846e58bcb832f84cb766beb1671 | CLEAN |
| Gate M4 (scoped, workspace) | scoped | 2d659f8a + review_edit fixture path | red twice then green except the 6 root-only permission tests: clippy `duplicated_attributes` (review_edit's `#[allow(dead_code)]`), `documentation_is_not_read` ("README.md" literal in review_edit's seed, now `src/lib.rs`); one run lost to a full disk |
| Gate M4 (full, raw commands, 0562c7e0) | full | green except the 6 root-only permission tests (pass in CI): fmt, clippy core+workspace, workspace tests, scripts/tests/*.test.sh |
| Visual pass M4 (T063) | 1 | a46f1c19 | B7, B8 (dark), B14 (light, after restart) pass; Send to session is M5 |
| Review B M4 (conformance, sonnet) | 1 | eecc77bb3f560f5dfed9108b8047f6423c45ad54:a46f1c19c1d7839c0e46269f3e52c3dd7bfceedb | CLEAN; 2 MINOR not fixed (F1 `review_edit` pushes `ReviewChanged` to every client, not only the project's, and snapshots the catalog per edit; F2 Verify not runnable in its sandbox, the scoped gate ran it) |

## Declined review findings

- Review A M2 F8 (MINOR, `file_diff` reads both blobs into `SideLines` though nothing uses them yet): data-model `LoadedDiff` carries them for M4 quotes and M7 outdated checks; kept.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
- Review A M4 F2 (MINOR, composer cleared before the service answers, text lost on a refused edit): a refusal is a validation or I/O failure the composer cannot fix by retrying; the error surfaces as an operation error. Not fixed: MINOR, and keeping the text needs a restore path on every refusal; left for M5's send work to revisit.
- Review A M4 F3 (MINOR, review-file write under the daemon's state lock): not fixed: MINOR; the lock also orders concurrent edits to one file.

## Handover

M5 unit 3 handed over at the 150k cap (2026-10-07).
- **Done (committed)**: R5 fixed (test sent `x` without a line end; the stand-in `cat` reads a canonical terminal line by line — now `x\r`); `review_send` 5/5 green. T067 run red (3 of 40), T073 implemented (`send_action`, `SendPressed`, `sent_text`, `send_error_text`; `ui/changes.rs` `push_maybe` → `if let` push), 40/40 green, client clippy clean. T074 user guide written. Cycle log rows for T070–T073. T067, T070–T074 ticked; Decision on MCP trust/paste recorded.
- **T075 visual pass (light, Xvfb :77)**: B9 with a session already running in `wt` PASS — Send to session (3) enabled, snackbar "Sent 3 comments to New session", button then "(0)" disabled, cards read Sent with no Edit/Delete, the terminal shows the one bracketed prompt matching the prompt contract. B10 PASS — one new comment, "Sent 1 comment to New session", the terminal gets only that comment. Crops in `visual-pass/m5-b9-*.png`, `m5-b10-*.png`; `visual-pass/m5.md` NOT written yet. Not run: dark scheme.
- **Defect found by the pass (fix in progress, real, MAJOR)**: with a session selected, the review composer takes no keys (works with no session). Cause: `State::terminal_focused()` (`crates/micold-client/src/app.rs:379`) ignores the Changes view, which replaces the terminal pane (V1), so `KeyboardElsewhere` (`ui/mod.rs:407`) strips the composer's focus. Red test added: `tests/terminal_focus.rs::the_changes_view_takes_the_keyboard_from_the_terminal_it_replaces` (written, red NOT yet confirmed — the last run's `head -30` cut the output; rerun `cargo test -p micold-client --test terminal_focus the_changes` and log the red). Fix: add `&& self.changes.open.is_none()` to `terminal_focused()`; check `view_facts` `main_area_taken` too (FR-002 attention: the session is not in view while the Changes view is) — decide and record. Then cycle-log row, re-run the pass to confirm typing with a session running (fake CLI: `$SCRATCHPAD/vp-fake-claude.sh` as `claude` on PATH, `HOME` with `$SCRATCHPAD/vp-claude.json` as `.claude.json`, `env_include_enabled` is false in `$SCRATCHPAD/vpdata77` settings; data home `$SCRATCHPAD/vpdata77`, fixture `$SCRATCHPAD/fx/P` now has 4 Sent comments in wt), write `visual-pass/m5.md`, tick T075.
- **Next**: the above, then verify.md (scoped gate + review A high, review B, full gate, `pr-body-482-M5.md`). Disk tight: `$SCRATCHPAD/prune.sh` before builds; `cargo check`/clippy rather than building all client test binaries; `$SCRATCHPAD/t.sh` needs an absolute log path.

## Open escalation

None.

## Follow-ups not done

None yet.

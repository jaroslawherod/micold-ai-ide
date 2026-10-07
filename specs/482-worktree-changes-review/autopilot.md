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
- **Next step**: close unit (T098–T100); every milestone done, PR #624 carries the whole branch

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
| M5 | T064–T075 | full | Send comments to the running session | #624 | done |
| M6 | T076–T080 | full | Send when no session is running | #624 | done |
| M7 | T081–T089 | full | Live refresh and outdated comments | #624 | done |
| M8 | T090–T097 | full | Clear, discard, and removal with the worktree | #624 | done |

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
- M5 T075 fix: the Changes view counts as taking the main area in `view_facts()` too (039's `ViewFacts` doc: "a screen fills the main area instead of the session"), so the selected session is not in view behind it — the same rule as Settings. Tested in `attention_view_report.rs`.

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
| Visual pass M5 (T075) | 1 | 2473be8a, e2cc6eba | B9, B10 (light) pass; found the composer taking no keys with a session selected (MAJOR), fixed in e2cc6eba (`terminal_focused()` and `view_facts().main_area_taken` count the Changes view); re-run in dark: typing works with a session running, B10 pass |
| Gate M5 (scoped, workspace) | scoped | 63c21b52..ace6b8b2 | red: clippy `too_many_arguments` on review_send's `add` helper (allowed); a disk-full link (stale test binaries removed); `documentation_is_not_read` ("README.md" fixture path in prompt.rs tests, now `notes.md`); then only the root-only permission test stopping the run — full gate next |
| Review A M5 (code-review high, 018d55df..HEAD) | 1 | e6497af342fd4835004625d99753fe74d44b76c2:63c21b52f0d70f946f038ce9de7d97fd61020965 | CLEAN, no findings |
| Review A M6 (code-review high, a9663735..HEAD) | 1 | 75695e6e3c3006d92d4fefc683466dd35de93462:49e3e80a95417d8f00e88955445b992d847955d4 | CLEAN, no findings |
| Review B M6 (conformance, sonnet) | 1 | 75695e6e3c3006d92d4fefc683466dd35de93462:49e3e80a95417d8f00e88955445b992d847955d4 | CHANGES: F1 MAJOR Verify not runnable in its sandbox (answer with the run); F2 MINOR fixed (Internal); F3 MINOR open |
| Review B M6 (conformance, sonnet, fix diff) | 2 | bfbf1d01ddb34f646b043551cc22b9f32779ee95:4a3eecb82206266566f4b2f51d468c258a8f2fdd | CLEAN, no findings (F1 answered by the run: review_send 12/12, daemon + client tests green but the 6 root-only tests; F2 fixed; F3 AsksTrust tested, NotStarted declined) |
| Visual pass M6 (T080) | 1 | bfbf1d01 | B16, B9 (no session running), B11 pass in dark (`visual-pass/m6.md`); no defect found |
| Gate M6 (full, raw commands, 63348a61) | full | b422fbc72864e7dcff6613b32d96ab6d793785bb:63348a61962fb962bba842c326d2c6b013319c6c | green except the 6 root-only permission tests (pass in CI); no SCRIPT_FAIL |
| Review B M5 (conformance, sonnet) | 1 | 74cbb069cdfc59ee86ddfa6f81e310a536652ab8:ace6b8b2dd36d05440a48a13e72326b7e2f3f3a3 | CLEAN; 1 MINOR (Verify not runnable in its sandbox; the full gate ran it: review::prompt 11/11, review_send 5/5) |
| Review A M7 (code-review high, 7bc6c029..HEAD) | 1 | d07862c24c2bdb7220d13123681afca799b78974:7ce47a8bcb738cc21f24ee8b58262cc24b29b72c | CHANGES: F1 MAJOR open (a New composer keeps its pick after the refresh drops it; Save then does nothing when the lines are gone); F2 MAJOR fixed (`Debouncer` `MAX_WAIT` 1 s); F3 MINOR fixed (`GIT_OPTIONAL_LOCKS=0` on review reads); F4 MINOR open (watch setup failures swallowed, no log); F5 MINOR open (`&GLOBAL[..3]` order dependence) |
| Gate M7 (full, raw commands, 2nd commit after 7ce47a8b, before the F2/F3 fix) | full | — | green except the 6 root-only permission tests (pass in CI); no SCRIPT_FAIL |
| Review A M7 fixes (unit 3) | — | — | F1 fixed red first: an unplaced New composer keeps its text, shows above the diff with a note and Save disabled, a new pick re-places it; a dropped selection closes it. F4 fixed (setup failures logged to stderr). F5 fixed (`CHECK_IGNORE_GLOBAL`) |
| Review A M7 (code-review high, sonnet, fix diff) | 2 | ae613a12fcf468dfd126b32bf9db60ca5b589736:de6531e0d2e66603cb8338420d0ee12a5a37faec | CLEAN, no findings (F1–F5 fixed) |
| Review B M7 (conformance, sonnet) | 1 | ae613a12fcf468dfd126b32bf9db60ca5b589736:de6531e0d2e66603cb8338420d0ee12a5a37faec | CLEAN; 1 MINOR (Verify not runnable in its sandbox; the full gate runs both commands) |
| Visual pass M7 (T089) | 1 | de6531e0 | B12 edit, commit, C4 unplaced composer, ignored file pass (light); SC-004 20/20 within 2 s, max 0.74 s (`visual-pass/m7.md`); no defect |
| Gate M7 (full, raw commands, b05e78ee) | full | 00aa5c2b98be8fb30202acd473c24c759a4fba20:b05e78eed6cf8f1a0629a8be84edbfdcf7de15ca | green except the 6 root-only permission tests (pass in CI); no SCRIPT_FAIL |
| Gate M8 (scoped, d5d77ffa) | scoped | — | red: `popover_displacement::every_popover_is_in_the_table` (dialog count 12 → 13); fixed. The client run after it hit a full disk (`No space left`): `cargo clean -p` of the three workspace crates freed 26 GiB |
| Review A M8 (code-review high) | 1 | 5550a1436bf5abe77ebb811936ce8f44ab624f28:d5d77ffaf356418bd3d77b00ed2b4da185669324 | CHANGES: F1 MAJOR (unreadable repo prunes every worktree) fixed red first (`an_unreadable_repository_prunes_nothing_on_refresh`); F2 MAJOR (offline included worktree pruned) fixed (an included path counts as gone only when its parent folder can be read); F4 MINOR fixed (confirm re-checks there is something pending; test added with the fix, not red first); F3, F5 MINOR declined |
| Review A M8 (code-review high, sonnet, fix diff) | 2 | f6e13d5bfa152a6e9b5b6f7f4ebcb29ccdc57f1a:2ecb60db9250d6d1a17e9ec726ea5ffc90a403aa | CLEAN, no findings (F1, F2, F4 fixed; F3, F5 declined stand) |
| Review B M8 (conformance, sonnet) | 1 | aa905d18ec5f1460e73484105375922b16cd6626:49051558c35a557ee260fadf3d01658ffef24424 | CLEAN; 3 MINOR: F1 Verify not runnable in its sandbox (the full gate ran `review_edit`: 10 passed); F2 declined; F3 fixed (`DiscardCancelled` arm moved above the connection arm's comment block) |
| Visual pass M8 (T097) | 1 | 49051558 | B13 (Clear sent; Discard pending… Cancel, then confirm) and B17 (delete `wt` with its view open; empty after restart) pass in light (`visual-pass/m8.md`); no defect |
| Gate M8 (full, raw commands, `--no-fail-fast`) | full | 37b6b00864c4ea23e70b9f64aa873c15d904a671:c7e2614d4918abd18d88c9d327f97efc0b354e18 (+ F3 move) | green except the 6 root-only permission tests (pass in CI) and one hang of `history_service_restart::a_stop_then_a_start_over_a_connection_saves_and_restores_in_that_order` (killed after ~16 min). Pre-existing flake, not M8: the binary built at M7 (411f25a5) hangs the same way (2 of 6 parallel runs each, M7 and M8); alone or `--test-threads=1` it passes 10 of 10. Scripts rerun outside the build lock (the first run deadlocked on a nested `build-lock.sh` because the gate itself held the lock): 18 scripts, no SCRIPT_FAIL. `review_edit` 10, `features_changes` 54 passed |

## Declined review findings

- Review A M2 F8 (MINOR, `file_diff` reads both blobs into `SideLines` though nothing uses them yet): data-model `LoadedDiff` carries them for M4 quotes and M7 outdated checks; kept.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
- Review A M4 F2 (MINOR, composer cleared before the service answers, text lost on a refused edit): a refusal is a validation or I/O failure the composer cannot fix by retrying; the error surfaces as an operation error. Not fixed: MINOR, and keeping the text needs a restore path on every refusal; left for M5's send work to revisit.
- Review A M4 F3 (MINOR, review-file write under the daemon's state lock): not fixed: MINOR; the lock also orders concurrent edits to one file.
- Review B M6 F3 (MINOR, `FirstPromptUndelivered::NotStarted` untested): `AsksTrust` now has a test; `NotStarted` needs `start_session` to return false, i.e. the spawn itself failing, which a stand-in CLI cannot cause deterministically (a CLI that exits at once still spawns); the missing-CLI test covers the earlier refusal. Not tested; MINOR.
- Review A M8 F3 (MINOR, memory dropped when the review file write fails): not fixed: MINOR; every other review edit already keeps memory as the truth and logs a failed save (M4 F3 shape).
- Review B M8 F2 (MINOR, `discard_dialog`'s unused `_env_include_outcome`): declined: every registered dialog builder takes it (`about.rs`, `confirm_delete.rs`, …); the registry calls them through one signature.
- Review A M8 F5 (MINOR, `forget_project` keeps reviews when the catalog removal fails): declined: the project is then still in the catalog, so keeping its comments is right; clients drop the project's views on removal, so no empty push is needed.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- `micold-daemon` `history_service_restart::a_stop_then_a_start_over_a_connection_saves_and_restores_in_that_order` hangs in about 1 of 3 parallel runs of its binary in this container, at M7 as at M8 (not this feature's code). Not investigated; worth an issue if CI shows it.

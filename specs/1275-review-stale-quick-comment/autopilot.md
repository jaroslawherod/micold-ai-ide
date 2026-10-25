# Autopilot ledger — #1275 review-stale-quick-comment

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #1275 "Review UI: stale-awareness and quick comment": "Blocked by: #1271.
  'n files changed since you looked' without reflowing the diff being read; drag over line numbers
  opens the composer; optional suggestion chips (off by default). Part of the review UI redesign:
  `docs/development/review-redesign.md`. Mockup: https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu
  (Review tab redesign). Acceptance also: pure logic in micold-core with unit tests; docs, layout
  snapshot and visual pass (1280x800, both themes) updated."; labels: enhancement, flow:feature
  (in-progress added by orchestrator)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1275
- **Worktree branch**: claude/project-thread-1l3w9s
- **Started**: 2026-10-24
- **Phase**: implement M3
- **Next step**: CI on #1424 after the red-CI fix (rebase on main bb931624b, `changes-review-held` fixture re-recorded), then merge M3.
- **Red CI 1 (#1424)**: `layout_snapshot` failed only in `changes-review-held`: main added a toolbar element (new child 0/0/0/0/0/0/3, 360x32 at x 578.6; the two 40px buttons shift to /4 and /5) under our state. Reproduced locally after `git rebase origin/main`; re-recorded, fixture diff confined to that state (15+/6-). No other code change.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1370 | design (spec, plan, tasks) | merged | c4ca075476319aaa90f54c1438fa910aada87585 |
| #1380 | M1 | merged | 8c0711181531461916b4e025a22d3da0b615f577 |
| #1413 | M2 | merged | 055754352429f96b08c2b5df561b32d6a34d764d |
| #1424 | M3 | open; CI red (layout snapshot drift from main), fixed by rebase on bb931624b + re-recorded `changes-review-held` | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T008 | full | change identity in every row + `review::stale` (core, tests) | #1380 | merged |
| M2 | T009–T014 | full | `review::drag`, `review::chips`, `Settings::suggestion_chips` (core, tests) | #1413 | merged |
| M3 | T015–T025 | full | Review tab holds list+diff; notice, dots, apply; docs, snapshot, visual pass | #1424 | red-CI fix pushed at 892c6760b (main bb931624b), full gate green; CI pending |
| M4 | T026–T034 | full | drag over line numbers opens composer; docs, visual pass | | gated: #1269 and #1271 merged |
| M5 | T035–T043 | full | suggestion chips + Settings switch; docs, snapshot, visual pass | | gated: #1269 and #1271 merged |
| M6 | T044–T046 | full | reviewed file with held change shows unseen + dot; docs, visual pass | | gated: #1271 merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-014: hold the whole view or only the open diff? | A, whole-view hold (decided by user 2026-10-25, decision card) | orchestrator (recommended; user not yet answered) | docs/development/review-redesign.md item 8 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| spec | 1 | 9e154017e95adc6a613e45c3f02e6da155dc11ba:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | 3 MAJOR, 2 MINOR. F1 fixed: FR-001a per-row change identity (git object ids from `diff --raw`, `hash-object` for working-tree files, size+mtime over limits, PR patch hash), AS12, FR-003, Context/Assumptions. F2 fixed: FR-009 + US1 AS9–11 (select non-held reads as today; held reads latest and leaves set; no-longer-listed says no changes), Key Entities. F3 fixed: whole-view-only MUSTs marked conditional on FR-014. F4 fixed: match by path, rename counts once under new path. F5 fixed: checklist item unticked. |
| spec | 2 | 89c07d79a52e9cb5ae89d0beff62b6ccf424039e:4a06e88388d0b4cd7395ec7ce70882b466871cf9 | CLEAN (sonnet), 1 MINOR: edge case *Selected between reads* not marked conditional on FR-014. Fixed: prefixed "under the whole-view hold (FR-014)" and noted it does not arise under open-diff-only. |
| plan | 1 | d4cee46d92e7567c10df1b1c2dd058d0e9665e4c:c9789f7dba0d05f1eaeb440c527c01daf3f56c20 | CHANGES: 1 MAJOR, 2 MINOR, all verified and fixed. F1 (MAJOR, held: `DIFF` in `review/git.rs:33` has no `--no-abbrev`, so `--raw` ids are abbreviated and never equal `hash-object`'s): R1, data-model §3–§4 now say `range_rows`/`range_paths` add `--no-abbrev`; I1 all-zero = 40 zeros; new I1a real-git check that ids are full length. F2 (MINOR, held: `StoredSettings` at `settings.rs:742`, conversions `:903`/`:950`): named in data-model §9 and K5. F3 (MINOR, held): `DiffRead.row` is now `DiffRow { NotRead, Unlisted, Row }` (data-model §12, R5, H10a; failed `change_list_of` → `NotRead`). Also per orchestrator: M3–M5 wait for #1269 and #1271 (plan, spec dependency note). |
| plan | 2 | 94877b893db9017f02f0ab873e42ac7c5df470eb:00519bfa6217bd0107f1f710cc7d94ebdf4b596f | CHANGES (sonnet): round-1 fixes confirmed; 2 MAJOR, 1 MINOR, all on the gating wording, fixed prose only (no design change): F1 spec.md blocker table (hold, drag, chips rows → "no: after #1269 and #1271 (core part ships now)"), US1/US2 priority notes, Assumptions notice line; F2 research R11 rewritten (core now, M3–M5 after #1269+#1271, M6 after #1271; rejected: build M3–M5 now in today's view, rework cost); F3 plan #1269 bullet and summary line, contract H title. Was: Round-1 snapshot is STALE after branch-start rebased onto origin/main (review-snapshot.sh diff exits 2): run a full round on round 1's model, or review the fix commit 00519bfa (`git show 00519bfa -- specs/`). |
| plan | 2 (fixes) | 1191f086 | ACCEPTED by the orchestrator: round-2 fixes are prose only (verified); no round 3. Branch rebased onto origin/main by the tasks unit (fix commit now 14fa6a7a). |
| tasks | 1 | 9cdc8b1a33e061cb296192d30af2188a2d21c73c:ea63a5eb357c21ec233f68fc0af40d4216fdb0e8 | CHANGES: 2 MAJOR, 2 MINOR, all verified and fixed. F1 (MAJOR, held: exhaustive `window_settings` `persist.rs:332`, `set_changed` destructure `:409`, pinned key list `settings_issue_mapping.rs:185-210`; persist test literal `:1238` ends in `..` (`:1171` does not: corrected in round 2); workspace scan for `Settings {` without `..` finds only core `settings.rs:935/948`, client `features/settings.rs:795`, `persist.rs:333/409`): T013 adds the key to the pinned list, T014 lists each site (`window_settings` takes `stored.suggestion_chips`, `set_changed` has `suggestion_chips: _` and no `set!`, `ValidSettings::into_settings` sets false, never written because the save goes through `set_changed`), T038 replaces all three with a keep-every-other-setting test. F2 (MAJOR, held): Skill tool still says Unknown skill; ran `.claude/skills/speckit-analyze/SKILL.md` steps by hand (result under *Plan*/*Tasks* below). F3 (MINOR, held): T002 no longer names `review_git.rs`, names `changes.rs` test helpers; T014 says `ValidSettings::into_settings`. F4 (MINOR, held): `pub mod` lines moved to T008/T010/T012. |
| tasks | 2 | 64b03cb3ab2547b2dce5040cd993877738b24612:b9a769ba4a2504e5d0c0b8d4bd71e19d73202d82 | CHANGES (sonnet): round-1 fixes F2–F4 confirmed; 1 MAJOR: `persist.rs:1171` test literal is exhaustive, not `..`. Fixed by the orchestrator in T014 (add `suggestion_chips: false`); a factual task-list correction the compiler would also catch, verified against the code, no round 3. |
| M1 A (code-review high) | 1 | 4a00118fcb2ed7417b603fca94e6e7a6b6a4c739:19a082ccf3ed3810272518621f9be51459286dbc | CHANGES (run by the orchestrator): 3 MAJOR, 3 MINOR, all verified and fixed, red test first. F1 (MAJOR, held: `ChangeList::put` never touched a rename's `from`, so `x` stayed shown beside `y`): `Held::adopt` also puts a fresh rename's `from` into shown and latest; test s11. F2 (MAJOR, held: adopting an undotted path the fresh read no longer lists left set ≠ difference): `adopt` recomputes `changed = changed_between(shown, latest)`; test s12 with an invariant check. F3 (MAJOR, held: one failing path made every row `Stat`): on a failed batch `identify` hashes each path alone (not after a timeout), so only that row falls back; test with a required clean filter that fails. F4 (MINOR, held: `hash-object --stdin-paths` C-unquotes a leading `"` and drops a trailing CR): such paths take `Stat` (`carried_verbatim`); test with `"q"` beside `q` and `cr\r`. F5 (MINOR, held: `--raw` gives a gitlink's worktree side as zeros, the dir's stat never moves): a directory with its own `.git` takes `git rev-parse HEAD` as `Object`; submodule test. F6 (MINOR, held): `NameStatus.mode_changed: bool` became `modes: [u32; 2]` (+ `mode_changed()`), `ChangeIdentity::Sides` gained `modes` (serde default); chmod test. |
| M1 B (conformance) | 1 | 4a00118fcb2ed7417b603fca94e6e7a6b6a4c739:19a082ccf3ed3810272518621f9be51459286dbc | CLEAN (sonnet, run by the orchestrator). Verify: lib 1578 passed incl. 10 `review::stale`; `review_git` and `pr_diff_split` passed; `settings_refuses_save_over_failed_read` failed only because the reviewer ran as root (`scripts/gate.sh` runs it unprivileged). |
| M2 A (code-review high) | 1 | e12a747f740c0ac99c3bd5caf92c4a4a84d04d4a:00fb53299583fcd03c4a0bc131e18d39d1ef0f05 | pending (unit had no Agent tool; orchestrator runs it) |
| M2 B (conformance) | 1 | e12a747f740c0ac99c3bd5caf92c4a4a84d04d4a:00fb53299583fcd03c4a0bc131e18d39d1ef0f05 | pending |
| M3 A (code-review high) | 1 | 3ca84763c (WIP) | CHANGES (run by the orchestrator): 4 MAJOR, 2 MINOR, all verified. F1 open file dropping out of a background read switched to NO_LONGER_CHANGED on its own → `OpenView.chose_gone` (set only when the user selects the file, or the read their selection started finds it gone; `stale::note_selection`), test `the_open_file_leaving_a_background_read_keeps_its_diff_until_selected` (red seen). F2 re-selecting the open dotted file did nothing → `FileSelected` re-reads it keeping the old diff, test `selecting_the_open_dotted_file_reads_its_latest_diff` (red seen). F3 ReadDiff job read the row after the diff → row read first (no seam for a test: off-thread closure over real git). F4 adopted PR file recorded the old head → `PullRequestView.adopted` + `stale::head_of` for the anchor's `head_commit` and the head-version read, test `a_comment_on_an_adopted_file_records_the_head_it_was_read_at` (red seen). F5 watch read ahead of a queued user read was applied → `OpenView.apply_queued`, test `a_watch_read_ahead_of_the_users_queued_read_is_not_shown` (red seen). F6 clones: fixed in part (see Declined). |
| M3 visual pass (T025) | 1 | M3 tree, debug build | PASS (unit, cloud Xvfb, 1280x800, light and dark): §A step 2 held state, re-selecting the open dotted file (A F2), notice applied. Observations: base line ellipsised while the notice shows; #1271's dot close to the `−2` count. Evidence `visual-pass/` and /mnt/project-files/issue-1275/. |
| M3 A (code-review high) | 2 | db7d27eba4cb67716924927acc1878a3d8749dfc:04e4a0496d37e182a3281b8e4dcf04ea932c504b | pending (orchestrator). Fix diff: `git diff b65c8c8ea..04e4a0496` (b65c8c8ea is 3ca84763c reworded, same tree). Full gate (`scripts/gate.sh`) green on 04e4a0496's code. |

Plan: written 2026-10-24 (plan.md, research.md R1–R11, data-model.md, contracts/core-api.md,
changes-view-hold.md, quick-comment.md, quickstart.md). Milestones proposed: M1 core stale tracking
(identity + `review::stale`), M2 core drag/chips/setting, M3 hold + notice, M4 drag, M5 chips +
visual pass, M6 FR-013. Only M1/M2 (micold-core) build now; M3–M5 touch the Changes/Review view and
wait until #1269 (Review tab, reshapes the view) and #1271 are merged; M6 needs #1271.

Tasks: written 2026-10-24 (T001–T047, 6 milestones + T047 Polish left to the close unit). Milestones
follow the plan's layer split, not milestones.md rule 1 (M1 = core only, no observable UI): the
orchestrator gated every client part on #1269/#1271, so the core milestones' deliverable is
developer-observable (tests) with `docs-not-needed`. M3 has 11 tasks, kept whole (rule 3): hold
without notice/dot/selection rules would leave invisible held changes on main (rule 6), and the PR
hold shares the apply path. Each client milestone carries its own docs, snapshot and 1280×800
two-theme visual pass (issue acceptance). Plan fixed: `ToggleChip` lives in
`crates/micold-material/src/toggle_chip.rs` (plan named `ui/material/`), and `ChangedDot` goes in
`micold-material` drawn with `unread_mark`'s `filled_circle`. M2 T014 adds a `Settings` field the
client's `SettingsDraft::into_settings` literal must set (stopgap until M5). `speckit-analyze`: the Skill
tool says Unknown skill, so its SKILL.md steps were run by hand on the feature dir (tasks review
round 1 F2): no analyze hooks in `.specify/extensions.yml`; no placeholders; 0 CRITICAL, 0 HIGH.
Coverage: all 30 FRs and 6 SCs map to tasks (11 FRs only through milestone *Satisfies* ranges:
FR-006/007/008/010/011 → T015–T022, FR-016/017/019/020 → T009/T010/T027/T029/T030, FR-023/025 →
T036/T039/T040). One MEDIUM fixed (Constitution I, logic in tested core/pure code): T019's
change-list-to-`DiffRow` mapping in the shell job had no test; now a pure `DiffRow::from_read` in
`features/changes.rs` with a unit test in T016. LOW, left: T030's subscription and T022/T031/T040
glue have no own test (glue, covered by the message tests).

Checklists: `checklists/requirements.md` has no unchecked items (16/16 ticked). Specs-only gate
(`check-criteria-observables.sh`) passed. A full `scripts/gate.sh` was also tried and failed on disk space (container has ~30 GB
free; the workspace build filled it); not needed for a specs-only diff (gate.md), target wiped.

Clarify: round 1 asked FR-014 (escalated; applied provisionally as A, which made the whole-view
items unconditional). Round 2 scan: no critical ambiguities left; clarify CLEAN.

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M3 | A r1 | F6 (MINOR), clones | Partly: `applied` no longer clones the shown PR diff, `list_read` takes the list instead of cloning it, `adopt_pr_selection` checks the set before building anything. Kept: the latest PR diff is still cloned on adoption (`Listing::put` places a file new to the shown diff by its neighbours in the latest one, so the whole latest diff is the `fresh` argument) and `hold_pr_refresh` builds the shown `PrDiff` to compare (the view stores files and patches apart). |
| M1 | A r1 | F6, untracked rows | Partly: an untracked row's modes stay `[0, 0]` (`--raw` does not list it); a chmod on an untracked file is not flagged. Its diff shows the mode only in the `new file mode` header; left for a follow-up if wanted. |

## Handover

None.

## Open escalation

None. FR-014 answered by the user on 2026-10-25: **A, whole-view hold** (as applied).

## Follow-ups not done

- Blockers not merged at spec time: #1271 (reviewed marks; only FR-013 needs it), #1269 (Review
  tab), #1270. Mockup not reachable from the cloud container; spec written from
  `docs/development/review-redesign.md`.
- M1 notes (for review): `assemble` marks a working-tree new side with no id as
  `VersionId::Unknown`, which `git::identify` fills (hash, `Stat`, or `Absent` when missing; a
  failed `hash-object` falls back to `Stat`). I6 counts `hash-object` by re-running the test binary
  in a child with `GIT_TRACE` set, so parallel tests do not write to the trace. TDD red shown by
  mutation (no `identify`, no `--no-abbrev`: I1a, I3–I6 fail); the unit lacked the Agent tool, so
  reviews A/B did not run in it.
- M1 review snapshot (code commit, gate green: `scripts/gate.sh` exit 0):
  `4a00118fcb2ed7417b603fca94e6e7a6b6a4c739:19a082ccf3ed3810272518621f9be51459286dbc` (reviews A
  and B round 1 ran on it).
- M1 review A round 1 fixes: commit 31cc0ed7, full `scripts/gate.sh` exit 0 on that tree; fix
  snapshot for round 2 (`review-snapshot.sh diff 4a00118fcb2ed7417b603fca94e6e7a6b6a4c739:19a082ccf3ed3810272518621f9be51459286dbc`):
  `56ae08e45f3bf70dfcbd5765e7d3d5f2bdede19c:31cc0ed72894052f56730d1f7c435baa3ce81dfd`.
- M1 PR body (review A round 1 fixes): `$SCRATCHPAD/pr-body-1275-m1.md` of the M1 part-2 unit;
  the orchestrator opens the PR after A round 2.
- M1 rebase onto main 36459b95 (2026-10-25, PR #1380 merge conflicts with #1271 M1: 31f760806,
  df90fcb23, 2473fbb43). The A-r1 fix code is folded into the rebased feat commit (the fix commit
  keeps only its ledger lines). Resolution, #1271 kept as it is: `NameStatus` keeps #1271's
  `mode_changed: bool` and string `old_mode`/`new_mode`/`old_id`/`new_id` (all zeros as git
  printed); ours reads them through `NameStatus::modes()` (octal) and `side_id(&str, absent)`
  (zeros → `Absent`/`Unknown`), so our `modes: [u32; 2]` field and `Option` ids are gone and our
  parse test duplicating #1271's was dropped. `untracked_paths` (#1271) takes our `paths` filter;
  `whole_change` passes `&[]`. One shared `hash_paths` runner now quotes with #1271's
  `stdin_path` and serves both `blob_ids` (#1271, behaviour unchanged) and `identify`.
  Behaviour change in ours: `carried_verbatim` is gone, so a path with a leading `"`, a trailing
  CR or a newline is now hashed (C-quoted) to its `Object` id instead of `Stat`; F4's test became
  `a_path_git_would_unquote_or_trim_gets_its_own_id` (adds a newline path). #1271's test fixture
  `marks::tests::pr_file` gained `identity: ChangeIdentity::Unknown` (new field only).
- Rebased again onto 8f38e9ca, then 8eceb3ad (main moved three times; clean; final `scripts/gate.sh` exit 0): #1274's `navigate::tests::changed` fixture
  gained `identity: ChangeIdentity::Unknown` (commit "give #1274's navigate test row…"). CI on the
  first push (d62d3a31) failed `card_grid::tests::a_link_scrolled_out_of_the_body_is_not_under_the_pointer`
  (overview cards, not in our diff); it passed in all three local gates here, so not ours. A gate
  run also hit `micold-daemon` `attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice`
  once (timing; 3/3 green on rerun, not in our diff).
- Follow-up (design, not done): unify `ChangeIdentity` (per-row identity, feature 1275) with
  #1271's `ChangeId` (SHA-1 of a file's whole change record) and `WholeChange`; both read
  `--raw --no-abbrev` and hash the same working-tree files in separate `hash-object` runs.
- Follow-up (not done): #1271's `disk_mode`/`trusts_file_mode` could give untracked rows real
  modes (the declined A-r1 F6 part); symlinks still take `Stat` in `identify`, where #1271's
  `blob_ids` hashes their target as git stores it.
- M3 red CI again on faedc2bb0 (2026-10-25): main moved to 35ac6de2d and dropped a toolbar child
  (the 103-wide element at x237.9), so `changes-review-held` shifted. Rebased onto 35ac6de2d,
  only `changes-review-held` failed in `layout_snapshot`; re-recorded (15+/20-, toolbar children
  renumbered 3→2, 4→3, 5→4, search box now at x520.9). Ran `layout_snapshot`, `features_changes`,
  `features_pr_changes` and `micold-core --lib review`: green. Pushed, then full `scripts/gate.sh`.
- Main moved again to d84e94860 (#1273 M3 tree state) before the push: rebased; conflicts in
  `features/changes.rs` (imports: kept both `stale::Held` and #1273's `tree::{…}`) and
  `tests/features_changes.rs` (both features append a module: kept #1273's `tree_state` then our
  `hold`; #1273's two `Msg::DiffRead` initialisers gained `row: DiffRow::NotRead`). Layout fixture
  unchanged by that move. Client lib + the three test targets + core `review` green.

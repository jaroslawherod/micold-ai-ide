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
- **Phase**: tasks
- **Next step**: orchestrator opens the design PR (body prepared by the tasks unit) and records its number; after merge, milestone M1. FR-014 is provisional A; if the user picks B or C, reopen clarify (FR-005/FR-008/FR-009/FR-014, US1 AS1, 5, 9–11) and revise plan M3 only (research R2–R4, contracts/changes-view-hold.md)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T008 | full | change identity in every row + `review::stale` (core, tests) | | planned (builds now) |
| M2 | T009–T014 | full | `review::drag`, `review::chips`, `Settings::suggestion_chips` (core, tests) | | planned (builds now) |
| M3 | T015–T025 | full | Review tab holds list+diff; notice, dots, apply; docs, snapshot, visual pass | | gated: #1269 and #1271 merged |
| M4 | T026–T034 | full | drag over line numbers opens composer; docs, visual pass | | gated: #1269 and #1271 merged |
| M5 | T035–T043 | full | suggestion chips + Settings switch; docs, snapshot, visual pass | | gated: #1269 and #1271 merged |
| M6 | T044–T046 | full | reviewed file with held change shows unseen + dot; docs, visual pass | | gated: #1271 merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-014: hold the whole view or only the open diff? | A, whole-view hold (provisional, awaiting user) | orchestrator (recommended; user not yet answered) | docs/development/review-redesign.md item 8 |

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

## Handover

None.

## Open escalation

**Pending user confirmation**: applied provisionally as A (whole-view hold) on the orchestrator's
word; spec.md records it as provisional. Clarify round 1, Q1 (category 1, product decision the repo does not settle): **FR-014 — while
changes are held, is the whole view frozen, or only the open file's diff?**

- **A. Whole-view hold (Recommended)**: the file list, its +/- counts and every file stay one
  snapshot until the notice is pressed; changed files get a dot and count in *n*; selecting a dotted
  file shows its latest diff (US1 AS9–11). Nothing on screen moves. Evidence:
  `docs/development/review-redesign.md` item 8 shows *3 files changed since you looked* with dots on
  the file rows while one diff is open, so *n* must count files other than the open one; a live list
  would insert, drop and re-sort rows (a tree after #1273) under the reader, the same movement the
  item exists to stop; the spec as reviewed (2 rounds) is written for it.
- **B. Live list, held open diff with dots**: list rows and counts update live as today; files that
  changed since the user last saw them get a dot and count in *n*; only the open diff is held until
  the notice (or reselecting) is pressed. Literal minimum of item 8 ("the open diff does not
  reflow"); simpler (no held list, no AS9–11), but rows still move.
- **C. Open diff only, no dots on other files**: the spec's current second answer; *n* is at most 1.
  Contradicts item 8's example of 3 files.

What the repo says: item 8 and the issue only require that the open diff not reflow, and that rows
carry a dot and the header a count of several files; neither says whether the list itself is held.
`docs/user-guide/reviewing-changes.md` *Keeping up with changes*: today the list and diff re-read
together within ~2 s.

## Follow-ups not done

- Blockers not merged at spec time: #1271 (reviewed marks; only FR-013 needs it), #1269 (Review
  tab), #1270. Mockup not reachable from the cloud container; spec written from
  `docs/development/review-redesign.md`.

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
- **Phase**: plan
- **Next step**: tasks unit (plan review round 2 findings fixed, prose only; orchestrator decides on a round 3). FR-014 is provisional A; if the user picks B or C, reopen clarify (FR-005/FR-008/FR-009/FR-014, US1 AS1, 5, 9–11) and revise plan M3 only (research R2–R4, contracts/changes-view-hold.md)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

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

Plan: written 2026-10-24 (plan.md, research.md R1–R11, data-model.md, contracts/core-api.md,
changes-view-hold.md, quick-comment.md, quickstart.md). Milestones proposed: M1 core stale tracking
(identity + `review::stale`), M2 core drag/chips/setting, M3 hold + notice, M4 drag, M5 chips +
visual pass, M6 FR-013. Only M1/M2 (micold-core) build now; M3–M5 touch the Changes/Review view and
wait until #1269 (Review tab, reshapes the view) and #1271 are merged; M6 needs #1271.

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

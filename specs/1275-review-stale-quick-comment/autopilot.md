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
- **Phase**: clarify
- **Next step**: clarify round 1 escalated (FR-014); on the answer, apply it to spec.md (Clarifications + FR-014 + whole-view-conditional items), then run clarify round 2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| spec | 1 | 9e154017e95adc6a613e45c3f02e6da155dc11ba:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | 3 MAJOR, 2 MINOR. F1 fixed: FR-001a per-row change identity (git object ids from `diff --raw`, `hash-object` for working-tree files, size+mtime over limits, PR patch hash), AS12, FR-003, Context/Assumptions. F2 fixed: FR-009 + US1 AS9–11 (select non-held reads as today; held reads latest and leaves set; no-longer-listed says no changes), Key Entities. F3 fixed: whole-view-only MUSTs marked conditional on FR-014. F4 fixed: match by path, rename counts once under new path. F5 fixed: checklist item unticked. |
| spec | 2 | 89c07d79a52e9cb5ae89d0beff62b6ccf424039e:4a06e88388d0b4cd7395ec7ce70882b466871cf9 | CLEAN (sonnet), 1 MINOR: edge case *Selected between reads* not marked conditional on FR-014. Fixed: prefixed "under the whole-view hold (FR-014)" and noted it does not arise under open-diff-only. |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, Q1 (category 1, product decision the repo does not settle): **FR-014 — while
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

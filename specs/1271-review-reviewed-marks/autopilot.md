# Autopilot ledger — #1271 review-reviewed-marks

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on issue #1271 "Review UI: reviewed marks and progress" (blocked by #1269, not merged; spec builds on its design in docs/development/review-redesign.md); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1271
- **Worktree branch**: claude/project-thread-umhor9
- **Started**: 2026-10-24
- **Phase**: M2 implemented, in review
- **Next step**: orchestrator runs M2 review A (high) and B on the snapshot in *Review rounds*; then full gate and PR text

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1360 | design (spec, plan, tasks) | merged | e683d5db1514b59179c91d03dc1b85ed4fc1a049 |
| #1374 | M1 micold-core marks | merged (after it, the orchestrator pushed two CI fixes: cf4073cf `as_chunks` for clippy 1.99; 59ff54bf quoted-path test `#[cfg(unix)]`, Windows refuses `"` in file names) | 96b90dd65dc6cd9920f111dba94c460b2a414c26 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | micold-core: change ids, whole-change read, EntryMarks, states, progress, review file v4 | #1374 | merged |
| M2 | T018–T024 | full | wire SetReviewed + ReviewChanged.marks, service keeps/pushes/removes marks, client holds them; no view change (docs-not-needed) | — | implemented, in review |
| M3 | T025–T034 | full | MVP: row states, Mark reviewed control, n of m reviewed + bar, user guide | — | waits for #1269 |
| M4 | T035–T039 | light | changed-since dot, other windows' marks in the view, guide text | — | planned |
| M5 | T040–T044 | full | pull request marks and progress, guide text | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Reset marks on every new head commit, or per file's change? | Per file's change (a mark holds while the file's whole change is the same) | agent-resolved | spec.md#Assumptions |
| D2 | spec | Row symbol for a marked file with drafts | Reviewed wins; draft count stays on the row | agent-resolved | spec.md#Assumptions |
| D3 | clarify | Header wording: *n of m reviewed* or *n/m reviewed*? | *n of m reviewed* | agent-resolved | issue #1271 body (spec.md#Input) |
| D4 | clarify | Mark a file while a toggle hides part of its change? | A: no — Mark reviewed unavailable (tooltip) for a file with changes of a hidden kind; unmark stays available. Applied to FR-002, FR-005, Edge Cases > Toggles | provisional, pending user | spec.md US2 rationale, FR-005; Open escalation Q1 |
| D5 | tasks | Milestone cut vs plan's proposal | M1, M2 as planned; `pr_change_ids` moved to M5 (US4); plan's M3 split by story into M3 (US1), M4 (US2 + US3 view), M5 (US4); guide text and snapshot states in each story's milestone; visual pass and redesign note to the close unit. M1 (17 tasks, all of micold-core, likely >800 lines) kept whole: core-only, no observable split along a scenario, and it must land before #1269. Plan fixed: `Icon` lives in `crates/micold-material/src/icons.rs`, not micold-client. milestones.md rule 1 waived: M1 and M2 are Foundational alone and carry no P1 story, because US1's view is the one #1269 rewrites (research R8); their items are pub in lib crates (no dead code) and they are checked by tests only (each milestone's Verify). `mark_block` moved from plan M1 to M3 (T025, T026), with the control it gates | agent-resolved | tasks.md#Milestones, milestones.md rules 1, 3, 4, research R8 |
| D6 | implement M1 | Where the plan's text does not fit the code | (1) `ChangeId::of_worktree` digests the resolved `NameStatus` record (kind, path, both modes and ids), since `WholeFile` holds only the resulting id; (2) `WholeChange::assemble` takes `committed: Option<&set>` (None = no base, all Uncommitted) and `untracked: Vec<(RelPath, mode)>`, the mode read from disk; (3) `--no-abbrev`, not `--full-index`: git 2.43 abbreviates `--raw` ids under `--full-index` (contract G2, research R2, tasks fixed); (4) a symlink's blob id is hashed in core over its target path (hash-object follows the link); a submodule's unhashed side gets no id; (5) the daemon's two `ReviewFile` literals get `marks: Default::default()` to compile, M2 fills them | agent-resolved | contracts/marks-wire.md G2; tests/review_git.rs G2 tests |
| D7 | implement M2 | Where the plan's text does not fit the code | (1) an unknown entry's `SetReviewed` is `NotFound`, as every review edit's (W2 in `check_entry_locked`), not `InvalidInput` as RM1's list says: RM1 cites W2, whose answer is `NotFound`; (2) the daemon's interim per-project marks map from M1 is gone: marks load into `EntryReview::from_parts` (a marks-only entry is an entry) and every save builds the file from the entries, so one place holds them; (3) RM5's send and report are tested on `Reviews` with a real store in review_edit.rs (send pushes and the file), plus one end-to-end delivered send in review_send.rs | agent-resolved | contracts/marks-wire.md RM1, RM5; review-wire W2 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 5cce47b02a0a2a5998d722bd75a1292810c331b6:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 1 MAJOR, 3 MINOR (run by the orchestrator). F1 fixed: FR-016 keeps a mark until unmarked/re-marked/entry removed, edge case and US2 scenario 6 added; F2 fixed (prose): US2 s5 limited to entries with a base; F3 fixed: older-window edge case; F4 fixed: US4 s2, PR marks kept like PR comments |
| Spec | 2 | ebf5c8ada3cf70b15771dbcb49161fbc5e7fd27f:029348b9eeae8148929f5bcf846f762b40db03b6 | CLEAN (model sonnet, scoped re-review of F1, F3, F4) |
| Clarify | 1 | b4b18fc6 | 2 questions: D3 agent-resolved, Q1 escalated, applied provisionally as A (D4) |
| Clarify | 2 | after D4 applied | No critical ambiguities (US1 s7 added to test D4) |
| Plan | 1 | 94d6338cedb4a1484792ddd227b4525777910dda:eff5ce1c9241292547074d7c78439557f5f162c8 | CHANGES: 2 MAJOR, 2 MINOR (run by the orchestrator). F1 fixed: untracked file digested as git's added file (no `U` tag), two G2 tests added (R1, data-model, contract G2); F2 fixed: `PendingOp::ReviewMark` moved to M3 with its sender; F3 fixed (prose): M2's edit of the `Msg::ReviewChanged` arm and the expected trivial conflict stated (plan, R8); F4 fixed (prose): M1 depends on D4 answered (A or B) |
| Plan | 2 | 4898b5511d4f5bfda0a94ab413c3e0ee3b0e07e7:1fffd3bd01fcc90f2a112ade02d6051e1800f7dc | CLEAN (model sonnet, run by the orchestrator, fix diff from round 1) |
| Tasks | 1 | 9585ec9b5c4444f1fb0f85c4ee8c95e4644fe2fd:2742a52748cad1ffbc15c420079a08ded6708f7c | CHANGES: 1 MAJOR, 3 MINOR (run by the orchestrator). F1 fixed, prose only: `[P]` removed from T004, T008, T012 (share marks.rs) and T021 (shares review_edit.rs), Dependencies and parallel example say so; F2 fixed, prose only: D5 records the rule 1 waiver (#1269, R8, tests only) and `mark_block` moved to M3; F3 fixed, prose only: D4 note names T025, T026, T027 s7, T028's `HiddenKind` arm; F4 fixed (MINOR, no new round): T029 starts with a failing daemon_sync test that a failed `ReviewMark` notifies and leaves state unchanged. Review done. Fix snapshot 44b93dad2fc8b2cfa901b4f068ffd6325655fcf7:7ec6e4fc0d4c9837d50a6bfc26ff768d1af15a11. speckit-analyze is not invocable here; FR/SC coverage checked by grep |
| M1 diff | 1 | d666189b8e5357d37344c06f55639874bdebb5c7:3f3d70abe73089804fe8fe57960c022bef086b75 | A (high): CHANGES, 2 MAJOR, 2 MINOR. F1 fixed: `hash-object --stdin-paths` input C-quoted for a path starting with `"` or holding CR/LF (newline paths now hashed too), test with a quoted path; F2 fixed in part: `core.fileMode=false` gives an untracked file 100644, G2 test; the `core.symlinks=false` half declined (see below); F3 fixed: last entry wins for marks too, store test; F4 fixed: the daemon carries the file's marks through `Reviews` and writes them back unchanged, review_edit test. B (conformance, sonnet): CLEAN; Verify `review::` 218 passed 1 ignored, `review_git` 32 passed; MINORs: daemon `marks: Default::default()` (gone with F4), no tdd/cycle-log.md (not fixed) |
| M1 diff | 2 | 91ddebc0bca0429a7d1369d72a451490b061fdbc:96b101ead03a0b38041050f7cd063eeb05b83e8d | A (sonnet, fix diff): CLEAN; F1–F4 confirmed, core.symlinks decline stands. 1 MINOR fixed (no new round): `forget_worktree`/`prune_worktrees` also drop the dir's marks and treat a marks-only entry as present, review_edit test. Review done. Scoped gate green before; 4 core permission tests fail as root on origin/main too: settings_refuses_save_over_failed_read, settings_write_is_logged, worktree_leftovers x2 |
| M2 diff | 1 | 24d097d8054d8a9489db208736b8c4bdeced7a84:f8b443698b06216bf66c3a26ded840b2d5ba617e | due: A (high) and B (conformance, sonnet), run by the orchestrator. Scoped gate before: fmt, clippy (core, workspace) and clippy 1.99 (core, daemon, client) clean; `cargo test --workspace --no-fail-fast` fails only the six root_only permission tests of scripts/gate.sh (as root here, also on main) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | F2 second half: with `core.symlinks=false` digest a symlink as 100644, hashed as a file | `git add` of a real symlink still records `120000` over its target path with `core.symlinks=false` (checked: git 2.43, `ln -s ex ln; git add ln; git ls-files -s` → 120000); the setting only changes checkout |

## Handover

M2 implemented (T018–T024 ticked), committed at f8b44369 and pushed; scoped gate clean (see *Review rounds* M2 diff 1). Next: the orchestrator runs review A (high) and B on snapshot 24d097d8…:f8b44369 and sends findings; then fix, full gate (`scripts/gate.sh`, as root it reruns root_only unprivileged) and the PR text in scratchpad pr-m2.md. Deviations: D7. Disk is tight here (~39 GB cap): build with `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`, a full-debug workspace test build filled the disk once.

## Open escalation

Clarify round 1, category 1 (product decision the repo does not settle). With the user; A applied provisionally (D4). If the user picks B or C, revise FR-002, FR-005, Edge Cases > Toggles and the Clarifications bullet.

- **Q1**: When a Committed or Uncommitted toggle hides part of a file's change, may the user still
  mark that file reviewed? FR-005 judges a mark on the whole file's change, so a mark made while
  only the committed part is shown would also cover uncommitted lines the user never saw — the
  "claims a review that did not happen" that US2's rationale rules out.
  - **A (Recommended)**: The mark control is unavailable, with a tooltip "Turn on Committed and
    Uncommitted to see this file's whole change", for a file that has changes of a hidden kind;
    files with no hidden part can be marked as usual. Evidence: spec.md US2 "Why this priority",
    FR-005, Edge Cases > Toggles.
  - **B**: Marking is always available; the mark covers the whole change, including the part the
    toggle hides.
  - **C**: Marking is always available, but the mark records only the shown part and covers the
    file only while the same toggles are on (changes FR-005).

## Follow-ups not done

None yet.

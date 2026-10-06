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
- **Next step**: milestone M1 (T001–T026)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #624 | whole branch (design + every milestone), opened by the orchestrator | draft | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T026 | full | Changes view with the changed-file list (MVP) | #624 | in progress |
| M2 | T027–T040 | full | Unified diff of the selected file | — | todo |
| M3 | T041–T049 | full | Side-by-side layout, syntax colouring, kept layout | — | todo |
| M4 | T050–T063 | full | Comments on lines and ranges | — | todo |
| M5 | T064–T075 | full | Send comments to the running session | — | todo |
| M6 | T076–T080 | full | Send when no session is running | — | todo |
| M7 | T081–T089 | full | Live refresh and outdated comments | — | todo |
| M8 | T090–T097 | full | Clear, discard, and removal with the worktree | — | todo |

## Decisions

- Clarify round 1: US3 scenario 4 and FR-003 answered by the orchestrator as defaults (user away); recorded in spec.md Clarifications. No further ambiguities, no second round.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

- Tasks: US1 split in three (list / unified diff / layouts+colour), US2 in two (comments / send), US4 in two (refresh+outdated / tidy), along acceptance scenarios. M1 stays at 26 tasks because Setup + Foundational (core value types, protocol v30 in one edit — a second bump would fail schema_hash.rs) must ride with it. Polish T098–T100 changes no code: close unit.
- Gate (tasks unit): specs-only, `scripts/check-criteria-observables.sh` passed (spec has no criteria table). Design PR: none of its own — this run ships one PR for the whole branch (orchestrator opens it); PR body written to the session scratchpad pr-body-482.md.
- speckit-analyze (0 CRITICAL, 2 HIGH, 6 MEDIUM, 5 LOW): fixed in tasks.md (D1 watch::Debouncer + cap_spans as tested pure functions, D2 two-session test moved to T066, D3–D7, D12); spec.md edits (D8 no-bracketed-paste edge, D3 worktree removed outside the app, D10 FR-016 wording, D13 toggles reset on open) and plan.md Constitution row I (D1). D9, D11 left: wording only.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c5e15968c86ce0c17544cde68a31ff74e2fb1e8b:107ab7ed3aa9b067a4dca5c02aca6e6709d4d158 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | c960571fb7bed1fa440f89fe3b9720fd30c8d897:c74e2d4a9e09412fc66dd4915e3c7e105b785552 | CLEAN: 2 MINOR (both fixed) |
| Plan | 1 | 76b23838a33fc28d5c9f6db389cc15f46acd39a1:3a5ac98c80a82e8c74fb02606f5bee99309eaeeb | CLEAN: 3 MINOR (all fixed, prose only) |
| Tasks | 1 | bdc7eb7285b7355f190266e00dd32362649c017c:a284e9b2ea58bc2c731608a1600a913ad9b91dec | CLEAN: 3 MINOR (all fixed: checklist ticked, T067 outdated list deferred to M7, M1 Verify + SC-001/SC-005 mapped) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 1 handed over at 155k context (it spent its budget reading the design). State, all in the
WIP commit `wip(482): M1 …` on this branch (not pushed; no gate has seen it, nothing compiled yet):

- **Done (not ticked, not built):** T001 (iced `highlighter` feature in workspace `Cargo.toml`,
  `notify` in `crates/micold-client/Cargo.toml`; there is **no `deny.toml`** in the repo, so the
  licence check is moot; `cargo tree -p micold-client -i onig_sys` still to run once Cargo.lock
  updates). T002 (module tree `crates/micold-core/src/review/`, `pub mod review;`, `local_only` /
  `run_git` now `pub(crate)`).
- **Red tests written against compiling-by-intent stubs (red not yet run/recorded):** T003
  (`review/mod.rs`: `RelPath`, `Side`, `LineRange`, `limits` — stubs return wrong answers),
  T005 (`review/comment.rs`: stub derives lack `rename_all`, `#[serde(flatten)]` on `range`, and
  the `{"pending": null}` shape — implement via a `Pending(())` repr enum with `#[serde(from/into)]`;
  `CommentId::new` stub returns nil), T011 (`review/base.rs`), T012 (`review/changes.rs`), T013
  (`crates/micold-core/tests/review_git.rs`, `review/git.rs` stub `GitCli::review_base` /
  `change_list(dir, scope, toggles)`).
- **Design decisions taken in the stubs:** `ChangeList { files, scope: ReviewScope }` (scope, not
  `base`, so the Default entry has no base); kinds parsed from `git diff -z -M --raw`, not
  `--name-status --summary` (git 2.43 prints no summary beside `--name-status`; `--raw` carries
  both modes, so mode-only = modes differ and numstat `0 0`) — the parser keeps the task's name
  `parse_name_status`; pure helpers `assemble(names, stats, committed_set, uncommitted_set,
  fallback_origin)`, `merge_untracked(files, Vec<Untracked>)`, `content_of(bytes)`,
  `classify(&mut ChangedFile, VersionSizes)`; `default_branch_from` returns `origin/<b>`, `main`
  or `master`; `DiffRange::for_view` with no base and uncommitted off gives `None` (committed only
  without a base lists nothing; the view shows the reason). Origin with both toggles = membership
  in the separate committed (`base..HEAD`) and uncommitted (`HEAD`→worktree + untracked) path sets.
- **TDD log:** no `tdd/test-list.md` (as 575/582): the test-first task pairs are the list; record
  reds in `tdd/cycle-log.md` as `specs/575-workspace-attention-list/tdd/cycle-log.md` does.
- **Next step:** build `cargo test -p micold-core --lib review` + `--test review_git`, record the
  reds, implement T004/T006/T016/T017/T018 to green; then T007–T010 (protocol v30 + daemon
  `diff_layout`; every `DaemonSettings`/`SettingsSet` literal across crates needs the new field —
  `grep -rn pr_status_enabled crates`), then client T014/T015/T019–T024, docs T025, visual T026,
  then verify.md. A baseline `cargo test --workspace` was started detached at handover
  (log in the session scratchpad `baseline.log`) to warm `target-shared/`.
- **Container:** no `mise` (run the gate's raw commands), `gh` unauthenticated, PR #624 is the one
  branch PR; review via the scratchpad `run-review.sh`.


## Open escalation

None.

## Follow-ups not done

None yet.

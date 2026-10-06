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

M1 unit 2 handed over at 150k. Commits on the branch (not pushed; no gate yet): `092451fe`
(review core) and the `wip(482)` commit after it (wire delta).
- **Done, green, not yet ticked in tasks.md:** T001–T006, T011–T013, T016–T018
  (`cargo test -p micold-core --lib --test review_git` ok; reds recorded in
  `tdd/cycle-log.md`). T001's `cargo tree -p micold-client -i onig_sys` still to run.
- **T007/T008 nearly done:** tests written (red recorded as compile errors: no `ReviewEdit`,
  `DiffLayout` …; add the row to cycle-log), wire delta made (messages.rs, settings.rs
  `DiffLayout`, version 30, schema pin 30), every literal across crates fixed, workspace
  `cargo check --all-targets` clean. **One failure left:**
  `protocol_roundtrip::the_review_messages_and_the_diff_layout_round_trip_on_both_wires` —
  almost certainly postcard refusing `#[serde(flatten)] range` on `ReviewComment` (postcard has
  no `deserialize_any`). Fix: drop `flatten`, give `ReviewComment` a hand-written
  Serialize/Deserialize via a `Stored { id, path, side, start, end, quote, text, state, created }`
  repr (`#[serde(try_from/into)]`), keeping comment.rs's JSON-shape tests green.
  `settings_roundtrip` and `schema_hash` pass.
- **T010 half done:** daemon serves `diff_layout` (catalog `set_diff_layout`, state, server
  `SettingsSet` arm) and `ReviewEdit`/`ReviewSend` answer the Refused placeholder; client
  persist keeps the stored `diff_layout`. **T009 test not written yet** (write it modelled on
  `crates/micold-daemon/tests/pr_status_setting.rs`; it will pass at once: record as test-after).
- **Next:** fix the postcard failure, T009, then client T014/T015/T019–T024, docs T025,
  visual T026, then verify.md. Scratchpad helpers: `run.sh <log> <cmd…>` (detached, build
  lock, CARGO_INCREMENTAL=0) + `hold.sh`; `fixlit.py` fixes missing-field literals from a
  `--message-format=short` check log.

## Open escalation

None.

## Follow-ups not done

None yet.

# Autopilot ledger — #1723 daemon-qualified-refs-routed-envelope

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1723 (issue #1723 "contract: daemon-qualified refs and routed envelope (protocol bump)", part of #1722; plan specs/orchestrator/); labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #1723
- **Worktree branch**: feat/1723_contract-daemon-qualified-refs-and-routed-envelope
- **Started**: 2026-11-05
- **Phase**: review (M2)
- **Next step**: M2 CI, merge

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1746 | design | merged | 3530aeba08a1047f1d73cc2f88bf5d4b85d8253b |
| #1753 | M1 | merged | 045d2f75cbc3c4098a1a489be95f3cabf2ecdb2d |
| #PRNUM | M2 | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012, T020–T022 | full | daemon id, qualified session/worktree rows, protocol 78 + hash, refusal names both versions | #1753 | merged |
| M2 | T013–T019 | full | Kind::Routed envelope frame, rejection rules | | in progress |
| M3 | T023–T029 | full | project identity on project rows | | pending |
| M4 | T030–T032 | full | contract file, docs/daemon.md, contract_example test | | pending |

## Decisions

- tasks: US3 (bump + refusal test) folded into M1 so main never carries a changed wire under 77 (analyze I1). US1 split: project row + identity moved to M3 (US4 acceptance 2) to keep M1 small. No Polish milestone: quickstart results and ticks go to the close unit. Analyze C1: `RoutedEnvelope::owns` helper added to T013/T014. No separate tdd test-list (optional hook skipped); plan Test strategy is the list.

- FR-008 project identity: normalised remote URL with per-project override; no remote = own project per daemon _(decided by user)_. Override name/location deferred to the plan.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

- tasks round 1 (1 MAJOR Verify commands not runnable, fixed; 1 MINOR 78-spans-milestones, noted in tasks.md): snapshot 3fc73845c9128614ba4e82d44a9e445007140b5f:d4e3967c4d6d34066e3a7c96f63cb69942688d7b

- plan round 1 (2 MAJOR, fixed in plan.md/research.md R8/contract; fixed); round 2 CLEAN (1 MINOR wording fixed): snapshot ce0b24a494d4883155b2267618bc0ebdc5e4fdb5:0dbeec202ce5bd41eef5b88e8730935b12bf1e78

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| M1 A (code-review high) | 1 | a5e3ecbe92ecefe6042eb4216ef33aeac508537d:78438d64861b2f818d68333ba837d352d8bee242 | 8 findings, 2 fixed (daemon.id restart key + test, stale doc), 6 declined |
| M2 A (autopilot-reviewer, correctness) | 1 | fca14197d (committed tree) | CLEAN; 1 MINOR fixed (InvalidTarget names the JSON type, not the value) |
| M2 B (conformance, sonnet) | 1 | same | CLEAN; 3 MINOR: non-JSON routed test added; header list test and cycle-log not added |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | set_daemon_id order-dependent | design: id is read once at startup via set_config, rows before it do not exist |
| M1 | A | default `local` collides across daemons | spec: absent id means `local`; orchestrator config names ids (later feature) |
| M1 | A | qualified()/ProjectRef unused | consumed by M2/M3 and the orchestrator |
| M1 | A | routing.rs tests hashed | same as messages.rs; hash input is by design (T021) |
| M1 | A | grammar in three places | MINOR taste |
| M1 | A | new test executable | T010 names unwrapped_client |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- specs/orchestrator/ (spec.md, issues.md) exists only on branch docs/orchestrator-and-web-client-plan, not on main; contracts/ under it must be created by this feature's PR.

## Review notes (spec, round 1)

Round 1 snapshot 0a3a5600:11174021: CHANGES 3 MAJOR. F1 declined: the FR-008 marker is left for the clarify unit by design (spec.md task). F2, F3 fixed (scenario, SC, row-type check, edge cases); F4-F6 fixed. Re-check by me against findings; no new round needed beyond the marker.

## CI notes

- PR #1753 attempt 1: the only red test was `streaming_child::unix::a_natural_exit_still_ends_what_it_left_running` (grandchild outlived the run; file untouched by this feature, passes on main). Treated as a flake; re-run by a docs-only push. The earlier local `schema_hash` failure did not reproduce in CI.
- PR #1753 attempt 2: rebased onto origin/main (one conflict, daemon_config/tests.rs, kept both tests); mise check and fmt green; targeted tests starved by the shared build lock, CI validates. Now MERGEABLE.
- PR M2: scoped gate: fmt, clippy and tests green except `ai_cli_config::a_configured_model_is_claude_codes_default_launch_model` (fails 2 of 2 locally: stand-in never records its launch in 5 s; file and area untouched by this milestone, probably environmental). Full gate left to CI.

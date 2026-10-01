# Autopilot ledger — 037-explain-hidden-cli

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: continue with next issue use autopilot
- **Kind**: feature
- **Issue**: #434
- **Worktree branch**: fix/github-issues
- **Started**: 2026-10-01
- **Phase**: 4-milestone (M1)
- **Next step**: Continue M1 from the Handover section (T005 next).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #515 | Spec | merged | 063fa77affbcbcc20a842bffb71dce22212582d6 |
| #517 | Design | merged | 976b0220d342f1e35608286f0bdec4a77b8ce247 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017, T038–T039 | full | The Settings note under Default AI CLI (and Image reference) gives the reason for the home directory's environment state and the action; the availability answer carries the state (protocol 18); user guide updated | | in progress |
| M2 | T018–T027, T040–T042 | full | The missing-default message, a start or restart failure and the reply to an AI session's `create_session` give the same reason and action; user guide updated | | pending |
| M3 | T028–T035, T043–T044 | full | A row's CLI list with two or more CLIs shows a non-pressable note naming the CLIs not offered, with reason and action; showcase entry; user guide updated | | pending |
| M4 | T036–T037 | docs | `evidence/README.md` records quickstart Part A, all of Part B and the wording cross-check | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Is #434 a bug against a Closed spec, or new behaviour? | New behaviour, own spec. 027 FR-023b already names a missing CLI under Default AI CLI, but no closed spec requires a reason or an action. | agent-resolved | specs/027-sandboxed-daemon-runtime/spec.md#FR-023b; specs/029-pi-cli-provider/bugs/BUG-001.md#spec.md ("no requirement that a hidden CLI be explained") |
| D2 | spec | Feature number | 037. 036 is taken by `036-tooltip-follow-cursor-delay` in the `fix-issue-430` worktree. | agent-resolved | `ls <worktree>/specs/03[6-9]*` on 2026-10-01 |
| D3 | spec | Do the environment reasons apply under container placement? | Yes. Environment-include applies where sessions run under both placements, so "the image lacks it" is true only when the environment resolved in full. Refines 027 FR-023b. | agent-resolved | specs/029-pi-cli-provider/spec.md#FR-003b; Spec review round 1, F5 |
| D4 | spec | Is the reply an AI session gets for a missing CLI in scope? | Yes (FR-009a). It says "is not installed", the claim FR-002 forbids. | agent-resolved | crates/micold-daemon/src/mcp/tools.rs:470; Spec review round 1, F4 |
| D5 | clarify 1 | On a row with fewer than two available CLIs (no chevron), where is a CLI that is not offered explained? | Option B: not at the row. The chevron rule stays. The reason appears only in a row list that already opens (two or more available, another missing). A one-CLI row is unchanged; Settings (Story 1) and the start and restart messages (Story 2) serve that user. No closed spec is amended. Options A (always show the chevron), D (show it only when the environment was not applied) and C (drop Story 3) were declined. | decided by user | specs/026-multi-provider-sessions/spec.md#FR-006; specs/033-directory-aware-start-affordance/spec.md#Clarifications; issue #434 |
| D6 | clarify 2 | Does the list a row with fewer than two available CLIs opens for a missing default (033 FR-010) name the CLIs not offered? | No. FR-010 applies only with two or more available. The FR-008 message carries the reason there; with nothing available the FR-009 failure does. | agent-resolved | D5; specs/033-directory-aware-start-affordance/spec.md#FR-010; crates/micold-client/src/ui/sidebar.rs#start_press |
| D7 | clarify 2 | Which reason does a row give while it is drawn from the home answer (033 FR-005)? | The home answer's: offer and reason come from one answer (FR-012), and both switch when the row's own arrives. FR-011's silence is for no answer in use. | agent-resolved | specs/033-directory-aware-start-affordance/spec.md#FR-005; crates/micold-client/src/features/session.rs#AvailabilityAnswers::for_dir |
| D8 | clarify 2 | Do SC-003 and SC-004 count the row's CLI list? | Yes. FR-001 covers FR-006 to FR-010; both criteria now name it. | agent-resolved | specs/037-explain-hidden-cli/spec.md#FR-001 |
| D9 | clarify 3 | Does a row's CLI list give the action as well as the reason? | Yes. FR-001 pairs each reason with its action and SC-003 already counts the list. FR-010, Story 3 scenario 1 and SC-007 now say "reason and action". | agent-resolved | specs/037-explain-hidden-cli/spec.md#FR-001; specs/037-explain-hidden-cli/spec.md#FR-003 |
| D10 | clarify 3 | Does a message already shown at an event (missing-default message, start failure, AI-session reply) change when a newer answer arrives? | No. It states the reason that held at its event. Only the Settings note and an open row list follow a newer answer. A start failure's reason comes from the environment that start resolved, the resolution the answer shares. Corrects the round-2 edge case that said the message "follows". | agent-resolved | crates/micold-client/src/features/session.rs#start_menu_toggled; specs/029-pi-cli-provider/spec.md#FR-003b; crates/micold-daemon/src/state.rs#ai_clis_available_in |
| D11 | clarify 3 | How does a reason stay true when it reports the home directory's attempt on a project row (D7)? | New FR-004a: a reason that reports an attempt (last four states) names the directory the attempt was for. The two settings states name none. | agent-resolved | specs/037-explain-hidden-cli/spec.md#User Story 1 (scenario 3); specs/037-explain-hidden-cli/spec.md#Edge Cases |
| D12 | clarify 4 | FR-004a makes the "last attempt succeeded" reason name a directory; FR-005 and Story 2 scenario 4 keep 027's container sentence unchanged. Which holds under container placement? | FR-005. The container sentence names the CLI and the image and no directory, in Settings and at a failed start. FR-004a covers the three failed-attempt states under both placements and the last state on the host only. Story 1 scenario 5 now names the home directory. | agent-resolved | specs/037-explain-hidden-cli/spec.md#FR-005; specs/027-sandboxed-daemon-runtime/spec.md#FR-023b; crates/micold-daemon/src/state.rs#missing_cli_reason; crates/micold-client/src/features/settings.rs#missing_cli_notice |
| D13 | design | FR-009 said a start failure must not tell the user to install the CLI in the first five states, but FR-001's table gives "or install the CLI on the login PATH" as part of the action in the first two, and FR-012 requires every surface to give the same action. Which holds? | FR-001 and SC-003. FR-009 now says installing is never the only action in those states and is not named at all in the three failed-attempt states. Story 2 scenario 2 (a failed-attempt state) stays true. | agent-resolved | specs/037-explain-hidden-cli/spec.md#FR-001; specs/037-explain-hidden-cli/spec.md#SC-003; specs/037-explain-hidden-cli/research.md#R7 |
| D14 | design | FR-001's third state read "script path names no readable file", but the resolver reports a missing script only when nothing exists at the path; a path that exists and cannot be sourced is attempted and fails. Which state is an unreadable path? | The fourth ("exited with an error"), which is what the environment-include group shows for it and what Story 1 scenario 4 requires ("in the same terms the environment-include group uses"). FR-001 row 3 and scenario 4 now say "no file", and FR-001 states the rule. No probe of the path is added (FR-014). | agent-resolved | crates/micold-core/src/env_include.rs:304; specs/037-explain-hidden-cli/research.md#R2; Plan review round 1, F1 |
| D15 | design | M1 (Setup + Foundational + US1) is 19 tasks, over the split guide of about 15. Split it? | No. All six states come from one `classify` and one `explain`, so no acceptance scenario of Story 1 is a deliverable without the whole Foundational phase and the note. | agent-resolved | .claude/skills/speckit-autopilot/references/milestones.md (rule 3); specs/037-explain-hidden-cli/tasks.md#Notes |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 39923bcd771235ee0060a1f1f6330830a153df7d:65fbe60069a3f9b434d1550233c450000099604e | CHANGES: 5 MAJOR, 3 MINOR — all fixed |
| Spec | 2 | b460803fa1576b80357bd110538b491691e8f6c3:65fbe60069a3f9b434d1550233c450000099604e | CLEAN: 1 MINOR, fixed |
| Plan | 1 | 36e2eb237df3c0946d4477309215238afed6789b:c1fb57e37f3409153c7d001c0e6d5b95c88124d8 | CHANGES: 2 MAJOR, 3 MINOR — all fixed |
| Plan | 2 | 91d8ac1c552b4aebf2935d2dbf27da809322cb24:f490b395f55a537ab5a1f99fea08cb1c9e5afb80 | CLEAN: 1 MINOR, fixed |
| Tasks | 1 | 6a70370da6ba5cd5650bcbdaf9f8760c347ac6da:fff351d523d61045b0b4faeabc1344331029a419 | CHANGES: 1 MAJOR, 3 MINOR — all fixed |
| Tasks | 2 | 0d9750169504fffafc0a6b973cfd025dbd58bcc9:1bc17b8d4e28e6454eec2393d3b2519435f6fb96 | CLEAN: no findings (scoped re-review of the round-1 fix diff) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1, first unit, stopped at the context cap. No PR is open. Nothing is pushed. Branch `fix/github-issues`
was reset to `origin/main` (`976b0220`) by `branch-start.sh 517`: do not run it again.

**Done and committed** (core subset 1475 passed, `cargo check --workspace --all-targets` clean, `cargo fmt` run):

- `58590956` T001, T002, T003, T007 and the core half of T008: `crates/micold-core/src/cli_reason.rs`, `tests/cli_reason.rs` (26 tests).
- `2e477df7` T004, T009: `env: Option<SpawnEnv>` on `DaemonMsg::AiCliAvailability`, `PROTOCOL_VERSION` 18, `docs/daemon.md`. The service sends `env: None` (`server.rs`), the client ignores it (`shell/daemon_sync.rs`).
- `da35f0df` T015, T016: labels from the constants, `docs/user-guide/settings.md`, `sandboxed-daemon.md`.
- `tdd/cycle-log.md` cycles 1 and 2, `tdd/test-list.md` U1–U27 `DONE`, tasks ticked: T001–T004, T007, T009, T015, T016.

**Next steps, in order** (tests first, record each red in `tdd/cycle-log.md`):

1. T008 remainder: delete the private `name_list` in `crates/micold-client/src/features/settings.rs:1674` and call `micold_core::cli_reason::name_list`. Then tick T008.
2. T005 then T010 (service). Tests in `crates/micold-daemon/tests/ai_cli_availability.rs` (fixtures `service_with_script`, `include_script`, `bin_with`, `ServicePath::without_clis`; `Settings::env_include_timeout_secs` for S5). Design chosen: `ResolvedEnv { vars, env }` in the `OnceLock` cell; `spawn_env_for(cwd)`; `availability_in(cwd) -> (Vec<AiCli>, SpawnEnv)`; and for S7 a `DaemonState` method the server arm calls with `Option<&Path>` after `cwd.or_else(home)` (for `None`: `available_here()` and `SpawnEnv::classify(enabled, path, None)`), so U36 is testable without removing the home directory. A cell's attempt always classifies to `Some` (the two settings states return before a cell is taken).
3. T006 then T011 (client): `env` and `asked_for` on `CliAvailability`; `answered` stamps `asked_for`. 13 fixtures build a `CliAvailability` literal (`grep -rn 'CliAvailability {' crates/micold-client`).
4. T038, T012, T013 (red), then T014: `missing_cli_notice` returns `explain(..)`'s `{reason} {action}`, `None` when `env` is `None`.
5. T039 (full gate, A7 mutant), T017 (`visual-pass`, quickstart §B B1–B8 and B14, light and dark).
6. Phase 4 steps 2–4: gate with review A, review B, PR `feat(037): …` with `Refs #434`.

**Open findings**: none. No review has run. The full suite has not run on this branch.

## Open escalation

None.

## Token usage

## Follow-ups not done

- The issue text says nothing names a hidden CLI. Settings already does (027 FR-023b): "*<name>*
  isn't installed on this computer, which is where sessions run." The spec treats that sentence as
  the thing to correct.

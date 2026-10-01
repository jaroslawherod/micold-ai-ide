# Autopilot ledger — 037-explain-hidden-cli

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: continue with next issue use autopilot
- **Kind**: feature
- **Issue**: #434
- **Worktree branch**: fix/github-issues
- **Started**: 2026-10-01
- **Phase**: 4-milestone (M2)
- **Next step**: M2: finish T025/T021/T040 (client), then gate, reviews A and B, visual pass B9/B10 (T027), PR.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #515 | Spec | merged | 063fa77affbcbcc20a842bffb71dce22212582d6 |
| #517 | Design | merged | 976b0220d342f1e35608286f0bdec4a77b8ce247 |
| #520 | M1 | merged | 7918f7bbfb44d7d5bb9134e17b825202a94c7c88 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017, T038–T039 | full | The Settings note under Default AI CLI (and Image reference) gives the reason for the home directory's environment state and the action; the availability answer carries the state (protocol 20); user guide updated | #520 | merged |
| M2 | T018–T027, T040–T042 | full | The missing-default message, a start or restart failure and the reply to an AI session's `create_session` give the same reason and action; user guide updated | | in progress |
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
| D16 | milestone | 034 merged PROTOCOL_VERSION 18 while M1 was in flight; which version does 037's wire change take? | 19 (18 -> 19); specs and tests updated in the rebase | agent-resolved | `origin/main` commit 746cfa6c |
| D17 | milestone | 034's M6 merged PROTOCOL_VERSION 19 after the first rebase; which version does 037's wire change take now? | 20 (19 -> 20). Supersedes D16's number. Code, tests, `docs/daemon.md` and the 037 artifacts updated in the second rebase | agent-resolved | `origin/main` commit 31c8b01b |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 39923bcd771235ee0060a1f1f6330830a153df7d:65fbe60069a3f9b434d1550233c450000099604e | CHANGES: 5 MAJOR, 3 MINOR — all fixed |
| Spec | 2 | b460803fa1576b80357bd110538b491691e8f6c3:65fbe60069a3f9b434d1550233c450000099604e | CLEAN: 1 MINOR, fixed |
| Plan | 1 | 36e2eb237df3c0946d4477309215238afed6789b:c1fb57e37f3409153c7d001c0e6d5b95c88124d8 | CHANGES: 2 MAJOR, 3 MINOR — all fixed |
| Plan | 2 | 91d8ac1c552b4aebf2935d2dbf27da809322cb24:f490b395f55a537ab5a1f99fea08cb1c9e5afb80 | CLEAN: 1 MINOR, fixed |
| Tasks | 1 | 6a70370da6ba5cd5650bcbdaf9f8760c347ac6da:fff351d523d61045b0b4faeabc1344331029a419 | CHANGES: 1 MAJOR, 3 MINOR — all fixed |
| Tasks | 2 | 0d9750169504fffafc0a6b973cfd025dbd58bcc9:1bc17b8d4e28e6454eec2393d3b2519435f6fb96 | CLEAN: no findings (scoped re-review of the round-1 fix diff) |
| A M1 | 1 | ca880275230916cc10aa96f574819965737f135a:01a574f62363e3826f84a4c842b5c56ba8575728 | CLEAN: 3 MINOR, none fixed (see Follow-ups) |
| B M1 | 1 | 7127de70ef5c3e65452575ce66c14909a318c21f:b205f81d1c577362dae687bce4268fdc09122bde | CLEAN: 3 MINOR, 1 fixed (U43, U44, U55 marked characterization) |
| Rebase M1 | 1 | 28f2150c18c1f7bd72942d62c2e26c6336863d3f:ce28161b9c42cff43cd15b7a7671eb1f306d51b8 | CLEAN: no findings (rebase resolution only: nothing of 034 dropped, 037's wire change is version 19 throughout; not counted) |
| Rebase M1 | 2 | 6a6fdd0d350be640ae526841aed9785ade15630a:65a5fc28465bbc276918bbccee1b30e83733f330 | CLEAN: 2 MINOR, 1 fixed (second rebase, onto 034's M6: nothing of 034 dropped, 037's wire change is version 20 throughout; not counted) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Token usage

## Follow-ups not done

- The issue text says nothing names a hidden CLI. Settings already does (027 FR-023b): "*<name>*
  isn't installed on this computer, which is where sessions run." The spec treats that sentence as
  the thing to correct.
- Review A M1, F1 (MINOR): a failed, timed-out or not-found attempt stays in the directory's cell of the
  service's environment cache, so after the user fixes the script file the note keeps the old state until
  an environment-include field is saved with a change or the service restarts. The cache predates 037
  (`crates/micold-daemon/src/state.rs`, `set_env_include`); not verified by a run. Candidate for the
  user guide or a bug record.
- Review A M1, F2 (MINOR): a baseline-probe timeout and a shell that could not be spawned are reported as
  `ScriptTimedOut` and `ScriptFailed` (`crates/micold-core/src/env_include.rs:313`, `:349`), so the note
  blames the script. Kept as FR-001's "attempted and fails" (D14).
- Review A M1, F3 (MINOR): `SpawnEnv` is on the wire but declared in `cli_reason.rs`, which `SCHEMA_HASH`
  does not cover (`crates/micold-core/build.rs`). A later variant change without a version bump would pass
  the handshake. Not fixed in M1: moving the type changes the hash after the green gate.
- Review B M1, F2 (MINOR): `spawn_env_for` maps an attempt's outcome with a total match (`attempted`), not
  through `SpawnEnv::classify` as T010's text says; the cycle log (cycle 3) records it, tasks.md does not.
- Review B M1, F3 (MINOR): one line over 100 columns in `docs/user-guide/sandboxed-daemon.md:106` and in
  the doc comment at `crates/micold-client/src/features/session.rs:262`.
- Rebase review round 2, F1 (MINOR): the doc comment above `FEATURE_026_PROTOCOL_VERSION` in
  `crates/micold-core/tests/schema_hash.rs` lacks the bare `///` line between 034's "18 → 19" paragraph
  and 037's "19 → 20" one. Fixed in M2.
- Quickstart B14 (container placement) was not run at a display in M1: `mise run image` would replace the
  `micold-daemon:dev` tag other worktrees share. It is covered by the automated image rows; M4 owns the
  full Part B record.


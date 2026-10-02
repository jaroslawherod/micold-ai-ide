# Autopilot ledger — 037-explain-hidden-cli

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: continue with next issue use autopilot
- **Kind**: feature
- **Issue**: #434
- **Worktree branch**: fix/github-issues
- **Started**: 2026-10-01
- **Phase**: 5-close
- **Next step**: close PR #542 open; the orchestrator waits for `ci complete` and merges, then the record PR.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #515 | Spec | merged | 063fa77affbcbcc20a842bffb71dce22212582d6 |
| #517 | Design | merged | 976b0220d342f1e35608286f0bdec4a77b8ce247 |
| #520 | M1 | merged | 7918f7bbfb44d7d5bb9134e17b825202a94c7c88 |
| #530 | M2 | merged | f169414780363e6e06fa837586ee93c215d9a2b2 |
| #535 | M3 | merged | bc5699922fd78fa9cc40346c7993827c54058839 |
| #537 | M4 | merged | c331d0f8c0556b5fb30f605a6ebc1bc966c519ab |
| #542 | Close | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017, T038–T039 | full | The Settings note under Default AI CLI (and Image reference) gives the reason for the home directory's environment state and the action; the availability answer carries the state (protocol 20); user guide updated | #520 | merged |
| M2 | T018–T027, T040–T042 | full | The missing-default message, a start or restart failure and the reply to an AI session's `create_session` give the same reason and action; user guide updated | #530 | merged |
| M3 | T028–T035, T043–T044 | full | A row's CLI list with two or more CLIs shows a non-pressable note naming the CLIs not offered, with reason and action; showcase entry; user guide updated | #535 | merged |
| M4 | T036–T037 | docs | `evidence/README.md` records quickstart Part A, all of Part B and the wording cross-check | #537 | merged |

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
| D18 | milestone | The refusal says "fix the script, then restart", but the service keeps a directory's resolution until environment-include is saved (issue #438 owns cache invalidation). How does the sentence stay true? | A start refused for a missing CLI drops that directory's cached resolution, so the next start and the next availability ask source the script again. Nothing else about the cache changes. A refusal and the answer that follows it are then two attempts and may differ; FR-012 holds per attempt. | agent-resolved | Review A M2 round 1, F1; `crates/micold-daemon/src/state.rs#refuse_and_forget_env`; orchestrator note on #438 |
| D19 | milestone | Where is a deleted session folder checked, given that resolving in a missing folder reports a timeout? | Inside the gate's CLI-not-found branch, not before the gate: before it, a resume whose folder and conversation are both gone would lose the conversation-gone sentence an existing test requires. | agent-resolved | Review A M2 round 1, F2; `crates/micold-daemon/tests/session_start.rs#resuming_a_conversation_the_cli_no_longer_has_reports_it_and_starts_nothing` |
| D20 | milestone | Quickstart B10 expected the pane and the banner to give the reason after a restart; at a display the pane kept its terminal and only the banner said it. Is that a defect of M2? | No. FR-009 governs the failure text where it is shown. The pane shows the service's sentence verbatim, and only when it has no terminal to keep; that rule predates 037 and no task of 037 changes it. The banner carries the sentence in the restart-in-place flow. B10's row now says so; a pane without a terminal (a session that fails to start with the service) is left for M4's Part B record. | agent-resolved | `crates/micold-client/src/ui/terminal.rs#empty_terminal_message`; `specs/037-explain-hidden-cli/evidence/README.md` (B10 rows); visual pass M2 |
| D21 | milestone | Where does the note sit in the panel, and how is its height known to the clamp? | Inside the panel's scrollable, under the items and a divider, so a short window scrolls it with them. It is `opaque`: a press on it is captured and publishes nothing. It wraps with `Wrapping::WordOrGlyph` because it names a path. `menu_panel_size_with_note(items, note)` shapes the text at `PANEL_WIDTH - 2 * ITEM_PADDING` to count its lines. Without a note `body` returns `item_column` itself, so other menus are laid out as before. | contract W7; 029 FR-009; `menu_anatomy` U87–U90 |
| D22 | 5-close | `speckit-converge` | Converged: FR-001–FR-017 (with FR-004a, FR-009a), SC-001–SC-007, the 20 acceptance scenarios, the behavioural edge cases and every plan touch-point are met; tasks.md unchanged by converge. No unbuilt behaviour. | agent-resolved | converge subagent |
| D23 | 5-close | `speckit-tdd-verify` | FAIL at c331d0f8 (suite and mutation unmeasured: the build lock was held): a loosened assertion (1), U43/U44/U55 without a red (2), plus MED/LOW smells. No spec behaviour unbuilt. Remediation T045–T050 (Phase 7): findings 1, 2, 3, 4 (client half), 6, 8 done; 9 of 10 hand mutants killed, the survivor is the showcase sample (cycle log, "Close"). | agent-resolved | `tdd/verification.md`; `tdd/cycle-log.md` |
| D24 | 5-close | Which tdd-verify findings are left? | Finding 4's daemon half (`session_start.rs` builds expectations with `start_refusal`; A10–A12 already add literals), 5 (U61 repeats U56/U57), 7 (timeout tests sleep a real script), 9 (magic seeds), 10 (`w2c`'s `matches!`): MED/LOW, no behaviour, each a refactor of a green test. Listed in Follow-ups. | agent-resolved | `tdd/verification.md` Findings |
| D25 | 5-close | `speckit-docguard-guard` | FAIL repo-wide (6 `STR001` for `docs-canonical/*` scaffolding, changelog, spec-number reuse). The only 037 findings are `TRC004` (`@req` annotations, a convention no feature here uses, 035 D42) and one low-confidence `DLC002` ("checked tasks and no open tasks"), answered by the Closed status. Nothing changed. | agent-resolved | `docguard-cli@latest guard`, filtered to 037 |

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
| A M2 | 1 | 658bb50d7c40201ec80791e76d82343d60275e9f:e7f3ec7352f5d48be2b4f45ee280a701ff24804a | CHANGES: 2 MAJOR (F1 a refused start left the directory's cached resolution, so "fix the script, then restart" was refused again; F2 a deleted session folder was reported as a script timeout), 3 MINOR — all five fixed |
| A M2 | 2 | 9754422054820536aa17744be7f7e4f0a960f307:e1253adc9da7adf6974b9eb5ac0b7ee27d8db917 | CLEAN: no findings (scoped re-review of the round-1 fix diff, Sonnet; it took 6 tool calls, so review B should look at `refuse_and_forget_env` and `folder_gone` with care) |
| B M2 | 1 | 59c1f515f13f2a004b4b96aa3356d9273be21f80:51f82c2c72413242e7253c7612aafaa1c6a4838a | CHANGES: 1 MAJOR (F1 `create_session`'s refusal left the directory's cached resolution, so D18 did not hold for an agent's retry), 3 MINOR — F1 fixed, F4's comment fixed, F2 and F4's cache half declined, F3 is in Follow-ups. Verify: `mise run test-core` 134 ok; `session_start` 27 passed, `mcp_create_session` 18 passed; client `unavailable_default_says_so` 22, `start_failure_notice` 4, `directory_availability` 13 passed |
| B M2 | 2 | 92862fb9b363a68b0b7c6157527284cdd7eb23fe:1e58735ed332e67f40ed229492165e39eedfddc6 | CLEAN: no findings (scoped re-review of the fix commit, Sonnet; round 1's snapshot went stale in a docs-only rebase, so the fix diff was given as the commit). Verify: `session_start` and `mcp_create_session` green |
| Visual M2 | 1 | 92862fb9b363a68b0b7c6157527284cdd7eb23fe:1e58735ed332e67f40ed229492165e39eedfddc6 | B9 PASS; B10 banner PASS, pane not shown (D20) |
| A M3 | 1 | 9a777a87fa00e2d729247bf566998492aa2acff9:8a5f4246e04f3ab5a2185bd793fe6c09d5682545 | CLEAN: 3 MINOR — F2 fixed (the estimate and the wrapping are now held for a path wider than the panel); F1 and F3 not fixed (see Follow-ups) |
| B M3 | 1 | 9a777a87fa00e2d729247bf566998492aa2acff9:8a5f4246e04f3ab5a2185bd793fe6c09d5682545 | CLEAN: 2 MINOR — F1 fixed (the WIP commits are squashed, so no commit on main fails to build); F2 noted (the reds of cycle 11 rest on the log: tests and implementation share a commit). Verify: `directory_availability` 30 passed, `menu_anatomy` 15 passed, `a_rows_cli_list_names_what_is_not_offered` 6 passed |
| Visual M3 | 1 | 9a777a87fa00e2d729247bf566998492aa2acff9:8a5f4246e04f3ab5a2185bd793fe6c09d5682545 | B11 and B13 PASS at the showcase, both themes; path wrapping not exercised (short sample path), B12 and the lowest row not reached |
| Visual M3 | 2 | tree 41ede9e7414bb4ff5f3b7cd02f2b9f077bc95401 (commit 5662df9b before the squash) | showcase B11 with a path wider than the panel: PASS, both themes. Real client: B11 light and dark PASS, lowest row PASS, B12 PASS |
| Close diff | 1 | 6b60322e3c14f738bde591e6d8d0220cdf31f53f:c331d0f8c0556b5fb30f605a6ebc1bc966c519ab | CLEAN: 3 MINOR, not fixed (they would change the tree the gate ran on): F1 the new `cli_reason.rs` messages name the case, not the rule (tdd-profile asks for the rule); F2 `main_tests.rs:6716-6718` labels a fragment "the CLI refused" where it checks the CLI is named; F3 three `assert_eq!` lines of ~150 columns in `cli_reason.rs:267,296,325` |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M2 | B round 1 | F2 (MINOR): the gate reports the folder gone on `!cwd.is_dir()`, which is also true of a path that is now a file or cannot be read | A path that is now a file is a folder that no longer exists, and the spawn-site check uses the same test. For an unreadable parent the alternative is the "script timed out" sentence, which is no truer. |
| M2 | B round 1 | F4, first half (MINOR): `refuse_and_forget_env` removes whatever cell is cached, so a resolve another start has in progress is evicted and the script runs once more | The result stays correct, and `invalidate_env_include` documents the same eviction as intended ("the invalidation wins"). Comparing cells is #438's ground. |

## Handover

None.

## Open escalation

None.

## Token usage

## Follow-ups not done

- Review A M3, F1 (MINOR): the start list's clamp is recomputed on every draw, and the note's height
  can change while the list is open (the row's own answer replaces the home answer, or the state
  changes). A list opened with too little room under its row then moves by the difference in lines
  (16dp each). Before 037 the height moved only with the item count. Not fixed: FR-012 wants the
  note to follow the newer answer, and holding the anchor would need the clamp stored in
  `StartMenu`. Candidate for a bug record if it is seen.
- Review A M3, F3 (MINOR): `menu_panel_size_with_note` shapes the note on every `view()` while the
  list is open, and `start_menu_note` formats it again each draw. Small, and only while that one
  list is open; not cached.
- Visual M3: opened with too little room under its row, the panel ends flush on the window's bottom
  edge with no margin. `clamp_menu_anchor` does the same for every menu (not 037's).

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
- Review A M2, F2 remainder (issue #438's ground): with the CLI found and the folder gone, the start fails
  at the spawn with the folder sentence, but the timed-out entry `env_include::resolve` reports for a
  missing folder stays cached; `availability_in` on a missing folder caches the same.
- Review B M2, F3 (MINOR, issue #438's ground): the spawn-site folder sentence (`state.rs`, the refused
  spawn) records the failure without dropping the directory's resolution, unlike the gate's two refusals.
- Flaky test outside this flow: `micold-daemon --test frame_coalescing`,
  `a_flood_is_coalesced_to_at_most_one_frame_per_frame_interval`, failed once in the gate on `e1253adc`
  while two other worktrees were building. It passed 5 of 5 runs alone on the same code and in the two
  gate runs after it; the diff touches no frame code. A timing test that fails under load.
- Visual pass M2, B10: after a failed restart in place the pane keeps the session's terminal and says
  nothing; the reason is in the banner and the bar reads "failed restart". Predates 037 (D20). Candidate
  for a bug record if the pane should say it too.
- Quickstart B14 (container placement) was not run at a display in M1: `mise run image` would replace the
  `micold-daemon:dev` tag other worktrees share. It is covered by the automated image rows; M4 owns the
  full Part B record. Recorded at M4 as covered by Part A (no image built).
- M4: no defect found in product code. The D20 pane without a terminal was seen at the real client (Resume form, no banner) and B13 at the real client.
- tdd-verify 037 (D24), not fixed: finding 4's daemon half (`crates/micold-daemon/tests/session_start.rs:1491-1500,1530,1735` take the expected sentence from `start_refusal`), 5 (`w3d_both_forms_begin_with_the_explanation` repeats U56/U57), 7 (`ai_cli_availability.rs:653-676` and `session_start.rs` sleep a real script past a 1 s timeout), 9 (magic seeds in `session_start.rs:1489,1730`), 10 (`w2c`'s `matches!`). The showcase sample's `AttemptDir` (`showcase/sections/floating.rs:160`) has no test.

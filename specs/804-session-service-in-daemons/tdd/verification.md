---
feature: 804-session-service-in-daemons
verdict: FAIL
verified_at: 0e3b543f
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
independent: true
suite: not run (build lock queued behind other worktrees, aborted after ~15 min; profile baseline 5071 passed, 0 failed)
mutation: unmeasured (no tool in profile; deliberate mutants not run, same reason)
behaviors: not countable (no tdd/test-list.md)
high_findings: 3
---

# TDD Verification: Session service settings in the Daemons manager

**Verdict: FAIL.** One HIGH tautological test, one weakened existing test, and an integration step (D3 legacy sync) ticked done with no test.
M1 and M4 have no red evidence; recorded honestly in the cycle log and not raised as findings, per instruction.

## Test-first evidence

No `tdd/test-list.md`: the feature was not planned through the extension, so behaviours are graded per task group, ordering from git.

| Group | Evidence | Class |
|---|---|---|
| M1 T001-T007 | log: none captured; one WIP commit 685d4613 | not raised (known) |
| M2 T014-T021 | no cycle-log entry (log promises one); tests and source share dc2f4034 / e6487c3e | LIKELY at best (F3) |
| M3 T022 | test-only commit 38e0f132; passes without a source change (characterises existing 491 migration), no cycle-log entry | NOT_APPLICABLE for red, F3 |
| M4 T025-T031 | log: none captured | not raised (known) |
| T007a | unticked, no test | NO_TEST (F1) |

Existing tests changed by the feature: renames `Daemon` to `Daemons`, dialog counts 19 to 18 and 21 to 20, `ALL.len()` 7 to 6, icon count 53 to 52 are all justified by the removed page. No `#[ignore]`, no gate or threshold lowered. Real weakening: F6.

## Findings: new behaviour

- **F1 HIGH.** `crates/micold-client/src/shell/routing.rs:237` (`settings.sync_legacy_daemon()` in `apply_daemon_outcome`, D3) has no test through the form-save path. Only the pure unit tests at `crates/micold-core/src/settings.rs:452-505` exist. `tasks.md` ticks T011 "covered by T007a" while T007a is unticked and no such test exists. Fix: write T007a (drive `Outcome::DaemonRegistryChanged` against a settings store; assert the legacy block follows the saved entry and an unrelated save leaves the registry unchanged). Prove it by deleting the call: the test must fail.
- **F2 HIGH.** Tautological assertion in `crates/micold-client/tests/upgrade_single_daemon.rs`, `a_daemon_added_after_the_upgrade_starts_at_defaults_not_a_copy`: the test builds the second daemon with `SandboxProfile::default()` itself, then asserts it is default. FR-005's "later daemon starts at defaults" is never exercised. Fix: add the daemon through the form (`begin_add`, set name and container name, `save`) from the migrated state and assert its profile equals the default.
- **F4 MED.** US4 AS1/SC-005 (five titled sections, Container open, rest closed) is asserted only as state (`daemons_feature.rs:586`). No test renders the container form and checks the five titles or that closed bodies are absent. `daemons_section_geometry.rs` has no container-form case, although T031 claims the gate was run with sections. `accordion_section.rs` tests never assert the title, the chevron direction (`:57`) or the focus ring, all listed in T025. Fix: a paint test of a container daemon form (titles present, closed-section field labels absent) and chevron/title/focus assertions in `accordion_section.rs`.
- **F5 MED.** `Section::holds` (`crates/micold-client/src/features/daemons.rs:136-141`) is tested for `cpus` only. `image`, `image_path`, `memory`, `pids`, `storage` arms are untested, so a closed Container or Limits section could hide those errors (FR-014). Fix: loop `sections_open_by_toggle_and_by_error...` over every `FormError.field` id.

## Findings: test strength and docs

- **F6 HIGH (weakened existing test).** `tests/sandbox_limits.rs` (dc2f4034) lost `a_limit_below_the_workable_minimum_is_refused_by_its_own_field`, which asserted for all four limits that the message names the accepted range (FR-016); `text_that_is_not_a_number_says_so` (parse failure is not the range message); `a_whole_core_count_has_no_decimal_point`; `a_rejected_form_saves_nothing`. Replacement `crates/micold-client/src/features/settings.rs:2130-2141` asserts only `!message.is_empty()`, covers one non-numeric and one too-low input, and never tests cpus or pids below minimum. `daemons_feature.rs:389` checks the field id only. Fix: restore the range-in-message and not-a-number-differs assertions in `parse_budget_tests`, with a case per limit, plus a "2" cores round trip.
- **F7 MED.** `saving_a_placement_moves_the_daemon.rs` and `settings_reports_the_live_placement.rs` were deleted with the page. `the_question_counts_the_sessions_it_would_stop` and `with_nothing_running_the_question_warns_about_nothing` have no successor: `daemons_feature.rs:451` asserts only that a prompt exists (`prompt().is_some()`), not what it says. FR-004 ("same effect on running sessions") is thinly covered. Fix: assert the prompt text counts the daemon's running sessions, none and one.
- **F8 LOW.** `tests/settings_sections.rs` `nothing_points_at_the_removed_session_service_section` uses five narrow needles, so plain "Session service" in docs or strings passes (FR-006/SC-004 are broader). It also skips `sandbox_limits.rs` and itself by path. Fix: case-insensitive search for "session service" with an explicit allowlist of the negative assertions.
- **F3 MED (process).** The cycle log says "Later milestones (M2, M3) write red before green and record it here", but has no M2/M3 entry. Fix: record the M2/M3 outcome honestly (M3 is a characterisation test, no red possible), and add `tdd/test-list.md` or state in the log that none exists.

## Mutation results

Unmeasured. Predicted survivors, to confirm by deliberate mutant (restore and re-run after each):
1. Delete `sync_legacy_daemon()` at `routing.rs:237` (F1).
2. Make `Section::Container => false` in `holds` (F5).
3. Swap the chevron condition at `accordion_section.rs:57` (F4).

## Traceability

| Criterion | Tests | Entry point |
|---|---|---|
| US1 AS1-2, FR-001/003 | `daemons_feature.rs:339,412` | yes (feature update/save) |
| US1 AS3, FR-004/010 | `:451` | prompt presence only (F7) |
| FR-011 | `:473` | yes |
| FR-008 | `:389` | field id only (F6) |
| US2 AS1, FR-002 | `settings_sections.rs` `the_rail_has_no_session_service_entry` | yes |
| US2 AS2/4, FR-006 | `sandbox_limits.rs` `the_limit_hint_names_the_daemons_manager`, `settings_sections.rs` stale-reference test | weak (F8) |
| US2 AS3, FR-007 | `daemons_feature.rs:546`, `settings_rail_motion.rs` | view model, plus rail motion |
| US3 AS1, FR-005, SC-003 | `upgrade_single_daemon.rs` tuned-install test | yes; "later daemon at defaults" is F2 |
| US3 AS2, FR-009 | stale-reference test | weak (F8) |
| US4 AS1, SC-005 | `daemons_feature.rs:586` | state only (F4) |
| US4 AS2 | `accordion_section.rs` header toggle test; `daemons_feature.rs:586` | yes |
| US4 AS3, FR-014 | `daemons_feature.rs:586` | `cpus` only (F5) |
| US4 AS4, FR-012 | `daemons_feature.rs:586` (draft unchanged) | yes |
| US4 AS5, FR-013 | showcase entry (`showcase/catalogue.rs`) | no test found (not verified) |
| D3 (plan) | none through entry point | NO_TEST (F1) |

Tests tracing to nothing: none found.

## What was not audited

Full suite run, coverage, mutation and all deliberate mutants (build lock queue). Determinism and speed. The `core` crate beyond `sync_legacy_daemon` tests. Docs content (`docs/`), the layout snapshot fixture rewrite in dc2f4034 (450 lines), and whether the showcase entry has a test. The smell pass was done by one auditor without a subagent.

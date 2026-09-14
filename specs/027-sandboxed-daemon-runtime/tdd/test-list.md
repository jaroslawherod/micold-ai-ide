---
feature: 027-sandboxed-daemon-runtime
loop: inside-out
profile: .specify/memory/tdd-profile.md
spec_criteria: 8
planned_at: b53e3449
updated_at: b53e3449
suite_baseline: green
---

# Test List: BUG-004 — a sandbox the application finds absent is brought up again

Scope is the BUG-004 increment only (`tasks.md` Phases 18 and 19). Feature 027's earlier phases
(T001–T166) were not planned through this extension and have no per-behavior evidence here.

`loop: inside-out` because the end-to-end path needs a real container runtime (`sandbox_real_*`,
T181). The acceptance behaviors below are the criteria, and they close through the client's
`update_inner` — the application's real message entry point — rather than through a runtime.

The eight criteria are the rows of `verification.md`'s traceability table: FR-002a, FR-036a,
FR-036b, SC-004c, US6 scenario 9, S-6, S-7 (`data-model.md` §7), and C-8's caller half
(`contracts/container-runtime.md`).

## Outer loop: acceptance behaviors

| id  | behavior                                                                                           | traces                  | kind    | state   | test |
| --- | -------------------------------------------------------------------------------------------------- | ----------------------- | ------- | ------- | ---- |
| A1  | A refused dial to a failed sandbox starts a bring-up the application runs                          | FR-002a, US6-9          | example | DONE | `main::tests::a_failed_sandbox_the_client_cannot_reach_is_brought_up_again` |
| A2  | Unattended bring-ups driven by messages alone stop at the bound, then the failure stands, reported | FR-036a, S-6            | example | DONE | `main::tests::once_the_unattended_bring_ups_are_spent_the_failure_is_reported` |
| A3  | While a bring-up is in flight the application does not present the service as a connection failure | FR-036b, SC-004c        | example | DONE | `main::tests::a_bring_up_in_flight_is_not_shown_as_a_lost_connection`, `main::tests::a_bring_up_in_flight_offers_no_failure_card_and_no_fallback` |
| A4  | Each failed unattended attempt's reason is still reported while the next one runs                   | FR-036a, US6-9          | example | DONE | `main::tests::the_previous_attempts_reason_stays_visible_while_the_next_one_runs` |

## Inner loop: unit behaviors

### `crates/micold-core/src/sandbox/lifecycle.rs` (Phase 18, T167 and T171)

| id  | behavior                                                                   | traces         | kind    | state | test |
| --- | -------------------------------------------------------------------------- | -------------- | ------- | ----- | ---- |
| U1  | A failed sandbox found absent moves to `Probing` and spends one attempt     | FR-002a, S-6   | example | DONE  | `sandbox_state::a_failed_sandbox_found_absent_is_brought_up_again_without_anyone_asking` |
| U2  | Unattended attempts are bounded, and a spent budget stays spent             | FR-036a, S-6   | example | DONE  | `sandbox_state::unattended_bring_ups_are_bounded_and_then_the_failure_stands` |
| U3  | Attempts after the first wait longer than a connection retry, never less than the one before | FR-036a, S-6 | example | DONE | `sandbox_state::unattended_bring_ups_never_wait_less_than_the_one_before`, `daemon::tests::unattended_bring_ups_after_the_first_wait_longer_than_a_reconnect` (split by T183) |
| U4  | Only `Failed` is brought up on absence                                      | S-7            | example | DONE  | `sandbox_state::absence_only_brings_up_a_sandbox_that_has_failed` (never red; test-after per `verification.md`) |
| U14 | `is_coming_up` is true for exactly `Probing`, `Acquiring` and `Starting`    | FR-036b, S-6   | example | DONE | `sandbox_state::only_a_bring_up_in_flight_is_coming_up` |

### `crates/micold-client/src/shell/daemon_sync.rs` (Phase 18, T168; Phase 19)

| id  | behavior                                                                                  | traces          | kind             | state    | test |
| --- | ----------------------------------------------------------------------------------------- | --------------- | ---------------- | -------- | ---- |
| U5  | A refused dial on a failed sandbox moves it to `Probing` without a failure toast           | FR-002a, FR-036b | example         | DONE     | `main::tests::a_failed_sandbox_the_client_cannot_reach_is_brought_up_again` |
| U6  | A refused dial during `Acquiring` starts nothing and reports nothing                       | FR-036b, S-6    | example          | DONE     | `main::tests::a_refused_dial_during_a_bring_up_is_not_reported_as_a_failure` |
| U10 | A spent budget reports the failure                                                        | FR-034          | characterization | BASELINE | `main::tests::once_the_unattended_bring_ups_are_spent_the_failure_is_reported` |
| U11 | With no boot plan the failure is reported                                                 | FR-034          | characterization | BASELINE | `main::tests::without_a_boot_plan_nothing_is_brought_up` |
| U12 | The host placement never brings a sandbox up                                              | FR-035, S-7     | characterization | BASELINE | `main::tests::a_host_process_placement_never_brings_a_sandbox_up` |
| U15 | A refused dial with no plan, on the host placement, or during a bring-up yields no task    | FR-035, S-6     | example          | DONE | `main::tests::without_a_boot_plan_nothing_is_brought_up`, `main::tests::a_host_process_placement_never_brings_a_sandbox_up`, `main::tests::a_refused_dial_during_a_bring_up_is_not_reported_as_a_failure` |
| U16 | A refused dial during `Probing` and `Starting` starts nothing and reports nothing          | FR-036b, S-6    | example          | DONE | `main::tests::a_refused_dial_during_a_bring_up_is_not_reported_as_a_failure` |
| U17 | The bring-up a refused dial starts carries the delay the budget gave it                    | FR-036a, S-6    | example          | DONE | `main::tests::the_bring_up_after_a_failed_attempt_waits_the_budgets_delay` |
| U18 | A refused dial while the state still reads `Running` is reported (the liveness check decides) | FR-036       | characterization | BASELINE | `main::tests::a_refused_dial_while_the_sandbox_reads_running_is_reported` |

### `crates/micold-client/src/shell/sandbox.rs` (Phase 18, T170; Phase 19)

| id  | behavior                                                                        | traces        | kind    | state   | test |
| --- | ------------------------------------------------------------------------------- | ------------- | ------- | ------- | ---- |
| U7  | Every stage a bring-up enters is reported in order, before how it ended          | SC-004c, C-8  | example | DONE    | `shell::sandbox::tests::every_stage_a_bring_up_enters_is_reported_before_how_it_ended` |
| U19 | An unattended bring-up does not start before its delay has passed                | FR-036a, S-6  | example | DONE | `shell::sandbox::tests::a_delayed_bring_up_does_not_start_before_its_delay` |
| U20 | The production bring-up reports `Probing` first and its outcome last             | SC-004c, C-8  | example | DONE | `shell::sandbox::tests::the_production_bring_up_reports_probing_first_and_how_it_ended_last` |

### `crates/micold-client/src/features/sandbox.rs` (Phase 18, T169; Phase 19)

| id  | behavior                                                                          | traces       | kind    | state   | test |
| --- | --------------------------------------------------------------------------------- | ------------ | ------- | ------- | ---- |
| U8  | A reported stage becomes the sandbox's state                                      | SC-004c, S-1 | example | DONE    | `main::tests::a_reported_stage_becomes_the_sandbox_state` (renamed by T185) |
| U9  | A sandbox that came up earns its unattended attempts back                         | S-6          | example | DONE    | `main::tests::a_sandbox_that_came_up_earns_its_unattended_bring_ups_back` |
| U21 | During a bring-up the connection banner is not `Disconnected`                     | FR-036b      | example | DONE | `main::tests::a_bring_up_in_flight_is_not_shown_as_a_lost_connection` |
| U22 | During a bring-up no "The sandbox did not start" card, and no fallback, is offered | FR-036b, FR-035a | example | DONE | `main::tests::a_bring_up_in_flight_offers_no_failure_card_and_no_fallback` |
| U23 | The previous attempt's reason stays visible while the next attempt runs           | FR-036a, US6-9 | example | DONE | `main::tests::the_previous_attempts_reason_stays_visible_while_the_next_one_runs` |

## Invariants and edge cases still to place

- none

## Out of scope

- A real-runtime recovery test (T181): needs `mise run image`, which replaces the `micold-daemon:dev`
  tag every worktree shares; tracked in `tasks.md`, not on this list.
- Test-only refactors T182–T185 (notification level, the real `RECONNECT_BACKOFF`, loop guards,
  renames): structural, recorded in the cycle log's notes rather than as behaviors.
- The rest of feature 027: see the scope note above.

## Verification commands

Copied from `.specify/memory/tdd-profile.md`:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
  (for the client binary's unit tests: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide {name}`)
- Full suite: `scripts/build-lock.sh cargo test --workspace`
- Coverage: none (`cargo-llvm-cov` absent)
- Mutation: none (`cargo-mutants` absent); deliberate mutants by hand

# Cycle Log: BUG-005 — a sandbox the application finds absent is brought up again

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and failed
before the implementation — or, for a behavior the code already had, that the test fails with the
named deliberate mutant applied (`tdd-loop-playbook.md` step 3).

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 2947 passed, 0 failed, 2 ignored (135s)
- commit: `b53e3449` plus the uncommitted Phase 18 (BUG-005) diff
- recorded: cycle 0, 2026-09-13, before any Phase 19 change

## Phase 18 (T167–T171): recorded after the fact

These cycles ran before this log existed. Their red output was kept only in a session scratchpad and
is copied here verbatim so it outlives the session. `verification.md` classifies them LIKELY at best.

- U1 red: `sandbox_state.rs:491` panicked: "a sandbox that is not running, with attempts left, has to be brought up again"
- U2 red: `sandbox_state.rs:532` panicked: "a bound of zero is the bug, not a fix for it"
- U3 red: `sandbox_state.rs:558` panicked: "no attempt was made to space"
- U5, U6, U7, U8, U9 red: client binary run "9 passed; 5 failed" against the pre-fix code
- U4, U10, U11, U12: never red (characterizations or satisfied by a stub)

## Cycle 1: A1 a refused dial to a failed sandbox starts a bring-up the application runs (T173)

- test: `main.rs::tests::a_failed_sandbox_the_client_cannot_reach_is_brought_up_again` (extended);
  helper `connection_failed` now returns the `Task` instead of discarding it
- first run: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide a_failed_sandbox_the_client_cannot_reach_is_brought_up_again`
  -> `1 passed` — the behavior already existed (Phase 18), so the red is a deliberate mutant
- red (M13, `daemon_sync.rs` `return boot_after(plan, after)` -> `{ let _ = (plan, after); return Task::none(); }`):
  same command -> `panicked at crates/micold-client/src/main.rs:3008:9: the state says a bring-up
  started, so one has to be scheduled` (1 failed). Restored; sha256 checked OK
- green: no production change
- refactor: none
- commit: not committed (user has not asked)

## Cycle 2: U15 a refused dial with no plan, on the host placement, or during a bring-up yields no task (T173)

- test: `without_a_boot_plan_nothing_is_brought_up`, `a_host_process_placement_never_brings_a_sandbox_up`,
  `a_refused_dial_during_a_bring_up_is_not_reported_as_a_failure` (each extended with `units() == 0`)
- first run: `... --bin micold-ai-ide -- --exact <the three>` -> `3 passed`
- red (mutant A, `on_connect_failed`'s final `Task::none()` -> `Task::done(Message::NoOp)`):
  -> `main.rs:3087:9 with no plan there is nothing to bring up, so nothing may be scheduled` and
  `main.rs:3107:9 the host placement's own connection starts its service` (2 failed). Restored, sha OK
- red (mutant B, the `is_coming_up` branch's `Task::none()` -> `Task::done(Message::NoOp)`):
  -> `main.rs:3039:9 the bring-up in flight is the only one there may be (S-6)` (1 failed). Restored, sha OK
- green: no production change
- refactor: none
- commit: not committed

## Cycle 3: U14 `is_coming_up` is true for exactly `Probing`, `Acquiring` and `Starting` (T176)

- test: `crates/micold-core/tests/sandbox_state.rs::only_a_bring_up_in_flight_is_coming_up`
- first run: `scripts/build-lock.sh cargo test --test sandbox_state only_a_bring_up_in_flight_is_coming_up -- --exact`
  -> `1 passed` — the predicate already existed, so the red is a deliberate mutant
- red (M15, `lifecycle.rs` `Probing | Acquiring(_) | Starting => true` -> `Acquiring(_) => true, Probing | Starting => false`):
  same command -> `panicked at crates/micold-core/tests/sandbox_state.rs:570:5: a bring-up in flight is
  Probing, Acquiring and Starting, and nothing else: [Acquiring(Progress { stage: "Downloading", detail:
  None, percent: Some(40) })]` (1 failed). Restored; sha256 checked OK
- green: no production change
- refactor: a first draft restated the production `matches!`; rewritten to filter `every_state()` and
  match the resulting slice, so the test no longer re-implements the predicate
- commit: not committed

## Cycle 4: U16 a refused dial during `Probing` and `Starting` starts nothing and reports nothing (T176)

- test: `main.rs::tests::a_refused_dial_during_a_bring_up_is_not_reported_as_a_failure`, now looping
  over `Probing`, `Acquiring` and `Starting` (it covered `Acquiring` only)
- first run: `... -p micold-client --bin micold-ai-ide -- --exact tests::a_refused_dial_during_a_bring_up_is_not_reported_as_a_failure`
  -> `1 passed`
- red (M15, as cycle 3): same command -> `panicked at crates/micold-client/src/main.rs:3053:13: Probing:
  the daemon is not listening *yet*; saying it could not be reached reads a working bring-up as a broken
  one (FR-036b)` (1 failed). Restored; sha256 checked OK
- green: no production change
- refactor: none
- commit: not committed

## Cycle 5: U19 an unattended bring-up does not start before its delay has passed (T174, first half)

- test: `crates/micold-client/src/shell/sandbox.rs::tests::a_delayed_bring_up_does_not_start_before_its_delay`
- first run: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- --exact shell::sandbox::tests::a_delayed_bring_up_does_not_start_before_its_delay`
  -> `1 passed; ... finished in 0.20s`
- red (M10, `reported` without `if !after.is_zero() { tokio::time::sleep(after).await; }`): same command ->
  `panicked at crates/micold-client/src/shell/sandbox.rs:532:9: the bring-up started 156.875µs after it was
  asked for, before its 200ms delay (S-6)` (1 failed). Restored; sha256 checked OK
- green: no production change
- refactor: none in this cycle; the `BringUp` seam U17/U20 need is its own structural step
- commit: not committed

## Structural step: a readable seam for the bring-up decision (T174/T177, before cycles 6 and 7)

- `shell/sandbox.rs`: `boot_after` replaced by `BringUp { plan, after }` with `now`, `task` and a
  private `stream(runner)`; `start` takes the `CommandRunner` instead of constructing `SystemRunner`.
  `boot(plan)` is `BringUp::now(plan).task()`
- `shell/daemon_sync.rs`: `on_connect_failed` split into `refused_dial(app, reason) -> Option<BringUp>`
  and `.map_or_else(Task::none, BringUp::task)`; the decision itself is unchanged
- `main.rs` tests: the fixture's failure extracted to `failed_sandbox_state()`
- suite after the step (with the cycle 6 test already present): `scripts/build-lock.sh cargo test --workspace`
  -> 2950 passed, 0 failed, 2 ignored (164s)

## Cycle 6: U17 the bring-up a refused dial starts carries the delay the budget gave it (T174, second half)

- test: `main.rs::tests::the_bring_up_after_a_failed_attempt_waits_the_budgets_delay`
- first run: in the workspace run above -> `ok`
- red (M11, `refused_dial` returning `BringUp { plan, after: Duration::ZERO }`):
  `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- --exact tests::the_bring_up_after_a_failed_attempt_waits_the_budgets_delay`
  -> `panicked at crates/micold-client/src/main.rs:3052:9: assertion left == right failed: each unattended
  bring-up waits the delay the budget gave it (S-6)  left: [0ns, 0ns]  right: [0ns, 5s]` (1 failed).
  Restored; sha256 checked OK
- green: no production change beyond the structural step
- refactor: none
- commit: not committed

## Cycle 7: U20 the production bring-up reports `Probing` first and its outcome last (T177)

- test: `shell/sandbox.rs::tests::the_production_bring_up_reports_probing_first_and_how_it_ended_last`
  (`BringUp::stream` over a `RecordingRunner`, state in a `tempfile::tempdir`)
- first run: `... --bin micold-ai-ide -- --exact shell::sandbox::tests::the_production_bring_up_reports_probing_first_and_how_it_ended_last`
  -> `1 passed`
- red (M16, `BringUp::stream` passing `&mut |_| {}` instead of `observe`): same command -> `panicked at
  crates/micold-client/src/shell/sandbox.rs:555:9: the first stage a bring-up enters has to reach the view
  (SC-004c): [Sandbox(Failed(Failure { stage: Probing, error: Unknown { stderr: "could not read the
  runtime's version output: EOF while parsing a value at line 1 column 0" } }))]` (1 failed). Restored; sha OK
- green: no production change beyond the structural step
- refactor: none
- commit: not committed

## Cycle 8: A2 unattended bring-ups driven by messages alone stop at the bound, then the failure stands, reported (T175)

- test: `main.rs::tests::once_the_unattended_bring_ups_are_spent_the_failure_is_reported`, rewritten to drive
  `ConnectFailed` / `SandboxMsg::Failed` until no task is scheduled, with no write to `app.sandbox.unattended`
  (its previous assertions — state stands, failure reported — kept). `a_sandbox_that_came_up_earns_its_unattended_bring_ups_back`
  now spends its attempt through a refused dial, with a setup guard
- first run: `... --bin micold-ai-ide -- --exact tests::once_the_unattended_bring_ups_are_spent_the_failure_is_reported tests::a_sandbox_that_came_up_earns_its_unattended_bring_ups_back`
  -> `2 passed`
- red (M14, `Sandbox::service_absent` dropping `self.unattended = again.budget`): same command ->
  `panicked at crates/micold-client/src/main.rs:3109:13: a bound that keeps bringing the sandbox up is no
  bound (S-6)` and `main.rs:3200:9: setup: the refused dial has to spend an attempt, or there is nothing to
  earn back  left: UnattendedBringUps { spent: 0 }` (2 failed). Restored; sha256 checked OK
- green: no production change
- refactor: the unused-`Task` warning in the old spent-budget test is gone with the rewrite
- commit: not committed

## Cycle 9: U21 during a bring-up the connection banner is not `Disconnected` (T178, first half)

- test: `main.rs::tests::a_bring_up_in_flight_is_not_shown_as_a_lost_connection`
- red (real, no mutant): `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- --exact tests::a_bring_up_in_flight_is_not_shown_as_a_lost_connection`
  -> `panicked at crates/micold-client/src/main.rs:3124:13: assertion left != right failed: Probing: the
  service is being brought up, and the sandbox view says so (FR-036b)  left: Disconnected  right: Disconnected` (1 failed)
- green: `main.rs` `connection_status(app)` passes `app.disconnected && !app.sandbox.state.is_coming_up()` as
  the disconnect fact; `app.disconnected` itself is unchanged, because the op gates read it.
  `scripts/build-lock.sh cargo test --workspace` -> 2952 passed, 0 failed, 2 ignored (163s)
- refactor: none. The toast assertions in the A1 and B6 tests are kept beside the banner test, not
  replaced: removing an assertion is not this cycle's to do
- commit: not committed

## Cycle 10: U22 during a bring-up no "The sandbox did not start" card, and no fallback, is offered (T178, second half)

- test: `main.rs::tests::a_bring_up_in_flight_offers_no_failure_card_and_no_fallback`
- first run: `... --bin micold-ai-ide -- --exact tests::a_bring_up_in_flight_offers_no_failure_card_and_no_fallback`
  -> `1 passed`. `verification.md` expected this half red; it is not — `persistent_notice` and
  `fallback_offer` were already `None` outside `Failed`/`Stale`, so the red is a deliberate mutant
- red (mutant A, `persistent_notice` with `SandboxState::Probing => Some(String::new())`): same command ->
  `panicked at crates/micold-client/src/main.rs:3157:13: Probing: the standing sandbox card is for a sandbox
  that did not start (FR-036b)  left: Some("")  right: None` (1 failed)
- red (mutant B, `fallback_offer` with `SandboxState::Starting => Some(ConsentedFallback { because: "" })`):
  -> `main.rs:3162:13: Starting: the host fallback is offered for a failure, not a bring-up (FR-035a)
  left: Some(ConsentedFallback { because: "" })  right: None` (1 failed). Restored; sha256 checked OK
- green: no production change
- refactor: none
- commit: not committed

## Cycle 11: U23 the previous attempt's reason stays visible while the next attempt runs (T179)

- test: `main.rs::tests::the_previous_attempts_reason_stays_visible_while_the_next_one_runs`; the symbol
  `ui::attempt_line(&Sandbox)` added first as a stub returning `stage_line(&sandbox.state)`, so the red
  is an assertion rather than a compile error
- red (real): `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- --exact tests::the_previous_attempts_reason_stays_visible_while_the_next_one_runs`
  -> `panicked at crates/micold-client/src/main.rs:3192:13: Probing: the attempt that failed has to say why
  while the next one runs (FR-036a): StageLine { label: "Checking the container runtime", detail: None }` (1 failed)
- green: `Sandbox.previous_attempt: Option<Failure>`, taken from `Failed` in `service_absent` and retired
  in `started`; `attempt_line` appends "Trying again: {reason}" to the detail; the window renders
  `sandbox_status::indicator(attempt_line(sandbox))` instead of `view(&sandbox.state)`; the daemon_sync
  log line names the previous reason (not asserted: exact log strings are a smell).
  `scripts/build-lock.sh cargo test --workspace` -> 2954 passed, 0 failed, 2 ignored (180s)
- refactor: `sandbox_status::view` now delegates to the new `indicator(line)`, so the tested indicator and
  the window's share one renderer
- commit: not committed

## Cycle 12: U18 a refused dial while the state still reads `Running` is reported (T180, characterization)

- test: `main.rs::tests::a_refused_dial_while_the_sandbox_reads_running_is_reported`
- decision pinned: reported, with no bring-up — only the liveness check `on_disconnected` starts may
  conclude a running sandbox is gone (FR-036); a bring-up here could start a second container
- first run: `... --bin micold-ai-ide -- --exact tests::a_refused_dial_while_the_sandbox_reads_running_is_reported` -> `1 passed` (expected for a characterization)
- mutant (`refused_dial` treating `accepts_sessions()` like `is_coming_up()`): same command -> `panicked at
  crates/micold-client/src/main.rs:3222:9: a running sandbox the client cannot reach is a connection failure,
  until shown otherwise` (1 failed). Restored; sha256 checked OK
- state: BASELINE
- commit: not committed

## Refactor: tests assert the report by level, and silence as nothing visible or pending (T182, finding 11)

- change: `main.rs` helper `a_connection_failure_was_reported` checks `visible()` for `Level::Error`
  instead of the substring "Could not connect"; new `nothing_was_reported` (`visible().is_none() &&
  pending() == 0`) replaces the two negative uses (`a_failed_sandbox_the_client_cannot_reach_is_brought_up_again`,
  `a_refused_dial_during_a_bring_up_is_not_reported_as_a_failure`); tests unchanged otherwise
- proof 1 (reword only, `daemon_sync.rs:331` "Could not connect to the session daemon" -> "The session
  daemon refused the connection"): `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`
  -> `130 passed; 0 failed` — the positives no longer depend on the sentence
- proof 2 (reword + `notify_error` in the `is_coming_up` branch): same command -> `panicked at
  crates/micold-client/src/main.rs:3098:13: Probing: the daemon is not listening *yet*; saying it could
  not be reached reads a working bring-up as a broken one (FR-036b)` (1 failed). Under the old helper this
  mutant passed vacuously: the reworded notice does not contain "Could not connect"
- proof 3 (reword + `notify_error` before `return Some(BringUp ..)`): same command -> `panicked at
  crates/micold-client/src/main.rs:3035:9` in `a_failed_sandbox_the_client_cannot_reach_is_brought_up_again` (1 failed)
- each proof restored with `cp` from a snapshot; `sha256sum -c` -> `daemon_sync.rs: OK`
- commit: not committed

## Refactor: S-6 checked against the real reconnect backoff; `10` named; the length moved to the bound test (T183, T184, findings 12 and 13)

- relocated, not removed: the "waits longer than a connection retry" check left
  `sandbox_state.rs` (where `CONNECTION_RETRY = 1s` was a hand copy) for a new
  `crates/micold-client/src/daemon.rs` test, `unattended_bring_ups_after_the_first_wait_longer_than_a_reconnect`,
  which compares `UNATTENDED_BRING_UP_DELAYS[1..]` with the private `RECONNECT_BACKOFF` itself
- `sandbox_state.rs`: `const NO_BOUND: usize = 10` names both ceilings; the spacing test (renamed
  `unattended_bring_ups_never_wait_less_than_the_one_before`, since it no longer checks the retry) guards
  its `.skip(1)` loop with `waits.len() >= 2` and loses `assert_eq!(waits.len(), ...)`, which moved to
  `unattended_bring_ups_are_bounded_and_then_the_failure_stands` as
  `assert_eq!(attempts, UNATTENDED_BRING_UP_DELAYS.len(), ...)`
- run: `scripts/build-lock.sh cargo test -p micold-core --test sandbox_state` -> `130 passed; 0 failed`;
  the `daemon.rs` test ran in the workspace suite below
- commit: not committed

## Refactor: names that match what is asserted; rule messages (T185, findings 14–16)

- `main.rs`: `a_reported_stage_is_what_the_sandbox_shows` -> `a_reported_stage_becomes_the_sandbox_state`
- `sandbox_state.rs`: `only_an_explicit_request_restarts_the_sandbox` ->
  `a_running_sandbox_restarts_only_on_an_explicit_request`
- the setup loop at the old `main.rs:3049` was already guarded by cycle 8's rewrite (`<= DELAYS.len()`)
- rule messages added to the bare asserts in `without_a_boot_plan_nothing_is_brought_up` and
  `a_host_process_placement_never_brings_a_sandbox_up`
- not addressed: finding 16's second half — `connection_failed` still writes real lines to the
  developer's client log through `log_line`; T185's text does not cover it
- run: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide` -> `130 passed; 0 failed`
- commit: not committed

## T181: the real-runtime recovery test — written, compiled, not run to a verdict

- test: `crates/micold-core/tests/sandbox_real_lifecycle.rs::sandbox_real_a_sandbox_stopped_under_an_attached_client_comes_back_without_user_action`
  (behind `sandbox-real-runtime`): `bring_up`, attach, `docker stop` from outside, then the client's
  sequence `container_lost` -> `service_absent` and its wait -> `bring_up` -> dial, never `restart`
- compile: `scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime --test sandbox_real_lifecycle --no-run` -> exit 0
- one run against the host's existing `micold-daemon:dev`: -> `panicked at
  crates/micold-core/tests/sandbox_real_lifecycle.rs:623:10: the client could not attach to a sandbox it
  had just brought up` (33s). Not evidence either way: that image belongs to another branch —
  `sandbox_real_handshake_succeeds_with_the_mounted_token` fails against it with
  `VersionMismatch { client: 10, daemon: 11, .. daemon_build: "micold-daemon 0.13.0" }`
- blocked on: `mise run image && mise run test-sandbox`, which replaces the tag every worktree shares

## Gates after Phase 19

- `cargo fmt --all` (reformatted the new code), then `cargo fmt --all -- --check` -> clean
- `scripts/build-lock.sh cargo clippy --workspace --all-targets -- -D warnings` -> exit 0; with
  `-p micold-core --features sandbox-real-runtime --tests` -> exit 0
- `scripts/build-lock.sh cargo test --workspace` -> 2956 passed, 0 failed, 2 ignored (191s)
- open: T178's visual re-run, T181's real-runtime run, T186's commit

## T181: the real-runtime recovery test — run to a verdict against a rebuilt image

- image: `mise run image` -> `micold-daemon:dev` = `sha256:5bc77699…`; the same ID before and after
  every run below, so no other worktree swapped it mid-run
- first `mise run test-sandbox` -> 25 passed, 1 failed: this test, at the first attach, `the client
  could not attach to a sandbox it had just brought up`; a rerun with the dial diagnostics added ->
  `(nothing listening)`, empty `docker logs`
- cause, in the test: `argv::create` publishes `127.0.0.1:{p}:{p}`, while the image sets
  `MICOLD_LISTEN_ADDR=0.0.0.0:7727` (`server.rs:49`). The test's port 17808 was published to a port
  nothing listens on. The application always uses `DEFAULT_SANDBOX_PORT` (7727), so it is unaffected;
  the test must not use 7727 (a developer's own sandbox may hold it)
- test change, before any production change: a test-local `ControlPublishedToTheImage` runner wraps
  `SystemRunner` and rewrites only that publish to `127.0.0.1:17808:7727`, which is what
  `sandbox_real_support` in the daemon crate does by hand. Nothing else in the argv changes
- run: `scripts/build-lock.sh cargo test --release -p micold-core --features sandbox-real-runtime --test sandbox_real_lifecycle -- --exact sandbox_real_a_sandbox_stopped_under_an_attached_client_comes_back_without_user_action`
  -> `1 passed; 0 failed` (13.11s); no leftover `micold-real-comes-back` container
- deliberate mutant 1, `lifecycle.rs` `Adoption::Start(id) => id` (skip the start): **survived**
  (`1 passed`, 12.89s). That branch is not on this path: the image carries no `io.micold.fingerprint`
  label, so a `LocalBuild` re-bring-up always takes `Adoption::Replace`
- deliberate mutant 2, `lifecycle.rs` `Adoption::Replace` creating without starting: **killed** ->
  `panicked at crates/micold-core/tests/sandbox_real_lifecycle.rs:657:13: all 3 unattended bring-ups
  were spent and the client never attached again; the last attempt ended as Failed(Failure { stage:
  Starting, error: SandboxStopped { name: "micold-real-comes-back" } }) (FR-036a)` (122.40s)
- restored from a snapshot, `sha256sum -c` -> OK; rerun -> `1 passed` (11.96s)
- gates: `cargo fmt --all -- --check` -> clean; `cargo clippy -p micold-core --features sandbox-real-runtime --tests -- -D warnings` -> exit 0
- not covered: re-adopting a stopped container through `Adoption::Start`, which a `:dev` image never reaches
- reported, not fixed (out of scope): (1) nothing in the repo writes the `io.micold.fingerprint`
  label that `parse.rs:138` reads, so `adopt` replaces every existing `LocalBuild` container, running
  or not, on each bring-up; (2) `argv::create`'s `{p}:{p}` publish works only when `p` equals the
  image's fixed listen port 7727
- commit: not committed

## Cycle 13: U24 a container found stopped is brought up without showing a failure (T187, T178 visual finding 1)

- test: `main.rs::tests::a_container_found_stopped_is_brought_up_without_showing_a_failure`
- red (real, no mutant): `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide a_container_found_stopped_is_brought_up_without_showing_a_failure`
  -> `panicked at crates/micold-client/src/main.rs:3194:9: a stopped container with attempts left is brought up at once (FR-036a)` (1 failed)
- green: `daemon_sync.rs` gains `bring_up_again(app)`, the unattended bring-up decision taken out of
  `refused_dial` unchanged; `shell/sandbox.rs` `Msg::Lost` calls it when `container_lost` changed the
  state, so `Failed(SandboxStopped)` becomes `Probing` in the same update and is never drawn.
  `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide` -> 131 passed, 0 failed
- refactor: the extraction above was the refactor; `refused_dial` calls the helper
- commit: see git history

## Cycle 14: U25 a started sandbox whose service has not answered is not a lost connection (T188, T178 visual finding 2)

- test: `main.rs::tests::a_started_sandbox_whose_service_has_not_answered_yet_is_not_a_lost_connection`
- red (real): `... --bin micold-ai-ide -- --exact tests::a_started_sandbox_whose_service_has_not_answered_yet_is_not_a_lost_connection`
  -> `panicked at crates/micold-client/src/main.rs:3250:9: assertion left != right failed: the service was started a moment ago and has not been dialled since (FR-036b)  left: Disconnected  right: Disconnected` (1 failed)
- green (fake it): `Sandbox::awaiting_service`, set by `started()`; `Sandbox::is_coming_up()` =
  `state.is_coming_up() || awaiting_service`; `connection_status` reads it. -> 132 passed.
  Hides the banner for ever after a start; the next three cycles bound it
- notes: the first suite run after this change was killed (exit 137, memory pressure) before a result;
  rerun detached -> 132 passed
- refactor: none
- commit: see git history

## Cycle 15: U26 a started service that answered and went away is a lost connection (T188)

- test: `main.rs::tests::a_started_service_that_answered_and_went_away_is_a_lost_connection`
- red: `... -- --exact tests::a_started_service_that_answered_and_went_away_is_a_lost_connection`
  -> `panicked at crates/micold-client/src/main.rs:3292:9: assertion left == right failed: a service that answered is no longer coming up; its disconnect is reported (FR-027)  left: Connected  right: Disconnected` (1 failed)
- green: `Sandbox::answered()`, called from `on_connected`, ends the wait. -> 133 passed
- refactor: none
- commit: see git history

## Cycle 16: U27 a started service that keeps refusing is reported (T188)

- test: `main.rs::tests::a_started_service_that_keeps_refusing_is_reported`
- red: `... -- --exact tests::a_started_service_that_keeps_refusing_is_reported`
  -> `panicked at crates/micold-client/src/main.rs:3315:9: assertion left == right failed: a service still refusing after its grace is not coming up any more (FR-027)  left: Connected  right: Disconnected` (1 failed)
- green: `Sandbox::service_refused()`, called at the top of `refused_dial`, ends the wait. -> 134 passed
- refactor: none
- commit: see git history

## Cycle 17: U28 the first refused dial after start is the service still starting (T188)

- test: `main.rs::tests::the_first_refused_dial_after_start_is_the_service_still_starting`
- red: `... -- --exact tests::the_first_refused_dial_after_start_is_the_service_still_starting`
  -> `panicked at crates/micold-client/src/main.rs:3341:9: assertion left != right failed: one reconnect is how long a started daemon may take to listen (FR-036b)  left: Disconnected  right: Disconnected` (1 failed)
- green (triangulated from U27): `awaiting_service: Option<u8>`, `started()` sets
  `Some(REFUSALS_WHILE_STARTING)` (= 1), `service_refused` spends one and ends at the last; `refused_dial`'s
  silence check reads `app.sandbox.is_coming_up()` instead of the bare state. -> 135 passed
- refactor: none
- commit: see git history

## Cycle 18: U29 a failure after start is not still waiting for the service (T188)

- test: `main.rs::tests::a_failure_after_start_is_not_still_waiting_for_the_service`
- first draft set the budget spent with a constructor that does not exist; `Started` resets the budget,
  so that path is unreachable. Rewritten to the reachable one before any run: `Started` -> `Lost`
  (brings it up) -> `Failed`
- red: `... -- --exact tests::a_failure_after_start_is_not_still_waiting_for_the_service`
  -> `panicked at crates/micold-client/src/main.rs:3379:9: assertion left == right failed: a failed sandbox is not coming up, whatever it was waiting for before (FR-036b)  left: Connected  right: Disconnected` (1 failed)
- green: `Sandbox::is_coming_up` honours the wait only while the state is `Running`. -> 136 passed
- refactor: `a_started_sandbox()` fixture shared with `a_sandbox_that_came_up_earns_its_unattended_bring_ups_back`,
  which built the same `Started` inline; `cargo fmt --all`. -> 136 passed (in the gates below)
- commit: see git history

## Gates after Cycles 13–18 (T187, T188)

- `scripts/build-lock.sh cargo test --workspace` -> 2962 passed, 0 failed, 2 ignored
- `cargo clippy --workspace --all-targets -- -D warnings` first failed on `clippy::collapsible_if` in
  `daemon_sync.rs::refused_dial` (the `LocalSandbox` / `is_coming_up` nesting); collapsed with no
  behavior change -> clean
- `cargo fmt --all -- --check` -> clean
- `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide` after the collapse -> 136 passed

## Test repair: U26's connect no longer marks the sandbox `Stale` (T189, re-audit finding 1)

- test: `main.rs::tests::a_started_service_that_answered_and_went_away_is_a_lost_connection`
- why: the re-audit's mutant N3 (drop `app.sandbox.answered();` in `on_connected`) survived. The fixture's
  catalog (`/repo/demo`) differed from the boot plan's empty `projects`, so `adopt_mount_set` marked the
  sandbox `Stale`, and a stale sandbox was not coming up whether the service had answered or not
- red (setup assertion added first, fixture unchanged): `... -- --exact tests::a_started_service_that_answered_and_went_away_is_a_lost_connection`
  -> `panicked at crates/micold-client/src/main.rs:3289:9: setup: the sandbox that answered is running — a stale one is not coming up either, and would pass this test without the answer ending the grace` (0 passed; 1 failed)
- green: `the_service_answers` connects with `CatalogSnapshot::default()`, matching the plan. The assertion
  under test is unchanged. -> 136 passed
- proof: N3 re-applied -> `panicked at crates/micold-client/src/main.rs:3302:9: assertion left == right failed: a service that answered is no longer coming up; its disconnect is reported (FR-027)  left: Connected  right: Disconnected`; restored, sha verified
- refactor: the setup message reworded once U30 made "a stale one is not coming up" untrue; the helper
  split into `the_service_answers` / `the_service_answers_with(catalog)` for U31
- commit: not committed

## Deliberate-mutant red for U4, which passed on its first run (T190, re-audit finding 2)

- test: `sandbox_state::absence_only_brings_up_a_sandbox_that_has_failed`
- mutant M3: `SandboxState::Failed(_) | SandboxState::Stale(_)` in `lifecycle.rs` `service_absent`
- red: `scripts/build-lock.sh cargo test -p micold-core --test sandbox_state`
  -> `panicked at crates/micold-core/tests/sandbox_state.rs:552:9: assertion left == right failed: Stale(ContainerId("9f2b")) was brought up again on a failed connection  left: Some(BringUpAgain { state: Probing, budget: UnattendedBringUps { spent: 1 }, after: 0ns })  right: None` (19 passed; 1 failed)
- restored from snapshot, sha verified; `... --test sandbox_state` green again in the gates below
- notes: this is the playbook's red for a test that passes first time, taken after the fact. The test was
  still written after its implementation; whether this lifts U4 out of `TEST_AFTER` is the next audit's call
- commit: not committed

## Cycle 19: U30 a started sandbox marked `Stale` before its service answered is still coming up (T193)

- test: `main.rs::tests::a_started_sandbox_marked_stale_before_its_service_answered_is_still_coming_up`
- red (real): `... -- --exact tests::a_started_sandbox_marked_stale_before_its_service_answered_is_still_coming_up`
  -> `panicked at crates/micold-client/src/main.rs:3418:9: assertion left != right failed: an out-of-date container is still up, and its service still starting (FR-036b)  left: Disconnected  right: Disconnected` (1 failed)
- green: `Sandbox::is_coming_up` honours the wait for `Running | Stale`. -> 137 passed
- proof: N11 (back to `Running` only) fails this test at `main.rs:3461`; restored, sha verified
- refactor: none
- notes: **deviates from T193's wording.** T193 says a sandbox that goes `Stale` in the grace is *not* coming
  up. Two routes reach `Stale`. The keep-running opt-in saved during the start (`survive_logout_changed`)
  leaves the container up with its service still starting, so FR-036b says it is still coming up, and that is
  what U30 pins. A catalog that differs is only seen once the service has answered (`on_connected` calls
  `answered()` before `adopt_mount_set`), so that route is past the grace. U31 pins it as T193 asked
- commit: not committed

## Cycle 20: U31 a service that answered with a changed mount set and went away is a lost connection (T193)

- test: `main.rs::tests::a_service_that_answered_with_a_changed_mount_set_and_went_away_is_a_lost_connection`
- passed on its first run (a pin: Cycle 15's `answered()` already covers it). Deliberate-mutant red, N3
  (drop `app.sandbox.answered();`): `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`
  -> `panicked at crates/micold-client/src/main.rs:3345:9: assertion left == right failed: an out-of-date service that answered is not coming up; its disconnect is reported (FR-027)  left: Connected  right: Disconnected` (136 passed; 2 failed, with U26)
- restored, sha verified -> 138 passed
- refactor: none
- commit: not committed

## Test strength: A1 and U24 name the bring-up, not "some task" (T191, re-audit finding 3)

- tests: `a_failed_sandbox_the_client_cannot_reach_is_brought_up_again`, `a_container_found_stopped_is_brought_up_without_showing_a_failure`
- seam: a `Task` cannot be read back (iced 0.14 does not re-export `into_stream`, and no dependency was
  added). `BringUp::task` records the bring-up in a `#[cfg(test)]` thread-local, and `BringUp::scheduled()`
  takes the list. Each test runs on its own thread, so the lists do not mix
- red (`scheduled()` declared, `task()` not yet recording; `units() > 0` replaced by `scheduled().len() == 1`):
  `... -- tests::a_failed_sandbox_the_client_cannot_reach_is_brought_up_again tests::a_container_found_stopped_is_brought_up_without_showing_a_failure`
  -> `panicked at crates/micold-client/src/main.rs:3024:9: assertion left == right failed: the state says a bring-up started, so one has to be scheduled — ...  left: 0  right: 1`
  and `panicked at crates/micold-client/src/main.rs:3195:9: assertion left == right failed: a stopped container with attempts left is brought up at once (FR-036a)  left: 0  right: 1` (0 passed; 2 failed)
- green: `task()` pushes a clone before building the stream. -> 137 passed
- proof: T191a (`daemon_sync.rs` `refused_dial(..).map_or_else(Task::none, |_| Task::done(Message::EscapePressed))`)
  fails A1 at `:3024`, `left: 0 right: 1`. T191b (`Msg::Lost` returns `Task::done(Message::EscapePressed)`) fails
  U24 at `:3195`. Both restored, sha verified. `Task::done` has one unit, so the old `units() > 0` passed both
  (by construction, not re-run)
- commit: not committed

## Refactor: shared bring-up stages, a rule message, no writes to the real client log (T195, re-audit findings 7–8)

- `bring_up_stages()` replaces the three copies of `[Probing, Acquiring(..), Starting]`
- `a_refused_dial_while_the_sandbox_reads_running_is_reported`: the bare `assert_eq!(app.sandbox.state, running)`
  now says why ("a refused dial is no evidence the running container went away (FR-036)")
- `log_line` returns at once under `cfg(test)`. Count of `attach: failed reason=Connection refused (os error 111)`
  in `~/.local/share/micold-ai-ide/micold-client.log` before and after a client test run: 1395 and 1395
- suite: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide` -> 138 passed
- commit: not committed

## Note: the commits behind the entries above that name none (T196, re-audit finding 9)

Appended rather than edited in place. The log is append-only.

- Cycles 1–12, the Phase 18 record, the structural step, the T182–T185 refactors and T181's first entry say
  "not committed". They landed together in `eb3edf21` (fix(027): bring a missing or stopped sandbox up without
  user action). Their evidence landed in `cd7fc151`.
- Cycles 13–18 and their gates say "see git history". They are `f8f12e4a` (fix(027): no failure card or banner
  around a sandbox bring-up), with this log's entries in the same commit.
- One commit per behavior was not kept. Ordering inside each commit rests on this log alone.

## Gates after Phase 20 (T189–T196)

- `scripts/build-lock.sh cargo test --workspace` -> 2964 passed, 0 failed, 2 ignored
- `cargo clippy --workspace --all-targets -- -D warnings` -> clean
- `cargo fmt --all -- --check` -> clean

## Test strength: A1 and U24 require the bring-up to be handed back (T197, third-audit finding 1)

- tests: `a_failed_sandbox_the_client_cannot_reach_is_brought_up_again`, `a_container_found_stopped_is_brought_up_without_showing_a_failure`
- red (before the change, from the third audit at `6b802791`): N13 (`let _ = bring_up.task(); return iced::Task::none();`
  at `shell/sandbox.rs:430`) left U24 passing. `scheduled()` records at build time, so a bring-up built and dropped
  satisfied `scheduled().len() == 1`
- change: both tests keep the update's returned work and assert `work.units() == 1` after the `scheduled()` assert.
  Test-only; no production change
- proof (`python3 mutants.py T191a T191b N12 N13`, each restored, sha verified):
  N13 fails U24 at `main.rs:3198` "the bring-up has to be handed back to run — one built and dropped leaves `Probing`
  with nothing running, which is BUG-005 left: 0 right: 1"; N12 fails A1 at `main.rs:3035` "the bring-up has to be
  handed back to run, not built and dropped left: 0 right: 1"; T191a still fails A1 (`:3029`), T191b still fails U24 (`:3193`)
- commit: not committed

## Spec and plan record the start grace for `Stale` (T198, third-audit finding 2)

- `spec.md`: bugfix note 2026-09-14 under FR-036b. A `Stale` reached before the first answer is still coming up; a
  `Stale` found on connect is past the bring-up (FR-027). Pinned by U30 and U31; T193's text left as written
- `plan.md`: bugfix note 2026-09-14. `Sandbox::is_coming_up` counts `Running | Stale` from `Started` to the first
  answer, and `on_connected` calls `answered()` before `adopt_mount_set`
- no code or test change
- commit: not committed

## U30 reaches `Stale` through the settings save (T199, third-audit finding 3)

- test: `a_started_sandbox_marked_stale_before_its_service_answered_is_still_coming_up`
- change: setup replaced `app.sandbox.survive_logout_changed()` with `SettingsMsg::Opened`,
  `SettingsMsg::SurviveLogoutToggled(true)` and `SettingsMsg::Saved` through `update_inner`. Assertions unchanged
- deviation: the first version ran on `Capabilities::real()` and wrote `~/.local/share/micold-ai-ide/settings.json`
  (mtime 12:12:51, `survive_logout: true`). Fixed with `app.caps = Capabilities::real().without_settings()`; the
  re-run left the mtime at 12:12:51. Without a store, `survival_before` is `false`, so the save still takes
  `SurvivalStep::Enable`
- proof (`python3 mutants.py N11 T199a`, each restored, sha verified):
  N11 (`Running(_) | Stale(_)` narrowed to `Running(_)`) fails U30 at `main.rs:3473` "an out-of-date container is still
  up, and its service still starting (FR-036b) left: Disconnected right: Disconnected"; T199a (removes
  `app.sandbox.survive_logout_changed();` from `shell/persist.rs`) fails U30's setup at `main.rs:3466` "setup: the
  settings saved during the start marked the running sandbox out of date" — so the save route is what the test drives
- not fixed, out of scope: `settings_saved_sends_settings_set_to_a_connected_daemon` and
  `settings_saved_is_a_silent_no_op_toward_the_daemon_when_disconnected` save on `base_app()`'s real store and rewrite
  the developer's `settings.json` on every client test run (seen at 12:16:53)
- commit: not committed

## Gates after Phase 21 (T197–T199)

- `scripts/build-lock.sh cargo test --workspace` -> 2964 passed, 0 failed, 2 ignored (116s)
- `cargo clippy --workspace --all-targets -- -D warnings` -> clean
- `cargo fmt --all -- --check` -> clean

## Test strength: A1 and U24 tie the returned work to the bring-up (T200, fourth-audit finding 1)

- tests: `a_failed_sandbox_the_client_cannot_reach_is_brought_up_again`, `a_container_found_stopped_is_brought_up_without_showing_a_failure`
- seam: under `cfg(test)`, `BringUp::task()` puts an `Arc<()>` token inside the task's stream and keeps a `Weak` in a
  thread-local. `BringUp::live_tasks()` counts the tokens still held. A test holding `work` sees the bring-up only if the
  update handed it back; one dropped inside the update drops its token. No dependency added
- red (`live_tasks()` declared as a stub returning 0; both tests gained `assert_eq!(BringUp::live_tasks(), 1, ..)`, no
  assertion removed):
  `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- tests::a_failed_sandbox_the_client_cannot_reach_is_brought_up_again tests::a_container_found_stopped_is_brought_up_without_showing_a_failure`
  -> `panicked at crates/micold-client/src/main.rs:3040:9: ... the work handed back has to be the bring-up itself, not other work beside a dropped one  left: 0  right: 1`
  and `panicked at crates/micold-client/src/main.rs:3209:9: ... one swapped for other work leaves `Probing` with nothing running, which is BUG-005  left: 0  right: 1` (0 passed; 2 failed)
- green: the token and the count. -> 138 passed; serial (`--test-threads=1`) 138 passed
- proof (`python3 mutants.py X1 X2 N12 N13 T191a T191b`, each restored, sha verified): X1 fails U24 at `main.rs:3209`;
  X2 fails A1 at `main.rs:3040`; N12 still fails A1 (`:3035`), N13 still fails U24 (`:3203`), T191a still fails A1
  (`:3029`), T191b still fails U24 (`:3198`)
- refactor: `cargo fmt` only
- commit: not committed

## Gates after Phase 22 (T200)

- `scripts/build-lock.sh cargo test --workspace` -> 2964 passed, 0 failed, 2 ignored (56s)
- `cargo clippy --workspace --all-targets -- -D warnings` -> clean
- `cargo fmt --all -- --check` -> clean

## Test strength: A1 and U24 run the work the update returns (T201, fifth-audit finding 1)

- tests: `a_failed_sandbox_the_client_cannot_reach_is_brought_up_again`, `a_container_found_stopped_is_brought_up_without_showing_a_failure`
- dependency (approved by the user 2026-09-14): `iced_runtime = "=0.14.0"` as a `micold-client` dev-dependency, for
  `iced_runtime::task::into_stream`. Already in `Cargo.lock` at `iced`'s version; the lock gains one dependency-list line
- seam: under `cfg(test)`, `BringUp::task()` drives `RecordingRunner` instead of `SystemRunner`, so running a returned
  bring-up in a test never reaches the host's container runtime. Release builds are unchanged
- test change: both tests take their boot plan's `state_dir` from a `tempdir` (running the bring-up writes a token
  there), and gain `assert!(is_probing(first_message(work).as_ref()), ..)` after the `live_tasks()` assertion.
  `first_message` runs the work's stream on a tokio runtime until its first `Action::Output`. No assertion removed
- red: none of its own — the production code already delivers the stages, so the new assertion passed on first run.
  Deliberate-mutant check instead (`python3 mutants.py X3 X4 X5 X6 X1 X2 N12 N13 T191a T191b`, each rebuilt, restored,
  sha verified):
  X3 (`iced::Task::stream(stream).discard()` in `task`) fails A1 at `main.rs:3085` and U24 at `main.rs:3262`;
  X5 (`return bring_up.task().discard();`) fails U24 at `main.rs:3262` "... one whose messages are discarded starts the
  container while the view stays on `Probing`, which is BUG-005: None";
  X6 (`map_or_else(Task::none, |b| b.task().discard())`) fails A1 at `main.rs:3085` "... starts a sandbox the view never
  hears of, which is BUG-005: None";
  X4 (`mem::forget` the task, return `Task::done(EscapePressed)`) fails U24 at `main.rs:3262` "...: Some(EscapePressed)".
  Still caught where they were: X1 U24 `:3255`, X2 A1 `:3079`, N12 A1 `:3074`, N13 U24 `:3249`, T191a A1 `:3068`,
  T191b U24 `:3244`
- green: 2 passed targeted; client 138 passed
- refactor: none. The `live_tasks()` token is now subsumed by the new assertion but was left in place: removing an
  assertion is a test change of its own, not part of this cycle
- commit: not committed

## Gates after Phase 23 (T201)

- `scripts/build-lock.sh cargo test --workspace` -> 2964 passed, 0 failed, 2 ignored (165s)
- `cargo clippy --workspace --all-targets -- -D warnings` -> clean
- `cargo fmt --all -- --check` -> clean

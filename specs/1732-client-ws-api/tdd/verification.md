---
feature: 1732-client-ws-api
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved: no override or preset copy exists
verified_at: 36bd9e028 # HEAD; feature range fbfa8360e..8a2eaba41 (M1-M6), no feature file changed between 8a2eaba41 and HEAD
behaviors: 29 # one per test-bearing task (no test-list.md exists), see the table
proven: 0
likely: 5
test_after: 23
no_test: 1
high_smells: 0 # catalogue HIGH smells; two literal matches graded MED with reasons, see F9
criteria_total: 18 # acceptance scenarios US1-US5
criteria_covered: 14
mutation_score: unmeasured # no cargo-mutants / stryker installed; deliberate mutants were not run (see Test strength)
mutants_survived: n/a
suite: feature-scoped only. core lib client_api 33 passed; core tests/main client_api 5 passed; daemon tests/orchestrator ws_ + read_structured 42 passed, 0 failed, 2 ignored (Postgres, unrelated), 2.3 s test time; jest (apps/client, 3 projects) 222 passed, 37 suites, 28 s; packages/protocol jest 77 passed; scripts/tests api-types, local-stack, local-stack-kind, local-stack-probe, dockerfile-web pass; chart-local SKIPPED (no helm). Full workspace suite and gate not run.
---

# TDD Verification: JSON-over-WebSocket client API and a working web client (#1732)

**Verdict: FAIL.** The feature has no test list and no cycle log, and its history is squashed to one commit per milestone, so 23 of 29 behaviors have no recorded red (`TEST_AFTER` under the rubric's fail-closed rule). Two spec requirements are also untested through the real entry point (F2, F3).

The tests that exist are good: they run against a real listener and real linked daemons, assert exact values, and the suites I ran are green and fast. The failure is about evidence and about two coverage holes, not test quality.

Independence: I did not write these tests. I read each file cold. I did not use a subagent for the smell pass; I read the daemon `ws_*` and `read_structured_events` tests, the TypeScript `real/*`, `order-vectors`, `SessionContent`, the changed parts of `ConnectionBanner`/`select`/`ApiProvider`/`layout`/`catalog`, `client_api_schema`, `client_api_vectors`, `classify`/`log`/`diff`/`content` unit tests and `stack.spec.ts` myself. Files not read are listed under "What was not audited".

## Test-first evidence

Evidence sources: `tdd/cycle-log.md` absent; `tdd/test-list.md` absent; git history is 6 squash-merge commits for the feature (#2033, #2034, #2037, #2038, #2039, #2040) plus one unrelated fix (#2036/a20aba7e2) in the range. History cannot show ordering inside any milestone. The only recorded reds are the "Red runs" table in `specs/1732-client-ws-api/autopilot.md` (5 rows, all M5) and the ledger line "F1 test proven red without the fix" (M4 fix review).

Behaviors are the test-bearing tasks of `tasks.md` (the nearest thing to a list). `LIKELY` = a red is recorded but order cannot be corroborated.

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| T044 `local-stack-kind.test.sh` | LIKELY | autopilot.md Red runs: 14 FAIL, `mktemp: command not found` |
| T045 `chart-local.test.sh` | LIKELY | Red runs: `ci/local.yaml` missing, old `#1740` message |
| T046 `a_given_data_dir_skips_the_users_daemon_toml` | LIKELY | Red runs: E0425 `implicit_config` not found |
| T053 `dockerfile-web.test.sh` | LIKELY | Red runs: no `Dockerfile.web` |
| T054 `local-stack-probe.test.sh` | LIKELY | Red runs: 3 FAIL, probe script missing |
| T003 `read_structured_events.rs` | TEST_AFTER | no red recorded; M1 squashed (10f82e095) |
| T005 `client_api/messages.rs` tests | TEST_AFTER | no red recorded; M2 squashed (a1c9514b3) |
| T006 `classify.rs` tests | TEST_AFTER | same |
| T007 `log.rs` tests | TEST_AFTER | same |
| T008 `diff.rs` tests | TEST_AFTER | same |
| T009 `origin.rs` tests | TEST_AFTER | same |
| T010 `ws_read_session.rs` | TEST_AFTER | same |
| T012 `ws_origin.rs` | TEST_AFTER | same |
| T013 `ws_protocol.rs` | TEST_AFTER | same |
| T014 `ws_liveness.rs` (planned `ws_heartbeat.rs`) | TEST_AFTER | same; rename documented in autopilot.md |
| T015 `ws_resume.rs` | TEST_AFTER | same |
| T018 `ws_listener.rs` | TEST_AFTER | same |
| T020 `client_api_schema.rs` | TEST_AFTER | no red recorded; M3 squashed (a7d7a3c12) |
| T022 `api-types.test.sh` | TEST_AFTER | same |
| T024/T025 `client_api_vectors.rs`, `order-vectors.test.ts` | TEST_AFTER | same |
| T027 `BrowserTransport.test.ts` | TEST_AFTER | no red recorded; M4 squashed (5ebc01334) |
| T028 `RealOrchestratorApi.test.ts` | TEST_AFTER | same (M4 review F1 fix test: "proven red without the fix", one test only, not named) |
| T029 `SessionContent.test.tsx`, banner tests | TEST_AFTER | same |
| T030 `catalog.test.ts` additions | TEST_AFTER | same |
| T036 `local-stack.test.sh` | TEST_AFTER | same |
| T040 port-conflict cases in `local-stack.test.sh` | TEST_AFTER | no red recorded; M6 squashed (8a2eaba41) |
| T038 `stack.spec.ts` | TEST_AFTER | same; also deviates from the task (F3) |
| T052 `local-stack-kind-live.sh` + CI job | NO_TEST | never executed anywhere yet (task correctly left `[ ]`) |

`tasks.md` vs evidence: no test list exists, so "ticked but not DONE" cannot be checked mechanically. T052 is the only unticked task and is honest. T038 is ticked `[x]` although its note records that the specified live-change scenario was not delivered (F3).

### What the change did to tests that already existed

No assertion was found weakened in a way the spec does not authorize. Each change, with before and after:

| File:line | Before | After | Judgment |
| --- | --- | --- | --- |
| `crates/micold-core/tests/protocol_roundtrip.rs:997` | test `the_protocol_version_is_81` asserts 83 | renamed `the_protocol_version_is_the_one_the_schema_hash_pins`, asserts 84 | Required by T004. Old name was already stale. No filter in the repo names the old test (grep). |
| `crates/micold-core/tests/schema_hash.rs:352` | `FEATURE_026_PROTOCOL_VERSION = 83` | `= 84` | Required by T004 |
| `apps/client/tests/api/select.test.ts:26` | real mode throws `REAL_CLIENT_MISSING` | real mode builds the real client, asserts the default and configured URLs | FR-013 behavior change; stronger, not weaker |
| `apps/client/tests/components/ConnectionBanner.test.tsx:23` | `toHaveTextContent("Offline. The orchestrator cannot be reached.")` | two `toHaveTextContent` regexes for the new title and body | New copy (FR-014/US1.4). Equivalent strength. |
| `apps/client/tests/api/ApiProvider.test.tsx`, `tests/app/layout.test.tsx` | "no real client built" state | `selectApi` spied to throw "no client" | The state no longer exists naturally; the unavailable screen is still pinned |
| `scripts/tests/chart-conformance.test.sh:197` | `postgres` and `sqlite` must fail, naming `#1948` and `#1740` | only `sqlite` must fail, with the new exact message | FR-020 behavior change. Postgres rendering now relies on `chart-local.test.sh`, which was skipped in my run (no helm). |
| `crates/micold-daemon/tests/orchestrator/store_scan.rs:197` | cases kept to end of test | each case in its own block | Belongs to #2035/#2036, outside the feature, no assertion changed |
| `crates/micold-daemon/tests/orchestrator_route/main.rs`, `tests/support/history.rs` | `Daemon` helper inline; `service()` builds catalog | helper moved to `support/linked_daemon.rs`; `catalog()` extracted | Refactor only; reuse, not duplication |
| `packages/protocol/tests/api.test.ts`, `fakes.ts` | stub lacks new members | adds `onSnapshot`, `readSession`, `retry`, close `code/fatal`, `onVisible` | Additive |
| `crates/micold-client/tests/kube_real_support/mod.rs:36` | `kindest/node:v1.34.0` | `v1.37.0@sha256:...` | Pin bump (T043), no test logic change |

No skip, `.only`, `#[ignore]`, `it.todo`, config exclusion or lowered threshold was added by the feature (grep of the feature's test files and `ci.yml`/`gate.sh` diffs). The `ci.yml` change to the llvm-cov step drops unreadable `.profraw` files and keeps the report: it removes counters for the unreadable test only, and it warns. It is out of feature scope and worth owner attention (follow-up already recorded in autopilot.md).

## Findings

> Close PR update: F2 fixed by `ws_protocol::a_dropped_daemons_sessions_stay_listed_as_unreachable_until_it_relinks` (drop, rows kept unreachable with their location, re-link, reachable again; proven red with `directory.rs` `reachable: false` flipped). The daemon's `list_daemons` status still reads `online` after the drop (ledger follow-up). Other findings unchanged.

Ordered by severity. IDs are the remediation items.

| # | Sev | Crate / area | Test file | Finding | Evidence |
| --- | --- | --- | --- | --- | --- |
| F1 | HIGH | process (all crates) | no `tdd/test-list.md`, no `tdd/cycle-log.md` | No test-first evidence for 23 of 29 behaviors. Reds exist only for the five M5 shell/chart/unit tests, self-reported in `autopilot.md`. History is squashed per milestone. The tasks file claims "written first and seen failing (Constitution I)". Nothing verifies that for M1-M4 and M6. | `specs/1732-client-ws-api/autopilot.md` "Red runs"; `git log fbfa8360e..8a2eaba41` (7 commits) |
| F2 | HIGH | micold-daemon | `tests/orchestrator/ws_resume.rs`, `ws_protocol.rs` (missing test) | FR-014a and the spec edge case "a daemon disconnects while a client is subscribed: sessions shown unreachable, not removed, recover on reconnect" have no test through `/api/ws` against a real daemon. The code that does it is `orchestrator/directory.rs:222-310` (previous-snapshot carry-over with `reachable: false`). `tests/support/linked_daemon.rs` provides `Daemon::drop_link` but no `ws_*` test calls it. The only `reachable: false` coverage is synthetic snapshots in `client_api_vectors.rs` and jest. | grep `reachable\|drop_link` in `tests/orchestrator/ws_*.rs`: only `ws_protocol.rs:40` (`reachable == true`) |
| F3 | HIGH | micold-daemon, apps/client | `tests/orchestrator/ws_resume.rs`; `apps/client/e2e/stack/stack.spec.ts` | US1.3 / SC-006 (a session created, renamed or ended on the daemon shows within 2 s) has no test with a real daemon. `tasks.md` T038 says "covered by `tests/orchestrator/ws_*.rs`". The files do not support that: the 2 s test (`ws_resume.rs:189`) covers a daemon linking, and the other change tests drive `Scripted::add_daemon`. Session state changes are covered only at unit level (`diff.rs` tests). The browser test covers only stop/restart of the whole stack. T038 is ticked `[x]` with the scenario undelivered. | `ws_resume.rs:75-87,189-207`; `stack.spec.ts:7-10`; `tasks.md:114` |
| F4 | MED | micold-daemon, deploy | `scripts/local-stack-kind-live.sh` (CI job `local-stack-kind`); `scripts/tests/chart-local.test.sh` | US5.1, US5.5 and SC-008 (cluster comes up, session listed, state survives an orchestrator pod restart) exist only as a live script that has never run (T052 open). US5.4 (chart renders Postgres, local values) is a test I could not run (SKIP, no helm). The kind and process stubs tests pass. | `tasks.md:109`; `chart-local.test: SKIP: helm is not on PATH` |
| F5 | MED | micold-core | `tests/client_api_vectors.rs:~92-140` | Self-approving expectations. `expected_keys`, `expected_sessions`, `expected_daemons` for three of the four vector cases are computed by the test's own `row_keys`/`diff`/`apply` and written to the file under `MICOLD_WRITE_VECTORS=1`; only the grouping case has a hand-written order (`the_grouping_case_expects_the_hand_written_order`). The TypeScript reducer is an independent implementation, which limits the damage. FR-012 says "same result as the Rust rules", but neither side is compared with the existing Rust sidebar order (`crates/micold-core/src/sidebar_layout.rs` and the client's stable order); the vectors compare two new implementations. | `client_api_vectors.rs:81-140`; `spec.md` FR-012; `plan.md:17` |
| F6 | MED | micold-daemon | `tests/orchestrator/ws_liveness.rs:27-30` | Pins a constant against itself (`assert_eq!(IDLE, 40s)`). Every behavior test overrides `ApiOptions.idle` (and `handshake`, `max_sockets`), so nothing proves `ApiOptions::default()` is what the listener started by `run` uses. Changing `ws.rs:65` (`idle: IDLE`) to another value would survive. Predicted survivor, not measured. | `ws.rs:62-70`; `ws_liveness.rs:34,50`; `ws_admission.rs:30,40` |
| F7 | MED | micold-daemon | `tests/orchestrator/ws_resume.rs:80,168,244-263`; `ws_liveness.rs` | Fixed sleeps (80 ms, 120 ms, 60 ms) and a timing-dependent lag test (`SlowRead` sleeps 600 ms against `backlog: 1`) decide ordering. They pass now (2.3 s total) but will flake on a loaded runner. The 2 s bound in `a_daemon_linking_appears_within_two_seconds` uses the real clock. | `ws_resume.rs:80,168,233,258`; `autopilot.md` M2 round 2 "timing-based lagged test" |
| F8 | MED | apps/client | `tests/components/ConnectionBanner.test.tsx:29-36,38-50` | (a) "asks the API to connect again" asserts only that the banner is gone after pressing Retry on the mock; it does not assert `retry` was called. (b) The class-based case builds its object with `Object.create`/`Object.assign` and passes `control: undefined as never`, bypassing `renderWithApi`'s contract, and asserts a counter on its own double. It does pin the `this` binding bug found by the visual pass, but through a contrived double. | file lines as cited |
| F9 | MED | apps/client, micold-daemon | `e2e/stack/stack.spec.ts:57-62`; `ws_resume.rs:153-165,157` | Conditional logic in tests. `stack.spec.ts:61` `if (await retry.isVisible()) await retry.click()` and a poll accepting "banner gone OR Retry visible"; `ws_resume.rs` chooses the read count with `if round == 0 { 5 } else { 10 }` and `if made < 100`. This matches the catalogue's `Conditional logic in the test` (HIGH). I graded it MED because every branch ends in the same hard assertions (banner gone, session visible within 2 s; exact seq list and ids). The owner may reasonably regrade. | as cited |
| F10 | MED | apps/client | `tests/sidebar/real.test.tsx:~36` | Vacuous: `snapshot?.sessions.length` `toBeGreaterThan(0)` after reconnect, where the mock fixture's session set is known. | `real.test.tsx` "loads again when the connection comes up" |
| F11 | MED | apps/client | `e2e/stack/stack.spec.ts:40-45` | US1.2 "content shown, matching what the daemon holds" is checked in the browser only as `session-content` being visible and `session-unreachable` absent. Content matching is proven only at the daemon level (`ws_read_session.rs`), not in the page. | file lines |
| F12 | LOW | micold-daemon | `tests/orchestrator/ws_resume.rs:210-242`, `ws_liveness.rs:91-110`, `src/orchestrator/feed.rs` tests (`SlowFirst`) | Three more hand-written `LinkDirectory` doubles each re-implement all three trait methods beside `Scripted` in `ws_support.rs`. Wrappers over `Scripted` would remove ~60 lines. | cited ranges |
| F13 | LOW | micold-daemon | `ws_protocol.rs`, `ws_origin.rs`, `ws_liveness.rs` | Profile convention: "every assertion carries a message explaining the rule". Most `assert_eq!` here have none (some print `{reply}`). | profile "Conventions to match" |
| F14 | LOW | micold-daemon | `ws_resume.rs:129` | Magic `1100` (ring cap 1024 plus margin) is unexplained; reference the log capacity constant. | line cited |
| F15 | LOW | process | `specs/1732-client-ws-api/tasks.md` T008 | Task says "property test"; `diff.rs` has example-based tests only (the stack has no proptest). The wording overstates. | `diff.rs:204-378`; profile `property: null` |

No tests were found that are assertion-free, tautological about a double's return value, doubled-subject, skipped, or snapshot-approved without a hand check (F5 is the nearest). No secret or credential appears in any file I read.

### Properties

- Isolation and determinism: daemon `ws_*` tests bind port 0 and use temp dirs; they ran in 2.3 s. F7 is the determinism risk. `stack.spec.ts` uses fixed ports 8730/8731 by design (port conflict handled in `local-stack.sh`).
- Speed: feature-scoped suites are fast (daemon 2.3 s, jest 28 s). Rust compile time dominated my run (about 2 min core lib, longer for the daemon test binary).
- Specificity: failure messages mostly print the offending reply (`{reply}`); fine.
- Refactoring insensitivity: the daemon tests drive the wire only. The TypeScript tests reach into a private field once (`RealOrchestratorApi.test.ts:126-127`, `(api as unknown as { clock })`, although `clock` is already destructured by `setup()`); LOW.

## Test strength (mutation and deliberate mutants)

**Measured: nothing.** `cargo mutants` and Stryker are not installed (`cargo mutants --version`: no such command), and the profile's `mise run mutants-diff` is unavailable in this container. I then started deliberate mutants on the highest-risk behaviors, but the sandbox's permission classifier denied the command that edits source files. I did not work around it, so no mutant was run and the tree is untouched (`git status` shows only the untracked `drift-M*.md` files that were already there). Test strength is therefore **unmeasured**, not passing.

Predicted survivors from reading the code against the tests (not measured, listed so the next run starts there):

| Mutant (proposed) | Behavior | Expected | Reason |
| --- | --- | --- | --- |
| `ws.rs:65` `idle: IDLE` to another duration | T014 | survives | F6: only the constant is pinned |
| `ws.rs` default `handshake` / `max_sockets` to another value | T012/T013 hardening | survives | tests override both |
| `log.rs:88` `seq + 1 < oldest` to `<=` | T007 | caught | `n_plus_one_at_the_oldest_held_is_servable` |
| `log.rs:91` `c.seq > seq` to `>=` | T007, T015 | caught | `changes_after_n_are_served_in_order` |
| `origin.rs:80` `peer_is_loopback` to `true` | T009 | caught | `no_origin_is_accepted_from_loopback_only` |
| `ws.rs:141` drop the origin check | T012 | caught | 403 asserts in `ws_origin.rs` |

Coverage was not run (`mise run coverage-diff` needs cargo-llvm-cov, not installed here).

## Traceability

Acceptance scenarios (US1-US5), through the real entry point. "Yes" means a test exercises the real `/api/ws` listener, the real `RealOrchestratorApi`, the real script or the real browser, not only units with doubles at every boundary.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1 list grouped and ordered | `ws_protocol.rs` list tests, `stack.spec.ts` seeded list, `order-vectors.test.ts` | Yes (order via vectors only, F5) |
| US1.2 content shown | `ws_read_session.rs`, `read_structured_events.rs`, `SessionContent.test.tsx`, `stack.spec.ts:40` | Yes at daemon, container-only in browser (F11) |
| US1.3 change shown within 2 s, no reload | `ws_resume.rs:189` (link only), `diff.rs`, `RealOrchestratorApi.test.ts` "applies a changes message" | **No** (F3) |
| US1.4 orchestrator unreachable, translated, retry | `RealOrchestratorApi.test.ts` (offline, Retry), `ConnectionBanner.test.tsx`, `layout.test.tsx`, `stack.spec.ts:47` (mid-session) | Yes (initial-open case in jest only) |
| US2.1 reconnect exact missed changes | `ws_resume.rs:90`, `:142`, `RealOrchestratorApi.test.ts` "resumes with since" | Yes |
| US2.2 too-old or restart gives resync and snapshot | `ws_resume.rs:112,127`, `RealOrchestratorApi.test.ts` "on resync_needed" | Yes |
| US2.3 "reconnecting" indicator, backoff | `RealOrchestratorApi.test.ts` backoff, `ConnectionBanner.test.tsx` | Yes |
| US3.1 allowed origin accepted | `ws_origin.rs:72`, `ws_listener.rs` | Yes |
| US3.2 foreign origin refused and logged | `ws_origin.rs:34-69`, `ws_admission.rs:54` | Yes |
| US3.3 local origin allowed without edits | `stack.spec.ts:31`, `local-stack.test.sh` | Yes in CI (Chromium only by the author; Firefox/WebKit not run here or by the author) |
| US4.1 drift check passes | `client_api_schema.rs`, `api-types.test.sh` | Yes |
| US4.2 stale type fails and names the command | `api-types.test.sh` (edited.ts, missing.ts) | Yes |
| US4.3 every kind classified, new kind fails | `classify.rs` tests plus exhaustive match | Yes (compile-time) |
| US5.1 kind cluster up, session listed within 180 s | `local-stack-kind.test.sh` (stubs), `local-stack-kind-live.sh` (never run) | **No** (F4) |
| US5.2 Ctrl-C stops forwards, deletes cluster unless KEEP | `local-stack-kind.test.sh` | Yes (stubbed tools) |
| US5.3 missing tool named, process mode works | `local-stack-kind.test.sh` | Yes |
| US5.4 chart renders with local values | `chart-local.test.sh` | Not verified here (SKIP, no helm) (F4) |
| US5.5 state survives pod restart | `local-stack-kind-live.sh` (never run) | **No** (F4) |

Criteria covered: 14 of 18 (not counted: US1.3, US5.1, US5.4 unrun here, US5.5).

Functional requirements and success criteria worth stating: FR-006 (no binary frame sent) is asserted only for a binary request being refused (`ws_protocol.rs:68`), plus the transport test that drops binary frames (`BrowserTransport.test.ts:38`); no test inspects every outgoing frame. FR-014a is covered only in synthetic form (F2). SC-001 (60 s in three browsers) and SC-002's browser half are CI-only. SC-006 in the browser is open (F3).

Untested criteria: US1.3, US5.1, US5.5, and US5.4 in this run. Tests tracing to nothing: `ws_admission.rs` (handshake timeout, socket cap, non-UTF-8 origin) and `a_subscriber_that_falls_behind...` trace to review findings M2-F1/F7, not to a spec id. They are legitimate hardening of FR-004/FR-003; record them in the spec or a test list.

## What was not audited

- Full `mise run gate` and the full workspace suite were not run (about 6 min plus a heavy link step; CLAUDE.md reserves it for the pre-push gate). Only the feature's suites listed in the frontmatter ran.
- Mutation testing: not installed. Deliberate mutants: blocked by the sandbox permission check and not run. Test strength is unmeasured. No coverage run (no cargo-llvm-cov).
- Playwright (`client-e2e-stack`) in any browser: not run (no stack/browsers set up). Firefox and WebKit have never been run by the author either (autopilot.md handover).
- Kind live run (T052), `chart-local.test.sh` (no helm), and the Docker image build in `dockerfile-web.test.sh` (needs docker): not run.
- Files in the feature's test set not read line by line: `ws_support.rs`, `client_api/messages.rs` and `origin.rs` unit tests (names and one run only), most of `diff.rs` tests after line 300, `ws.rs`/`directory.rs` unit tests, `local-stack*.test.sh` and `chart-local.test.sh` bodies, `SessionContent.tsx` and the production sources except the lines cited, the rest of `ConnectionBanner.test.tsx` and `real.test.tsx`.
- Performance and load behavior beyond the 1024-entry ring and the socket cap: no criterion.
- The documentation (`docs/`) and Helm chart contents were out of scope for test grading.
- `cargo` and `pnpm install` were run in this container (the latter created `node_modules/`, which is untracked build state, not source).
- Remediation tasks were not appended to `tasks.md`: the request said to write only `verification.md`. The finding IDs above are the remediation items; order for the lifecycle: F1 (decide: record reds retroactively as `/speckit.tdd.run` does, or accept and state the waiver), F2, F3, then F4-F11.

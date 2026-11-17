---
feature: 1735-rn-client-scaffold
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: c4d1b3ac3 # short SHA audited
behaviors: 25 # test-first task groups from tasks.md (no tdd/test-list.md exists)
proven: 0
likely: 21
test_after: 4
no_test: 0
high_smells: 1
criteria_total: 30 # FR-001..021, SC-001..009
criteria_covered: not verified # no test list, so no mechanical trace; see Traceability
mutation_score: unmeasured # no JS mutation tool; 6 deliberate mutants, 5 caught, 1 survived
mutants_survived: 1 # not equivalent, inside a ticked behavior
suite: 179 passed, 0 failed, 18.7s (with coverage; 4.6s warm), 32 suites; coverage 97.2% stmts, 91.3% branches
---

# TDD Verification: React Native (Expo) web client scaffold

**Verdict: FAIL.** A real boundary bug in `ConnectionManager` (offline after `maxAttempts`) survives in a behavior ticked done (T026/T027). Four behaviors have no recorded red, and no ordering can be proven because each milestone was squashed to one commit.

Audit scope: `apps/client` and `packages/protocol` Jest suite, run by `pnpm --dir apps/client test` after `pnpm install --frozen-lockfile` (node_modules was absent). Independent: this session did not write the tests. `check-prerequisites.sh` resolved `specs/1726-app-via-orchestrator`; the feature dir given in the request was used instead. The suite was green before and after all mutants (tree clean).

## Test-first evidence

History: M1-M4 each landed as one squashed commit (`18aadedf6`, `b863fd339`, `dfd448cec`, `630fe595e`), so git cannot show test-before-source. Nothing is `PROVEN`; a recorded red in `tdd/cycle-log.md` gives `LIKELY`.

| Behaviors | Class | Evidence |
| --- | --- | --- |
| T005, T010, T012, T014, T016, T018, T021 (M1) | LIKELY | red recorded in cycle log; history squashed |
| T024, T026, T028, T030, T032, T034, T036, T038 (M2) | LIKELY | same |
| T041, T043, T047, T049, T051, T052 (M3) | LIKELY | same |
| T045 stale tests | TEST_AFTER | log: "green on arrival"; shown to bite only afterwards by stubbing `isStale` |
| T054, T056 (Playwright smoke, pseudo-locale) | TEST_AFTER | no M4 section in the cycle log; not run in this audit |
| T058 (bundle-check planted emulator) | TEST_AFTER | no cycle-log entry; `scripts/tests/bundle-check.test.sh` changed in `630fe595e` together with the source |

Existing tests: none weakened, skipped or excluded. `guard.test.ts` and `layout.test.tsx` were extended or re-pointed at `renderWithApi` with the same assertions. No coverage or mutation threshold exists in the Jest config, so none was lowered (and no gate on it either).

`tasks.md`: T060a and T062 are unticked and correctly so. No ticked task contradicts the evidence except the three above, which are ticked with no red on record.

## Findings

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Mutant `this.attempt > max` to `>=` survives. The test never asserts the state after the `maxAttempts`-th drop (should be `reconnecting` attempt 2), only after the 3rd. Off-by-one in when the app goes offline is uncaught. | `packages/protocol/src/connection/ConnectionManager.ts:132`; `packages/protocol/tests/connection.test.ts:62-77` |
| 2 | HIGH | Conditional logic in a test: `if (c.kind === "session-state")` decides whether to assert. Always true today (`stateChanges` emits only that kind), so it is latent, but it also builds a `""` map key for other kinds. Assert unconditionally. | `packages/protocol/tests/order.test.ts:53` (in `describe("applyChanges")`, "keeps order for one batch") |
| 3 | HIGH | T045 and T054/T056/T058 have no red on record (TEST_AFTER, see table). | `tdd/cycle-log.md:45`; no M4 section |
| 4 | MED | Latency test uses the real clock (`performance.now() < 100`), so it is load-sensitive; it also measures inside jsdom with reduce-motion mocked. SC-002 proper is covered only by the Playwright spec (not run here). | `apps/client/tests/sidebar/latency.test.tsx:30-33,85` |
| 5 | MED | Test greps `src/auth/*` source with a regex for `localStorage` etc. Coupled to implementation text; behavior is already pinned by the first test in the file. | `apps/client/tests/auth/sessionStore.test.ts:44-49` |
| 6 | LOW | Test named "shows the title, the tagline and the language picker" asserts only the header and the radiogroup (no tagline). | `apps/client/tests/app/layout.test.tsx:56-63` |
| 7 | MED | Profile gap: `.specify/memory/tdd-profile.md` describes only the Rust stack (detected at `cdc473ab`). No Jest command, exemplar, helpers or conventions for the TS packages, so Redundant, Foreign style, Bypassed utility and Framework under test could not be graded. `renderWithApi` and `protocol/tests/fakes.ts` were used as de facto helpers. | `.specify/memory/tdd-profile.md` |

Properties: suite is fast (4.6-18.7 s), seeded randomness only (`rng(7)`), no sleeps beyond `setTimeout(r, 0)` flush (`sidebar/order.test.ts:7`). Tautology, doubled-subject and vacuous-assertion patterns were checked on `buildTheme`, `sessionStore`, `guard`, `order`, `connection`, `layout`, `latency`, `spikeScreen`: none found; the 38 `toBeTruthy` uses are role/text lookups (a missing element throws first). `buildTheme.test.ts` compares to the tokens file, not to the mapping logic, so it is not a re-implemented expectation.

## Mutation results

No JS mutation tool in the repo. Six deliberate mutants, one at a time, each restored with `git checkout` and the suite re-run green. Not exhaustive: sampled auth, protocol ordering, reconnect, layout.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `order.ts:65` sort replaced by `() => 0` | T041 | No | caught (arrival-order test) |
| `sessionStore.ts` `setSession` also writes the token to `localStorage` | T032 | No | caught by 2 tests (FR-006) |
| `layout.ts:8` `>=` to `>` on breakpoint | T047 | No | caught by 2 tests, 600/599 pinned |
| `ConnectionManager.ts:137` `2 ** (attempt-1)` to `2 ** attempt` | T026 | No | caught (delay table) |
| `ConnectionManager.ts:132` `>` to `>=` on `maxAttempts` | T026 | **Yes** | real survivor, finding 1 |
| `Overview.tsx:22` drop `Math.floor` | T047 | No | caught |

Also noted: `order.ts:64` filter and `:66` reduce both guard `seq > snap.seq`, so a single-guard mutant there is equivalent; not counted.

## Traceability

30 criteria (FR-001..021, SC-001..009). With no `test-list.md` there is no `traces` column to verify, and only 7 test files cite an FR/SC id. Covered with evidence in this audit: FR-006 (sessionStore, mutant caught), FR-009 (connection, banner), FR-011/FR-012 (order, layout), FR-014/FR-015 (i18n), SC-001, SC-004. Not shown by Jest in this audit: SC-002 end to end, SC-003, SC-005 and SC-009 (T062 open), FR-017, FR-018, FR-021. Untested criteria cannot be listed with confidence; this is a gap, not a pass. Tests tracing to nothing: none seen.

## What was not audited

- Playwright e2e (`apps/client/e2e/*`), the screenshot spec and the visual pass: not run, no browsers.
- `scripts/tests/*.test.sh` (classify-change, bundle-check, client-pins): read in the diff stat only, not run or graded.
- Mutation: 6 deliberate mutants only, no tool; test strength is otherwise unmeasured.
- Repository-relative smells (Redundant, Foreign style, Bypassed utility, Framework under test): no JS profile to grade against.
- Full criterion-to-test mapping, CI timing (SC-005) and diff coverage (SC-009).
- Test-first order: unprovable from squashed history; rests on the self-reported cycle log.

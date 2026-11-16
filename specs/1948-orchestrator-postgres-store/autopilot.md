# Autopilot ledger — #1948 orchestrator-postgres-store

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1948 "feat: PostgreSQL store for the orchestrator (sqlx, migrations, store traits; SQLite for local mode)" (part of #1722; FR-042, FR-046). labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1948
- **Worktree branch**: claude/project-thread-0phzsu
- **Started**: 2026-11-15
- **Phase**: close
- **Next step**: orchestrator decision on the second tdd-verify FAIL (cae46cf): cut T071–T075 into M12, or accept mutant reds for the TEST_AFTER rows (F4)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1967 | Design (spec, plan, tasks) | merged | f5f8e812a |
| #1970 | M1 | merged | bfb463f60 |
| #1980 | M2 | merged | 952b8a818 |
| #1992 | M3 | merged | b3f47d077 |
| #1997 | M4 | merged | c3944869a |
| #2001 | M5 | merged | 8e68fa947 |
| #2003 | M6 | merged | 3d5485c30 |
| #2006 | M7 | merged | 3a74d3004 |
| #2008 | M8 | merged | 2243de02d |
| #2010 | M9 | merged | 83926c52f |
| #2011 | M10 | merged | 8005293fe |
| #2014 | M11 | merged | c5c184a03 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Registry, auth and audit stores in memory, shared suite (MVP) | #1970 | merged |
| M2 | T014–T020 | full | SQLite handle, schema, registry store | #1980 | merged |
| M3 | T021–T025 | full | SQLite auth and audit stores | #1992 | merged |
| M4 | T026–T031 | full | PostgreSQL store and CI service container | #1997 | merged |
| M5 | T032–T035 | full | Migration safety | #2001 | merged |
| M6 | T036–T038 | full | No private key in any table | #2003 | merged (3d5485c30) |
| M7 | T039–T044 | full | [database] config and backend rule | #2006 | merged (3a74d3004) |
| M8 | T045–T050 | full | Orchestrator starts on the chosen store | #2008 | merged (2243de02d) |
| M9 | T051–T054 | full | Import registry.json once | #2010 | merged (83926c52f) |
| M10 | T055–T060 | full | Ownership columns and adoption; quickstart recorded | #2011 | merged (8005293fe) |
| M11 | T061–T070 | full | TDD remediation (verification.md FAIL: F1–F12), tests and evidence only | #2014 | merged (c5c184a03) |

## Decisions

- M10: on every backend an `unowned` record never clears a stored owner and an owned record sets its owner (SQL: `CASE WHEN excluded.owner_user_id = 'unowned'` in the conflict branch, review A-F1; memory/JSON: `keeping_owners`). This refines plan.md's "insert only" so all backends agree. The registry still announces relinks `unowned`. SQL `reassign_unowned` returns the moved `daemons` count (projects/sessions have no rows in v1). The scan also skips SQLite's internal `sqlite_sequence`.
- M10: quickstart §B step 1 path was `micold`, really `micold-ai-ide`; step 4 needs `config check --files` to fail (without it, exit 0 with `(missing)`), doc fixed.
- M6: T036's tests passed on first run: `CertSerial::new` already validated hex (M1), and a PEM block fails both `CertSerial::new` and `TokenHash::from_hex`. T038 needed no `auth.rs` change; the tests pin the rule. Red evidence is T037's negative control.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec | d3937c8969e34c08a0ce2f78559cde430b5c65f9 | 09cb2cfc0a611043c58abc651afcc426257803e6:d65a0b90b4bab89ddf4da88fcd33c56490018866 | CHANGES: 5 MAJOR 3 MINOR, all fixed |
| Spec | 2 | spec | 69983c1058338b7230c9e70066dc73eca578a606 | 1faf1b6e8adebd019dd1d2e6f8675ad1d5a15b12:d3937c8969e34c08a0ce2f78559cde430b5c65f9 | CLEAN (2 MINOR fixed, prose only) |
| Plan | 1 | plan | (pending commit) | 039972fca59028890bb8b22565c8ee7ab6bca81a:b1cd1965e0f01444b33b7d262dbda5e068ff3b06 | CHANGES: 1 BLOCKER 1 MAJOR 3 MINOR, all fixed |
| Plan | 2 | plan fix diff | (this commit) | 1c7862375e7765f0e55c0e9fc8331fd4e4c8c56b:8123b3e49d68aa61553fe7273ad201b44137eebf | CLEAN (1 MINOR fixed, prose only) |
| Plan | 2 | plan | 190e9973277b43ecb3c28cee728d38243108c2e5 | 1c7862375e7765f0e55c0e9fc8331fd4e4c8c56b:8123b3e49d68aa61553fe7273ad201b44137eebf | CLEAN |
| Tasks | 1 | tasks | (pending) | e8c4d28d157e06b94aac968d5bbbf982906eeb65:7e5052b09b288667942286e771b4b91a672f3412 | CHANGES: 4 MAJOR 3 MINOR, all fixed (M1+M2 merged, milestones renumbered, Verify one filter per command, T012 not [P], T027 unreachable case, T041 config test before T042, quickstart wording) |
| Tasks | 2 | tasks fix diff | (pending) | b2d38ef36db8d9a9c95fa37b0ededa0f6b612709:ab2d68a634bf73201e1ffe5c3c6d7b122d741812 | CHANGES: 1 MAJOR (Verify joined with "and" not &&), fixed |
| Tasks | 3 | design | 51d20197b6bb9ca92ffbf64dec4be986cd56f4fa | 3d3ec8c874fcffe3ca8aecf860c97d3efe962a9a:c9db7e595c1f9181c01606cca34854d161c8a40e | CLEAN |
| M1 A | 1 | M1 diff (code-review high, run by the unit: no Agent tool) | 6d4d88cadd058fbd51cb0e2c2de1ebbf21dae2c9 | 311e34200b5cce2ae68cd49c3a73e1e8bd98ffea:6d4d88cadd058fbd51cb0e2c2de1ebbf21dae2c9 | CLEAN (1 MINOR noted as a follow-up) |
| M1 A | 1 | M1 diff | 6d4d88cadd058fbd51cb0e2c2de1ebbf21dae2c9 | 311e34200b5cce2ae68cd49c3a73e1e8bd98ffea:6d4d88cadd058fbd51cb0e2c2de1ebbf21dae2c9 | CLEAN |
| M2 A | 1 | M2 diff (code-review high, run by the unit: no Agent tool) | 1c5e8c6bda32fc3ca960ce3a62d7282506a25826 | (see commit) | CHANGES: 0 BLOCKER, 3 MAJOR-ish (non-ASCII URL, SQLITE_BUSY as Refused, save lock upgrade) all fixed with 4 MINOR; 3 declined (see below) |
| M4 A | 1 | M4 diff (code-review high, run by the unit: no Agent tool) | HEAD at return | (see commit) | CLEAN (1 MINOR fixed: long doc comments; noted: one-character password redaction is over-eager, safe) |
| M4 B | 1 | M4 diff (conformance, sonnet, run by the orchestrator) | b4a39c7c3 | (see commit) | MINOR-ONLY (one-char password redaction garbles text; CI postgres creds duplicated; error-arm tests test-after, recorded) |
| M3 A | 1 | M3 diff (code-review high, run by the unit: no Agent tool) | HEAD at return | (see commit) | CLEAN (1 MINOR fixed: `put_refresh` dropped `revoked_at`; noted: PostgreSQL `ORDER BY id` collation may differ from the memory order, M4) |
| M1 B | 1 | M1 | 7e2b729df2134721ab66d77f85e9f3b65a3422c6 | ffae1d956ca5a1bf2f113872c8d6ce62e5df7d1c:7e2b729df2134721ab66d77f85e9f3b65a3422c6 | CLEAN (2 MINOR: test-after cycle 3 accepted as recorded; redeem_invite unknown user -> M3 follow-up) |
| M2 B | 1 | M2 | 60bc1cfd3c813ffadbb45340e45bcd0d2b3670f3 | 5b6a0087c24bfaa47a2eee1e527d2ff41066c334:60bc1cfd3c813ffadbb45340e45bcd0d2b3670f3 | CHANGES: 1 MAJOR (cycle 2 no red for review A fixes), fixed by red-by-revert evidence; 1 MINOR (db/ blanket skip in no_outbound scan) declined |
| M2 B | 2 | M2 | ea2972ed97ec4ceff9cb503b819dd57387a3a328 | 1ac7f9902a0d54aa80ffda09815d4d620b824d07:ea2972ed97ec4ceff9cb503b819dd57387a3a328 | CLEAN (fix diff: cycle-log red evidence) |
| M2 B | 3 | M2 | 195e0b5d85d6086e57868e0f627f6e92d1dd4bb0 | b5009125e83d663182f1adc7f4be9bdf5a8cc5f1:195e0b5d85d6086e57868e0f627f6e92d1dd4bb0 | CLEAN (coverage tests; 1 MINOR -> M5 follow-up) |
| M3 B | 1 | M3 | b071596c50a13aba7866a7442129758ed98cf556 | d8fb2cf1746d72e9c998e885c4ca241604bd3bd6:b071596c50a13aba7866a7442129758ed98cf556 | MINOR-ONLY (1 MINOR: cycle 3 error-arm tests test-after, recorded; accepted) |
| M5 A | 1 | M5 diff (code-review high, run by the unit: no Agent tool) | HEAD at return | (see commit) | CHANGES: 4 fixed (one SQLite retry not enough past one migration; WAL sidecars left by a refused read-only check; PG check/run race names VersionMissing; read-only URL by string replace) and 2 MINOR fixed; 4 declined (see below) |
| M5 B | 1 | M5 diff (conformance, sonnet, run by the orchestrator) | eec56ae53 | (see commit) | CHANGES: drift record missing (F1 MAJOR), T035 "retries once" text (F2 MINOR); both fixed in drift-M5.md |
| A | 1 | M6 | ad4e9f7661bbfb976df082ceb69bd7255cc0f290 | 51b9ed137f67d821c60d4d72fb36ae33ff930585:c19f093a6971d0bd33e5d660278e33dfd2105806 | MINOR-ONLY (opus fresh context, covers A and B: 2 nits fixed, prose/test-comment only: PEM[..64] slice, key_material scan limit noted) |
| B | 1 | M6 | ad4e9f7661bbfb976df082ceb69bd7255cc0f290 | 51b9ed137f67d821c60d4d72fb36ae33ff930585:c19f093a6971d0bd33e5d660278e33dfd2105806 | CLEAN (same opus review, APPROVE) |
| Code A+B (opus) | 1 | M7 | 16c2b1e | | MINOR-ONLY: cycle-log, unreadable case (directory, exercised under root), D6 note fixed |
| Code A+B (opus) | 1 | M8 | 71e4c32 | 3a74d3004:71e4c32 | CHANGES: 2 MAJOR 2 MINOR, all fixed (F1 refuse unimported registry.json, F2 M8 cycle-log red-by-revert, F3 process stderr test on PostgreSQL, F4 full gate green) |
| Re-review (sonnet) | 2 | M8 fix diff | 1de38e5 | 71e4c32:1de38e5 | MINOR-ONLY: FR id in comment and vacuous password check fixed; cycle-log line numbers noted |

- M10 full gate (scripts/gate.sh, PostgreSQL, debug=0): fmt, clippy, 13498 nextest passed; 1 failed only because the local test password `postgres` is a substring of `sqlx_postgres` (rerun with another password: store_ 129/129, 4 runs); scripts/tests all pass.
- M10 round 1 (A CHANGES, B CLEAN): A-F1 MAJOR SQL conflict branch now sets an owned record's owner (CASE); A-F2 MAJOR suite case `an_owned_record_over_an_unowned_one_takes_its_owner` (upsert and save, all backends; import never writes over an existing row: rows without a matching marker refuse, so the CASE covers it with no import case); A-F3/B-F1 store_scan `*_reassign_moves_projects_and_sessions_of_one_tenant` + doc that the count is daemon rows; A-F4/B-F2 JSON save fails on a read error, falls back only on NotFound/parse error (+ core test); B-F3 quickstart §A gate count inline. All fixed; store_ 135/135 with PostgreSQL; full gate green (scripts/gate.sh exit 0, CARGO_INCREMENTAL=0 after wiping target-shared for disk).
- M9 round 1 (opus A+B): MINOR-ONLY, 2 MINOR fixed: a non-NotFound read error now says `cannot read it`; the start test seeds a record and checks the started store serves it (corrupt-file start failure already covered).
- M11 round 1 fixes (A-F1..F2, B-F1..F6): A-F1 the Google "same subject, other provider" assertion is its own identity case `an_identity_subject_is_scoped_by_provider`; A-F2 comment on why only the db dir needs `code_only`; B-F1 T069 swap-order red entry; B-F2 T063 CI count (job 133640258650: orchestrator 247 passed, 0 skipped); B-F3 T066 jscpd/similarity-rs equal on base and head; B-F4 T068 mutant 7 re-run fails sqlite+postgres cases; B-F5 serials/enrollments split into five cases; B-F6 rows 7/11 note. Orchestrator suite with PostgreSQL (:5433, max_connections=500): 259 passed, 0 ignored.

## Declined review findings

- M2 A: `Drop` skips `pool.close()` on a `spawn_blocking` thread (WAL is recovered on the next open; the context probe cannot tell the thread kinds apart); one runtime per handle (plan decision); `Failed { version: 0 }` and `VersionMissing` to `DatabaseNewer` belong to M5 T035.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| #1967 | Design (spec, plan, tasks) | merged | f5f8e812a |
| #1970 | M1 | merged | bfb463f60 |
| tasks | analyze | D1 Principle IV CRITICAL | Decided by jaro (2026-11-14/15), recorded in plan Complexity Tracking; #1949 amends the text; tasks.md header says a network deployment waits on it |
| tasks | analyze | D2 Principle VI CRITICAL | Linux only per jaro (#1897); CI linux-daemons job covers the build |
| M5 | A | `lost_the_race` matches text, also on PostgreSQL | A spurious retry re-runs the migrator and then fails the same way; the advisory lock makes it harmless on PostgreSQL |
| M5 | A | dialect sniffed by `backend_name` and by `Option` in `start` | Two backends; a third would add its own open |
| M5 | A | `DatabaseNewer` text lost its remediation hint | The text is contracts/config.md's verbatim |
| M5 | A | test `with_pool` builds a runtime per query | Test-only; a few ms per case |
| M2 | B | F2 MINOR: no_outbound scan skips all of `src/orchestrator/db/` | The pool `connect` is the only dial-like construct there; recorded in drift-M2.md; M4 (PostgreSQL) should narrow it to the `connect` call sites |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M11 (unit 2): T068–T070 done at cbca04d (T068 split cases, T069 test clock / store-level failure mapping / named-column inserts, T070 baseline 49 passed at 60277d73d). Gate (scripts/gate.sh, PG 16 at :5433): fmt, clippy, 13569/13570 tests green; the one failure is session_extras signing (ssh-keygen not installed in the cloud box), unrelated. The older-database case already runs 0002+0003, where 0003 reads 0002's table, so swapping them fails. Reviews A and B not run (no Agent tool in this unit). PR body: scratchpad pr-body.md.
- Close: plan drift for M6–M10 recorded in drift-M6..M10.md (no unbuilt work). speckit-tdd-verify verdict FAIL at 40309293a (tdd/verification.md): cut into M11, since this close unit has no Agent tool for per-crate workers and the work is ~10 test tasks. converge skipped: review B CLEAN (or A+B combined CLEAN after fixes) for every milestone.

- M2 and M4: `load` is ordered by `(tenant_id, id)`; the JSON and memory stores sort in `load`, the SQL ones with `ORDER BY`.
- `sha1` in `micold-daemon` is used by M9's import digest (`sha1:<hex>` in `imports.digest`).
- M2 `StoreHandle` takes the `Handle`-less guard `run_on`; M3/M4 stores must call it from `spawn_blocking` or a plain thread. 
- Cloud gate: `scripts/gate.sh` was red twice on script tests that passed alone (`check-assertions-frozen` case 028, `autopilot.test.sh` facts cases) and green on the third run on the same tree; the scoped gate fails two root-only permission tests in micold-core as root (gate.sh runs them unprivileged).

- M1–M6 and M10 carry no user-guide change (daemon crate is not micold-user-facing): use label docs-not-needed. M7–M9 carry guide tasks.
- Cloud thread: preflight failed only on `mise` (missing) and `gh` auth (known cloud setup); the run proceeded. Use `scripts/gate.sh` and the GitHub MCP tools.
- #1949 (constitution 2.0.0, PostgreSQL for network deployments) is not merged: nothing here depends on it; it must merge before a network deployment ships. Constitution Principle IV currently says local-first storage.
- Constitution conflict: it still says local-only storage and three OSes; jaro's Linux-only decision (#1897, 2026-11-14) and #1949 supersede. The spec keeps Linux-only and cites #1897.
- kube_real tests cannot run in cloud (no kind).
- Close (after M11): speckit-tdd-verify re-run at cae46cf: FAIL. Resolved F1-F3, F5-F7, F11; partly F8, F9, F12; not F4 (14/25 TEST_AFTER, history) or F10. New N1 (HIGH, migrate.rs:71 retry bound, mutant survives ~half), N2-N4 (MED), N5-N7 (LOW). verification.md overwritten, tasks.md Phase 14 T071-T075 appended. Not cut into a milestone: returned to orchestrator.

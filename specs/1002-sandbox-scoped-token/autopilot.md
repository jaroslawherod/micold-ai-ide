# Autopilot ledger — #1002 sandbox-scoped-token

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1002 — "Sandbox container with daemon should use a fine-grained token, not the user's GitHub token and SSH key" (body in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1002
- **Worktree branch**: claude/project-thread-2cdrmk
- **Started**: 2026-10-19
- **Phase**: close
- **Next step**: close PR #1139 (draft): record T055/T056 (live PAT check, how it runs is on jaro's decision card), set the spec's Status, finish the ledger, then merge.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1090 | design (spec, plan, tasks) | merged | 04b29291209b144376228864f40d0f46ac87d103 |
| #1096 | M1 | merged | 07ad3c1f936358fd25ecf021e7009459c9ca2458 |
| #1099 | M2 | merged | abb25cf8907de6acef23805b2334f679f0060af7 |
| #1107 | M3 | merged | 5c0ba07dfc32194eec443241efee61836931b4cb |
| #1113 | M5 | merged | 38a9ce97b751560df8c8feaddf1271cbceba0cc2 |
| #1132 | M6 | merged | 0b267cf632915083624acd3d8b44e7c4cfe3f66f |
| #1139 | close | draft | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | No user git identity in a sandbox (FR-001–003, FR-014) | #1096 | merged (07ad3c1f) |
| M2 | T014–T022 | full | Sanitized shared git config, helper reset (FR-004) | #1099 | merged |
| M3 | T023–T029 | full | In-container helper + in-memory per-repo file (FR-006/007/015) | #1107 | merged (5c0ba07d) |
| M4 | T030–T038 | full | Protocol delivery, Schedule, revocation with FakeSource (FR-008–010, 012) | — | merged (on main by 03e1fb38) |
| M5 | T039–T049 | full | [source] Pasted-PAT source + client wiring (FR-005, FR-008–010); the GitHub App M5 is abandoned (bundle backups/1002-m5-github-app.bundle) | #1113 | merged (38a9ce97) |
| M6 | T050–T055 | full | Status rows (FR-013, SC-005); FR-011's guidance text is in M5, T051/T053 withdrawn | #1132 | merged (0b267cf6); T055 is the close unit's |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which runtimes are in scope? | Container and Kubernetes daemons; remote SSH (user's own machine) out | agent-resolved | spec.md#Context |
| D2 | spec | Non-GitHub remotes? | No git credentials in a sandbox; old opt-ins not kept for them (least privilege) | agent-resolved | spec.md#Assumptions |
| D3 | spec | Default permissions? | contents rw + metadata r; PRs and issues per-daemon opt-ins, off by default | agent-resolved | spec.md FR-011  Superseded in part by D7/D19 (pasted PAT: no App key, no 1 h renewal, no per-daemon toggles; M5/M6 content changed). |
| D4 | spec | One credential per daemon or per repo? | Per repository | agent-resolved | spec.md#Assumptions |
| D5 | clarify | Keep old opt-ins for non-GitHub remotes? | No; no credentials, status says host unsupported (confirms D2) | agent-resolved | issue #1002 body ("rather than falling back") |
| D7 | clarify | Where does the scoped credential come from (FR-005)? | (B) **Pasted PAT**: the user creates a fine-grained PAT per repository by hand and pastes it into Settings; GitHub's shortest expiry is 1 day; no automatic renewal. The GitHub App option (A) is rejected | jaro (decision card, 2026-10-19T16:07Z) | spec.md#Clarifications; constitution.md#IV |
| D8 | clarify | Round 2 | No critical ambiguities; App key storage and setup UX deferred to plan | agent-resolved | spec.md  Superseded in part by D7/D19 (pasted PAT: no App key, no 1 h renewal, no per-daemon toggles; M5/M6 content changed). |
| D9 | plan | Where does the App private key live on the device? | Sibling of data_dir (`micold-ai-ide-secrets/`), owner-only; data_dir is mounted whole into the legacy default sandbox | agent-resolved | research.md R5  Superseded in part by D7/D19 (pasted PAT: no App key, no 1 h renewal, no per-daemon toggles; M5/M6 content changed). |
| D10 | plan | Who renews while no client is connected? | Nobody: tokens lapse within 1 h, pushes fail closed, next connect re-grants (US3-4) | agent-resolved | research.md R8  Superseded in part by D7/D19 (pasted PAT: no App key, no 1 h renewal, no per-daemon toggles; M5/M6 content changed). |
| D19 | spec rework | What the PAT source changes in FR-008..011 and M4 | FR-008: no renewal; warn at ≤ 24 h, withdraw at expiry, replacement delivered in place. FR-009: the 1 h cap is dropped; the client accepts only a fine-grained PAT with a known future expiry that GitHub grants push on the repo (and, where the API shows it, reaches no other repo); lifetime is the user's choice. FR-010: discard copies, never revoke the user's PAT. FR-011: no per-daemon toggles (a PAT cannot be narrowed): guidance text instead. M4 follow-up (new tasks in M5): `accept`'s 1 h cap and `MAX_LIFETIME_SECS` become per-source, and `Schedule` stops renewing/revoking for the PAT source (warn, withdraw at expiry). M6's toggles (T051, T053) become guidance text | agent-resolved (follows from D7) | spec.md FR-008–011; tasks.md M5, M6 |
| D6 | clarify | May a project-hosted service mint credentials? | No; client mints/holds on the device | agent-resolved | constitution.md#IV, #Technology constraints |
| D11 | tasks | Milestone cut vs plan's 3 | 6 milestones: plan's M1 split by FR (US1 removal / FR-004 sanitizer), plan's M2 split into git side (M3) and delivery (M4); the [source] GitHub App is confined to **M5** (was "M3" in the plan); M6 = status + FR-011 toggles. M1 (13 tasks) not split: the enum removal forces the client items to change in the same compile, and the `#[serde(skip)]` removal flag means the notice cannot lag the removal. M5 (11 tasks) not split: App settings without the wiring would be half-wired UI | agent-resolved | tasks.md#Milestones; milestones.md rules 3, 6  Superseded in part by D7/D19 (pasted PAT: no App key, no 1 h renewal, no per-daemon toggles; M5/M6 content changed). |
| D12 | tasks | speckit-analyze (14 findings, 2 HIGH) | All fixed: Verify/Satisfies aligned with quickstart steps; fixture placed in `sandbox/hostile_host.rs`; kept items enabled in FR-014 tests; old container git becomes `NoCredentialReason::GitTooOld` via `GitCredentialTargets.git_too_old` (plan Risks, contracts/protocol.md, data-model §6, spec Key entity updated — plan fix); `MAX_LIFETIME_SECS` const; two-renewal test; quickstart step 12 for FR-011; notice visual pass moved to M1 | agent-resolved | tasks.md, plan.md#Risks |
| D13 | M1 | No `tdd/test-list.md` (design skipped `speckit-tdd-plan`); the mandatory `tdd.run` hook needs one | Drive M1 test-first by hand in tasks.md's order (T001–T007 tests before T008–T011): write each test, run it and record its red line in this ledger's *Review rounds*-adjacent note or the commit message, then implement. No test list is created for one milestone | agent-resolved | precedent: specs/9xx ledgers "test tasks are ordered first by hand" |
| D14 | M1 | Where the `RemovedGitCredentials` notice is stored, given `ContainerSettings.migration` is one `Option` already used by `MovedOffReleasedImage` (cleared on a basic-image start, which would also wipe ours) and `KubeSettings` has no notice field | On `DaemonEntry`: new `#[serde(default, skip_serializing_if = "Option::is_none")] notice: Option<MigrationNotice>`, DaemonEntry read through a `RawDaemonEntry` (`#[serde(from)]`) whose `From` takes `SandboxProfile::take_removed_git_credentials()` / `KubeSettings::take_removed_git_credentials()` and sets it; `migrated_entry` does the same for the legacy `daemon` block. `DaemonRegistry::dismiss_notice(id)`; client row shows it beside the image notice with a dismiss message. 11 `DaemonEntry {` literals gain `notice: None` | agent-resolved | daemons.rs:46-85, features/daemons.rs:588-593 |
| D16 | M1 | D13's test-first order was not kept: units 2–3 wrote the core removal (T008–T010) before its tests T002–T006, and T009's Kubernetes tests after the code | Tests kept as written (each asserts the removed item's absence, which the code now guarantees); the PR body says T002–T005, T008, T009 tests came after the code | agent-resolved | commits d22a1a75, 71e98833 |
| D15 | M1 | Kubernetes secrets `<base>-git-credentials` / `<base>-git-ssh` already in a cluster from before the change: removing the variants drops them from `every_kind()`, so start (start.rs:409) and remove (lifecycle.rs:101) would never delete them and the user's key would stay in the cluster | Keep their names as `ResourceNames::retired_secrets()`; start deletes them (ours-labelled only) beside the withdrawn kinds, and `remove` deletes them too (`delete_ours` tolerates absence). Test it in `start_tests.rs` with the fake cluster | agent-resolved | kube/start.rs:409-424, kube/lifecycle.rs:94-114 |
| D17 | M1 | Review A F1: a pre-#1002 container's SSH-agent mount is recognised only when today's `SSH_AUTH_SOCK` equals its creation path | Least privilege: new containers carry the label `io.micold.git-identity=none`; `adopt` replaces any existing container without it (running, legacy released image or not), ahead of every other branch. The retired-path check stays as a second line. Cost: every container made before #1002 is replaced once on its next start, ending sessions in it; the `RemovedGitCredentials` notice says so (F3) | agent-resolved | reviewA.out F1/F3; sandbox/lifecycle.rs `adopt` |
| D18 | M1 | Red CI: `sandbox_real_provision_a_running_legacy_container_is_attached_then_replaced` got `Replace{MadeBeforeCredentialRemoval}` where FR-017 expects `Legacy` | Not a D17/FR-017 conflict: the real-runtime harness (`start_sandbox_on`) builds its `create` argv by hand, not via `argv::create`, and omitted the `io.micold.git-identity=none` label, so its container really was unlabelled. Harness now passes the label (mirroring argv); D17 stands (an unlabelled container is replaced, least privilege), FR-017 still holds for labelled released-image containers. Other real-runtime tests do not call `adopt`/read facts, so none shares the assumption | agent-resolved | sandbox_real_support/mod.rs; CI log |
| D20 | M6 | Where T052's "project's daemon status line" goes | In the daemon's row in Settings → Daemons, one line per project naming it (the row is the daemon's status: state, worktrees, notices). The sidebar chip is one short label keyed by host paths, while the credential wiring knows the daemon's path, so no second copy there. The worker reports changed statuses over an event-driven subscription; the 24 h warning is worked out at view time | jaro (decision card 2026-10-19: *Settings only*) | features/daemon_sections.rs `credential_lines`; shell/scoped_credential.rs `changed_statuses` |
| D21 | close | No red evidence for any behaviour (verification finding 1, T058) | Accepted exception, as D16 did for M1: no tdd/test-list.md or cycle log was kept, and re-deriving red runs means reverting merged code against tests written for it. The close unit instead proved the tests that matter now bite: each new test (T057, T059, T060) is shown to fail on the audit's mutant and pass on the code | agent-resolved | tdd/verification.md; tasks.md T058 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 98d24270a31995887551b4940385e98d06de202d:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | pending |
| Plan | 1 | c9fa31d9664b54c8a2795dccf16d38b1abf14079:6d822ec17c37bcde07f294ff2509e6682eda33e8 | CLEAN (4 MINOR, fixed, prose only) |
| Tasks | 1 | b6f5094100888a88679ecc6fe8c89bc1dc627695:3bbb1d8d627753e4926daa27fa7e7c7c12d3df48 | CHANGES (1 MAJOR: SourceNotSetUp App wording in M4, fixed via SOURCE_SETUP_HINT; 2 MINOR fixed) |
| Tasks | 2 | b248659e6d8d9fd04545078d5e6bf2c4668c4bb7:402875090f6540516eb724bb15368a2067a33956 | CLEAN (sonnet) |
| M1 A (code-review high) | 1 | e42b32340533c275d5ebd756929ac2e0dd7212c6:71e9883368ab4624e997e3c39e96939f9bc60f76 | CHANGES (2 MAJOR F1, F2 fixed (D17); 6 MINOR F3–F8 fixed) |
| M1 A (sonnet) | 2 | 833375deedd427f1a1c0dfa38313f7936923038e:17370db2c46b156d528932658e24a5ca2fa58b7e | CLEAN |
| M1 B (conformance, sonnet) | 1 | 833375deedd427f1a1c0dfa38313f7936923038e:17370db2c46b156d528932658e24a5ca2fa58b7e | CHANGES (F1 MAJOR declined, see below; F2 MINOR: Verify covered by the full gate's `cargo test --workspace`; F3 MINOR: D17 stated in the PR body) |
| M1 red-CI fix D18 (sonnet) | 1 | 045a4f1811c6371aa92bff326aa70e42bbdbbf38:8e97a4027dbbf4c58e3fd6b7c40a4ecc5f8d5971 | CLEAN |
| M2 A (code-review high) | 1 | 0c7576a111f7bca073769e7193c66f5d11d9aac6:07ad3c1f936358fd25ecf021e7009459c9ca2458 | CHANGES (F1 MAJOR fixed: value-with-userinfo and smtp/imap keys dropped, contract §3 updated; F2 MINOR kube tests need git, as CI has it; F3 MINOR git>=2.31 noted) |
| M2 B (conformance, sonnet) | 1 | 70552b50588c4b06e4e005f3105dcb4274fe43dc:07ad3c1f936358fd25ecf021e7009459c9ca2458 | CLEAN (2 MINOR left: proxyAuthMethod drop is harmless; client test skips without git) |
| M3 A (code-review high) | 1 | c75e6c9544fffc82c433e36f5fe2f5bbbfc143ae:7b33318943986e21f128454d80907d89e9890a85 | CHANGES (F1 MAJOR fixed: kube pod's daemon is not on PATH, so kube daemon_env names it by its build dir via git_env_with_helper; F2–F7 MINOR declined, below) |
| M3 A (sonnet) | 2 | fb5dee54b57bf5ff0f8af7c3389b5cad4ecf069d:7b33318943986e21f128454d80907d89e9890a85 | CLEAN (3 MINOR: no start-level test of the helper path, no direct git_env_with_helper test, a daemon already running keeps its old env) |
| M3 B (conformance + correctness, sonnet; one reviewer for A and B) | 1 | f62bba4f347d8a6361d83cac442b621f119d2a6f:79e2d3e0e3335139d58a5d999979f26b0472e0b0 | CLEAN (2 MINOR: no tdd/cycle-log.md; sandbox_argv run collided with a build, green in the gate). Review A as the `code-review` skill not run separately |
| M4 A+B (sonnet, one reviewer, conformance + code-review high) | 2 | 6e7d225a1c67b191916f3701b61708f112f66edf:15904e76d07ecff2f2045c7acfdadcddbcc0bef5 | CHANGES (F1 MAJOR fixed: `Schedule::run` no longer revokes a superseded token, `Renewal::revoke_superseded` does, after delivery (SC-004); F2 MAJOR fixed: core `CLOCK_SKEW_SECS` shared by `accept` and the daemon; F3 MINOR declined; F4 Verify not run by the reviewer, covered by the full gate) |
| PAT rework: Spec | 1 | 99e778617792b9359807a328889b1c4f5de652fa:03e1fb3897f4a8725e33951ee5ea1e5f697d2e2e | CHANGES (F1 MAJOR fixed: Assumptions still described per-daemon toggles; 3 MINOR fixed: SC-003 public reads, FR-009 opening sentence, git-too-old in FR-012) |
| PAT rework: Spec | 2 | 3c3a91e137ad5f196f3bb3cfe51420b0e0c6ba44:03e1fb3897f4a8725e33951ee5ea1e5f697d2e2e | CLEAN (sonnet; 1 MINOR fixed, prose only: 1-day PAT is inside the warning window from paste) |
| PAT rework: Plan/research/data-model/contract/quickstart | 1 | 3c3a91e137ad5f196f3bb3cfe51420b0e0c6ba44:03e1fb3897f4a8725e33951ee5ea1e5f697d2e2e | CHANGES (4 MAJOR fixed: 1 h row in data-model §2, secrets_dir/PatStore not marked new, PatRefused variant missing, PAT removed while disconnected stayed in the container (withdraw-on-failure); 3 MINOR fixed) |
| PAT rework: Plan etc. | 2 | 666786835aa6017373962abcb43e7ba01b5462b0:03e1fb3897f4a8725e33951ee5ea1e5f697d2e2e | CLEAN (sonnet; 2 MINOR fixed: D10 marked superseded, helper.rs doc listed) |
| PAT rework: Tasks (after speckit-analyze: 0 CRITICAL/HIGH, 9 MEDIUM fixed) | 1 | 666786835aa6017373962abcb43e7ba01b5462b0:03e1fb3897f4a8725e33951ee5ea1e5f697d2e2e | CHANGES (3 MAJOR fixed: M5 Verify cited M6 status steps, docs/daemon.md missing from T048, [P] tasks sharing mod.rs; 2 MINOR fixed) |
| PAT rework: Tasks | 2 | 666786835aa6017373962abcb43e7ba01b5462b0:03e1fb3897f4a8725e33951ee5ea1e5f697d2e2e | CHANGES (1 MAJOR fixed: M6 Verify cited live steps; 2 MINOR fixed) |
| PAT rework: Tasks | 3 | 0841d9c1950c9f05c164b07cfd2899b20fc4da8d:03e1fb3897f4a8725e33951ee5ea1e5f697d2e2e | CHANGES (sonnet; 1 MAJOR: the M6 visual pass on fixture states cannot be run, the showcase has no daemon; fixed by moving T055's pass to the close unit with T056 and making M6 Verify the `daemon_sections` tests: a removed claim, prose only, no 4th round (limit 3)) |
| M5 A (code-review high) | 1 | (not recorded by part 1) | CLEAN (2 MINOR fixed in 069c651a) |
| M5 B (conformance, sonnet) | 1 | (not recorded by part 1) | CLEAN; Verify green |

| M6 A (code-review high) | 1 | (pre-fix HEAD 96367608) | CHANGES (MAJOR: the 24 h warning never redrew at its threshold — the worker now wakes at expiry − 24 h and reports it, and views use the report's time; MAJOR: a lost report advanced `reported` — `unreported` resends; MINOR fixed: same-named project folders show their paths, `credentials` dropped on Remove/forget and for unknown ids, `utc_date` moved to `micold_core::local_time`, reducer tests added; declined: see *Declined review findings*) |
| M6 A2 + B (code-review medium with the conformance checks, one fork: no Agent tool in this session) | 2 | 26808f9b | CHANGES (MAJOR fixed: the resend-on-failed-send path could never help and is gone (same take-once channel as `desktop_notify`); MAJOR fixed: the worker sleeps at most 5 min, so a suspend delays the warning or the expiry by 5 min at most; MAJOR: T052's sidebar line, D20, put to jaro on a decision card, M6 goes on with the recommended *Settings only*; MINOR fixed: reducer test for None → Held, linear folder count; declined: warning wording follows contracts/pat-source.md and M5, not T050's sketch; `next_wake` rebuilding status maps, cheap at this size) |

## M4 review notes

Review A (high) round 1: CHANGES, F1–F7 fixed (validating GithubRepo Deserialize, unique temp names, lower-cased schedule keys, grant token/expiry validation, req u64::MAX sentinel, revoke of superseded token, git_too_old in change detection); F8 (cache git version) declined as MINOR. Review B (sonnet): CLEAN.

## M5 review notes

- Review A (code-review high, fresh reviewer) CLEAN, 2 MINOR fixed: reach listing 429/5xx/rate-limit now `SourceUnreachable` (was fail-open); AtExpiry retry backoff clamped to the held PAT's expiry. Review B (conformance, sonnet) CLEAN; Verify green (85 core, 8 daemon, 7 client). Visual pass PASS (visual-pass/m5-github-tokens.md).
- Deviation: wiring uses a worker thread sleeping to `next_wake`, not the subscription clock; Add/Replace/Remove state lives in `settings::State.github_tokens` (not the settings document).

## Declined review findings

- M4 A+B F3 [MINOR]: announce_git_targets ordering race — the caller (M5 wiring) does not exist yet; revisit there.

- M1 B F1 [MAJOR] no `tdd/cycle-log.md` red evidence for T002–T005, T008, T009 (Constitution I): declined — already decided in D16 (tests kept as written, each asserts the removed item's absence), and the PR body discloses that those tests came after the code. Re-deriving red runs would mean reverting the removal against tests that no longer compile on the old code.
| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M3 | A | F2 [MINOR] insteadOf hardcodes github.com | base is always GITHUB_BASE (contract git-in-container.md §1) |
| M3 | A | F3/F4 [MINOR] pod drift stamp lacks the git volume; emptyDir mode | pods are rebuilt on a new image/spec stamp; one uid in the pod, so the daemon is the only reader; follow-up |
| M3 | A | F5 [MINOR] no expiry margin | the Schedule (M4) renews 10 min before expiry |
| M3 | A | F6/F7 [MINOR] test comment, main.rs dispatch | taste |

- M6 A: the take-once status channel copies `desktop_notify`'s pattern; a generic helper needs a per-type static (a macro) for two users — kept as is.
- M6 A: `changed_statuses` rebuilds every daemon's map per worker wake — a handful of projects per daemon, wakes are rare; not worth dirty tracking.
- M6 A: the expiry shows as a UTC date, as the M5 settings rows do; one convention for both.
- Close verification F10 (MED, eager grant test in `git_credential_grant.rs`): the steps are one sequence on one daemon (a replacement needs a prior grant, an idempotent revoke a prior revoke); splitting repeats the daemon setup per step for no new coverage.
- Close verification F12, `GIT_CREDENTIAL_ROOT == "/run/micold/git"`: kept; that path is a contract with the container's tmpfs mount and the image's helper, not an internal constant. The lifetime pins were removed.

## Handover

None.

## Open escalation

None. D7 answered by jaro (option B, pasted PAT).

## Follow-ups not done

- Delete the dead minted-token path (`ScopedCredential::accept`, `RenewPolicy::Renew`, `MINTED_LIFETIME_SECS`, the `Grant` extras) and move the core `Schedule` tests onto a PAT-shaped fake (research R13, tasks T061).
- Run the FR-014 container start-up test under `cargo test -p micold-core` too: it needs `kube-fake`, so today only `cargo test --workspace` runs it (as CI does). A self dev-dependency would turn the feature on for the core-only clippy step as well, which is the one check of core with it off (tasks T063).
- Jira API tokens live under `data_dir()/jira`, which the legacy default container daemon mounts whole as its state dir: readable inside that sandbox. Out of #1002's scope (not a git credential); worth its own issue.

# Autopilot ledger — #1532 daemon-config-file (Issue 11)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1532` (AI CLI configuration from the file for every kind); labels: `flow:feature`, `effort:high`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526, #1527, #1528, #1534 and #1535 (see `autopilot-1527.md` D1); the tasks unit only cuts milestones for the "Issue 11" group (T059–T067 incl. T065a–T065c, T096–T100).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1532
- **Worktree branch**: feat/1532_ai-cli-configuration-from-the-file-for-every-kind
- **Started**: 2026-11-05
- **Phase**: milestone M1
- **Next step**: open the milestone PR, wait on CI, merge

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1699 | design (tasks + milestones for Issue 11) | merged | 98aa6edcc561301f70c671b74113b9877ce2d9fd |

## Milestones

| M | Deliverable | Tasks | Tier | PR |
|---|---|---|---|---|
| M1 | each `[[ai.cli]]` field maps to the CLI's launch; configured args/env add to the app's own | T059–T064 | full | pending |
| M2 | AI sign-in delivery, read-only on every kind; stays valid without a CLI refresh | T065, T065a, T065b, T065c | full | — |
| M3 | gap-review extras: seeded `.claude.json`, onboarding, seeded settings files, `use_machine_sign_in`, sign-in expiry | T096–T100 | full | — |
| M4 | Settings → Daemons renders `[[ai.cli]]`, and the user guide | T066, T067 | full | — |

Split along milestones.md rule 3 (17 tasks, one user story US4, no sub-stories to split on):
M1 is the field-to-launch mapping (tests + implementation together, rule 7); M2 is the read-only
sign-in delivery plus the OAuth-refresh replacements that make removing the writable mount safe
(T065 cannot ship without T065a/T065b, else a relayed sign-in goes stale mid-session); M3 is the
gap-review (research D15) amendments that depend on M2's delivery actually existing; M4 holds the
UI back until the behaviour it configures exists (rule 6, no half-wired UI on `main`) and carries
the user-guide task with it (rule 4).

Checked 2026-11-05 against the working tree (sibling runs found stale references — fixed here):
- `crates/micold-core/src/sandbox/mod.rs`: `CredentialShare::writable` is at lines 318-323 (fn
  body 321-323), not `317-322` as tasks.md said — fixed in T065.
- `crates/micold-core/src/sandbox/mod.rs`: the `CredentialMount` struct is at 704-709 (`pub struct
  CredentialMount` at line 704), not `700-735` — fixed in T065 (the stated range covered more than
  the struct; narrowed to what T065 actually points at).
- `crates/micold-core/src/kube/credentials.rs`: the `mount()` fn with the `sub_path` the pod
  Secret drops is at 86-99 (`pub fn mount` at line 86, `SignIn` arm at 98), not `73-79` — fixed in
  T065.
- `crates/micold-daemon/src/state.rs`: `cli_would_ask_trust` is at 2078-2095, not `1914-1935` —
  fixed in T096.
- `crates/micold-core/src/sandbox/mod.rs`: `ONBOARDING_KEY`/`onboarding_done` are at 508-543 (was
  `507-530`, close but off by one and short of the full fn); the "today only with a writable
  share" context (`onboarding_record`, reading `CredentialShare::AiCliAuth`) is at 963-975, not
  `930-939` — fixed in T097.
- Files T059–T064 name that do not exist yet (`crates/micold-core/src/daemon_config/ai_cli.rs`,
  `crates/micold-daemon/tests/ai_cli_config.rs`) are new files the failing tests create; not
  stale. `crates/micold-daemon/tests/ai_cli_availability.rs`, `crates/micold-core/src/provision/
  ai_cli.rs`, `crates/micold-daemon/src/state.rs`, `crates/micold-daemon/src/hooks.rs`,
  `crates/micold-client/src/ui/settings/daemons.rs`, `docs/user-guide/sandboxed-daemon.md`,
  `docs/user-guide/settings.md`, `docs/user-guide/daemon-configuration.md` all exist as named.
- `AiCli::ALL` (`crates/micold-core/src/session.rs:76`) lists `ClaudeCode, Copilot, Pi, Codex,
  OpenCode` — matches the CLIs named across T059–T100.
- Dependencies: Issue 5 (#1526) M1a and M1b merged; Issue 6 (#1527) M1 merged; Issue 7 (#1528) M1
  (PR #1690) merged on `main` (issue #1528 itself stays open only for its docs close PR #1692) —
  M1's "Depends on" line is satisfiable today.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec? | No: design merged on main (PR #1509, `specs/daemon-config-file/spec.md` FR-013/FR-013a-g/FR-025/FR-033/FR-034/FR-040/FR-043, SC-006/SC-008; contract "`[[ai.cli]]`" section). Same precedent as #1524, #1525, #1526, #1527, #1528 and #1534 | agent-resolved | specs/daemon-config-file/autopilot-1527.md D1 |
| D2 | tasks | `speckit-analyze` F1 (HIGH): FR-013a's "unless the CLI documents which of the two wins" exception contradicts the contract's and data-model's unconditional "at most one of `sign_in_file`/`token_file`/`api_key_file`/`use_machine_sign_in`" | Dropped the exception clause from FR-013a: no CLI in `AiCli::ALL` documents a precedence, and the schema (contract + data-model, already merged) is deliberately the stricter, unconditional rule. Prose-only fix, no new round (review-rounds.md exception) | agent-resolved | speckit-analyze F1; contracts/daemon-config.md row `api_key_file`; data-model.md "AiCli" |
| D3 | tasks | Resumed unit found `origin/main` 11 commits ahead of this branch's HEAD (the ledger's recorded tip `93bc2a7f9` was stale), including one commit (#1531) editing `tasks.md`'s milestones heading line that our own uncommitted work also touched | Committed the uncommitted tasks/milestones work, then `git rebase origin/main`; one textual conflict on the shared heading line (both add an issue name to the same list) — resolved by keeping both additions (`Issue 10 (#1531)` and `Issue 11 (#1532)`), no content lost on either side. Verified no other conflict markers and all four Issue 11 milestone sections intact after the rebase | agent-resolved | `git log 93bc2a7f9..origin/main --oneline`; `git diff` of resolved `tasks.md` |
| D4 | milestone M1 | T062 names "no sign-in is ever mounted writable" and "the sign-in location is a link, re-created at every session start" — but sign-in delivery is M2's T065 (not yet implemented), and M1's Deliverable line only covers field-to-launch mapping | Scoped T062 to its first two clauses — `settings_file`/`place` land under the CLI's config dir, `place.as` cannot escape it — as pure path-mapping functions (`daemon_config::ai_cli::place_target`, `settings_file_name`), consistent with M1's Verify line (no sandbox/kube integration command listed) and Deliverable text. The sign-in clauses are M2/T065's own stated scope; left for that milestone | agent-resolved | tasks.md M1 Verify line; tasks.md T065 |
| D5 | milestone M1 | Review A round 2 F1 (BLOCKER): `AiCliTable::configured(...)` (T061's version/enabled override) is never called from the real client/kube provisioning paths — is wiring them in-scope for M1? | No: neither `kube::start::pod_target` nor `micold-client::daemon::container_target` has a per-CLI `version`/`args`/`model` value available today — both build their `AiCliSelection`/`KubeSettings.providers` from a bare `BTreeSet<AiCli>`, with no field for any `[[ai.cli]]` key beyond which CLIs are chosen. That data model, and the Settings UI that would produce it, is M4's own named scope ("Settings → Daemons renders `[[ai.cli]]`", T066/T067) — wiring `.configured()` into those call sites now would mean inventing M4's data model inside M1. `AiCliTable::configured` stays the pure, tested builder M1's Deliverable and Verify line (`provision::ai_cli`, core-only) call for; M4 calls it once its own Settings-side `[[ai.cli]]` values exist | agent-resolved | `grep -rn "AiCliTable::pinned()\|table: AiCliTable" crates/` outside tests; tasks.md M4 line; `KubeSettings.providers`/`AiCliSelection` definitions |

## Review rounds

Tasks review A round 1 (snapshot `08da6558cfb79253faee3b170a8c04b4666d9efa:0144800f6c91e1ce455e0ed636ba5c517d601826`): CHANGES. F1 [MAJOR] T096/T097/T099/T100 named no failing test — fixed, each now leads with "Failing test in `<file>`". F2 [MAJOR] M3's Verify line missed a command for T100's pod-Secret re-delivery — fixed, added `cargo test -p micold-daemon --test config_reload` and `cargo test -p micold-core kube::credentials`. F3 [MINOR] FR-040 looked fully satisfied in M2's Satisfies line — fixed, M2's Deliverable and Satisfies note it is only partial there (T065a's expiry warning), full `expiresAt` display is M3's T100. All three fixed, no new round needed if the fix diff holds (review-rounds.md round 2).

Tasks review A round 2 (sonnet, fix diff only; snapshot `a0459e0031fc4415dd96b496c1c308ca07e11886:c8d8ea65b9e5a5d8c25228f0383c1c912b0538a7`): CLEAN. All three round 1 fixes confirmed to hold; both new Verify targets (`config_reload.rs`, `kube::credentials`) confirmed to exist. (Earlier snapshot `3ad4fced4370a500be17fe9f2d2a4553dfbaba24:93bc2a7f9a0a8723aa5ec8061cfe36c235c9f6f3` went stale when the branch rebased onto `origin/main` past 11 new commits, one of which touched `tasks.md`'s milestones heading line; resolved with no content loss, see note below.)

Milestone M1 review A round 1 (code-review skill equivalent, general-purpose agent, high effort; no snapshot — pre-commit working tree): CHANGES. F1 [BLOCKER] two `StatusContext` literals (`tests/mcp/worktree_guard_sessions.rs`, `tests/settings/plan_usage_setting.rs`) missed the new `api_key_helper` field — fixed, both literals now carry `api_key_helper: None`. F2 [MAJOR] a config-disabled CLI's start refusal reused the "not on PATH" sentence, misleading when the CLI is actually installed — fixed in `state.rs`'s `start_session_gated`: `ai_cli_disabled` is checked first and gets its own reason ("… is disabled in the daemon's configuration"). No other findings. All fixed, confirmed by a green `mise run check`, `cargo clippy --workspace --all-targets -D warnings`, and the scoped gate (one unrelated pre-existing failure, `exclusivity::a_displaced_client_is_not_terminated`, reproduced identically on `git stash`'s clean `98aa6edcc` baseline — not this diff's).

Milestone M1 review A round 2 (sonnet, full diff — round 1 took no snapshot): CHANGES. F1 [BLOCKER] `AiCliTable::configured(...)` (the `version`/`enabled` override, T061) is never called from the real provisioning call sites (`crates/micold-core/src/kube/start.rs`'s `pod_target`, `crates/micold-client/src/daemon.rs`'s `container_target`) — declined, see D5. F2 [MAJOR] `launch_effort`'s model lookup used the session's raw, pre-override model while the sibling `model` field in the same `LaunchSpec` used `effective_model_for`'s overridden one, so an effort level could pass unfiltered for a model that is not what actually launches — fixed at both `LaunchSpec` construction sites (start and respawn paths) in `state.rs`: the effective model is now resolved once and fed to both `launch_effort` and `model`. F3 [MINOR] `launch_overlay`'s `args`/`model` fields are unit-tested but unread in production (`state.rs` re-derives the same mapping through `ai_cli_args_for`/`effective_model_for`) — declined, see D5. F4 [MINOR] the disabled-CLI refusal path (round 1's F2 fix) had no direct test of the refusal reason itself — fixed, added `a_disabled_cli_refuses_the_start_with_its_own_reason_not_not_on_path` to `tests/ai_cli_config.rs`, asserting the reason names "disabled" and never "path", and that the CLI never actually launches. F2 and F4 fixed, confirmed by a green `mise run check`, `cargo fmt --check`, and `cargo test -p micold-daemon --test ai_cli_config --test ai_cli_availability` (24 passed). F1 and F3 declined (D5): no new round required, since review-rounds.md's round limit counts rounds that fix a BLOCKER/MAJOR of *that* round, and this round's remaining open findings are declined, not deferred-but-agreed.

## Gate

Rebased onto `origin/main` (8005e00e9 on top), clean (no conflicts). `mise run check` and `cargo clippy --workspace --all-targets -- -D warnings` both clean at `8005e00e9`. The full `mise run gate` was run twice; both runs failed only on tests outside this diff's files (`exclusivity::a_displaced_client_is_not_terminated`, `history_damaged::*`, `history_stop_request::*`, `mcp::worktree_refresh::*`, `micold-material::ellipsized::*`, and once `micold-core::streaming_child::*`) — none of them touch `daemon_config`, `provider`, `terminal`, `provision`, `state.rs`'s AI-CLI code, `hooks.rs`, or `ai_key.rs`, and the set and content changed between the two runs (classic load-timing flakiness; the machine was building several other worktrees concurrently throughout, per the repeated `build-lock.sh: another build holds the lock` lines). `a_displaced_client_is_not_terminated` was separately reproduced on a clean `git stash`'d checkout of this branch's pre-M1 tip (`98aa6edcc`), confirming it predates this diff. The scoped gate (`scripts/autopilot/scoped-gate.sh`, fmt + clippy + the changed crates' tests) is green except that same pre-existing flake. Per `gate.md`'s fallback, recorded the scoped-gate-passing tree and pushed with "Local gate: scoped only, full gate left to CI" — CI's full-workspace run, on a quieter machine, is the tie-breaker; a red CI finding for one of these six tests is this gate failing (`red-ci.md`), any other red is not.

## Declined review findings

- M1 review A round 2, F1 [BLOCKER]: `AiCliTable::configured(...)` not called from the real provisioning call sites (`kube::start::pod_target`, `micold-client::daemon::container_target`). Declined — see D5: neither production call site has a per-CLI `version`/`enabled` value to pass in today (`KubeSettings.providers` and `AiCliSelection` are both a bare `AiCli` set, no version/args/model field exists on the client side yet); that data model is M4's own stated scope ("Settings → Daemons renders `[[ai.cli]]`", T066/T067). `AiCliTable::configured` is exactly what M1's Deliverable and Verify line (`cargo test -p micold-core provision::ai_cli`, core-only) call for: the pure builder, ready for M4 to call once the Settings-side data exists.
- M1 review A round 2, F3 [MINOR]: `daemon_config::ai_cli::launch_overlay`'s `args`/`model` fields unread in production (state.rs re-derives via `ai_cli_args_for`/`effective_model_for`). Declined: `effective_model_for` additionally validates the model string with `ModelName::parse` (`launch_overlay` does not — it is a pure capability-mapping function tested in isolation, not the only place the mapping has to be applied correctly), and `ai_cli_args_for` is a one-line accessor; collapsing them would couple the launch-spec wiring to a type (`LaunchOverlay`) built for a different purpose (env mapping, including the two API-key-reaching-env rules). Left as two small, independently correct functions rather than one with a secondary mode.

## Handover

None.

## Open escalation

None.

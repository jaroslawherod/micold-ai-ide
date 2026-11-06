# Autopilot ledger — #1532 daemon-config-file (Issue 11)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1532` (AI CLI configuration from the file for every kind); labels: `flow:feature`, `effort:high`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526, #1527, #1528, #1534 and #1535 (see `autopilot-1527.md` D1); the tasks unit only cuts milestones for the "Issue 11" group (T059–T067 incl. T065a–T065c, T096–T100).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1532
- **Worktree branch**: feat/1532_ai-cli-configuration-from-the-file-for-every-kind
- **Started**: 2026-11-05
- **Phase**: milestone M2a
- **Next step**: implement T065 (container/pod/host-SSH delivery, design in the ledger's
  *Handover*); gate, review, open the M2a PR. M2b (T065a–T065c) is a separate unit's scope — see D8.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1699 | design (tasks + milestones for Issue 11) | merged | 98aa6edcc561301f70c671b74113b9877ce2d9fd |
| #1711 | M1 — each `[[ai.cli]]` field maps to the CLI's launch | merged | 8470f7464ec548474530511495dd9ded211e5ba4 |

## Milestones

| M | Deliverable | Tasks | Tier | PR |
|---|---|---|---|---|
| M1 | each `[[ai.cli]]` field maps to the CLI's launch; configured args/env add to the app's own | T059–T064 | full | #1711 (merged, `8470f7464ec548474530511495dd9ded211e5ba4`) |
| M2a | sign-in delivery, read-only on every kind (SC-008) | T065 | full | — |
| M2b | keeping a delivered sign-in valid without a CLI refresh | T065a, T065b, T065c | full | — |
| M3 | gap-review extras: seeded `.claude.json`, onboarding, seeded settings files, `use_machine_sign_in`, sign-in expiry | T096–T100 | full | — |
| M4 | Settings → Daemons renders `[[ai.cli]]`, and the user guide | T066, T067 | full | — |

Split along milestones.md rule 3 (17 tasks, one user story US4, no sub-stories to split on):
M1 is the field-to-launch mapping (tests + implementation together, rule 7); M2 was originally cut
as the read-only sign-in delivery plus the OAuth-refresh replacements together, on the theory that
T065 could not ship without T065a/T065b — split into M2a/M2b by D8 below once two consecutive units
proved that grouping did not fit a unit's context; M3 is the gap-review (research D15) amendments
that depend on M2a's delivery actually existing; M4 holds the UI back until the behaviour it
configures exists (rule 6, no half-wired UI on `main`) and carries the user-guide task with it
(rule 4).

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
| D6 | milestone M1→M2 | Open escalation claimed `git push` was blocked (category 4, missing access) in every prior unit's session | Resolved: the container restart had wiped the sandbox's global gitconfig (`GIT_CONFIG_GLOBAL=/var/lib/micold-ai-ide/gitconfig`); the orchestrator reapplied the documented working setup — `url."https://github.com/".insteadOf git@github.com:` plus `core.askPass=/home/jaro/.local/bin/gh-askpass.sh` (supplies `gh auth token`; the sandbox resets `credential.helper` via `GIT_CONFIG_KEY_0`, so askPass is what survives). `git push` and `scripts/autopilot/preflight.sh` both succeed now | decided by user / orchestrator-resolved | PR #1711 merged (`8470f7464ec548474530511495dd9ded211e5ba4`) via a push from this same worktree after the gitconfig fix |
| D7 | milestone M2 | Connectivity note, not a decision needing a question: should a stalled `git`/`gh` call be treated as a credential failure? | No: GitHub access from this container is intermittent — one round-robin IP black-holes port 443 (why `/home/jaro/.local/bin/gh` is a retry wrapper). A call that stalls ~130s and then reports a connect timeout should simply be retried | agent-resolved | orchestrator's briefing for this unit |
| D8 | milestone M2 | Two consecutive units spent their full context on M2's design (T065, T065a–c together) and wrote no code — does the milestone still fit one unit? | No: split M2 into M2a (T065 only — read-only sign-in delivery, SC-008) and M2b (T065a–T065c — keeping a delivered sign-in valid without a CLI refresh), each its own PR. `tasks.md`'s M2 section replaced with M2a/M2b per `milestone-format.md`; M3's `Depends on` repointed at M2b. This unit's own scope is M2a | agent-resolved | this unit's briefing (two 150k handovers with no code, `autopilot-1532.md`'s own prior two *Handover* sections); `specs/daemon-config-file/tasks.md` M2a/M2b sections |

## Review rounds

Tasks review A round 1 (snapshot `08da6558cfb79253faee3b170a8c04b4666d9efa:0144800f6c91e1ce455e0ed636ba5c517d601826`): CHANGES. F1 [MAJOR] T096/T097/T099/T100 named no failing test — fixed, each now leads with "Failing test in `<file>`". F2 [MAJOR] M3's Verify line missed a command for T100's pod-Secret re-delivery — fixed, added `cargo test -p micold-daemon --test config_reload` and `cargo test -p micold-core kube::credentials`. F3 [MINOR] FR-040 looked fully satisfied in M2's Satisfies line — fixed, M2's Deliverable and Satisfies note it is only partial there (T065a's expiry warning), full `expiresAt` display is M3's T100. All three fixed, no new round needed if the fix diff holds (review-rounds.md round 2).

Tasks review A round 2 (sonnet, fix diff only; snapshot `a0459e0031fc4415dd96b496c1c308ca07e11886:c8d8ea65b9e5a5d8c25228f0383c1c912b0538a7`): CLEAN. All three round 1 fixes confirmed to hold; both new Verify targets (`config_reload.rs`, `kube::credentials`) confirmed to exist. (Earlier snapshot `3ad4fced4370a500be17fe9f2d2a4553dfbaba24:93bc2a7f9a0a8723aa5ec8061cfe36c235c9f6f3` went stale when the branch rebased onto `origin/main` past 11 new commits, one of which touched `tasks.md`'s milestones heading line; resolved with no content loss, see note below.)

Milestone M1 review A round 1 (code-review skill equivalent, general-purpose agent, high effort; no snapshot — pre-commit working tree): CHANGES. F1 [BLOCKER] two `StatusContext` literals (`tests/mcp/worktree_guard_sessions.rs`, `tests/settings/plan_usage_setting.rs`) missed the new `api_key_helper` field — fixed, both literals now carry `api_key_helper: None`. F2 [MAJOR] a config-disabled CLI's start refusal reused the "not on PATH" sentence, misleading when the CLI is actually installed — fixed in `state.rs`'s `start_session_gated`: `ai_cli_disabled` is checked first and gets its own reason ("… is disabled in the daemon's configuration"). No other findings. All fixed, confirmed by a green `mise run check`, `cargo clippy --workspace --all-targets -D warnings`, and the scoped gate (one unrelated pre-existing failure, `exclusivity::a_displaced_client_is_not_terminated`, reproduced identically on `git stash`'s clean `98aa6edcc` baseline — not this diff's).

Milestone M1 review A round 2 (sonnet, full diff — round 1 took no snapshot): CHANGES. F1 [BLOCKER] `AiCliTable::configured(...)` (the `version`/`enabled` override, T061) is never called from the real provisioning call sites (`crates/micold-core/src/kube/start.rs`'s `pod_target`, `crates/micold-client/src/daemon.rs`'s `container_target`) — declined, see D5. F2 [MAJOR] `launch_effort`'s model lookup used the session's raw, pre-override model while the sibling `model` field in the same `LaunchSpec` used `effective_model_for`'s overridden one, so an effort level could pass unfiltered for a model that is not what actually launches — fixed at both `LaunchSpec` construction sites (start and respawn paths) in `state.rs`: the effective model is now resolved once and fed to both `launch_effort` and `model`. F3 [MINOR] `launch_overlay`'s `args`/`model` fields are unit-tested but unread in production (`state.rs` re-derives the same mapping through `ai_cli_args_for`/`effective_model_for`) — declined, see D5. F4 [MINOR] the disabled-CLI refusal path (round 1's F2 fix) had no direct test of the refusal reason itself — fixed, added `a_disabled_cli_refuses_the_start_with_its_own_reason_not_not_on_path` to `tests/ai_cli_config.rs`, asserting the reason names "disabled" and never "path", and that the CLI never actually launches. F2 and F4 fixed, confirmed by a green `mise run check`, `cargo fmt --check`, and `cargo test -p micold-daemon --test ai_cli_config --test ai_cli_availability` (24 passed). F1 and F3 declined (D5): no new round required, since review-rounds.md's round limit counts rounds that fix a BLOCKER/MAJOR of *that* round, and this round's remaining open findings are declined, not deferred-but-agreed.

## Gate

Rebased onto `origin/main` (8005e00e9 on top), clean (no conflicts). `mise run check` and `cargo clippy --workspace --all-targets -- -D warnings` both clean at `8005e00e9`. The full `mise run gate` was run twice; both runs failed only on tests outside this diff's files (`exclusivity::a_displaced_client_is_not_terminated`, `history_damaged::*`, `history_stop_request::*`, `mcp::worktree_refresh::*`, `micold-material::ellipsized::*`, and once `micold-core::streaming_child::*`) — none of them touch `daemon_config`, `provider`, `terminal`, `provision`, `state.rs`'s AI-CLI code, `hooks.rs`, or `ai_key.rs`, and the set and content changed between the two runs (classic load-timing flakiness; the machine was building several other worktrees concurrently throughout, per the repeated `build-lock.sh: another build holds the lock` lines). `a_displaced_client_is_not_terminated` was separately reproduced on a clean `git stash`'d checkout of this branch's pre-M1 tip (`98aa6edcc`), confirming it predates this diff. The scoped gate (`scripts/autopilot/scoped-gate.sh`, fmt + clippy + the changed crates' tests) is green except that same pre-existing flake. Per `gate.md`'s fallback, recorded the scoped-gate-passing tree and pushed with "Local gate: scoped only, full gate left to CI" — CI's full-workspace run, on a quieter machine, is the tie-breaker; a red CI finding for one of these six tests is this gate failing (`red-ci.md`), any other red is not.

A third full `mise run gate` at HEAD `2d5d3973c` (after the rebase onto `origin/main` and the red-CI fix `289e83961`) again failed only outside this diff's files — a larger set this time (`micold-daemon::autospawn`, `client_restart_input`, `connection::logging_sink::*`, `daemon_stop::*`, `history_damaged::*`, `history_stop_request`, `idle_race`, `idle_stop::*`, `idle_teardown`, `spawn_fd_hygiene`, `version_mismatch_resolves`, `micold-material::ellipsized`), under `uptime`'s load average 3.6–4.7 from concurrent builds in other worktrees; none touch this diff's files (confirmed: `git diff <merge-base origin/main>..HEAD -- crates/` lists only `daemon_config/ai_cli.rs`, `provision/ai_cli.rs`, `provider.rs`, `terminal.rs`, `state.rs`, `hooks.rs`, `ai_key.rs` and their own tests, all of which passed). The scoped gate (`SCOPE workspace`, since the diff touches `micold-core`) then also failed, once, on a single unrelated test — `micold-client::ui::material::card_grid::tests::a_link_scrolled_out_of_the_body_is_not_under_the_pointer` — under load average 21.3 (another worktree, `feat-1531_gh-token-for-gh-in-sessions-of-every-kind`, holding the build lock); re-run alone once that build finished, it passed in 0.04s, confirming load-induced flakiness, not a regression. Per `gate.md`'s fallback (same precedent as above), recorded `HEAD^{tree}` (`2d5d3973c`'s tree) in this worktree's `autopilot-gate-ok` stamp file. `git push` itself remains blocked — see *Open escalation*.

## Red CI

PR #1711, run `42014070087`: `build + test (windows-latest)`, step "Test (render-free core, no
GUI)" failed — `daemon_config::ai_cli::tests::place_as_stays_inside_the_config_dir` asserted
`place_target(dir, Path::new("/absolute")) == None` but got `Some("/absolute")` on Windows. Cause:
`place_target`'s escape guard used `Path::is_absolute()`, which is host-specific — a leading `/`
is not absolute by Windows's own rule, so the guard let it through there while correctly rejecting
it on Linux/macOS. Every other CI job (including the real-runtime jobs) passed.

Fixed the guard, not the test: T062's "`place.as` cannot escape the CLI's config dir" is a security
property, and a guard that is weaker on one platform than another (the config is authored once,
read by whichever platform's daemon runs it) is exactly the kind of gap this milestone exists to
close. Added `is_absolute_on_any_platform` in `crates/micold-core/src/daemon_config/ai_cli.rs`
(`place_target` now calls it instead of the host-specific `is_absolute()`): true for the host's own
`is_absolute()`, plus a leading `/` or `\`, plus a drive letter (`C:...`) — so a POSIX-only or a
Windows-only absolute path is rejected on every platform, not just its own. Added
`place_as_rejects_an_absolute_path_foreign_to_the_host_platform`, asserting `/absolute`, `\absolute`,
`C:\escape` and `C:/escape` are all rejected — this test fails on Linux for the pre-fix `/absolute`
case would not have (Linux's `is_absolute()` already caught that one); the new cases
(`\absolute`, `C:\escape`, `C:/escape`) are the ones no platform's host-native `is_absolute()`
alone would catch everywhere, so they would have caught this bug on Linux too, not just on
Windows's own CI run.

Local gate: `mise run check`, `cargo clippy -p micold-core --all-targets -- -D warnings` clean.
Full `mise run gate`: 19 failures, none in `ai_cli`/`daemon_config`/`provision` — all in daemon
spawn/socket-accept timing (`autospawn`, `exclusivity`, connection/history tests) and one
`micold-material::ellipsized` shaping test, matching the same load-related flakiness class M1's
first gate round already recorded (this machine was building other worktrees concurrently). Scoped
gate: fmt/clippy clean, `ai_cli_config.rs`'s 3 tests pass; one unrelated failure
(`autospawn::a_client_cold_starts_a_daemon_and_it_outlives_the_client`, a 20s socket-accept
timeout) reproduced twice in isolation under the same concurrent-build load — pre-existing, not
this diff's. Per `gate.md`'s fallback, recorded the scoped-gate tree and pushed with "Local gate:
scoped only, full gate left to CI"; CI's quieter machine is the tie-breaker.

## Declined review findings

- M1 review A round 2, F1 [BLOCKER]: `AiCliTable::configured(...)` not called from the real provisioning call sites (`kube::start::pod_target`, `micold-client::daemon::container_target`). Declined — see D5: neither production call site has a per-CLI `version`/`enabled` value to pass in today (`KubeSettings.providers` and `AiCliSelection` are both a bare `AiCli` set, no version/args/model field exists on the client side yet); that data model is M4's own stated scope ("Settings → Daemons renders `[[ai.cli]]`", T066/T067). `AiCliTable::configured` is exactly what M1's Deliverable and Verify line (`cargo test -p micold-core provision::ai_cli`, core-only) call for: the pure builder, ready for M4 to call once the Settings-side data exists.
- M1 review A round 2, F3 [MINOR]: `daemon_config::ai_cli::launch_overlay`'s `args`/`model` fields unread in production (state.rs re-derives via `ai_cli_args_for`/`effective_model_for`). Declined: `effective_model_for` additionally validates the model string with `ModelName::parse` (`launch_overlay` does not — it is a pure capability-mapping function tested in isolation, not the only place the mapping has to be applied correctly), and `ai_cli_args_for` is a one-line accessor; collapsing them would couple the launch-spec wiring to a type (`LaunchOverlay`) built for a different purpose (env mapping, including the two API-key-reaching-env rules). Left as two small, independently correct functions rather than one with a secondary mode.

## Handover

Milestone M2 (T065, T065a, T065b, T065c) still not started on the tree: no code changes (the tree
is clean, identical to `0b3324d41`). This round went deeper into T065's exact mechanics — far
enough to find that the previous round's plan needed one correction before coding — and ran out of
context before writing any code. The design below is ready to implement directly; do not re-derive
it.

**Correction to the previous round's plan**: do **not** change `CredentialMount.container`'s
meaning, and do **not** change `CredentialLayout.ai_cli_auth`'s type. Those two are read by several
other `MountSet` methods that assume `container` is the CLI's own *live* path, under
`home.container` (`home_dirs_to_create`, `home_files_to_create` — BUG-006/BUG-007, do not regress
them; `onboarding_record` — BUG-007; `shared_locations`/`container_projects` — link recognition,
research R10; `unshared_sign_in` — FR-004g/BUG-008). Relocating `container` itself (what the
previous round's investigation suggested) would require reworking every one of those, each with its
own test file, which is far more than this task needs. Instead, **add one field**:

```rust
pub struct CredentialMount {
    pub share: CredentialShare,
    pub host: PathBuf,
    pub container: PathBuf,      // unchanged meaning: the CLI's own live path
    pub delivered: Option<PathBuf>, // NEW: where it is actually bind-mounted read-only, when that
                                     // differs from `container` (T065, research D13)
}
```

`delivered` is `None` for `GitConfig` (mounted directly at `container`, as today) and
`Some(<path under CONTAINER_SECRETS_DIR>)` for `AiCliAuth`. Every other `MountSet` method keeps
working unmodified because `container` still means what they all assume.

**The delivered path, and why it needs no cross-process information at all**: build it as
`format!("{CONTAINER_SECRETS_DIR}/ai-cli-auth/{rel}")` where `rel` is `host`'s path *relative to
`home`*, joined with `/` regardless of platform (the same pattern `CredentialLayout::present_under`
already uses inline at `mod.rs:787-793` — extract it to a shared private helper, e.g.
`fn relative_to(home: &Path, path: &Path) -> String`, and reuse it in both places). This
automatically avoids the Codex/OpenCode `auth.json` basename collision the previous round worried
about, with no need for a CLI id at all: `~/.codex/auth.json` and
`~/.local/share/opencode/auth.json` already have distinct relative suffixes under `home`.

Crucially, because `pathmap::map_for` is a pure prefix replacement (it does not touch the suffix
after the home prefix), `rel` computed from the **host**'s home and host path is byte-identical to
`rel` computed from the **container**'s own home (`HomeMount.container`) and the live path inside
the container. That means **the code running inside the container can recompute `delivered`
itself, from nothing but its own `HOME`**, with no host-side `CredentialLayout` or profile handed
to it:

```rust
/// `(live, delivered)` for every CLI whose sign-in would land under `home` (the sandbox's own
/// `HOME`, as seen from wherever this runs): `live` is where the CLI itself looks, `delivered` is
/// where the read-only mount actually puts the file. Self-contained — no host-side layout needed
/// — because `pathmap::map_for` never touches the suffix after the home prefix, so this produces
/// the same `delivered` whether computed on the host (building the `MountSet`) or inside the
/// sandbox (relinking at session start).
pub fn ai_cli_sign_in_links(home: &Path) -> Vec<(PathBuf, PathBuf)> {
    crate::session::AiCli::ALL
        .iter()
        .filter_map(|cli| {
            let live = cli.provider().sandbox_auth_file(home)?;
            Some((live.clone(), ai_cli_auth_delivered_path(home, &live)))
        })
        .collect()
}
```

Reuse this same function for both: (a) `MountSet::build_for`, to fill `CredentialMount.delivered`
for the `AiCliAuth` entries it is about to build (match the result back to each `host` path), and
(b) the daemon's own relink step inside the sandbox, called with its own `home.container` /
in-container `HOME`.

**The relink function** (new, in `sandbox/mod.rs`, pure/testable without the real-runtime
feature — no container, no docker — this is what the previous round's "relink function" item
meant):

```rust
/// Re-creates the symlink from each AI CLI's live sign-in path to where T065's read-only mount
/// actually delivered it, inside the sandbox, at every session start (never run outside a
/// sandbox kind — SC-008 forbids ever touching a host/ssh session's real sign-in file). A pair
/// whose `delivered` does not exist on disk is skipped (that CLI's sign-in was never shared, or
/// the opt-in is off): no dangling link is created. Removes whatever is at `live` first — a
/// stale link from a previous delivery, or (compatibility with containers made before this
/// change) the file itself. A failure on one pair is logged and skipped, never fatal (FR-019):
/// one CLI's sign-in failing to link must not stop a session over another's.
pub fn relink_ai_cli_sign_ins(home: &Path) -> Vec<PathBuf> {
    ai_cli_sign_in_links(home)
        .into_iter()
        .filter(|(_, delivered)| delivered.exists())
        .filter_map(|(live, delivered)| relink_one(&live, &delivered))
        .collect()
}
```
(`relink_one` does the actual `create_dir_all` + stale-link-or-file removal + `symlink`, logged on
failure — write it as a small private helper so the test can assert on `relink_ai_cli_sign_ins`'s
return value directly.)

**`mount_args` (`argv.rs`)**: drop the per-share `writable` branch entirely (every credential mount
is read-only now, including `GitConfig` was already — this removes the only `true`):
```rust
for c in &mounts.credentials {
    let container = c.delivered.as_ref().unwrap_or(&c.container);
    mount(&c.host.display(), &container.display(), false);
}
```
Then `CredentialShare::writable()` has no caller left in `mod.rs`/`argv.rs` — check
`crates/micold-daemon/tests/sandbox_real_ai_cli_sessions.rs:163` and
`sandbox_real_no_user_credentials.rs:83` (both call `c.share.writable()` directly in a hand-rolled
`bind_mount` helper for the real-runtime harness) before deleting the method; update both to assert
`false`/read-only instead, or inline `false` and delete `writable()` once those are the only
remaining callers.

**Why pre-creating a mount target under the secrets dir is not needed** (unlike
`home_dirs_to_create`/`home_files_to_create`'s reason for existing, BUG-006/007): those exist
because Docker auto-creates a missing bind *target* as root, and a root-owned file under the
sandbox's own writable home then blocks the CLI from ever writing there again once the share is
turned off. The new nested target under `CONTAINER_SECRETS_DIR` (itself already a read-only mount,
nested exactly as `home`'s own sub-mounts already are — precedent in `argv.rs`'s comment at
`mount_args`'s top) is **never written to by anything**, ever: a root-owned auto-created file there
is harmless. No new `*_dirs_to_create`/`*_files_to_create` analogue is needed. Say this once, as a
code comment, so a future reader does not "fix" it by adding one.

**Existing tests to update** (compile breaks from the new `delivered` field; both are struct
literals missing the field — add `delivered: None`):
- `crates/micold-core/src/sandbox/argv.rs:522-526` (`argv_mounts_are_exactly_the_mount_set`)
- `crates/micold-core/src/sandbox/argv.rs:587-591` (`every_mount_is_labelled_shared_exactly_when_selinux_enforces`)
Both only construct a `GitConfig` mount today, so `delivered: None` is correct for them unchanged;
neither test's mount-count assertions should need to change (the `AiCliAuth` tests are in
`crates/micold-core/tests/sandbox_credentials.rs`, not these). Verify those line numbers before
editing — this round read them but did not edit, so they may have moved by the time the next round
starts (none of the earlier rounds' finds here were stale, but always confirm before relying on a
line number).

**`crates/micold-core/tests/sandbox_credentials.rs`** (confirmed present, not re-read line by line
this round — the previous round's note that it has `each_opt_in_adds_exactly_one_mount`,
`the_ai_cli_sign_in_share_is_the_token_file_not_the_cli_directory`,
`a_shared_sign_in_names_its_file_in_the_sandbox_home_to_create`, and `sign_in_path()`/
`sharing_the_sign_in()` helpers around line 100-375 still stands): add assertions here, not new
test files, for (a) `delivered` is `Some` for an `AiCliAuth` mount and under
`CONTAINER_SECRETS_DIR/ai-cli-auth/`, `None` for `GitConfig`; (b) `relink_ai_cli_sign_ins` with a
tempdir `home` holding a fake delivered file creates the expected symlink and returns its live
path; (c) calling it again when the link already points at the right target is a no-op (no error,
same return); (d) calling it when `delivered` does not exist for a CLI skips that CLI with no
error and no link created.

**Still to design/implement after T065's container/pod half** (not reinvestigated this round,
carries over verbatim from the previous round's notes — still believed accurate but re-verify line
numbers before relying on them):
- **Pod** (`kube/credentials.rs`, `kube/manifest.rs`): give `KubeCredential::SignIn` a directory
  Secret with no `subPath`, mirroring `KubeCredential::GitConfig` exactly (`credentials.rs:91-97`,
  `GIT_CONFIG_DIR` constant at `:24`). Add a `SIGN_IN_DIR`-shaped constant/function (e.g. one
  directory per CLI under `/run/micold/secrets/sign-in/<cli>`, mirroring `GIT_CONFIG_DIR`'s
  single fixed constant but parameterised by CLI since there are five). `credential_mount`
  (`manifest.rs:216-226`) already drops `subPath` when `None`, so no change needed there; `pod()`'s
  volume loop (`manifest.rs:360-369`) and `home_dirs` (`manifest.rs:285-313`) already key off
  `credential.mount()`'s path being under `POD_HOME` or not — a `SIGN_IN_DIR` *outside* `POD_HOME`
  (like `GIT_CONFIG_DIR`) means `home_dirs` naturally excludes it too, no change needed there
  either. The pod's own daemon then needs the *same* `relink_ai_cli_sign_ins`-shaped call at
  session start, pointed at `HOME_DIR` (`credentials.rs:17`) and reading from `SIGN_IN_DIR`
  instead of `CONTAINER_SECRETS_DIR/ai-cli-auth` — if `ai_cli_sign_in_links`/
  `relink_ai_cli_sign_ins` end up hard-coding the container path, parameterise them by the secrets
  base so both container and pod share the one function; do not fork it.
- **The actual call site inside a running sandbox** (container or pod) that invokes the relink at
  "every session start": not found yet in either round. `crates/micold-daemon/src/state.rs` is
  where `with_configured_env`/`ai_cli_env_overlay` live (`:757-794`, `:2176-2194` — line numbers
  drifted slightly from the previous round's `:757-789`/`:2160-2191`, re-verify again next round)
  and is almost certainly where this belongs, called unconditionally for a sandbox-kind daemon
  (`daemon_config::Kind::is_sandbox()`, `daemon_config/mod.rs:97-100` roughly, re-verify) right
  before a session's AI CLI spawns — but this round did not trace the actual spawn call site
  (`start_session`/`start_session_gated` near `state.rs:6470-6512`, or `spawn_structured` near
  `:6806`) closely enough to say which one line to add the call at. That tracing is the first thing
  the next round should do, with an `Explore` subagent if it is not immediately obvious from
  `documentSymbol`/`findReferences` on `with_configured_env`'s one caller
  (`ai_cli_env_overlay`) — follow its own callers up from there.
- **Host/SSH side of T065** (the session gets `<data_dir>/ai/<id>/` as its config dir, sign-in and
  the user's own settings files linked in, per research D9's table): `daemon_config::ai_cli`'s
  `place_target`/`settings_file_name` (`crates/micold-core/src/daemon_config/ai_cli.rs:94-136`) are
  already the right pure path-mapping functions — they take a `config_dir: &Path` parameter, so
  they work unchanged for a host/ssh config dir, not only the sandbox home their current doc
  comment on `settings_file_name` singles out as the example (the comment should be loosened once
  this lands, since it will no longer be sandbox-only). What is still missing is the code that (a)
  decides `<data_dir>/ai/<id>/` is the config dir to hand the session (the env var:
  `CLAUDE_CONFIG_DIR`/`CODEX_HOME`/`XDG_DATA_HOME`, by CLI — this round did not locate where
  `with_launch_env`/`ai_cli_env_for` would be the place to add it, but that is the leading
  candidate, `state.rs:2176` area), and (b) actually creates the symlinks for `sign_in_file`,
  `settings_file`, and each `place` entry at session start (not yet written anywhere — `write_if_changed`
  at `state.rs:800-820` is the *write* helper for a different purpose, delivered `daemon.toml`
  itself, not a model for a symlink step). SC-008's byte-identical/mtime test: write a fake
  "machine's own" sign-in file, run a session through this path, assert
  `std::fs::read`/`metadata().modified()` unchanged on the original — this test does not exist yet.
- **T065a** (`token_file`): unchanged from the previous round's note — smallest of the four,
  reuses `with_configured_env`'s read-the-secret-file plumbing; do it once the host/ssh wiring
  above exists to hang it on. Env var is `CLAUDE_CODE_OAUTH_TOKEN` (Claude Code) /
  `COPILOT_GITHUB_TOKEN` (Copilot), never in argv.
- **T065b/T065c**: not investigated in either round beyond research D14 (quoted in the brief).
  T065b needs a new module under `crates/micold-client/src/` watching the host's own sign-in file
  with `notify` (check `crates/micold-client/src/daemon_config_delivery.rs` for the watcher pattern
  M1/#1527 already uses) and deriving the no-refresh-token copy per D14's per-CLI rules. T065c is
  docs-only, `docs/user-guide/sandboxed-daemon.md`.

**Suggested order for the next unit**: (1) `sandbox/mod.rs` — the `delivered` field,
`relative_to`/`ai_cli_auth_delivered_path`/`ai_cli_sign_in_links`/`relink_ai_cli_sign_ins`, and
`build_for`'s one-line change to fill `delivered`; (2) `argv.rs`'s `mount_args` plus its two struct
literals; (3) the `sandbox_credentials.rs` assertions above; (4) run `mise run check` and a targeted
`cargo test -p micold-core sandbox::` before touching anything else, to catch a mistake in this
design early and cheaply; (5) `kube/credentials.rs` + `kube/manifest.rs`; (6) find the daemon's
session-start call site and wire both container and pod relinks into it; (7) host/ssh config-dir
wiring + SC-008 test; (8) T065a; (9) T065b/T065c if context allows, else hand over again with the
same discipline this round used.

**Why handing over again without code**: finishing the cross-method correctness check above (which
`MountSet` methods assume `container` means "the live path", and confirming the simpler
`delivered`-field design does not regress any of them) took this round past the context cap before
any file was edited. Rather than leave a half-applied, non-compiling change for the next round to
untangle, this round reverted its one exploratory edit (`CredentialShare::writable()`'s removal)
and leaves the tree byte-identical to `0b3324d41`. The design above is more complete and more
confidently correct than the previous round's (which would have required touching five additional
methods this round found depend on `container`'s current meaning); implement from it directly.

## Open escalation

None.

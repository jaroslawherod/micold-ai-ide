# Autopilot ledger — #1532 daemon-config-file (Issue 11)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1532` (AI CLI configuration from the file for every kind); labels: `flow:feature`, `effort:high`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526, #1527, #1528, #1534 and #1535 (see `autopilot-1527.md` D1); the tasks unit only cuts milestones for the "Issue 11" group (T059–T067 incl. T065a–T065c, T096–T100).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1532
- **Worktree branch**: feat/1532_ai-cli-configuration-from-the-file-for-every-kind
- **Started**: 2026-11-05
- **Phase**: tasks
- **Next step**: wait on CI, merge the design PR; next unit: milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| M | Deliverable | Tasks | Tier | PR |
|---|---|---|---|---|
| M1 | each `[[ai.cli]]` field maps to the CLI's launch; configured args/env add to the app's own | T059–T064 | full | — |
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

## Review rounds

Tasks review A round 1 (snapshot `08da6558cfb79253faee3b170a8c04b4666d9efa:0144800f6c91e1ce455e0ed636ba5c517d601826`): CHANGES. F1 [MAJOR] T096/T097/T099/T100 named no failing test — fixed, each now leads with "Failing test in `<file>`". F2 [MAJOR] M3's Verify line missed a command for T100's pod-Secret re-delivery — fixed, added `cargo test -p micold-daemon --test config_reload` and `cargo test -p micold-core kube::credentials`. F3 [MINOR] FR-040 looked fully satisfied in M2's Satisfies line — fixed, M2's Deliverable and Satisfies note it is only partial there (T065a's expiry warning), full `expiresAt` display is M3's T100. All three fixed, no new round needed if the fix diff holds (review-rounds.md round 2).

Tasks review A round 2 (sonnet, fix diff only; snapshot `a0459e0031fc4415dd96b496c1c308ca07e11886:c8d8ea65b9e5a5d8c25228f0383c1c912b0538a7`): CLEAN. All three round 1 fixes confirmed to hold; both new Verify targets (`config_reload.rs`, `kube::credentials`) confirmed to exist. (Earlier snapshot `3ad4fced4370a500be17fe9f2d2a4553dfbaba24:93bc2a7f9a0a8723aa5ec8061cfe36c235c9f6f3` went stale when the branch rebased onto `origin/main` past 11 new commits, one of which touched `tasks.md`'s milestones heading line; resolved with no content loss, see note below.)

## Declined review findings

None yet.

## Handover

None.

## Open escalation

None.

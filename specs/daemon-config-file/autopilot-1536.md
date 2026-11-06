# Autopilot ledger — #1536 daemon-config-file (Issue 16)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1536` (SSH key, signing key, CA file and session files for sandboxes; part of #1521); labels: `flow:feature`, `effort:low`. Spec, Clarify, Plan skipped; design on main. Scope: the "Issue 16" task group (T101–T106), FR-036, FR-037, FR-038, contract `#git`, `#session`, `#network`.
- **Kind**: feature
- **Effort**: low
- **Issue**: #1536
- **Worktree branch**: feat/1536_ssh-key-signing-key-ca-file-and-session-files-for
- **Started**: 2026-11-06
- **Phase**: implement M1 (handover)
- **Next step**: after design PR merges: milestone M1 (T101–T106)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1748 | design: M1 milestone cut | merged | 3dafe3fdeff78991721162255acdeb460c724f98 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T101–T106 | full | SSH key, signing key, CA file, session files reach sandbox sessions | - | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | New spec for #1536? | No. FR-036..038 and the Issue 16 tasks already live in specs/daemon-config-file/; spec.md unchanged, so no spec review | agent-resolved | spec.md FR-036..038; tasks.md "Issue 16" |
| D2 | spec | Blockers #1524, #1527, #1528 | All CLOSED | agent-resolved | gh issue view |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Unit 1 of M1 hit 150k waiting on a long shared build-lock queue (use `setsid nohup` + `hold.sh`; a script file under the scratchpad written with Write, because a hook blocks the word `cargo` in a heredoc). Branch is reset to origin/main plus this uncommitted-then-committed WIP.

Done (code written, daemon tests green: `cargo test -p micold-daemon --lib session_extras` 9 passed, `supervisor` 26 passed):
- core: `daemon_config::session_file_refusal` (V18 extracted from validate.rs `session()`, exported); `daemons::{SessionExtras, SessionFileSetting}` and `FileSettings.extras`.
- daemon: `crates/micold-daemon/src/session_extras/{mod,tests}.rs` (T101 GIT_SSH_COMMAND, T102 signing key / gpgsign off, T103 CA env + `Egress` for ureq CA/HTTPS_PROXY/NO_PROXY, T104 `deliver_files`); wired in `supervisor.rs` `env_hygiene` via `config_extra_edits`, and `credential_config.rs` `UreqTransport` uses `Egress`.
- client: `daemon_config_render.rs` renders `[git]` extras, `[network]`, `[[session.files]]` from `DaemonSettings.extras` (T105 render part); `daemon.rs` `named_secrets` adds ssh-key / signing-key; test `the_sandbox_extras_render_and_parse_back` added in `tests/daemon_config_render.rs`; docs added in `docs/user-guide/daemon-configuration.md` (T106).
- tasks.md: T101-T104, T106 ticked. T105 NOT ticked.

Next step:
1. Run (queued, via build-lock): `cargo test -p micold-client --test main daemon_config` (the `FileSettings.extras` literal in `tests/daemon_config_validate.rs` was just fixed; also check other `DaemonSettings {..}` / `FileSettings {..}` literals compile, and goldens unchanged), `cargo test -p micold-client --lib daemon`, `cargo test -p micold-core --lib daemon_config` (last run failed only on a stale shared-target build.rs race: "read routing.rs" - rerun).
2. T105 gap: Settings -> Daemons UI inputs for the four extras (draft fields in `features/daemons.rs`, rows in `ui/settings/daemons.rs`) are NOT done; the model + render + secret delivery are. Decide: add the draft fields (mirror `client_token_file`) or record the UI inputs as a follow-up in *Follow-ups not done*; then tick T105.
3. Then verify.md, review A (high) in parallel with scoped gate, review B (sonnet), full `mise run gate`, PR per tasks/pr.md (title `feat(daemon-config-file): SSH key, signing key, CA file and session files for sandbox sessions (#1536)`, body ends `Refs #1536`).


## Open escalation

None.

## Follow-ups not done

None.

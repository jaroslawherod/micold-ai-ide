# Test redistribution disposition record (T079)

**Invariant (FR-041): no test was silently deleted.** The pre-split suite was redistributed into the
three-crate workspace by *role*; every pre-split concern has a current owner, and the total test
count **grew** (redistribution + the new daemon-feature coverage), it never shrank.

## Count reconciliation

| Point | Count | Source |
|---|---|---|
| Pre-split render-free core (`--no-default-features`) | 220 | T007 baseline note |
| Pre-split full suite | 259 | T007 baseline note |
| Post-split workspace total at Phase 1 (incl. per-crate doctests) | 294 | T007 baseline note |
| **Current workspace** (US1–US7 + Phase 10 added) | **690 test fns** across **98 test-binary groups** | `grep -c` of `#[test]`/`#[tokio::test]`; `cargo test --workspace` |

The current total (690) far exceeds the 259 pre-split baseline: the delta is (a) the redistribution
below and (b) the tests added for the daemon re-architecture (protocol, transport, catalog,
supervision, exclusivity, contract-recovery, logout, diagnostics). A silent deletion would show as a
*drop* against the baseline; the count only rises.

## Disposition by role (owning crate)

Redistribution rule (plan W0): **pure logic → `micold-core`; supervision / protocol / transport /
lifecycle → `micold-daemon`; render-coupled → `micold-client`.**

### `micold-core` — pure, render-free logic (279 test fns)
Pre-split pure-logic tests **moved** here unchanged where the type moved unchanged (session lifecycle,
workspace/project model, worktree model + discovery + rollback, naming, settings, store round-trips +
fault isolation, fs-scan, selector, theme, metadata, env-include, AI-CLI provider). Protocol tests are
**new** (the wire contract is new): `protocol_roundtrip`, `handshake`, `schema_hash`,
`input_ordering`, plus the `keepalive` unit tests. `session_crash_restart` was **rewritten** around the
`RestartDecision` FSM the daemon supervisor drives.

### `micold-daemon` — supervision / transport / lifecycle (103 test fns)
Almost entirely **new** (the daemon did not exist pre-split): `daemon_singleton`, `daemon_lifecycle`,
`autospawn`, `handshake_flow`, `framing`, `drive_loop`, `stream_view`, `slow_client`,
`reattach_snapshot`, `scrollback_range`, `scroll_cost`, `catalog_adoption`, `mutation_atomicity`,
`mutation_semantics`, `session_start`, `session_survival`, `session_isolation`, `shell_instances`,
`supervision_restart`, `supervision_giveup` (US4), `exclusivity`, `liveness` (US5), `version_recovery`
(US6), `diagnostics`, `log_events`, `log_redaction` (Phase 10). The pre-split root `tests/pty_routing.rs`
was **retired with reason**: the client no longer hosts a PTY (the daemon does), so PTY routing is now
covered by the daemon's `drive_loop`/`stream_view`/`session_isolation` suites rather than a client-side
routing test.

### `micold-client` — render-coupled (308 test fns)
Pre-split UI/interaction tests **moved** here (icons, keymap/keyboard, tokens, toolbar, sidebar,
notifications, project switcher, about dialog, theme, session archive/isolation/terminal-mode,
switch-active, worktree-delete, open-project git gate). `terminal_focus`, `client_input`,
`background_restart`, `session_title_sync` were **rewritten** against the daemon-streamed grid + the
`Outbox` drive path (the client no longer owns the terminal).

## Retired-with-reason (explicit, non-silent)

| Pre-split test | Reason retired | Where the concern now lives |
|---|---|---|
| `tests/pty_routing.rs` | Client no longer hosts a PTY (daemon owns it) | `micold-daemon` `drive_loop` / `stream_view` / `session_isolation` |
| root `src/lib.rs` unit tests (removed with the crate root) | The monolithic crate root was split into three crates | the owning crate's `src` unit tests |

## Gate

`cargo test --workspace` is green (98 groups, 0 failures), with the T007 baseline accounted for above.

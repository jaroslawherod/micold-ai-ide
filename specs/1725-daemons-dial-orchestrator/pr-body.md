## Milestone M1 of 5 — daemons dial the orchestrator (specs/1725-daemons-dial-orchestrator)

**Deliverable**: a host daemon with `[orchestrator_link] enabled = true` dials the local orchestrator and the registry lists it `online` with its announced fields; with the table absent nothing changes.
**Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_dial host` (plus `process::`, a real daemon process).
**Tasks**: T001–T015 · **Satisfies**: US1 scenarios 1–4; FR-001, FR-002, FR-003, FR-006, FR-010, FR-013; SC-001 (host), SC-005

What changed: the `[orchestrator_link]` table and rule L1; `encode_announcement`; pure `RetryPolicy`, `LinkStatus`, `jittered` and the endpoint rules in `micold-core`; the `orchestrator_link` task in `micold-daemon` (spawned, never awaited, after both listeners bind); `connect::hello_message` extracted from `handshake_with`; docs.

## Agent review
- Review A (headless `autopilot-reviewer`, opus, then sonnet on the fix diff): 4 findings, 4 fixed. MAJOR: no test of the wiring in `server::run`, fixed by `tests/orchestrator_dial/process.rs` (real `micold-daemon --config`; a mutant dropping the spawn fails it). MINORs: warn when an enabled link's endpoint cannot be resolved, non-empty asserts in an independence test, Windows wording in docs. Round 2: CLEAN.
- Review B (conformance, sonnet): dispatched by the orchestrator after this PR opens.

## Local gate
`scripts/gate.sh` passed (see the ledger for the tree). No `cfg(target_os)` arm and no sandbox code changed; the change is not visual.

Refs #1725

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvKoC4hgVEmG5JeBfrz

# Research: supervisor skill

## D1. Split into skill (effects) and helper script (decisions)
Decision: `supervisor.sh` computes gate, mapping, overlap and the start list; the skill executes.
Rejected: all logic in skill prose (not reproducible, FR-010 fails); a Rust tool (over-weight for
tooling, the autopilot helpers are bash, and the build is the scarce resource).

## D2. Ledger format: markdown table at `.autopilot/supervisor.md`
Decision: header lines (`- **Owner**: <session id>`, `- **Cap**: n`) plus one table row per issue;
parsed with `awk`. Consistent with the autopilot's markdown ledgers, human-readable, diff-free
(gitignored). Rejected: JSON (unreadable by the person mid-run); a file in `specs/` (would land on a
feature branch; clarify decision).

## D3. State comes from `run-status.sh --json <worktree>` only
Decision: per recorded worktree, one call. Rejected: `read_session_output` (transcript growth,
FR-007); `gh pr view` per issue (the run-status script already folds PR state in).

## D4. Liveness from `list_sessions` metadata
Decision: used only for a missing session id and for the owner check. The skill passes the listed
ids to the helper (`--live-sessions "id1 id2"`) so decisions stay fixture-driven.
Rejected: reading session state from `list_sessions` (metadata has no autopilot phase; FR-007).

## D5. Overlap
Decision: `Touches:` line in the issue body (comma or space separated paths/globs); else infer from
the labels `flow:*` and any `specs/<dir>` path in the body; else empty = disjoint, noted. Two sets
overlap when any path equals the other or one is a path prefix of the other (directory containment).
Rejected: glob expansion against the tree (the files may not exist yet).

## D6. Cross-platform readings
Decision: `/proc/meminfo` `MemAvailable` on Linux; on macOS free+inactive+speculative pages from
`vm_stat` times page size. Load from `uptime`. Env overrides `AUTOPILOT_SUP_MEM_MIB`,
`AUTOPILOT_SUP_LOAD`, `AUTOPILOT_SUP_CORES` replace readings (fixtures). Windows: the whole
autopilot toolchain is bash and unsupported there; recorded, not solved.
Rejected: a Rust or `sysinfo` reader (a build for a tooling check); a fixed threshold with no override.

## D7. Wake-up
Decision: `supervisor-watch.sh`, started once with `run_in_background`, loops `run-status.sh --json`
per recorded worktree every 30 s outside the supervisor's context, compares the state field with
the ledger, and exits on the first change or at 600 s. Rejected: the supervisor polling (context
cost, `waiting.md` "Never poll"); a cron (needs a harness feature we do not rely on).

## D8. Supervised marker
Decision: `create_session` has no environment argument (its schema: `ai_cli`, `allow_additional_session`,
`prompt`, `worktree`), so the marker travels in the prompt: `/speckit-autopilot #<n> supervised`.
This feature adds one line to `speckit-autopilot/SKILL.md`: a trailing `supervised` argument makes
the run write `- **Supervised**: yes` into its ledger on creation (`supervised.sh` already reads
it). A grep case in `supervisor.test.sh` asserts that line and the supervisor skill's prompt agree.
Rejected: an `env` argument on `create_session` (needs a micold MCP change outside this feature);
a marker pre-written in the worktree (the ledger does not exist before the autopilot runs);
`export AUTOPILOT_SUPERVISED=1` typed in the prompt (not a documented way to set a session's env).

## D9. Failure and tries
Decision: `tries` column in the ledger; each failed start adds one; at 3 the state is `failed`.
Rejected: unbounded retry (endless loop on a persistent error); per-issue backoff (state the person cannot read in the ledger).

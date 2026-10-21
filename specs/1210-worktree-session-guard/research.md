# Research: Worktree session guard

## R1 Enforcement channel
Decision: answer `PreToolUse` on the existing loopback hook receiver with `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":<msg>}}`, plus static `permissions.deny` for the enter and `git worktree add` rules, both in the per-session `--settings` file.
Rationale: the receiver already gets every `PreToolUse` with `tool_input`, `cwd` and the session id in the URL; no new process, no user config touched (FR-013), works for every project (FR-001).
Evidence: Claude Code processes a 2xx HTTP-hook response body as hook output (JSON `hookSpecificOutput` as for command hooks); the receiver's body-less `200 OK` is accepted today as no decision. A daemon test proves the receiver emits the deny body; quickstart B step 1 proves the agent sees it.

**T002 spike (Claude Code 2.1.306, `claude -p`, scratch Python HTTP hook server, `--settings` file with an `http` PreToolUse hook and the three `permissions.deny` entries):** (1) a `Write` answered with the contract's deny body (200, `application/json`) was blocked: the file was not created and the agent quoted the reason text verbatim (shown as `PreToolUse:Write hook error: <reason>`); an unchanged empty 200 for `Bash` let the call run. (2) `git -C . worktree add ../x` was denied ("Permission to use Bash ... has been denied") and no worktree was created. (3) A project `.claude/settings.json` deny rule (`Bash(echo PROJDENY*)`) still denied next to `--settings`, and its command `PreToolUse` hook still ran for every Bash call (FR-013, SC-007). Channel confirmed; no escalation. Side note: Claude Code warns when a deny rule mixes `*` with the `:*` suffix (`Bash(git * worktree add:*)` form); the contract's `Bash(git * worktree add*)` has no `:*`, so it is unaffected. The receiver needs a `respond` variant with a JSON body and a PreToolUse branch calling core `decide`; the hook timeout bounds the response time (SC-005).
Rejected: a command hook binary (extra process per call, packaging on 3 OSes); per-repo `.claude/settings.json` (per-repo, #944's flaw); sandbox filesystem rules (do not cover the Default session and give no `create_session` message).

## R2 Deny rule coverage of `git -C <dir> worktree add`
Decision: `permissions.deny` entries `EnterWorktree`, `Bash(git worktree add:*)`, `Bash(git * worktree add*)`; the PreToolUse shell analyser also denies any command whose tokenised git invocation has `worktree` followed by `add` after skipping global options (`-C <dir>`, `-c k=v`, `--git-dir=`).
Rationale: prefix rules miss option-interleaved forms; the tokenising check in core is testable.
Rejected: regex-only rules (miss quoting forms).

## R3 Shell analysis depth (FR-004, clarified best effort)
Decision: whitespace/quote-aware tokeniser; split on `&&`, `;`, `||`, newline; track `cd`/`pushd` effect on the running directory; judge each token that looks like a path (absolute, `./`, `../`, `~`-free) plus the final directory. No expansion, substitution, variables or nested scripts.
Rejected: full shell parser crate (new dependency, still incomplete); denying all Bash (breaks legitimate work).

## R4 Path containment (FR-014)
Decision: `paths.rs` takes a `PathFacts` trait (canonicalise, case-sensitivity) so tests inject Windows/macOS/Linux behaviour. Resolution: lexical `..` and `.` removal, then canonicalise the deepest existing ancestor (symlinks), fold case when the volume is case-insensitive, normalise drive letters and separators, then component-wise prefix test. Longest-matching worktree wins (worktrees live under `.claude/worktrees/` inside the project root, so the root contains them).
Rejected: string `starts_with` (separator and case bugs).

## R5 Allowlist (FR-007)
Decision: the shared build dir is `<project root>/target-shared` (the root is a worktree entry, so this is carved out before the root test). No code defines an "autopilot prefetch worktree" today (grep of `crates/`, `scripts/`, `.claude/skills` finds none), so the plan introduces a core constant: a worktree whose directory name starts with `autopilot-prefetch` is allowlisted. Confirmed in the tasks unit (T001): `grep -rniI prefetch scripts .claude docs CLAUDE.md` finds nothing, so no repo convention exists; the prefix is a new constant. If a prefetch worktree is named differently later, change the constant only.
Rejected: per-project allowlist setting (violates FR-001/FR-008 simplicity).

## R6 Worktree list source and freshness
Decision: the receiver resolves the project's current list from the daemon catalog (already reconciled via `worktree::parse_worktrees`/`reconcile`), not `git` per call. A removed worktree vanishes from the list (edge case satisfied).
Access: a `DaemonState` accessor over the catalog's reconciled worktrees; the receiver gets it through a handle passed with the per-session GuardState in `prepare_settings`.
Rejected: spawning `git worktree list` per call (latency, SC-005).

## R7 Session binding and opt-out freeze
Decision: at launch the daemon computes the session's guard context (own path, project root, Default flag) and whether the setting is on; off means no guard hook response and no deny rules written. The guard state is stored per session id in the receiver map next to its token, so toggling later affects only new sessions.
Rejected: reading the setting per call (violates the edge case); a shared flag in the receiver (leaks across sessions, Principle II).

## R8 Opt-out storage
Decision: `worktree_guard_enabled: bool` (default true) in `DaemonSettings`, mirroring `tool_server_enabled` (catalog.rs, messages.rs, protocol version bump, `docs/user-guide/settings.md`).
Rejected: env var (not discoverable); per-project setting (clarified out).

## R9 Providers
Decision: guard covers Claude Code (hooks + `--settings`). Copilot and other CLIs have no equivalent hook channel today (see `state.rs` note on `--settings`); documented as a limit, not a silent gap.
Rejected: wrapping other CLIs' shells (out of scope).

## R10 Fail-open (FR-009)
Decision: unknown own binding, empty/failed list, unparsable payload => allow and `tracing::warn!` with the reason. The static deny rules are unaffected.
Rejected: fail closed (blocks legitimate work when state is merely unknown; spec FR-009 chose open).

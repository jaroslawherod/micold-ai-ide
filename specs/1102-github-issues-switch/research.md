# Research: GitHub issues switch (#1102)

## R1. Default value and how a missing entry reads
- **Decision**: a `default_true` serde function on `Settings`/`SettingsFile`; hand `Default` sets `true`. Same device as `default_desktop_notifications` (`micold-core/src/settings.rs`).
- **Rejected**: storing a `github_issues_disabled` flag so `false` is the natural default. It inverts the checkbox and every gate, and the neighbouring switches read positively.

## R2. Where the value lives
- **Decision**: service-owned, on `DaemonSettings`/`SettingsSet` like `jira_enabled` and `pr_status_enabled`, so every window and daemon follows one value and the MCP path in the daemon can read it (FR-008, FR-010).
- **Rejected**: client-only storage (the daemon's MCP `create_worktree` could not see it); per-project storage (spec: out of scope).

## R3. Gating mechanism in the form
- **Decision**: a form flag that makes `github_repo()` return `None`. Every GitHub starter, the provider choice and `issue_source_ready` already derive from `github_repo()`, so one choke point covers FR-002/FR-003.
- **Rejected**: a new `GithubAvailability::Disabled` variant (the remote answer would be lost on turn-on, forcing a second `RemoteList`); a check in each starter (misses a future caller).

## R4. Discarding in-flight results
- **Decision**: bump `issue_request_seq` and `reset_issues()` on a change, as `jira_settings_changed` does; loads, searches and detail results already ignore a stale seq.
- **Rejected**: a separate cancellation channel (no request is cancellable today).

## R5. MCP `github_issue` while off
- **Decision**: refuse in `resolve_naming` before any read, with an `invalid_input` error naming the switch (clarification Q1, FR-010).
- **Rejected**: silently ignoring `github_issue` (would create a worktree the caller did not ask for); reading the issue anyway (breaks FR-002).

## R6. Protocol version
- **Decision**: one bump 55 → 56 in one edit (hash covers `messages.rs`); `DaemonSettings` is decoded strictly by the non-self-describing codec, so an older peer cannot decode it. `tests/schema_hash.rs` forces the bump.
- **Rejected**: no bump (handshake would refuse the hash mismatch anyway).

## R7. Placement
- **Decision**: GitHub section, beside the pull request status checkbox (clarification Q2); focus id `FieldId::SettingsGithubIssues`.

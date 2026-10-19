# Contract: client ↔ daemon messages for scoped git credentials (#1002)

Added to `micold-core/src/protocol/messages.rs`. `PROTOCOL_VERSION` is bumped and `SCHEMA_HASH`
changes. Only sandboxed daemons (container, Kubernetes) act on them; a host-process or remote SSH
daemon answers `GitCredentialGrant` with an `OperationError` (`Refused`, `req: u64::MAX`) and never sends targets.

## DaemonMsg::GitCredentialTargets

```text
GitCredentialTargets { targets: Vec<CredentialTarget>, git_too_old: Option<String> }
// CredentialTarget { project, push_url: Option<String> }
```

- Sent by a sandboxed daemon after a successful `Attach`, and again whenever a project is added,
  removed, or its `origin` push URL changes (checked on project add/remove and on worktree refresh).
- `push_url` is `git remote get-url --push origin` in the project root, verbatim; `None` when there
  is no `origin` or the project is not a repository.
- The whole list every time: the client diffs.
- `git_too_old` is the container's `git --version` when it is older than 2.31 (no
  `GIT_CONFIG_COUNT`, so the helper reset would not apply); the client then issues nothing and
  every target's status is `NoCredentialReason::GitTooOld`, and the daemon refuses every grant.

## ClientMsg::GitCredentialGrant

```text
GitCredentialGrant { repo: GithubRepo, token: SecretToken, expires_at: UnixTime }
```

- The daemon writes `/run/micold/git/<owner>/<repo>` atomically (`0600`), replacing any previous
  file (renewal). No reply on success; a write failure is reported with the existing
  `DaemonMsg::Error`-style failure naming the repository, never the token.
- The daemon never logs the token; `SecretToken`'s `Debug` is redacted, and the wire-level debug
  log (`LogSink`) prints the message with the token redacted.

## ClientMsg::GitCredentialRevoke

```text
GitCredentialRevoke { repo: GithubRepo }
```

- The daemon removes the file. Idempotent.

## Invariants

- The daemon never forwards a token to another daemon, a session's environment, or a file outside
  `/run/micold/git`.
- `GitCredentialGrant` for a repository none of the daemon's projects pushes to is refused
  (an `OperationError`, `Refused`), so a client bug cannot widen a daemon's reach.

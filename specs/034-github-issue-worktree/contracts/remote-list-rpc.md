# Contract: `RemoteList` RPC and remote selection

**Feature**: [spec.md](../spec.md) · Research [R5](../research.md#r5--finding-the-repository-a-new-read-only-remotelist-rpc)

## 1. Git trait (`micold_core::git`)

```rust
trait Git {
    // … existing …
    /// Raw `git config --local --get-regexp ^remote\..+\.url$` output: the repository's own
    /// remote URLs, un-rewritten. Local config only — MUST NOT contact a remote (Principle IV) and
    /// MUST NOT read global config, so the answer is the same on every daemon placement (FR-026).
    fn remote_list(&self, repo: &Path) -> io::Result<String>;
}
pub struct GitRemote { pub name: String, pub url: String }       // serde, for the wire
pub fn parse_remote_list(raw: &str) -> Vec<GitRemote>;
```

- `GitCli::remote_list` = `run_git(repo, &["config", "--local", "--get-regexp", r"^remote\..+\.url$"])`.
  `git config --get-regexp` exits **1** when nothing matches; `remote_list` maps that exit to
  `Ok(String::new())` (no remotes), not an error.
- `parse_remote_list`: each line is `remote.<name>.url <url>`; `<name>` is everything between the
  `remote.` prefix and the last `.url` (remote names may contain dots); the URL is the rest of the
  line after the first space. The first URL per name wins (a remote may list several). Config
  order is preserved — that is "git's listing order" for the spec's "first GitHub remote".
- `FakeGit::with_remote(repo, name, url)` records remotes in insertion order.

Tests: `crates/micold-core/tests/git_remotes.rs` — the line format, a dotted remote name, a
duplicate `url` for one remote, empty output → empty; a `GitCli` test against a temp repo with two
remotes, and one with none (exit 1 → `Ok`), and one proving a global `insteadOf` is **not** applied
(`GIT_CONFIG_GLOBAL` pointed at a temp file with a rewrite).

## 2. Wire messages (`micold_core::protocol::messages`, `protocol::version`)

```rust
ClientMsg::RemoteList { req: u64, project: PathBuf }
OperationResult::RemoteList { remotes: Vec<GitRemote> }
```

- `PROTOCOL_VERSION` **15 → 16**, with a doc line in `version.rs` naming this feature's change,
  and the pin in `crates/micold-core/tests/schema_hash.rs` updated in the same edit (the repository
  rule: one bump per feature's wire change).

Daemon handling (`crates/micold-daemon/src/server.rs`), a copy of `BranchList`'s shape:

1. `state.project_repo(&project)` not a repository → `reject_non_repo` (existing).
2. `spawn_blocking(GitCli::new().remote_list(&repo))` → `parse_remote_list`.
3. `Ok` → `OperationOk { req, result: RemoteList { remotes } }`;
   `Err` → `OperationError { kind: GitFailed, message: "could not list remotes", detail }`.

Read-only; mutates no state; broadcasts nothing.

Tests: `crates/micold-daemon/tests/remote_list.rs` — a repo with `origin` on GitHub answers both
remotes in order; a non-repo is rejected like `BranchList`; the protocol round-trip test in
`micold-core` covers the new variants' encoding.

## 3. Client side (`shell/daemon_sync.rs`)

- `PendingOp::RemoteList { project }`, described as "read the repository's remotes".
- Sent from the shell's `Msg::Opened` handling for the active project (the form opens with
  `github = Checking`).
- **Not connected.** When no daemon connection exists at that moment, the shell does not call
  `send_op` (whose "Not connected…" toast would land behind the modal) and dispatches
  `Msg::RemotesListed(Err("not connected to the session service"))` directly. A pending
  `RemoteList` whose connection drops resolves the same way: `on_disconnected` already drains
  `app.pending_ops`, and gains a `PendingOp::RemoteList` arm that dispatches that `Err`, so
  `GithubAvailability` never stays `Checking`.
- On `OperationOk(RemoteList)` for the still-active project → `Msg::RemotesListed(Ok(remotes))`;
  on `OperationError` → `Msg::RemotesListed(Err(message))`; a reply for a project that is no longer
  active is dropped (as `BranchList`'s is), and with no form open the reducer ignores it.

## 4. Selection (`micold_core::github`)

```rust
pub fn choose_remote(remotes: &[GitRemote]) -> RemoteChoice;
```

1. `origin`, if present **and** a GitHub remote → it.
2. Otherwise the first remote, in config order, that is a GitHub remote.
3. Otherwise `NoGithubRemote`.

The form's chip reason for `NoGithubRemote` is "This repository has no GitHub remote." (FR-002); a
lookup error reads "Couldn't read this repository's remotes: <detail>". Both keep the chip disabled.

Tests: `crates/micold-core/tests/github_remote.rs` — every accepted URL form of research R5,
including userinfo (`https://user@github.com/o/r`, `https://x-access-token:T@github.com/o/r` — the
token never appears in `GithubRepo` or its `Display`) and `ssh://git@ssh.github.com:443/o/r.git`;
`GITHUB.COM` host; `www.github.com`, `github.example.com`, GitLab, local paths and an alias
(`gh:o/r`) rejected; origin-on-GitLab + upstream-on-GitHub picks upstream; two GitHub remotes
without origin picks the first.

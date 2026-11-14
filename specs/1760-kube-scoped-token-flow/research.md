# Research: Scoped token reaching a pod daemon (#1760)

## R1. Where does the token value travel?
- **Decision**: a key `github-token-<n>` of the existing `<name>-startup` Secret, mounted read-only at `STARTUP_ENV_MOUNT`; the entry's `token_file` names that path. Via `PodStartup.files` into `StartRequest::secret_files` (already merged into the Secret by `startup_secret`).
- **Why**: same pattern as `env_file`, SSH/CA extras; replaced/deleted/labelled/removed by existing code (FR-010 for free); Secret never in pod spec env (FR-002).
- **Rejected**: a Secret per token (new names, lifecycle, RBAC, more drift); an env var (leaks into sessions, FR-005); the ConfigMap (not a secret; would sit in the rendered `daemon.toml` neighbourhood); a new wire message (#1530 retired them).

## R2. Opt-in shape (FR-008, decided by the user)
- **Decision**: `KubeSettings.github_tokens: bool`, `#[serde(default, skip_serializing_if = "std::ops::Not::not")]`; a tick row in the form below the allowed-credentials list; label states the exposure.
- **Why**: saved with the daemon's settings, old `daemons.json` loads off (clarification 4), no change to `KubeCredential` (whose methods, `names::secret` and lifecycle assume one Secret per credential).
- **Rejected**: a new `KubeCredential::GithubTokens` variant (touches `names.rs`, `lifecycle.rs`, `manifest.rs` matches for a credential with no Secret of its own); a global Settings switch (user chose per daemon).

## R3. What are "the projects of this daemon" (FR-009)?
- **Decision**: the repositories named by the `remote` of the daemon's saved `[[projects]]` (`DaemonSettings::from_entry(entry).projects`, parsed with `GithubRepo::from_remote_url`, lower-cased). They reach `kube_attempt` and `kube_reconnect` through the new `Dial::Kubernetes.projects: ProjectsDial` (the sorted repositories, part of the dial identity). A pod's runtime-added projects are pod paths with no remote the client can read (the `Kubernetes` arm of `actor_specs_answered` ignores `runtime_projects`; `for_pod` leaves `projects` empty), so they are not a source: tokens follow the saved projects and a change applies at the next start. This narrows FR-009 to "saved projects"; the spec's Assumptions gain that line (tasks: edit spec.md).
- A token is selected when any of its `repos` patterns (`Exact`/`Owner`, case-insensitive) matches a repository. A token with empty `repos` (every repository it reaches) is delivered when the daemon has at least one project: its reach is unknown client-side, so it may be unusable for the project; the opt-in help says so.
- **Rejected**: all tokens (violates FR-009); reading `origin` of a local checkout (a pod project has none on the client); asking the pod (it does not exist at start).

## R4. Reading token files
- **Decision**: `PodGithubTokens::read(selected)`: for each entry read the file named by `token_file` and send it **verbatim** (the daemon's `parse_token_file` accepts the stored shape, so no parsing client-side). New code (the PAT store only writes 0600/0700, it has no read check): reject, on Unix, a file with group/other bits, and an empty, non-UTF-8 or over-`POD_FILE_LIMIT` file; each error naming the token (edge cases; same stance as `pod_startup`, #1533). The start fails before any cluster call.
- **Rejected**: skipping a bad token silently (a pod with a token missing is the exact silent failure #1533 refused).

## R5. The `host` key for the test
- **Decision**: add `host: Option<String>` to `GithubTokenSetting` (rendered as `host = "..."` only when set). `from_store` leaves it `None`. The seam is a new `KubeEnv.tokens: PodTokenSource { pat_root: Option<PathBuf>, host: Option<String> }` (`KubeEnv` already carries `home` and `keys_root`): production sets `PatStore::on_device()` and no host; the test builds its own store under a temp dir and sets `host` to the fake forge address, so it never touches the developer's PAT store. `kube_attempt` reads the store and host from `env`. The shipped default stays `github.com` (#1542 FR-003/FR-004).
- **Rejected**: an environment override (forbidden by #1542); a stored per-token host (a new Settings feature, out of scope).

## R6. Fake forge reachable from a pod
- **Decision**: a std `TcpListener` thread on `0.0.0.0` (like `services::Registry`) speaking HTTP/1.1 on the docker gateway address `HOST_IP`: `GET /api/v3/...` answers the token check, and the git smart-HTTP endpoints (`/<owner>/<repo>.git/info/refs`, `git-upload-pack`, `git-receive-pack`) are served by spawning `git http-backend` (CGI) after checking `Authorization: Basic` (user `x-access-token`, password = the run token, constant-time compare); anything else gets 401 with `WWW-Authenticate`. Every request's credential is recorded for the assertions.
- **Why**: the pod needs real git over HTTP to prove push and pull (FR-004); `git http-backend` ships with git, which the suite already requires.
- **Rejected**: reusing `git daemon` (`git://` has no authentication); a canned-response mock (cannot prove a real push); an external forge (not offline, Principle IV).

## R7. Inspecting a pod session
- **Decision**: one path: open a terminal session in the pod through the existing `work` helper (`kube_real_support/mod.rs`) and run `env` and `ls -a ~` in it, since FR-005 is about a session's environment. No `pod_exec`; the plan's harness row means this `work`-based helper (`session_env_and_home`). Test-only.
- **Rejected**: `kubectl exec` (a different process tree, not a session's environment); reading the pod spec (cannot see the session env).

## R8. Red tests first
- **Decision**: each unit above has a failing test before code; the live test is written first against the harness, red until delivery lands; the revert check (US3) goes in the cycle log.
- **Rejected**: writing the live test last (it would never be seen failing).

## R9. Documentation defect found
`docs/user-guide/sandboxed-daemon.md` promises a pasted token for a container while no client code delivers one. Out of scope (ledger follow-up); the Kubernetes guide is corrected here.
- **Rejected**: fixing the container here (spec Out of scope).

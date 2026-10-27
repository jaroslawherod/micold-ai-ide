# Feature Specification: One configuration file for every daemon

**Feature Branch**: `docs/daemon-config-file-plan`

**Created**: 2026-10-27

**Status**: Draft (planning only)

**Input**: User description: "Unify how configuration is passed from the application to a daemon. It
should be passed the same way to any daemon, whether it runs on the host, in a container or in
Kubernetes. It should be a YAML, INI or JSON file, whatever fits the Rust ecosystem best. From the
file the daemon discovers the location of its SSH keys, its AI and GitHub tokens, its connected
repositories and so on. A GitHub PAT can be defined for several repositories without naming a
repository."

> **Planning only.** This feature produces a plan and GitHub issues, not code. `tasks.md` is the
> issue breakdown: every task group there becomes one GitHub issue, implemented later in its own
> worktree. No source file changes on this branch.

## Clarifications

### Session 2026-10-27

- Q: Who writes the daemon configuration file? → A: The application generates it from its
  Settings, one file per daemon, and delivers it. A user may hand-write one for a daemon the
  application does not manage.
- Q: Do secrets live inside the file or does the file point at them? → A: The file names secret
  *files*; it never holds a secret value. The file is safe to print, diff and log.
- Q: How does a PAT covering several repositories work, given the one-repository rule of #1002? → A:
  The one-repository refusal is dropped. A token is used for the repositories it reaches, which the
  daemon learns from GitHub; an optional repository filter narrows it. The most specific match wins.
- Q: Do unsandboxed daemons (host process, the user's own SSH host) get credentials from the file
  too? → A: Yes, every kind alike. A daemon given no credential falls back to its host's own git and
  `gh` credentials.
- Q: Which file format? → A: TOML (decided without asking: serde/`toml` is the Rust ecosystem's
  configuration format, it allows comments, `serde_yaml` is unmaintained and JSON has no comments).
- Q: Does a changed token need a daemon restart? → A: No (decided: keeps today's behaviour). The
  daemon notices a changed configuration or secret file and re-reads it.
- Q: May the file pin an AI CLI version other than the build's? → A: Yes, the version only; where
  the CLI is installed from stays the application's choice.
- Q: How does a replaced AI API key reach a running CLI? → A: Through the CLI's own key helper where
  one exists (Claude Code's `apiKeyHelper` reads the key file on each use); other CLIs pick the new
  key up at the next session start.
- Q: Two tokens match a repository equally (two unfiltered tokens, or the same filter)? → A: The
  first in the file wins; file order is the order in Settings. The application shows which token
  each repository uses. Nothing is refused for an overlap.
- Q: On the host process or the user's SSH host, a configured AI sign-in and one already on that
  machine: which wins? → A: The configured one. The machine's own sign-in file is never overwritten
  or removed: the CLI is pointed at the configured sign-in for the daemon's sessions instead.
  (Revised by the credential rule below: the configured sign-in is read-only and kept fresh by the
  application.)
- Q: Is a configuration checked before it is delivered? → A: Yes: the application runs the same
  check as `micold-daemon config check` and shows its errors in Settings.
- Q: Pod sign-ins? → A: They stay read-only, as today. (Revised by the credential rule below:
  read-only now holds for every kind, and the application re-delivers a fresh sign-in, so nothing is
  lost on restart.)
- Q: Start-script run policy name? → A: `once_per_container`. On a host or SSH daemon it means once
  per data directory.
- Q: Do the start script's secret file values reach sessions? → A: No. They are visible to the
  script only; a variable reaches the daemon and its sessions only when the script exports it.
- Rule (overrides the earlier answers on sign-ins): only the external party — the user, through the
  application on their own machine — refreshes or rotates a credential or token. The daemon and any
  agent or AI CLI under it never refresh, rotate or write one back. A new credential reaches a daemon
  only as an updated file (rewritten on the host, copied again over SSH, a changed bind-mounted
  file, an updated Kubernetes Secret); the daemon notices and reloads.
- Q: How does an AI CLI sign-in stay valid if the CLI may not refresh it? → A: The application keeps
  it fresh: preferably with a long-lived token (nothing to refresh), else by relaying the user's own
  sign-in without its refresh token and re-delivering it whenever the user's own sign-in changes.
- Q: The gap review (research D15) left five choices open; what did the user decide? → A (user, 2026-10-27):
  - Other forges: each token names its forge host (default GitHub); only GitHub and GitHub
    Enterprise are implemented in this feature.
  - SSH remotes in sandboxes: optional SSH key and known-hosts files.
  - Commit signing: an optional signing key file; without it, signing is switched off in the shared
    git configuration.
  - Custom CA and proxies: an optional CA file, used by sessions and the daemon; proxies through the
    session environment.
  - Package-registry files (`.npmrc` and similar): optional files copied into the session home.
  - "Use this machine's own sign-in" on host and SSH: an explicit choice per AI CLI, off by
    default; in that mode the read-only credential rule does not apply.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Every daemon starts from one file (Priority: P1)

A user runs daemons of several kinds: one on their computer, one in a Podman container, one in a
Kubernetes pod and one on a server over SSH. Each one starts from a configuration file the
application wrote for it, in the same format, and nothing else about its configuration comes from
environment variables or start-up arguments. When something goes wrong the user (or support) opens
that one file and sees everything the daemon was told.

**Why this priority**: The four start-up paths have drifted apart today. A container daemon started
over SSH gets a different environment from the same container started at boot, and a pod daemon
lacks what its git credential needs (the daemon in a pod never learns it is sandboxed, so it
refuses every GitHub token grant). #1438 patched the container-over-SSH git environment inside the
daemon; one file removes the drift at its root instead of patching each path.

**Independent Test**: Start one daemon of each kind; on each, the daemon's diagnostic reports the
configuration file it read and the values it took from it, and those match the file.

**Acceptance Scenarios**:

1. **Given** a container daemon, **When** it is started by the container at boot and again over SSH,
   **Then** both start with identical configuration.
2. **Given** a daemon of any kind, **When** its configuration file is invalid (wrong syntax,
   unknown key, a named file that does not exist), **Then** the daemon does not start and the
   application shows the file, the line and what is wrong.
3. **Given** a configuration file written for an older schema version, **When** a newer daemon
   reads it, **Then** it is accepted or refused with a message naming both versions; it is never
   misread.

---

### User Story 2 - One GitHub token for many repositories (Priority: P1)

A user has a fine-grained token that covers all repositories of their organisation. They paste it
once in Settings, without choosing a repository. Every daemon then pushes and pulls each of those
repositories with it, and `gh` inside sessions works with it too.

**Why this priority**: Today a token must be pasted per repository and is refused when it reaches
more than one private repository; `gh` in a session has no token at all.

**Independent Test**: Paste one token reaching two repositories; push from a session in each of the
two, and run `gh pr list` in each; all four succeed with no further input.

**Acceptance Scenarios**:

1. **Given** a token reaching `acme/a` and `acme/b`, **When** a session in a worktree of either
   pushes, **Then** git uses that token without asking.
2. **Given** two tokens, one limited by the user to `acme/a` and one for all of `acme/*`, **When** a
   session in `acme/a` pushes, **Then** the more specific token is used.
3. **Given** a session in a repository no token reaches, **When** it pushes, **Then** git falls back
   to the host's own credentials (host and SSH daemons) or fails at once with "no GitHub token
   reaches owner/repo" (sandboxes), never with an interactive prompt.
4. **Given** a session in a repository a token reaches, **When** `gh` runs in it, **Then** `gh` is
   authenticated with that token.
5. **Given** two tokens that both reach `acme/a` with no filter, **When** a session in `acme/a`
   pushes, **Then** the token listed first in Settings is used, and Settings shows that `acme/a`
   uses it.
6. **Given** a token the user replaces in Settings, **When** the next git or `gh` command runs in a
   running session, **Then** it uses the new token; no daemon or session restart is needed.

---

### User Story 3 - The daemon finds its keys, sign-ins and repositories itself (Priority: P2)

From its configuration file the daemon learns where its SSH host key and authorised keys are, where
each AI CLI's sign-in is, which projects it serves and where it clones repositories. The
application no longer sends this piecemeal after start-up.

**Why this priority**: Removes the per-kind special cases (mounts here, Secrets there, protocol
messages elsewhere) that this feature unifies; depends on Story 1.

**Independent Test**: Write a configuration by hand for a daemon on a fresh SSH host, naming one
project, one AI sign-in and one token; start the daemon; a session there starts signed in and can
push.

**Acceptance Scenarios**:

1. **Given** a configuration naming three projects, **When** the daemon starts, **Then** it serves
   exactly those three, cloning the ones that are absent when a clone root is configured.
2. **Given** a configuration naming the Claude Code sign-in file, **When** a Claude Code session
   starts, **Then** it is signed in, and the sign-in file is unchanged after the session.
2a. **Given** a delivered sign-in, **When** the application delivers a newer one, **Then** the next
   session uses the newer one, with no restart of the daemon.
3. **Given** a host or SSH daemon whose machine already has its own Claude Code sign-in and a
   configuration naming a different one, **When** a session starts, **Then** it uses the configured
   sign-in, and the machine's own sign-in file is unchanged afterwards.
4. **Given** a project the user adds in the application while a daemon runs, **When** it is added,
   **Then** the daemon serves it at once and the next generated configuration file includes it.

---

### User Story 4 - Each daemon's AI CLIs are configured in the same file (Priority: P2)

For each daemon the user chooses in Settings which AI CLIs it offers, the version of each, how a
new session of it is launched (extra arguments, model, API endpoint, environment), how it signs in
(a sign-in file or an API key file) and the CLI's own settings files (for example Claude Code's
`settings.json`, Codex's `config.toml`). The same choices work on a container, a pod, an SSH host
and the host process, and the daemon reads them from its configuration file.

**Why this priority**: Today the offered CLIs are chosen in three places (a container profile, a
Kubernetes form, the provisioning table), the version cannot be chosen at all, and a sandbox can
only share a sign-in file, never an API key or the CLI's own settings. Depends on Story 1.

**Independent Test**: Configure a container daemon with Claude Code at a chosen version, a model,
one extra launch argument, an API key file and a settings file; start a session; the CLI reports
that version and model, uses the key, and applies the settings.

**Acceptance Scenarios**:

1. **Given** a configuration offering Claude Code and Codex, **When** the user opens the new-session
   menu for that daemon, **Then** exactly those two are offered.
2. **Given** a configured version for a CLI, **When** the daemon is provisioned, **Then** that
   version is installed; with no version configured, the build's pinned version is installed.
3. **Given** a CLI with an API key file and no sign-in file, **When** a session of it starts,
   **Then** the CLI is authenticated with that key, and the key never appears in the configuration,
   a log line or the session's command line. **When** the key file is replaced, **Then** a running
   Claude Code session uses the new key on its next request; other CLIs use it from their next
   session.
4. **Given** a CLI with extra launch arguments, a model and environment variables, **When** a
   session of it starts, **Then** it is launched with them, after the arguments the application
   itself needs (session id, tool server, hooks) and never in place of them.
5. **Given** a settings file for a CLI on a container daemon, **When** a session starts, **Then**
   the CLI finds it where it reads its user settings; the per-session settings the application adds
   still apply.

---

### User Story 5 - A start script prepares each daemon (Priority: P2)

A user wants extra tools in their sandbox (a language toolchain, a linter), a few environment
variables in every session, and a secret loaded from a file. In Settings → Daemons they write or pick
a start script for the daemon. The application delivers it with the configuration, the daemon runs
it before it starts serving, and what the script exports reaches the daemon and its sessions. If the
script fails, the user sees its output in the application.

**Why this priority**: Today a sandbox can only be customised by building another image; the
environment-include setting sources a profile per session but cannot install anything once.
Depends on Story 1.

**Independent Test**: Give a container daemon a script that installs a tool under `~/.local` and
exports `FOO=bar`; restart it; a new session finds the tool and sees `FOO=bar`. Make the script
exit 1 with `on_failure = "refuse"`; the daemon does not start and the application shows the
script's output.

**Acceptance Scenarios**:

1. **Given** a start script that exports variables, **When** the daemon starts, **Then** every
   session it starts sees them, and so does the daemon itself.
2. **Given** a container daemon with a start script, **When** it is started at boot and again over
   SSH, **Then** the script runs the same way on both paths, before the daemon serves anything.
3. **Given** a script that fails or times out and `on_failure = "refuse"`, **When** the daemon
   starts, **Then** it does not start, and the application shows the exit status and the script's
   output; with `on_failure = "warn"` it starts and shows the same as a warning.
4. **Given** `run = "once_per_container"`, **When** the daemon restarts and the script is unchanged, **Then** the
   script does not run again; when the script or its secret file changed, it runs again.
5. **Given** a changed start script, **When** the application delivers it to a running daemon,
   **Then** it does not re-run; Settings says a restart is needed to apply it.

---

### User Story 6 - The user can read what a daemon was given (Priority: P3)

In Settings → Daemons the user can open the configuration file of any daemon, read only, with the
secret files it names listed by path and never by content.

**Why this priority**: Support and trust; the feature is complete without it.

**Independent Test**: Open the view for a container daemon; it shows the TOML; no token value
appears anywhere in it.

### Edge Cases

- The application delivers a new config or secret that is invalid (bad TOML, a failing rule, an
  unreadable secret): the daemon keeps the previous configuration in force and Settings shows why.
- A pod's config names a secret that the cluster has not mounted yet: the daemon keeps the previous
  configuration and retries; it reports an error only if the secret is still missing two minutes
  later.
- An agent in a container or pod session tries to rewrite a secret or the config: the write fails
  (read-only); what the daemon uses does not change.
- An agent in a host or SSH session rewrites a secret or the config: it can (same OS user); the
  daemon applies it, and Settings shows that the daemon's file differs from what the application
  delivered.
- An AI CLI's delivered access token expires before the application has delivered a newer one: new
  sessions of that CLI start signed out with a message naming the fix; running sessions may need a
  restart.
- The user removes a credential in Settings: the application removes the file; the credential is
  unavailable from the next use.
- The configuration sets a variable the daemon owns (`PATH`, `HOME`, `GH_TOKEN`, a CLI's config-dir
  variable, …): the file is refused, naming the variable.
- A sandbox session commits: it uses the user's name and email from the shared git configuration;
  if the user signs commits but no signing key is delivered, signing is off rather than failing.
- A daemon is started from inside a session (a development build): it refuses to load the default
  configuration location and needs an explicit `--config`.
- A placed tool configuration already defines a server named `micold`: the file is refused, because
  the application's tool server would otherwise be silently skipped.
- A host daemon whose user has "use this machine's own sign-in" off and no delivered sign-in: the
  CLI's sessions start signed out and Settings offers to deliver a sign-in.

- The daemon starts offline and cannot ask GitHub which repositories a token reaches: it uses the
  last answer it saved; with none saved, a token with a repository filter is used for the
  repositories that filter names, and a token without one is not used until GitHub answers.
- A secret file named by the configuration is missing or unreadable at start-up: the daemon starts,
  reports that credential as unavailable, and uses it as soon as the file appears.
- A Kubernetes Secret is updated: the pod sees the new file without a restart (directory mounts, not
  single-file mounts, which Kubernetes never updates).
- Two tokens match a repository equally (same filter, or both unfiltered): the first in the file
  (the Settings order) wins; Settings shows which token each repository uses and that the other is
  shadowed there.
- A host daemon is given a sign-in while the user is signed in to the same CLI on that machine: the
  daemon's sessions use the configured sign-in; the user's own CLI, run outside the application,
  keeps its own.
- The start script's secret env file defines `NPM_TOKEN` and the script does not export it: no
  session sees it.
- A token expires: it is no longer handed to git or `gh` from its expiry on, and the application
  shows the expiry as today.
- The user's own SSH host already has `gh` signed in and a git credential helper: with no token in
  the configuration they keep working as before; with a token configured for that repository, the
  token is used.
- A start script hangs: it is stopped at its timeout (whole process group) and treated as failed.
- A start script writes a large amount of output: the daemon log keeps all of it; the application
  shows the last 200 lines.
- A start script tries to reach host credentials from inside a sandbox: there are none to reach;
  it sees only the files the configuration names and its own secret env file.
- A start script `run = "once_per_container"` fails: no marker is written, so the next start runs it again.
- A start script and the environment-include setting both set a variable: the environment-include
  value wins for the session (it is resolved later and per directory).
- A daemon from an older build reads a configuration from a newer application: the schema version
  check refuses it with an "update the daemon" message.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every daemon kind (host process, SSH host, container, Kubernetes pod) MUST read its
  configuration from one file in one format, TOML.
- **FR-002**: The daemon MUST find the file without any environment variable: an explicit
  `--config` argument first, then one fixed default location per kind, the same location the
  container entrypoint and the SSH bridge both use.
- **FR-003**: The file MUST carry a schema version, and the daemon MUST refuse an unknown key or a
  newer major version with a message naming the key or the versions.
- **FR-004**: The file MUST NOT contain a secret value. Every secret (GitHub token, AI sign-in, the
  client authentication token, SSH host key) MUST be named by the path of a file holding it.
- **FR-005**: The application MUST generate each daemon's file from its Settings and deliver it,
  with the secret files it names, by the kind's own means (local file, copy over SSH, container
  mount, Kubernetes ConfigMap and Secrets).
- **FR-006**: The daemon MUST re-read the configuration and every named secret file when they
  change, without a restart, applying credential changes to the next git or `gh` command.
- **FR-007**: The configuration MUST be able to list any number of GitHub tokens, each with an
  optional repository filter (`owner/repo`, `owner/*`), and no repository is required.
- **FR-008**: The daemon MUST learn which repositories each token reaches from GitHub, keep the last
  answer, and choose for a repository the matching token with the most specific filter; among equally
  specific matches the first in the file (the Settings order) MUST win, and the application MUST
  show which token each repository uses.
- **FR-009**: The one-repository refusal of #1002 MUST be removed; the application MUST instead show
  which repositories each token reaches.
- **FR-010**: Git in every session MUST use the chosen token through the daemon's credential helper;
  in sandboxes inherited credential helpers MUST still be cleared (#1002 FR-004), on the host and
  SSH kinds the host's own helpers remain as the fallback.
- **FR-011**: Every session MUST get `GH_TOKEN` set to the token chosen for its repository, read at
  the moment `gh` runs so a replaced token takes effect without restarting the session.
- **FR-012**: The configuration MUST name the daemon's data directory, home, listen endpoint, SSH
  key directory, client token file, idle-stop policy, time zone, image reference and log level;
  each environment variable that carries one of these today MUST be retired.
- **FR-013**: The configuration MUST list, per AI CLI, whether the daemon offers it, its version
  (absent = the build's pinned version), extra launch arguments, a default model, an API base URL,
  environment variables, a sign-in file or long-lived token file, an API key file, and the CLI's
  own settings files to place. The offered list replaces today's per-kind selections.
- **FR-013a**: The configuration MUST NOT hold an API key, sign-in or other secret value for an AI
  CLI; each is named by file (FR-004). A CLI MUST NOT be given both a sign-in file and an API key
  file unless the CLI documents which of the two wins. A replaced API key file MUST reach a running
  session through the CLI's key helper where it has one (Claude Code `apiKeyHelper`), else the next
  session.
- **FR-013e**: On every kind a configured sign-in MUST win over a sign-in already on that machine,
  and the daemon MUST NOT overwrite, move or remove the machine's own sign-in file: it points the
  CLI's sessions at the configured sign-in instead. With no sign-in configured on the host or SSH
  kind, the CLI uses the machine's own sign-in as the user's own CLI would, and Settings says so.
- **FR-013g**: The application MUST keep a delivered AI CLI sign-in valid without the CLI refreshing
  it: by a long-lived token where the CLI supports one, or by re-delivering the user's own sign-in,
  without its refresh token, whenever that sign-in changes and before the delivered one expires.
- **FR-013f**: The application MUST run the same validation as `micold-daemon config check` before
  it delivers a configuration, and show any error in Settings without delivering.
- **FR-013b**: Configured launch arguments, model and environment MUST add to what the application
  passes a session (session id, tool server binding, hook settings), never replace or remove it.
- **FR-013c**: The CLI's own settings files MUST be placed only in a sandbox's home (container,
  pod); on the host process and the user's SSH host the CLI's existing user configuration is never
  overwritten (feature 034 FR-003).
- **FR-013d**: Settings the user changes live from the UI and the daemon mirrors to every window
  (the default AI CLI for new sessions, the tool server switch, Pi's activity component) MUST stay in
  `settings.json`; the configuration file holds what is fixed when the daemon is deployed.
- **FR-014**: The configuration MUST list the daemon's projects (path, and the remote to clone from)
  and its clone root; projects added at runtime keep working through the existing protocol.
- **FR-015**: The configuration MUST be able to add plain environment variables for sessions, and
  secret ones named by file.
- **FR-016**: The `GitCredentialGrant`/`GitCredentialRevoke` messages MUST be retired once every
  kind reads tokens from the configuration.
- **FR-017**: `micold-daemon config check <file>` MUST validate a file and print what it resolved,
  never a secret's content.
- **FR-018**: The user guide MUST document the file, every key, and the per-kind locations in the
  same change that ships each part.
- **FR-019**: The configuration MAY name a start script by file, with its shell, a timeout, a
  failure policy (`refuse` or `warn`), a run policy (`every_start`, or `once_per_container`: once until the
  script or its secret file changes — once per container, and on a host or SSH daemon once per data
  directory) and a secret `KEY=VALUE` file only the script sees. No script content and no
  secret value is inline.
- **FR-020**: The daemon MUST run the start script before it serves any client, the same way on
  every start path of a kind (container boot and SSH bridge alike), as the daemon's own user —
  never root in a sandbox.
- **FR-021**: When `export_env` is on, the variables the script exports MUST reach the daemon and
  every session it starts; a value from the secret env file MUST reach them only if the script
  explicitly exports it; the environment-include setting is still applied per session on top.
- **FR-022**: The script's output MUST go to the daemon log; on failure or timeout the application
  MUST show the exit status and the end of the output, and with `refuse` the daemon MUST NOT start.
- **FR-023**: A changed start script MUST NOT re-run in a running daemon; the application MUST say
  that a restart is needed.
- **FR-024**: Settings → Daemons MUST let the user write or pick the start script per daemon.
- **FR-025**: The daemon, its sessions and the AI CLIs MUST NOT refresh, rotate or write any
  credential or token. Only the application does, on the user's machine.
- **FR-026**: A refreshed or rotated credential MUST reach the daemon only by updating files: the file
  on the host, a new copy over SSH, the host side of a bind-mounted directory, or an updated
  Kubernetes Secret.
- **FR-027**: The daemon MUST notice a changed configuration or secret file on every kind — including
  bind mounts and network filesystems that report no change events, and a Kubernetes Secret update —
  and use it within 5 seconds of the file changing on its filesystem.
- **FR-028**: A reload MUST be atomic: the new configuration is parsed and validated in full first;
  if it is invalid the previous one stays in force and the daemon reports the error to the
  application.
- **FR-029**: In container and pod daemons the configuration and every secret file MUST NOT be
  writable by the session's user, so an agent in a session cannot change what the daemon uses. On the
  host and SSH kinds sessions run as the same OS user and this cannot be enforced; the application
  MUST show when a daemon's file differs from what it delivered.
- **FR-030**: The configuration MUST deliver the user's git identity as a sanitized, read-only git
  configuration file, so commits in every kind carry the user's name and email.
- **FR-031**: Sessions in a container or pod MUST get the git credential helper, the HTTPS rewrite
  and the identity from the configuration's kind, never from a start-up variable, on every start path.
- **FR-032**: The configuration MUST NOT set an environment variable the daemon owns; the order in
  which configured variables apply MUST be stated and fixed.
- **FR-033**: When a Claude Code sign-in is delivered on host or SSH, the CLI's per-daemon state
  (onboarding, folder trust, user tool servers) MUST start from a copy of the user's own, made once.
  In a container or pod with any Claude Code sign-in or key, first-run setup MUST be marked done.
- **FR-034**: CLI settings files MUST be copied into place, not mounted read-only, so the CLI can
  record folder trust; a changed source replaces the copy at the next session start.
- **FR-035**: Each token MUST name its forge host (default GitHub); only GitHub and
  GitHub Enterprise are implemented.
- **FR-036**: The configuration MAY name an SSH key and known-hosts file for SSH
  remotes, and a commit-signing key; without a signing key, signing MUST be switched off in the shared
  git configuration.
- **FR-037**: The configuration MAY name a CA file used by sessions and by the daemon's
  own HTTPS; proxy variables in the session environment MUST be honoured by the daemon too.
- **FR-038**: The configuration MAY list files copied into the session home (such as
  `.npmrc`); a target outside the home or inside a CLI's config dir MUST be refused.
- **FR-039**: Container and pod sessions MUST get a UTF-8 locale by default and the configured time
  zone (pods included).
- **FR-040**: The application MUST show when each delivered AI sign-in expires.
- **FR-041**: A daemon started from inside a session MUST require an explicit configuration path.
- **FR-042**: A placed tool configuration that defines a server named `micold` MUST be refused.
- **FR-043**: On host and SSH, the user MAY choose per AI CLI to use the machine's own
  sign-in (off by default); in that mode the CLI manages that sign-in and FR-025 does not apply to it.

### Key Entities

- **Daemon configuration**: one TOML file per daemon; schema version, daemon section, listen, auth,
  SSH, storage, projects, GitHub tokens, AI CLIs, session environment.
- **AI CLI entry**: one per CLI the daemon knows: offered or not, version, launch arguments, model,
  API base URL, environment, sign-in file, API key file, settings files.
- **Secret file**: a file the configuration names; holds one token or sign-in; owner-only.
- **GitHub token entry**: a token file, an optional repository filter, and the reach GitHub reported.
- **Start script**: a user-written file, its shell, timeout, failure and run policies, an optional
  secret env file, and whether its exports reach sessions.
- **Project entry**: a path on the daemon and optionally the remote it is cloned from.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For all four daemon kinds, 100% of daemon configuration comes from the file; no
  `MICOLD_*` start-up variable remains outside test overrides.
- **SC-002**: A user with access to N repositories in one organisation pastes 1 token, not N.
- **SC-003**: `gh` works in a session of every daemon kind with no step beyond pasting the token.
- **SC-004**: A replaced token or configuration is in use within 5 seconds of the file changing on
  the daemon's filesystem (next git or `gh` command, next session), with no restart. For a pod, the
  cluster's own delay in updating a Secret volume comes before that and is shown in Settings.
- **SC-005**: No secret value appears in any configuration file, log line or diagnostic view.
- **SC-008**: After a session of any kind, every delivered credential file is byte-identical to what
  the application delivered.
- **SC-006**: A user configures which AI CLIs a daemon offers, and their version, sign-in and
  settings, in one place in Settings for all four kinds, not three.
- **SC-007**: A start script's failure is visible in the application with its output in 100% of
  failed starts, and a sandbox can be given an extra tool without building another image.

## Assumptions

- The application and its provisioned daemons run the same build, so the protocol messages this
  feature retires need no compatibility period (provisioning already pins the build id).
- `settings.json` keeps the settings edited live from the UI (scrollback, default CLI, prompt
  template, …); the configuration file covers how a daemon is deployed. Merging them is out of
  scope.
- Fine-grained and classic PATs are both accepted; the reach query works for both.
- The pod daemon's refusal of GitHub token grants is a bug of today's code, fixed on its own before
  this feature (it is the first issue of the split), so pod users are not left waiting for the epic.
- The SSH keys used to reach a daemon stay generated by the application; only where the daemon finds
  them moves into the file.

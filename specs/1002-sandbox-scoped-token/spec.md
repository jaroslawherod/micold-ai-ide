# Feature Specification: Scoped, short-lived git credential for sandboxed daemons

**Feature Branch**: `claude/project-thread-2cdrmk`

**Created**: 2026-10-19

**Status**: Draft

**Input**: User description: "Issue #1002 — Sandbox container with daemon should use a fine-grained
token, not the user's GitHub token and SSH key. Starting a sandbox container with a daemon gives it
the user's own GitHub token and SSH key so it can push and pull. Those credentials carry the user's
full access, so anything running in the sandbox (agent, build scripts, dependencies) can reach every
repo and org the user can. Expected: a fine-grained, scoped credential limited to the repository the
worktree/session belongs to; minimal permissions (contents read/write, plus pull requests / issues
only if the workflow needs them); short-lived where possible, with expiry and rotation; the user's
SSH private key and personal token never mounted or copied into the container; git inside the
sandbox authenticates over HTTPS with that token through a credential helper. If no scoped token can
be obtained, the sandbox starts without git credentials and says so, rather than falling back to the
user's credentials."

## Context

Two daemon runtimes run a session daemon inside an isolated container that the user does not own as
a whole machine:

- the **container ("sandbox") daemon** (features 027, 805, 806), which today offers per-daemon
  credential opt-ins: *Git configuration*, *SSH agent*, *Git credentials* (the host's credential
  store) and *AI CLI sign-in*;
- the **Kubernetes daemon** (feature 686), which today lets the user allow *the git config*, *the
  git credentials*, *one SSH private key*, AI CLI sign-ins and registry pull credentials.

The *SSH agent*, *Git credentials* and *SSH key* items hand the container the user's own git
identity: every repository and organisation the user can reach. This feature replaces them, for
these two runtimes, with a credential the application obtains per repository, limited to that
repository and to push/pull, and expiring on its own.

The **remote SSH runtime** (feature 489) runs the daemon on the user's own machine under the user's
own account; it is not a sandbox and is out of scope. *Git configuration* (commit identity, no
secret), *AI CLI sign-in* and *registry pull* credentials are not git push/pull credentials and are
unchanged.

## Clarifications

### Session 2026-10-19

- Q: Should non-GitHub remotes (GitLab, Bitbucket, self-hosted) keep the old *SSH agent* / *Git
  credentials* opt-ins until they get a scoped credential of their own? → A: No. They get no git
  credentials in a sandbox, and the status says the host is not supported (FR-012, US3 scenario 3).
  _(agent-resolved: issue #1002 body, "If no scoped token can be obtained, the sandbox starts without
  git credentials and says so, rather than falling back to the user's credentials")_
- Q: May scoped credentials be minted by a service the project hosts (a published GitHub App whose
  private key lives on a server)? → A: No; whatever the source, the client obtains credentials on
  the user's device, with no project-run server in the path. _(agent-resolved:
  .specify/memory/constitution.md#IV. Local-First Storage, #Technology, Storage & Licensing
  Constraints "no separate server process")_
- Q: Where does the scoped credential come from (FR-005)? → A: A GitHub App the user registers
  and installs; the client holds its ID and private key on the device and mints 1-hour,
  single-repository installation tokens. _(provisional, pending the user's answer on the decision
  card; agent recommendation)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A sandboxed daemon never holds my personal git credentials (Priority: P1)

A developer starts a container or Kubernetes daemon. Nothing the user owns that grants git access —
the SSH private keys, the SSH agent socket, the host's git credential store, a personal GitHub token
in the environment or in a CLI's config — is mounted, copied, forwarded or put in the environment of
the container, whatever the daemon's saved settings say.

**Why this priority**: This is the security guarantee the issue asks for. It holds even before any
scoped credential exists: a sandbox that cannot push is safe; a sandbox holding the user's key is not.

**Independent Test**: With a host that has an SSH key, a running SSH agent, a populated git credential
store and a personal GitHub token in its environment and CLI config, start a daemon of each runtime
whose saved settings still enable every old credential item. Inspect the container's environment,
mounts, secrets and files: none of those items, and none of their contents, is present.

**Acceptance Scenarios**:

1. **Given** a container daemon whose saved settings enable *SSH agent* and *Git credentials*,
   **When** it starts, **Then** no agent socket and no credential store is mounted, and the host's
   personal token is in none of its environment variables.
2. **Given** a Kubernetes daemon whose saved settings allow an SSH key and the git credentials,
   **When** it starts, **Then** no secret holding the key or the credential store is created or
   mounted in the pod.
3. **Given** either daemon with such settings, **When** the user opens its settings, **Then** the
   old items are no longer offered, and a one-time notice says they were removed and why, naming the
   scoped credential that replaces them.
4. **Given** the user's personal token value and private key contents, **When** the container's
   environment, mounted paths and home directory are searched for them, **Then** neither is found.

---

### User Story 2 - Push and pull work with a credential scoped to the one repository (Priority: P1)

A developer works in a session on a sandboxed daemon whose project's remote is a GitHub repository.
The agent commits and pushes; it pulls. Both succeed, authenticated over HTTPS with a credential that
can reach only that repository, can only read and write its contents (plus the repository metadata
read that every such credential carries), and expires on its own.

**Why this priority**: Without it, story 1 leaves sandboxes unable to push — the reason users enabled
the old items in the first place.

**Independent Test**: Start a sandboxed daemon serving a project whose remote is a GitHub repository,
with a scoped credential obtainable. From inside the container push a branch and pull: both succeed.
With the same credential, try to read a second repository the user can access and to change a
setting the credential was not granted: both are refused by the host.

**Acceptance Scenarios**:

1. **Given** a sandboxed daemon serving a project whose `origin` is a GitHub HTTPS or SSH URL,
   **When** a session in it runs `git push` and `git pull` on that remote, **Then** both succeed
   over HTTPS, without any SSH key.
2. **Given** the credential the container received, **When** it is used against any other
   repository, **Then** the host refuses it.
3. **Given** the credential, **When** its granted permissions are listed, **Then** they are
   repository contents read/write and metadata read, and nothing else unless the user turned on an
   extra permission for that daemon (FR-011).
4. **Given** a daemon serving two projects with different GitHub repositories, **When** a session in
   each pushes, **Then** each push uses a credential limited to its own repository, and neither
   credential reaches the other repository.
5. **Given** a daemon that keeps running past the credential's expiry, **When** a session pushes
   after that point, **Then** the push succeeds with a renewed credential, with no user action.

---

### User Story 3 - No scoped credential: start without git credentials, and say so (Priority: P1)

When the application cannot obtain a scoped credential for a project — the user has not set up the
credential source, the source refused, the network is down, or the remote is not on GitHub — the
daemon still starts, with no git credentials for that project, and the user is told which project
has none and why. It never falls back to the user's own credentials.

**Why this priority**: The fail-closed rule is what keeps story 1's guarantee true on a bad day.

**Independent Test**: Make the scoped credential unobtainable in each way listed, start the daemon:
it starts; a push from inside fails for want of credentials; the daemon's status names the project
and the reason; story 1's inspection still finds none of the user's items.

**Acceptance Scenarios**:

1. **Given** no credential source is set up, **When** a sandboxed daemon starts, **Then** it starts,
   and its status says that its projects have no git credentials and what to set up to get them.
2. **Given** the credential source refuses or cannot be reached, **When** the daemon starts, **Then**
   it starts, and the status names the project and the source's reason.
3. **Given** a project whose remote is not a GitHub repository, **When** the daemon starts, **Then**
   that project gets no git credentials and the status says the remote's host is not supported for
   scoped credentials.
4. **Given** renewal fails while the daemon runs, **When** the current credential expires, **Then**
   pushes fail for want of credentials, the status says renewal failed and why, and the next
   successful renewal restores pushing without a restart.

---

### Edge Cases

- **Credential expires mid-push**: renewal happens before expiry with a safety margin (FR-008), so a
  push started with a valid credential is not cut off by expiry.
- **Project added to a running daemon**: it gets its own scoped credential on the next start, or as
  soon as it is registered if the daemon supports adding projects live; until then it has none and
  the status says so.
- **Project's remote changes to another repository**: the old credential is not reused for the new
  repository; a credential is obtained for the new one, or none.
- **Remote with several URLs, or a fork with `upstream`**: only the repository the worktree pushes to
  (its `origin` push URL) gets a credential. Pulling from a public `upstream` needs none; pulling from
  a private `upstream` fails for want of credentials and is reported as such.
- **SSH-form remote URL** (`git@github.com:o/r.git`): git inside the container rewrites it to HTTPS
  for that host so the scoped credential is used and no SSH key is asked for; the project's own
  configuration on the host is not modified.
- **Old saved settings** with *SSH agent*, *Git credentials* or an SSH key: loaded without error,
  ignored, and dropped on next save; the notice of story 1 is shown once.
- **The user's personal token is in the host environment** (`GH_TOKEN`, `GITHUB_TOKEN`) at client
  start: it is never passed through to the container, even when other environment is.
- **Client on macOS, Windows or Linux**: host paths, the agent socket and the system credential
  store differ by platform (Principle VI); the removal guarantee of FR-001 and the scoped credential
  behave the same on each, and a host without `SSH_AUTH_SOCK` or a credential store changes nothing.
- **Worktree with no remote, a local-path remote or an unborn repository**: no credential is
  obtained, the daemon starts, and the status says the project has no GitHub remote.
- **Two daemons, or two sessions of one daemon, on the same repository**: each daemon gets its own
  credential; stopping or revoking one daemon's credential leaves the other's working. Sessions of
  one daemon share that daemon's credential for the repository.
- **Renewal racing a push**: a push in progress keeps using the credential it started with, which
  is still valid (FR-008's margin); the next git operation uses the renewed one.
- **Daemon stopped or deleted**: the credentials it held are revoked (FR-010); one whose revocation
  fails is left to expire within the hour.

## Requirements *(mandatory)*

### Functional Requirements

**Removing the user's own credentials**

- **FR-001**: For container and Kubernetes daemons, the system MUST NOT mount, copy, forward or put
  in the environment any of: the user's SSH private keys, the user's SSH agent socket, the host's git
  credential store, or a personal access token of the user (from the environment, a CLI's config or
  the credential store). This holds whatever the daemon's saved settings say.
- **FR-002**: The *SSH agent* and *Git credentials* opt-ins of the container daemon and the *git
  credentials* and *SSH key* items of the Kubernetes daemon MUST be removed from the daemon settings.
  Saved settings that still name them MUST load without error, MUST have no effect, and MUST drop
  them on the next save.
- **FR-003**: When a daemon's saved settings named a removed item, the settings view MUST show, once
  per daemon, a notice that it was removed, why, and what replaces it.
- **FR-004**: *Git configuration* (commit identity), *AI CLI sign-in* and registry pull credentials
  MUST keep their current behaviour, except that nothing in a shared host git configuration may
  reach a credential: with a host `~/.gitconfig` that sets credential helpers (`store`,
  `osxkeychain`, `manager`, `cache` or any other) or embeds a user name or token in a URL (for
  example in a `url.*.insteadOf` rule), git inside the container MUST consult only the scoped
  credential of FR-007 and MUST NOT run a host helper, read the host's store, or send an embedded
  host credential. Git helpers accumulate rather than replace one another, so the host's helper
  list MUST be reset inside the container, not merely followed by another entry.

**The scoped credential**

- **FR-005**: For each project a sandboxed daemon serves whose push remote is a GitHub repository,
  the system MUST obtain a credential that (a) can reach only that one repository, (b) is granted
  repository contents read/write and metadata read and nothing else by default, and (c) expires on
  its own. The source is a **GitHub App the user registers once and installs** on the accounts or
  organisations (and repositories) they choose. The client holds the App's ID and private key on
  the user's device and, per project, mints an installation token restricted to that one repository
  and to those permissions. The App's private key never enters a container. *(Provisional: decided
  by the agent pending the user's answer.)*
- **FR-006**: The credential MUST be readable only inside the container it was issued for. It MUST
  NOT be written into the project's files, the project's git configuration on the host, or any path
  the container shares with the host.
- **FR-007**: Git inside the container MUST push and pull the project's GitHub repository over HTTPS
  using only that project's scoped credential, with no SSH key, including when the project's remote is
  written in SSH form; the project's configuration on the host MUST NOT be modified to achieve this.
  Git MUST NOT offer that credential to any other repository or host.
- **FR-008**: While a daemon runs, the system MUST renew each credential before it expires, with a
  margin of at least 5 minutes, without restarting the daemon or interrupting sessions.
- **FR-009**: Every credential MUST expire no more than 1 hour after issue (the installation
  token's fixed lifetime), and none MAY be issued without an expiry.
- **FR-010**: When a daemon stops or is deleted, the system MUST revoke the installation tokens it
  obtained (best effort: a token whose revocation fails, for example offline, is left to expire
  within FR-009's hour), and MUST discard its copy of them in every case.
- **FR-011**: Per daemon, the user MUST be able to turn on, separately, *pull requests (read/write)*
  and *issues (read/write)* for the scoped credential; both MUST be off by default, and each MUST say
  at the point of choosing what it allows.

**Failing closed**

- **FR-012**: When no scoped credential can be obtained for a project — no source set up, App not
  installed on the repository, source refused or unreachable, remote not on GitHub, or renewal failed — the daemon MUST still start (or
  keep running) with no git credentials for that project, and MUST NOT use any of the items of
  FR-001 instead.
- **FR-013**: The daemon's status MUST say, per project without a credential, that it has none and
  the reason, in words that name what the user can do (set up the source, fix the remote, retry).
  The status MUST clear when a credential is obtained.

**Proof**

- **FR-014**: An automated test MUST start, or build the full start-up of, a daemon of each runtime
  on a host that has an SSH key, an agent socket, a populated credential store and a personal token
  in the environment, with saved settings enabling every removed item, and MUST assert that none of
  them — path or content — is in the container's environment, mounts, secrets or files.
- **FR-015**: An automated test MUST show that the credential the container receives is limited to
  the project's repository and to FR-005's permissions, and that git push and pull from inside the
  container use it over HTTPS.

### Key Entities

- **Scoped repository credential**: a secret value bound to one daemon and one repository
  (`owner/name` on github.com), with its granted permissions and its expiry time. Held by the client
  and the daemon's own state; never by the project or the host's git configuration.
- **Credential source**: the user's GitHub App (FR-005): its App ID and private key, held on the
  user's device; per user, set up once in the application's settings.
- **Credential status (per daemon project)**: *has credential (expires at …)*, or *none* with a reason
  from: source not set up, App not installed on the repository, source refused, source unreachable,
  remote not on GitHub, renewal failed.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of sandboxed daemon starts, a search of the container's environment, mounts,
  secrets and files finds zero occurrences of the user's SSH keys, agent socket, credential store or
  personal token.
- **SC-002**: A session in a sandboxed daemon can push and pull its project's GitHub repository with
  no step by the user beyond the one-time credential source setup (registering the GitHub App,
  installing it on the repository's account or organisation, and giving the client its ID and key).
- **SC-003**: The credential found inside a sandbox is refused by GitHub for every repository other
  than the project's own, and for every permission other than those granted.
- **SC-004**: Every credential issued has an expiry no later than 1 hour after issue, and a push
  after two consecutive renewals
  succeeds without user action.
- **SC-005**: When no credential can be obtained, the daemon starts in 100% of cases, and the user can
  tell from its status, without logs, which project lacks credentials and why.

## Assumptions

- **Runtimes in scope**: container and Kubernetes daemons. The remote SSH runtime runs on the user's
  own machine under their own account and is out of scope. *(agent-resolved, least privilege where
  the user's credentials leave the user's control)*
- **Hosts in scope**: GitHub (github.com) only. Other hosts (GitLab, Bitbucket, self-hosted) get no
  git credentials in a sandbox after this feature; the old opt-ins are not kept for them, since
  keeping them would keep the very exposure the issue removes. *(agent-resolved, least privilege;
  recorded in the ledger as a decision the clarify unit may revisit)*
- **Default permissions**: contents read/write and metadata read only; pull requests and issues are
  per-daemon opt-ins, off by default (FR-011). *(agent-resolved, least privilege)*
- **Scope unit**: one credential per repository, not one per daemon covering all its projects, so a
  daemon serving two repositories cannot use one project's credential on the other. *(agent-resolved,
  least privilege; the issue says "limited to the repository the worktree/session belongs to")*
- **Which remote**: the repository the worktree pushes to (`origin`'s push URL). *(agent-resolved)*
- **Pull of public repositories** needs no credential and keeps working without one.
- The client, not the container, talks to the credential source; the container never holds anything
  that can mint new credentials.

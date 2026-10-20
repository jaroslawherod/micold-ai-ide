# Feature Specification: Scoped, expiring git credential for sandboxed daemons

**Feature Branch**: `claude/project-thread-2cdrmk`

**Created**: 2026-10-19

**Status**: Closed 2026-10-20 — shipped in PRs #1090, #1096, #1099, #1107, #1113, #1132, #1139; the live check with a real PAT is #1147

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
these two runtimes, with a credential per repository, limited to that repository and to push/pull,
and expiring on its own. The credential is a fine-grained personal access token that the user
creates for that one repository and pastes into the application; its lifetime is the user's choice
(at least 1 day, GitHub's minimum), not the 1 hour a minted token would have.

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
- Q: Where does the scoped credential come from (FR-005)? → A: A **fine-grained personal access
  token (PAT), one per repository**, that the user creates on GitHub by hand and pastes into the
  application's settings. GitHub's shortest expiry for one is 1 day, and there is no automatic
  renewal: the user replaces it, and the application warns before it expires. The GitHub App
  alternative (client-minted 1-hour installation tokens) is rejected. _(answered by jaro,
  2026-10-19, decision D7, option B "Pasted PAT")_

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
read that every such credential carries), and expires on its own, on the date the user chose when
creating it.

**Why this priority**: Without it, story 1 leaves sandboxes unable to push — the reason users enabled
the old items in the first place.

**Independent Test**: Start a sandboxed daemon serving a project whose remote is a GitHub repository,
with a fine-grained PAT for it pasted into the settings. From inside the container push a branch and pull: both succeed.
With the same credential, try to read a second repository the user can access and to change a
setting the credential was not granted: both are refused by the host.

**Acceptance Scenarios**:

1. **Given** a sandboxed daemon serving a project whose `origin` is a GitHub HTTPS or SSH URL,
   **When** a session in it runs `git push` and `git pull` on that remote, **Then** both succeed
   over HTTPS, without any SSH key.
2. **Given** the credential the container received, **When** it is used against any other
   repository, **Then** the host refuses it.
3. **Given** the settings section where the user pastes the PAT, **When** it is shown, **Then** it
   says which permissions to give the PAT (repository contents read/write and metadata read) and
   that any further permission the user gives it (pull requests, issues, …) is then held by
   the sandbox too (FR-011).
4. **Given** a daemon serving two projects with different GitHub repositories, **When** a session in
   each pushes, **Then** each push uses a credential limited to its own repository, and neither
   credential reaches the other repository.
5. **Given** a running daemon whose PAT has expired or is about to, **When** the user pastes a new
   PAT for that repository, **Then** the next push succeeds without restarting the daemon or
   ending its sessions.

---

### User Story 3 - No scoped credential: start without git credentials, and say so (Priority: P1)

When the application has no usable scoped credential for a project — the user has pasted no PAT
for its repository, the PAT has expired or GitHub refuses it, or the remote is not on GitHub — the
daemon still starts, with no git credentials for that project, and the user is told which project
has none and why. It never falls back to the user's own credentials.

**Why this priority**: The fail-closed rule is what keeps story 1's guarantee true on a bad day.

**Independent Test**: Make the scoped credential unusable in each way listed, start the daemon:
it starts; a push from inside fails for want of credentials; the daemon's status names the project
and the reason; story 1's inspection still finds none of the user's items.

**Acceptance Scenarios**:

1. **Given** no PAT is stored for a project's repository, **When** a sandboxed daemon starts,
   **Then** it starts, and its status says that the project has no git credentials and that the
   user can paste a PAT for that repository in the settings.
2. **Given** a stored PAT that GitHub refuses (revoked, expired, or not reaching the repository) or
   that cannot be checked because GitHub is unreachable, **When** the daemon starts, **Then** it
   starts, and the status names the project and the reason.
3. **Given** a project whose remote is not a GitHub repository, **When** the daemon starts, **Then**
   that project gets no git credentials and the status says the remote's host is not supported for
   scoped credentials.
4. **Given** a daemon running with a PAT, **When** the PAT expires, **Then** the credential is
   withdrawn from the container, pushes fail for want of credentials, the status says the PAT
   expired and to paste a new one, and pasting a valid PAT restores pushing without a restart.
   Beforehand, the status and the settings warn that it is about to expire (FR-008).

---

### Edge Cases

- **PAT expires while the daemon runs, or mid-push**: nothing renews it (FR-008). The warning came
  earlier; a push in flight at the expiry instant may fail, and the following ones fail until the
  user pastes a new PAT.
- **Project added to a running daemon**: if a PAT is stored for its repository it is delivered as
  soon as the project is registered; otherwise it has none and the status says so.
- **Project's remote changes to another repository**: the old PAT is not reused for the new
  repository; the PAT stored for the new one is used, or none.
- **Pasted token is not acceptable**: a classic PAT (not fine-grained), a PAT with no expiry,
  an expired one, one GitHub refuses for the repository, or (where GitHub lets the client see it,
  FR-009) one that reaches another repository, is rejected at paste time with the reason, and
  nothing is stored.
- **A PAT given more than asked** (extra permissions, or reach over other repositories that the
  client cannot see): the client cannot narrow a PAT. The settings text says so, and that a PAT
  leaked from a sandbox stays valid until it expires or the user deletes it on GitHub, which is why
  a short expiry is recommended.
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
- **Two daemons, or two sessions of one daemon, on the same repository**: they use the same stored
  PAT (it is the user's, one per repository). Stopping one daemon removes that daemon's copy and
  leaves the other's working.
- **User replaces or removes a PAT while daemons run**: a replacement is delivered to every running
  daemon that uses the repository, in place; removing it withdraws it from them (FR-010).
- **Daemon stopped or deleted**: the copy it held is withdrawn (FR-010). The PAT itself stays valid
  until it expires or the user deletes it on GitHub; the application does not revoke it, because it
  is the user's own and other daemons may use it.

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
  the system MUST use a credential that (a) can reach only that one repository, (b) is granted
  repository contents read/write and metadata read (the permissions the settings tell the user to
  give), and (c) expires on its own. The source is a **fine-grained personal access token that the
  user creates on GitHub for that one repository and pastes into the application's settings**, one
  per repository (jaro, D7). The client stores it on the user's device only, in the OS keychain
  or the codebase's existing owner-only secret store (outside every path a container mounts), and
  never logs it. It reaches a container only as FR-006 and FR-007 allow, and only the PAT of the
  container's own project's repository. No GitHub App, no project-run server and no client-minted token.
- **FR-006**: The credential MUST be readable only inside the container it was issued for. It MUST
  NOT be written into the project's files, the project's git configuration on the host, or any path
  the container shares with the host.
- **FR-007**: Git inside the container MUST push and pull the project's GitHub repository over HTTPS
  using only that project's scoped credential, with no SSH key, including when the project's remote is
  written in SSH form; the project's configuration on the host MUST NOT be modified to achieve this.
  Git MUST NOT offer that credential to any other repository or host.
- **FR-008**: The system MUST NOT renew a credential (a PAT cannot be renewed by the client; GitHub
  has no such call). Instead it MUST (a) show each stored PAT's expiry in the settings, (b) warn, in
  the settings and in the daemon status of every project using it, once it has 24 hours or less to
  live (so a 1-day PAT shows the warning from the day it is pasted), (c) stop delivering it at its expiry and withdraw it from running containers, and (d) when
  the user pastes a replacement, deliver it to running daemons in place, without restarting them or
  interrupting sessions.
- **FR-009**: A PAT's lifetime is the user's choice, at least 1 day (there is no 1-hour cap). The
  client MUST accept a PAT only if all of these hold at paste
  time and re-check them at each delivery: it is a fine-grained PAT (a classic one is rejected);
  GitHub reports an expiry for it (no expiry: rejected) that has not passed; GitHub grants it
  access to the target repository, with push; and, where GitHub's API lets the client list the
  repositories the PAT reaches, it reaches no other. Where reach cannot be seen, the client says
  in the settings that it cannot verify it. No credential MAY be delivered without a known expiry.
  Making the expiry short is the user's choice, and the settings recommend the shortest the user
  can live with.
- **FR-010**: When a daemon stops or is deleted, the system MUST discard the client's per-daemon
  copy and remove the PAT from the container, in every case. It MUST NOT revoke or delete the PAT
  on GitHub (it is the user's, and other daemons may use it). When the user removes a stored PAT,
  the system MUST withdraw it from every running daemon, and the settings MUST tell the user it is
  still valid on GitHub until they delete it there.
- **FR-011**: The client cannot narrow a PAT, so no per-daemon permission toggle exists. At the
  point of pasting, the settings MUST tell the user which permissions to give the PAT (repository
  contents read/write, metadata read; nothing else) and that every further permission they give it
  is held by the sandbox as well.

**Failing closed**

- **FR-012**: When no scoped credential can be used for a project — no PAT stored for its
  repository, the PAT expired, refused by GitHub or unverifiable because GitHub is unreachable, or
  the remote not on GitHub, or the container's git too old for the credential helper — the daemon MUST still start (or keep running) with no git credentials
  for that project, and MUST NOT use any of the items of FR-001 instead.
- **FR-013**: The daemon's status MUST say, per project without a credential, that it has none and
  the reason, in words that name what the user can do (paste a PAT, paste a new one, fix the
  remote, retry). The status MUST clear when a credential is delivered.

**Proof**

- **FR-014**: An automated test MUST start, or build the full start-up of, a daemon of each runtime
  on a host that has an SSH key, an agent socket, a populated credential store and a personal token
  in the environment, with saved settings enabling every removed item, and MUST assert that none of
  them — path or content — is in the container's environment, mounts, secrets or files.
- **FR-015**: An automated test MUST show that the credential the container receives is the PAT
  stored for the project's repository and no other, that FR-009's checks reject a classic PAT, a PAT
  without expiry, an expired one and one that GitHub refuses for the repository, and that git push
  and pull from inside the container use it over HTTPS.

### Key Entities

- **Scoped repository credential**: a fine-grained PAT bound to one repository (`owner/name` on
  github.com), with its expiry time (chosen by the user, at least 1 day). Stored on the user's
  device only; copied into a daemon's container while that daemon runs; never held by the project
  or the host's git configuration.
- **Credential source**: the user's pasted PATs (FR-005): one per repository, kept in the user's
  device-local secret store, managed in the application's settings (paste, replace, remove, see the
  expiry). No App ID, no private key.
- **Credential status (per daemon project)**: *has credential (expires at …, and a warning within
  24 h)*, or *none* with a reason from: no PAT for this repository, PAT expired, PAT refused by
  GitHub (revoked, not reaching the repository), GitHub unreachable, remote not on GitHub, the
  container's git too old for scoped credentials.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of sandboxed daemon starts, a search of the container's environment, mounts,
  secrets and files finds zero occurrences of the user's SSH keys, agent socket, credential store or
  personal token.
- **SC-002**: A session in a sandboxed daemon can push and pull its project's GitHub repository with
  no step by the user beyond creating a fine-grained PAT for that repository on GitHub and pasting
  it into the settings, once per repository and again after each expiry.
- **SC-003**: For a PAT the user created as the settings instruct (one repository; contents
  read/write and metadata read), the credential found inside a sandbox is refused by GitHub for
  every other private repository and every permission it was not given (public repositories stay
  readable, as for any token). A PAT the user made wider is wider; the client
  rejects what it can see (FR-009) and warns about the rest (FR-011).
- **SC-004**: No credential is ever delivered without a known, future expiry; 24 hours or less
  before it, the user is warned in the settings and in the status of each project using it; at
  expiry it is withdrawn from the container; and pasting a replacement restores pushing in a
  running daemon without restart.
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
- **Default permissions**: the settings tell the user to give the PAT contents read/write and
  metadata read only. The client cannot narrow a PAT, so any extra permission the user gives it is
  held by the sandbox too (FR-011). *(agent-resolved, least privilege; reworked for D7)*
- **Scope unit**: one credential per repository, not one per daemon covering all its projects, so a
  daemon serving two repositories cannot use one project's credential on the other. *(agent-resolved,
  least privilege; the issue says "limited to the repository the worktree/session belongs to")*
- **Which remote**: the repository the worktree pushes to (`origin`'s push URL). *(agent-resolved)*
- **Pull of public repositories** needs no credential and keeps working without one.
- The client, not the container, talks to GitHub to check a PAT; the container holds only the PAT
  of its own project's repository and nothing that can mint or list others. A PAT in a container
  is exposed to code running there for up to its whole lifetime, which is why FR-009 insists on an
  expiry and the settings recommend a short one. *(Accepted consequence of jaro's choice, D7.)*
- Whether GitHub's API lets the client list the repositories a fine-grained PAT reaches, and report
  its expiry on a probe request, is confirmed against the real API in the close unit's live run (T056, research R2), with M5 built on recorded responses; FR-009 is
  written to hold either way.

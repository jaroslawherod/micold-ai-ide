# Research: daemon-qualified refs and the routed envelope

## R1. Where the routed envelope lives on the wire

Decision: a new frame `Kind::Routed = 2` with a JSON body `{target, conn, seq, msg}`.
Rejected: (a) a `ClientMsg::Routed` variant: a plain connection would accept it and an envelope
could nest by construction, and "distinguish from an unwrapped message" would rest on a variant tag
inside the payload the router must not interpret; (b) a routing prefix inside the 4-byte header's
reserved `u16`: reserved must stay zero (`EnvelopeHeader::parse`), and a daemon id does not fit;
(c) a second stream: the framed stream must not be split (`envelope.rs` docs). The header kind byte
is read before the body, which gives FR-005 and FR-006 for free.

## R2. Wire daemon id shape

Decision: ASCII text `[a-z0-9][a-z0-9._-]{0,62}` (1 to 63 chars), lowercase only; validated in the
constructor and in `Deserialize`. Rejected: a UUID (unreadable in logs and the `daemon.id` setting);
reusing `daemons::DaemonId(u32)` (client-local registry slot, differs per client).
Default `local` when `daemon.id` is unset: the orchestrator assigns the real id from its registry
and certificate, so a default only has to be valid, not unique.

## R3. Project identity override: name and location

Decision: git local config key `micold.projectId` in the repository, read through a new local-only
`Git::config_local_get`. Value: 1 to 128 chars, no whitespace or control characters; identity
`override:<value>`. Rejected: (a) a committed file such as `.micold/project-id`: writes into the
user's tree and shows in `git status`; (b) a field in the daemon config or project catalog: per
daemon, so two daemons must be told the same value twice and nothing travels with the repository;
(c) an environment variable: invisible to the daemon's per-project reads. Local git config is
readable without a fetch (Principle IV) and the user sets it on each clone once.
Which remote: `origin`, else the first in config order; Rejected: all remotes (two forks of one
repository would then split or merge by accident).

## R4. Schema hash input

Decision: `routing.rs` becomes a fourth input of `schema_hash` (`build.rs`, `hashing.rs`,
`tests/schema_hash.rs`). Rejected: defining the types in `messages.rs` (2921 lines already) or
`envelope.rs` (no serde there). Leaving the file unhashed would let the envelope change without a
version bump, the failure the hash exists to catch. `project_identity.rs` is not hashed: the wire
carries only its `String` value.

## R5. Worktree local ref

Decision: `WorktreeRef { daemon, project: PathBuf, dir_name }`. A `dir_name` is unique only inside a
project, and `(project path, dir_name)` is what `ClientMsg` already addresses a worktree by.
Rejected: `dir_name` alone (collides across projects).

## R6. Row-versus-target check

Decision: a pure `RoutedEnvelope::owns(&self, row_daemon)` helper returning `RoutingError::ForeignRow`;
rows flow daemon to client, so enforcement is the orchestrator's (#1724); this feature supplies and
tests the check. Rejected: enforcing in the codec (it never sees rows).

## R7. Normalisation port and case

Decision: drop the port and scheme, lowercase the host only, keep path case. Reuses `split_remote`,
which already rejects local paths and unknown schemes. Rejected: keeping the port (ssh port 22 vs
https 443 would split one repository). Cost: two hosts on one name with different ports merge; the
override covers it.

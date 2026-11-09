# Contract: daemon-qualified refs, the routed envelope and the project identity

Feature 1723 (issue #1723, epic #1722). Covers orchestrator FR-004 and FR-006. Protocol version 78 introduced the daemon-qualified rows and the envelope; the version is
80 once project rows carry `identity` (79 is #1530's, unrelated).
Source of truth in code: `crates/micold-core/src/protocol/routing.rs` and
`crates/micold-core/src/project_identity.rs`. This file is the only one this feature adds under
`specs/orchestrator/`.

## 1. Daemon id

Text, 1 to 63 chars, `[a-z0-9][a-z0-9._-]*`, lowercase. Anything else is invalid and is rejected
where it is decoded. The orchestrator takes a daemon's id from its certificate (later), never from
what the daemon says; a daemon sets the id on its own rows from `[daemon] id` (default `local`).

## 2. Qualified refs

| Ref | Fields | Equal when |
|---|---|---|
| Session | `daemon`, `id` (the daemon's `SessionId`) | both parts equal |
| Worktree | `daemon`, `project` (path), `dir_name` | all parts equal |
| Project | `daemon`, `path` | both parts equal |

`SessionSummary`, `WorktreeSnapshot` and `ProjectSnapshot` carry `daemon`; `ProjectSnapshot` also
carries `identity` (section 4). A ref for an object that no longer exists decodes; absence is the
owner's answer.

## 3. Routed envelope

Frame header kind byte `2` (`Routed`), encoding byte `0` (JSON), reserved `0`. Same stream and
length prefix as every frame. Body object:

| Field | Type | Range / rule |
|---|---|---|
| `target` | string | a valid daemon id; absent, empty or invalid is rejected before `msg` is read |
| `conn` | u64 | `0..=2^64-1`; one value with two targets is two conversations |
| `seq` | u64 | `0..=2^64-1`; carried as is, no order or limit check (resume policy is #1730). A non-Rust peer needs 64-bit integers above 2^53-1 |
| `msg` | `ClientMsg` | any variant; an envelope in `msg` is rejected |

Errors (`RoutingError`): `MissingTarget` (field absent or empty), `InvalidTarget` (text not a valid
daemon id), `NestedEnvelope` (`msg` or `target` is itself an envelope body), `ForeignRow` (a row whose `daemon`
differs from the target; checked by the orchestrator with `RoutedEnvelope::owns`), `Malformed` (not
JSON, not an object, or `conn`, `seq` or `msg` missing or of the wrong type). Codec error
`WrongRouting`: a `Routed` frame on a plain connection (kind byte `0`), or a `Control` frame on a
routed one. A rejected frame delivers nothing.

Handshake: `Hello`, `Welcome` and `Refused` are plain `Control` frames; a routed link handshakes plain
and switches to routed after `Welcome`. A `Hello` is never wrapped.

Worked example (kind `2`, encoding `0`, the body below; the test `contract_example` decodes it):

```json routed-example
{"target":"build-box","conn":7,"seq":42,"msg":{"Ping":{"nonce":9}}}
```

## 4. Project identity ("same project")

Daemons serve the same project when their `ProjectSnapshot.identity` is equal. The daemon derives
it; the orchestrator never reads a daemon's filesystem.

1. If the repository's local git config has `micold.projectId`, identity is `override:<value>`
   (surrounding whitespace is trimmed first; 1 to 128 chars, no whitespace or control characters; an invalid value is ignored and the next
   step applies).
2. Else take remote `origin`, or the first remote in config order. Normalise its URL: drop the
   scheme, `user@` and the port; write `host:o/r` as `host/o/r`; lowercase the host (path case is
   kept); remove a trailing `/` and one trailing `.git`. Identity is `remote:<host>/<path>`.
3. No remote, or a remote that is a local path (a drive-letter path such as `C:/x` included): identity is `local:<daemon id>:<project path>`, so
   such a repository is never grouped with another.

Examples: `git@host:o/r.git` and `https://host/o/r` both give `remote:host/o/r`; two repositories
with no remote give two different `local:` values. Failure case: two hosts that share a name but
differ only by port merge; set `micold.projectId` to separate them. A Windows path appears only in a
`local:` identity.

## 5. Sequence number and remote normalisation

`seq` is carried, not interpreted: ordering, gaps and resume belong to the link (#1730). The
normalisation steps of section 4 are the whole rule; `crates/micold-core/src/project_identity.rs`
holds the tests for each.

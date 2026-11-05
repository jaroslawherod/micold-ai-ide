# Data model: daemon-qualified refs and the routed envelope

All in `crates/micold-core`. `serde` JSON on the control plane; the same derives give postcard.

| Type | Shape | Rules |
|---|---|---|
| `WireDaemonId` (`protocol/routing.rs`) | newtype over `String` | 1 to 63 chars, `[a-z0-9][a-z0-9._-]*`; `new`/`FromStr`/`Deserialize` validate; `Display`, `Eq`, `Hash`, `Ord`; serialises as a string |
| `SessionRef` | `{daemon: WireDaemonId, id: SessionId}` | equal only when both parts equal |
| `WorktreeRef` | `{daemon, project: PathBuf, dir_name: String}` | as above |
| `ProjectRef` | `{daemon, path: PathBuf}` | as above |
| `ConnectionId` | `u64` newtype | names one client conversation on a link; the same value with two targets is two conversations |
| `Sequence` | `u64` newtype | carried as is; no ordering or limit checks (#1730) |
| `RoutedEnvelope<M>` | `{target: WireDaemonId, conn: ConnectionId, seq: Sequence, msg: M}` | decoded with the target checked first; `M = ClientMsg` today |
| `RoutingError` | `MissingTarget`, `InvalidTarget(String)`, `NestedEnvelope`, `ForeignRow{target,row}` | Display names the target problem |
| `Routing` (`codec.rs`) | `Plain` (default), `Routed` | selects which control kind a codec accepts |
| `ProjectIdentity` (`project_identity.rs`) | newtype over `String` | `remote:<host>/<path>`, `override:<value>`, or `local:<daemon>:<path>`; derived by the daemon, compared by the orchestrator |

Row changes in `protocol/messages.rs` (no `serde(default)`: peers that differ are refused at the handshake):

| Row | New field(s) |
|---|---|
| `SessionSummary` | `daemon: WireDaemonId` |
| `WorktreeSnapshot` | `daemon: WireDaemonId` |
| `ProjectSnapshot` | `daemon: WireDaemonId`, `identity: ProjectIdentity` |

Builders: `SessionSummary::qualified() -> SessionRef`, `WorktreeSnapshot::qualified(project: &Path) -> WorktreeRef`,
`ProjectSnapshot::qualified() -> ProjectRef`.

Config: `[daemon] id = "<WireDaemonId>"` (optional, default `local`). Git: `micold.projectId` in the
repository's local config (optional).

Normalisation (FR-008), applied to the chosen remote URL: trim; split with `github::split_remote`
(drops scheme, `user@`, port; `host:o/r` and `ssh://host/o/r` give the same pair); lowercase host;
strip leading `/`, trailing `/`, one trailing `.git`; join `host/path`. `git@host:o/r.git` and
`https://host/o/r` both give `remote:host/o/r`.

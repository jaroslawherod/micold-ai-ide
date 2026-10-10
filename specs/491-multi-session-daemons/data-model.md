# Data model

All types in `micold-core/src/daemons.rs` unless noted; serde `snake_case`, every new field
`#[serde(default)]`.

## DaemonId
Stable key of a daemon: `DaemonId(u32)`, allocated from the persisted `Settings.next_daemon_id` and never reused, so removing a daemon and adding another of the same name cannot inherit its bindings, and renaming keeps them. The migrated daemon is id 1. Bindings, `legacy_default_daemon` and actor identity use the id.

## DaemonName
Newtype over `String`, the display name. Trimmed, non-empty, unique case-insensitively within a registry (FR-001). Renaming changes nothing else.

## DaemonRuntime
```text
enum DaemonRuntime {
    Host,
    Container(ContainerSettings),   // SandboxProfile + container_name + port
    Unsupported { kind: String },   // unknown `kind` from a newer file: kept verbatim on save, state Stopped with reason "unsupported runtime", cannot start, can be removed
    // SSH (#687), Kubernetes (#688): later variants, #[non_exhaustive]
}
```
`ContainerSettings { profile: SandboxProfile, container_name: String, port: u16 }`.
Validation: image present and valid, container name unique across the registry (edge case "two
daemons, one container"), port unique and in range.
`DaemonRuntime::label()` gives the plain value shown in lists (FR-015).

## DaemonEntry
`{ id: DaemonId, name: DaemonName, runtime: DaemonRuntime, auto_start: bool }`. Persisted. `auto_start`: connect (spawn or start) this daemon when the app launches; a daemon the user stopped (`Stopped`) is neither spawned nor reconnected until the user starts it.

## DaemonRegistry
Ordered `Vec<DaemonEntry>`. `add`, `edit`, `remove` return `Result<_, RegistryError>`
(`DuplicateName`, `BlankName`, `ContainerInUse{other}`, `InvalidField{field}`, `HostExists`).

## DaemonState (runtime only, never persisted)
`Starting | Connected | Unreachable{reason} | VersionMismatch{client, daemon} | Stopped`
(FR-009) plus `version: Option<String>`, `endpoint: String`, `errors: RingBuffer<ConnectionError>`,
`log_tail`. Transition table and user-facing strings in [contracts/client-daemon-routing.md](./contracts/client-daemon-routing.md).

## Binding
```text
enum Binding { Bound(DaemonId), NoDaemon }
```
Stored per project as `bindings: BTreeMap<String, DaemonId>` keyed by worktree `dir_name`, with
the reserved key `""` for Default. Missing key => `Settings.legacy_default_daemon` if set and still in the registry, else
`NoDaemon` (never a silent guess). `legacy_default_daemon` is written once by the migration and
never by anything else, so adding more daemons later does not unbind legacy worktrees. Every new
worktree/Default binding is written explicitly. Removal of a daemon deletes its keys' effect by
resolving to `NoDaemon` without rewriting disk (nothing deleted, FR-014).

## DaemonView (projection for Settings and rows)
`{ id, name, runtime_label, endpoint, version, state, bound: Vec<WorktreeRef> }`; no runtime-specific
field (FR-015).

## Relationships
Registry 1..* DaemonEntry; Project 0..* Binding -> 0..1 DaemonEntry; DaemonEntry 1 -> 1 live
`DaemonState` and 1 actor.

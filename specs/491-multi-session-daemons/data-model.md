# Data model

All types in `micold-core/src/daemons.rs` unless noted; serde `snake_case`, every new field
`#[serde(default)]`.

## DaemonName
Newtype over `String`. Trimmed, non-empty, unique case-insensitively within a registry (FR-001).

## DaemonRuntime
```text
enum DaemonRuntime {
    Host,
    Container(ContainerSettings),   // SandboxProfile + container_name + port
    // SSH (#687), Kubernetes (#688): later variants, #[non_exhaustive]
}
```
`ContainerSettings { profile: SandboxProfile, container_name: String, port: u16 }`.
Validation: image present and valid, container name unique across the registry (edge case "two
daemons, one container"), port unique and in range.
`DaemonRuntime::label()` gives the plain value shown in lists (FR-015).

## DaemonEntry
`{ name: DaemonName, runtime: DaemonRuntime, auto_start: bool }`. Persisted.

## DaemonRegistry
Ordered `Vec<DaemonEntry>`. `add`, `edit`, `remove` return `Result<_, RegistryError>`
(`DuplicateName`, `BlankName`, `ContainerInUse{other}`, `InvalidField{field}`, `HostExists`).

## DaemonState (runtime only, never persisted)
`Starting | Connected | Unreachable{reason} | VersionMismatch{client, daemon} | Stopped`
(FR-009) plus `version: Option<String>`, `endpoint: String`, `errors: RingBuffer<ConnectionError>`,
`log_tail`. Transition table in [contracts/client-daemon-routing.md](./contracts/client-daemon-routing.md).

## Binding
```text
enum Binding { Bound(DaemonName), NoDaemon }
```
Stored per project as `bindings: BTreeMap<String, DaemonName>` keyed by worktree `dir_name`, with
the reserved key `""` for Default. Missing key => `Settings.legacy_default_daemon` if set and still in the registry, else
`NoDaemon` (never a silent guess). `legacy_default_daemon` is written once by the migration and
never by anything else, so adding more daemons later does not unbind legacy worktrees. Every new
worktree/Default binding is written explicitly. Removal of a daemon deletes its keys' effect by
resolving to `NoDaemon` without rewriting disk (nothing deleted, FR-014).

## DaemonView (projection for Settings and rows)
`{ name, runtime_label, endpoint, version, state, bound: Vec<WorktreeRef> }`; no runtime-specific
field (FR-015).

## Relationships
Registry 1..* DaemonEntry; Project 0..* Binding -> 0..1 DaemonEntry; DaemonEntry 1 -> 1 live
`DaemonState` and 1 actor.

# Data Model: The Session Daemon in a Sandbox

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

The spec's key entities as concrete Rust shapes, with the validation rules traced to the requirement
that demands each one. Everything here lives in `micold-core` and is render-free: no type below
depends on `iced`, and none of the validation spawns a process. That is what makes the whole model
testable under Principle I, and it is why the impure parts (`sandbox/exec.rs`, the client's render
glue) hold no rules of their own.

## Entity map

```text
Placement ──selects──> SandboxProfile ──validated against──> RuntimeCapabilities
                            │                                        ▲
                            ├── ResourceBudget                       │
                            ├── NetworkPosture                  RuntimeKind
                            ├── CredentialShare (set)                │
                            ├── ImageSource                          │
                            └── MountSet ◀──derived from── registered projects
                                                                     │
SandboxState ◀──lifecycle of──────────────────────────────────────────
```

## 1. `Placement` — where the daemon runs (FR-001, FR-003, FR-003a)

```rust
pub enum Placement {
    /// Today's behaviour: a detached host process. Unchanged, still the default.
    HostProcess,
    /// This feature: a container on the local machine.
    LocalSandbox(SandboxProfile),
    /// Reserved by FR-003a. Not constructible in this release; the variant exists so that
    /// resolution, connection and error handling are written against a set, not an if.
    #[non_exhaustive]
    Remote(RemotePlacement),
}
```

**Why a variant that cannot be built.** FR-003a requires the placement model to describe a
non-local daemon. Adding the variant now costs one `match` arm per site and forces every
placement-dependent decision to be *expressed* rather than assumed; adding it later means finding
every place `HostProcess` was assumed by omission. `connect_or_spawn` becomes
`connect_or_start(placement)` for the same reason.

**Rules.**
- P-1 (FR-002): resolution is a pure function of settings; it never touches the network or a runtime.
- P-2 (FR-035): resolution never silently substitutes one placement for another. A `LocalSandbox`
  that cannot start yields an error, not a `HostProcess`.
- P-3 (FR-035a): a fallback to `HostProcess` is representable only as a distinct, user-consented
  action carrying the reason it was offered — never as a resolution outcome.

## 2. `SandboxProfile` — the user's configuration (FR-005 … FR-019)

```rust
pub struct SandboxProfile {
    pub runtime: RuntimeKind,
    pub image: ImageSource,
    pub budget: ResourceBudget,
    pub network: NetworkPosture,
    pub credentials: BTreeSet<CredentialShare>,
    pub survive_logout: bool,        // mirrors the existing opt-in (FR-014a, R6)
}
```

**Rules.**
- SP-1 (FR-004a): `credentials` defaults to **empty**. Deserialising a v3 document, or a v4 document
  with the field missing, produces an empty set — never an inferred one.
- SP-2 (FR-021): `runtime` defaults to `RuntimeKind::Docker`.
- SP-3: a profile is *valid in isolation* (ranges, well-formed image reference) and separately
  *satisfiable against a runtime* (§5). The two are different functions returning different errors,
  because the first is the user's mistake and the second is the environment's limitation.

## 3. `ResourceBudget` — the limits (FR-012 … FR-016)

```rust
pub struct ResourceBudget {
    pub cpus: Option<MilliCpus>,     // 1000 = one core
    pub memory: Option<Bytes>,
    pub pids: Option<u32>,
    pub storage: Option<Bytes>,      // the one R5 showed is not portable
}
```

Each limit is `Option` because *unset* and *set to the maximum* are different user intents and must
round-trip differently. Newtypes rather than bare integers so a megabyte can never be passed where a
millicpu is expected (Principle V: "a type-level fact, not a runtime string comparison").

**Rules.**
- RB-1 (FR-016): every limit has a documented minimum below which the daemon cannot function.
  Validation clamps into range and reports the clamp, following `settings.rs`'s existing
  `clamp_scrollback` / `clamp_env_include_timeout` precedent rather than inventing a second idiom.
- RB-2 (FR-013): `None` means "the runtime's default", which is rendered as *unlimited* in the view,
  not as a blank field.
- RB-3 (FR-015, R5): a limit the selected runtime cannot enforce is **not silently dropped**. See §5.

## 4. `NetworkPosture` and `CredentialShare`

```rust
pub enum NetworkPosture {
    /// R4's decision: user-defined bridge with IP masquerade disabled. Outbound connections
    /// fail; the published control port still works. DNS lookups still resolve — documented.
    NoOutbound,
    /// Full egress, for users whose sessions need to fetch dependencies.
    Outbound,
}

pub enum CredentialShare {
    GitConfig,      // ~/.gitconfig
    SshAgent,       // the agent socket, not the keys
    GitCredentials, // the credential helper's store
    AiCliAuth,      // the AI CLI's own auth material
}
```

**Rules.**
- N-1 (FR-004a/b): `CredentialShare` is an opt-in enumeration, never a free-text path list. A user
  cannot mount an arbitrary host directory by typing it into a credentials field.
- N-2 (FR-004c): each active share must be individually visible in the view while it is active —
  so the set is rendered from the set, not summarised as a count.
- N-3 (R4): `NoOutbound` is the default. It does not claim to block DNS resolution, and the docs say
  so; a posture that overstates what it blocks is worse than one that understates it.

## 5. `RuntimeCapabilities` — what the environment can actually do (FR-020, FR-022, R10)

```rust
pub struct RuntimeCapabilities {
    pub kind: RuntimeKind,
    pub version: String,             // the cache key: re-probe when this changes
    pub cpus: LimitSupport,
    pub memory: LimitSupport,
    pub pids: LimitSupport,
    pub storage: LimitSupport,
    pub identity_mapping: IdentityMapping,   // --user vs --userns=keep-id (R3)
}

pub enum LimitSupport {
    Supported,
    /// Carries the reason, because the view shows it (FR-015, SC-009).
    Unsupported { reason: String },
}
```

**Rules.**
- RC-1: capabilities are *probed*, not tabulated. A static table of runtime versions goes stale on
  the next release of either runtime (R10).
- RC-2 (FR-015): `reconcile(profile, caps) -> Vec<UnsatisfiableLimit>` is pure and total. The view
  renders an `Unsupported` limit disabled **with its reason**; the argv builder never emits a flag
  for it. These are the same fact consumed twice, so the UI cannot drift from the behaviour.
- RC-3: reconciliation never mutates the profile. The user's stored intent survives a move to a
  runtime that cannot honour it and takes effect again on a runtime that can.

## 6. `MountSet` — what the sandbox can see (FR-006 … FR-010, R2)

```rust
pub struct MountSet {
    pub projects: Vec<ProjectMount>,   // one per registered project
    pub state: NamedVolume,            // daemon state; survives recreation (FR-011)
    pub secret: SecretMount,           // the R1 token, read-only, 0600 on the host
}

pub struct ProjectMount {
    pub host: PathBuf,
    pub container: PathBuf,   // == host on Linux/macOS (R2); mapped on Windows
    pub writable: bool,
}
```

**Rules.**
- M-1 (FR-006/FR-007): only registered project directories are mounted. The user's home, the
  runtime's own socket, and anything not registered are absent — the sandbox's guarantee is what it
  *cannot* reach, so this is the load-bearing rule of the feature.
- M-2 (R2): on Linux and macOS `container == host`, enforced by a test, because git records absolute
  paths in worktree metadata and both processes run git.
- M-3 (FR-011): daemon state is a named volume, not a bind mount, so recreating the container keeps
  `projects.json`, per-project state and logs.
- M-4 (R9): the set is fixed at creation. Changing the registered projects marks the sandbox stale
  and surfaces an explicit restart; nothing restarts on its own.

## 7. `SandboxState` — the lifecycle (FR-032 … FR-036, SC-004)

```text
             ┌──────────────┐
             │  Disabled    │  placement = HostProcess
             └──────┬───────┘
                    │ user enables
             ┌──────▼───────┐  runtime present? version? capabilities?
             │  Probing     │─────────────┐
             └──────┬───────┘             │ runtime missing / too old
                    │ ok                  ▼
             ┌──────▼───────┐      ┌─────────────┐
             │  Acquiring   │─────>│  Failed     │  reason + remedy, persistently visible
             │  (image)     │ err  │             │  (FR-035b); offers consented fallback
             └──────┬───────┘      └─────────────┘  (FR-035a) — never takes it
                    │ present             ▲
             ┌──────▼───────┐             │
             │  Starting    │─────────────┘
             └──────┬───────┘
                    │ handshake ok
             ┌──────▼───────┐   projects changed (R9)   ┌──────────┐
             │  Running     │──────────────────────────>│  Stale   │
             └──────────────┘<── user restarts ─────────└──────────┘
```

**Rules.**
- S-1 (SC-004, FR-032): `Acquiring` reports continuous progress. It is the only state that may last
  minutes, and it is the first thing a new user sees — silence here reads as a hang.
- S-2 (FR-035, FR-035a): no edge leaves `Failed` for a working unsandboxed daemon without an explicit
  per-occurrence user action. There is no automatic path out of this state.
- S-3 (FR-035b): `Failed` and `Stale` are persistently visible via `ConnectionBanner`, not a toast
  that scrolls away.
- S-4 (FR-034): every terminal failure carries a *reason* and a *remedy*, both from a closed
  enumeration — so they are testable strings, not formatted-at-the-call-site prose.
- S-5: this state machine is pure and lives in `micold-core`; the client's `features/sandbox.rs`
  holds only the current value and the messages that advance it.

## Traceability

| Entity | Requirements | Contract |
|---|---|---|
| `Placement` | FR-001, FR-002, FR-003, FR-003a, FR-035 | [protocol-delta](./contracts/protocol-delta.md) |
| `SandboxProfile` | FR-004a–c, FR-005, FR-021 | [sandbox-settings-schema](./contracts/sandbox-settings-schema.md) |
| `ResourceBudget` | FR-012 – FR-016 | [container-runtime](./contracts/container-runtime.md) |
| `NetworkPosture` | FR-017, FR-018 | [container-runtime](./contracts/container-runtime.md) |
| `RuntimeCapabilities` | FR-020, FR-022, SC-009 | [container-runtime](./contracts/container-runtime.md) |
| `MountSet` | FR-006 – FR-011 | [container-runtime](./contracts/container-runtime.md) |
| `SandboxState` | FR-032 – FR-036, SC-004 | — (client-side; covered by quickstart Part B) |

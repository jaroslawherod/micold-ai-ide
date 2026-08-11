# Contract: Service Capabilities

**Feature**: 021 | **Satisfies**: FR-015 – FR-019a, SC-005

The contract between the render-free core (which *declares* an I/O need), the binary (which
*supplies* the real implementation) and a test (which *supplies* a fake).

## Narrowness rule

A capability MUST be narrow enough that a consumer needing one operation is not forced to supply or
fake unrelated ones (FR-016).

**Test for narrowness**: if a test must implement a method it does not exercise merely to satisfy
the trait, the capability is too wide and must be split.

## Inventory

### Existing (7, now 8) — one split by this feature

| Capability | Declared in | Real | Fake |
|---|---|---|---|
| `Git` | `core/git.rs` | `GitCli` | `FakeGit` |
| `ProjectStore` | `core/store.rs` | `JsonFileStore` | `FakeProjectStore` (T048) |
| `SettingsStore` | `core/settings.rs` | `JsonFileSettingsStore` | `FakeSettingsStore` (T048) |
| `FolderScanner` | `core/fs_scan.rs` | `StdFolderScanner` | `FakeFolderScanner` (T048) |
| `FolderBrowser` | `core/fs_scan.rs` | `StdFolderScanner` | `FakeFolderScanner` (T048) |
| `TerminalBackend` | `core/terminal.rs` | daemon-backed | `FakeTerminalBackend` |
| `TerminalHandle` | `core/terminal.rs` | daemon-backed | `FakeHandle` |
| `AiCliProvider` | `core/provider.rs` | `ClaudeProvider` | `FakeAiCliProvider` (T048) |

`FakeGit` is the shape the others match. SC-005 requires every capability to have a fake *and* at
least one test exercising real behavior through it; T048 supplied the four that were missing.

**`FolderScanner` was split at T048** (FR-016). It declared `is_git_repo`, `is_available` and
`list_subdirs`, but every consumer that took it *as a trait* — `Workspace::open_or_activate`,
`Workspace::refresh_availability`, `Catalog::add_project` — asked only the two predicates, while
the sole caller of `list_subdirs` reached for `StdFolderScanner` concretely. Listing is now
`FolderBrowser`; `StdFolderScanner` and `FakeFolderScanner` each implement both.

**`ProjectStore` was examined and deliberately not split.** T042's narrowness check flagged
`save` as supplied-unexercised in three daemon test fakes, but its only trait consumer, `Catalog`,
exercises all three operations. The hand-written fakes existed because no shared one did — a
coverage gap presenting as a width symptom — and deleting them in favour of `FakeProjectStore` is
what resolved it.

**`AiCliProvider` has a fake but no substitutable consumer.** Every caller names `ClaudeProvider`
concretely, so nothing can be handed a different provider. `core/tests/ai_cli_provider_seam.rs`
records this and asserts what is assertable today; T049's assembly point is where it becomes
answerable.

### To declare (3)

```rust
// core/env_include.rs
pub trait EnvIncludeResolver {
    fn resolve(&self, cwd: &Path) -> EnvIncludeSnapshot;
}

// core/os_theme.rs
pub trait OsThemeProbe {
    fn detect(&self) -> Result<SystemScheme, ()>;
}
```

Both wrap logic that is already isolated — `main.rs:397–450` and `main.rs:2678` respectively — so
each is a move, not a design.

`OsThemeProbe` also serves Principle VI: `dark_light::detect()` is the codebase's only direct
operating-system branch, and putting it behind a port is what "platform-specific behavior MUST be
isolated behind clear abstractions" asks for.

### Clipboard — an outcome, not a port

All three real clipboard operations (`main.rs:1840`, `1847`, `1856`) go through
`iced::clipboard::write`/`read`, which return an `iced::Task` rather than a value. A synchronous
trait cannot wrap them without blocking.

**Contract**: a feature emits `Outcome::ClipboardWrite(String)`; the shell translates it to
`iced::clipboard::write`. Reads arrive back as an ordinary message, exactly as today.

| # | Obligation | Requirement |
|---|---|---|
| C1 | A feature never calls `iced::clipboard` directly | FR-017 |
| C2 | A test asserts the emitted request without any real clipboard access | FR-019, SC-005 |
| C3 | The shell's translation contains no decision logic | Principle I's GUI-wiring exception |

## Supply

The binary MUST be the **single** place where concrete implementations are chosen (FR-018).

```rust
// shell/capabilities.rs
pub struct Capabilities {
    pub git: Box<dyn Git>,
    pub projects: Box<dyn ProjectStore>,
    pub settings: Box<dyn SettingsStore>,
    pub scanner: Box<dyn FolderScanner>,
    pub env_include: Box<dyn EnvIncludeResolver>,
    pub os_theme: Box<dyn OsThemeProbe>,
}
```

Assembled once during boot and threaded to consumers. Replaces the nine inline construction sites:

| Site | Constructs |
|---|---|
| `main.rs:523` | `StdFolderScanner` |
| `main.rs:532` | `JsonFileSettingsStore` |
| `main.rs:649` | `JsonFileSettingsStore` |
| `main.rs:1295` | `GitCli` — **inside `update_inner`** |
| `main.rs:1310` | `StdFolderScanner` — **inside `update_inner`** |
| `main.rs:1330` | `StdFolderScanner` — **inside `update_inner`** |
| `main.rs:1924` | `JsonFileSettingsStore` — **inside `update_inner`** |
| `main.rs:2604` | `GitCli` |
| `main.rs:2709` | `StdFolderScanner` |

## Obligations

| # | Obligation | Requirement | Verified by |
|---|---|---|---|
| S1 | Non-shell code names no concrete implementation | FR-017 | `no_concrete_implementations.rs` (new) |
| S2 | Concrete implementations are chosen in exactly one place | FR-018 | Same guard |
| S3 | Every capability has a fake | FR-019 | `service_capability_fakes.rs` (new) |
| S4 | Every I/O-dependent behavior is testable with zero real filesystem, repository, clipboard or OS access | FR-019, SC-005 | Per-capability tests |

**S1 already holds for `app.rs`** — it constructs nothing (research.md §8). The guard test is a
regression lock on an existing property, and the actual migration work is S2.

## Shell split (FR-019a)

The shell divides by **external system**, never by feature.

| Module | System | Absorbs from `main.rs` |
|---|---|---|
| `startup.rs` | Process launch, window | `boot`, `window_settings`, `main` |
| `capabilities.rs` | — (assembly) | The nine sites above |
| `persist.rs` | Local filesystem | `persist`, `persist_settings`, `prune_empty_sessions` |
| `daemon_sync.rs` | Session daemon | `send_op`, `switch_daemon_attachment`, `reconcile_catalog`, `PendingOp` |
| `subscriptions.rs` | iced event loop | `subscription`, `cursor_move_events`, `window_focus_events`, `os_theme_poll` |
| `env_include.rs` | Env-include scripts | `resolve_env_include`, `refresh_env_include`, `default_resolution_cwd` |
| `os_theme.rs` | OS theme preference | `detect_system_scheme`, `map_system_scheme`, `os_theme_poll_interval` |

Inline `#[cfg(test)]` tests move with their subjects (research.md §3).

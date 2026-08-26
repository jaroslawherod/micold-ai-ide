# Data Model: Choose which AI CLI a session runs on

**Feature**: 026-multi-provider-sessions | **Date**: 2026-08-14

One new value type, one new field on `Session`, one new setting, one new field on the wire. Nothing
else about a session changes.

---

## `AiCli` — the provider discriminant

The new value type. A closed enum in `micold-core`, because the set of supported CLIs is known at
compile time and Principle V asks for invalid states to be unrepresentable — "some string a session
record happens to contain" is not a provider.

```rust
pub enum AiCli {
    #[default]
    ClaudeCode,
    Copilot,
}
```

- `Default` is `ClaudeCode`. This single choice satisfies FR-003 (initial default) and FR-013
  (pre-feature sessions) at once, in both cases by never being written down.
- It is `Copy`, `Eq`, `Hash` and `Ord` — it is used as a registry key (R9) and sorts the choices in
  the UI deterministically.
- It carries no behaviour. Behaviour lives behind `AiCliProvider`; this is the name you persist and
  look up.

### Why an enum rather than the trait object

`Session` is in the render-free core, is cloned constantly, and is compared in tests. An
`Arc<dyn AiCliProvider>` on it would make the session neither `PartialEq` nor cheaply comparable,
and would put a live capability inside a value that gets serialised. The session records *which*
provider; the shell resolves that to *the* provider.

---

## `Session` — one new field

```rust
pub struct Session {
    pub id: SessionId,
    pub location: SessionLocation,
    pub provider: AiCli,          // ← new
    pub label: SessionLabel,
    pub lifecycle: SessionLifecycle,
    pub activity: ActivitySignal,
    pub mode: TerminalMode,
    pub shells: Vec<ShellInstance>,
    pub active_shell: Option<ShellInstanceId>,
    pub next_shell_id: u32,
    pub archived: bool,
}
```

### Invariants

1. **Set once, at construction — by both constructors.** `Session::start_new(location)` becomes
   `Session::start_new(location, provider)`, and `Session::restored(id, location, label, mode)`
   takes a provider too. `restored` is the one that matters most in practice: it is how a session
   comes back from disk (`store.rs:207`) and how a daemon-reported one enters the client
   (`main.rs:2303`, `reconcile_catalog`) — between them, FR-012, FR-013 and FR-016. There is no
   setter and no message that changes it (FR-001). Making this a construction-time argument rather
   than a mutable field is what makes FR-005 ("changing the default affects nothing that exists")
   true by shape rather than by discipline — and the shape only holds if *both* constructors take
   it. `Session`'s fields are `pub` (the client already assigns `lifecycle` and `activity` across a
   crate boundary), so a `provider` written after construction, the way `archived` is, would look
   like compliance and be the opposite.
2. **Two sessions in one location may differ.** `provider` is independent of `location`; nothing
   groups or constrains sessions by it.
3. **It is not derived from disk.** Reconciliation may *discover* a session and assign it the
   provider whose store it was found in, but it never re-derives the provider of a session the
   application already knows about — the persisted value wins. This is what stops an id colliding
   across two stores (spec edge case) from changing a live session's CLI.
4. **Discovery runs on every project open**, for each of the project's locations, at a cost
   proportional to the number of locations rather than to the conversations recorded in them
   (FR-014, Clarifications 2026-08-18). A first-open-only rule would never surface the second
   session a user starts outside the application. It runs in the daemon's `AttachProject` arm
   (R15), and a discovered session's id **is** the CLI's conversation uuid — which is what makes a
   reopen idempotent instead of a duplicate.
5. **A discovered session is not an observed one.** It is listed and identified, and its `activity`
   is `Unknown` — the application is not supervising it and opens no watch on its storage, however
   many such sessions a project holds (FR-018, SC-006, SC-009). Starting it here makes it ordinary,
   badge included.

### Fields that now mean "per this session's provider"

`label`, `activity` and `archived` are unchanged in type and meaning, but their *sources* are now
provider-dependent — and `activity` additionally gains a scope: it is a claim about a supervised
session, never about a merely discovered one (invariant 5) — the title reader, the activity source, and the durable marker all resolve
through `provider`. See `contracts/ai-cli-provider.md`.

---

## Persisted shape

`StoredSession` in `micold-core/src/store.rs` gains one optional field. **No `schema_version`
bump** — see research R8 for why, and for the two precedents in this same struct.

```rust
#[derive(Debug, Serialize, Deserialize)]
struct StoredSession {
    id: uuid::Uuid,
    #[serde(default)] worktree_dir: Option<String>,
    #[serde(default)] title: Option<String>,
    #[serde(default)] mode: StoredTerminalMode,
    #[serde(default)] archived: bool,
    #[serde(default)] provider: StoredAiCli,   // ← new
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
enum StoredAiCli {
    #[default]
    ClaudeCode,
    Copilot,
}
```

`StoredAiCli` mirrors `AiCli` rather than deriving serde on the core enum, exactly as
`StoredTerminalMode` mirrors `TerminalMode` — the persisted spelling is then free to diverge from
the in-memory one without touching the core.

### Round-trip table

| On disk | Loads as | Why |
|---|---|---|
| field absent | `ClaudeCode` | FR-013 — every session written before this feature |
| `"ClaudeCode"` | `ClaudeCode` | |
| `"Copilot"` | `Copilot` | |
| an unknown string | *load error for that project file* | Existing behaviour for a malformed store; the recovery path already covers it. A forward-compatible `#[serde(other)]` fallback is **not** used: silently loading a future `Codex` session as `ClaudeCode` would start the wrong CLI in the user's worktree, which is worse than declining to load. |

---

## `Settings` — one new preference

```rust
pub struct Settings {
    pub theme: ThemePreference,
    pub scrollback_lines: usize,
    pub env_include_enabled: bool,
    pub env_include_script_path: String,
    pub env_include_timeout_secs: u64,
    #[serde(default)]
    pub default_ai_cli: AiCli,      // ← new
}
```

Additive and `#[serde(default)]`, like every preference before it: a settings file written by an
older build loads with `ClaudeCode` and the user sees no change (FR-003).

**Not validated against availability.** A default naming an uninstalled CLI is kept, not silently
rewritten — FR-004's acceptance scenario asks the application to *tell* the user, and a preference
that quietly repairs itself would also lose the user's choice across a temporary `PATH` problem
(research R11).

---

## Wire protocol — one new field

```rust
ClientMsg::SessionCreate {
    req: RequestId,
    project: PathBuf,
    worktree_dir: String,
    provider: AiCli,        // ← new
}
```

The client resolves default-or-override before sending, so no launch depends on the two processes
agreeing about a file. The daemon *does* read `settings.json` — `Catalog::new` loads it at boot — it
is simply not asked to resolve the choice. This is a wire-format change: it alters the protocol
schema hash and requires a protocol version bump (`protocol/version.rs`, `contracts/protocol.md` §4).

`SessionSummary` carries the provider outward the way it already carries `worktree_dir` and `title`,
so the client can label rows (FR-016) without a second request. **Not** the way `mode` does — `mode`
does not travel at all, and citing it as the precedent sends a reader looking for a field that is not
there.

`default_ai_cli` is **service-owned**, joining `DaemonSettings` / `SettingsSet` / `SettingsChanged`
beside the scrollback limit rather than sitting with `theme` on the client side. The daemon's
`set_scrollback` and `set_env_include` persist their whole `Settings` struct from the copy loaded at
boot, so a client-written field is reverted by the next unrelated settings change — a defect `theme`
already carries and this preference must not inherit.

---

## Provider-side artifacts (not the application's data model, but addressed by it)

Per provider, all derived purely from `(config_dir, cwd, session_id)`:

| | Claude Code | Copilot |
|---|---|---|
| base dir | `~/.claude`, `CLAUDE_CONFIG_DIR` | `~/.copilot`, `COPILOT_HOME` |
| sessions for a cwd | `projects/<slug(cwd)>/` listing | `sidebar-sessions-state/<sha256(cwd)>.json` → `sessionIds[]` |
| conversation record | `projects/<slug(cwd)>/<uuid>.jsonl` | `session-state/<uuid>/events.jsonl` |
| title | latest `ai-title` record | `name:` in `session-state/<uuid>/workspace.yaml` |
| activity source | HTTP hook receiver | tail of `events.jsonl` |
| closed marker | `projects/<slug(cwd)>/<uuid>.archived` | `session-state/<uuid>/micold.archived` |

Details and verification in `contracts/copilot-cli.md` and research R3–R6.

---

## Entity mapping to the spec

| Spec entity | Here |
|---|---|
| AI CLI provider | `AiCli` (the name) + `AiCliProvider` (the behaviour) |
| Session | `Session::provider`, `StoredSession::provider` |
| Provider availability | `AiCliProvider::is_available()` — computed, never stored (R11) |
| Default AI CLI setting | `Settings::default_ai_cli` |

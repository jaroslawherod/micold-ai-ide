# Contract: session-name persistence

**Feature**: [029-persistent-session-names](../spec.md) | **Plan**: [plan.md](../plan.md)

This application exposes no network API. Its contracts are the interfaces its own layers hold each
other to: the daemon↔client wire protocol, the on-disk catalog, and the typed methods through which
the daemon's single-writer rule is enforced. This feature changes exactly one of those three — the
internal daemon interface — and this document pins what the other two must keep doing unchanged.

---

## 1. On-disk catalog — **unchanged shape, newly populated**

Per-project state file, written by `micold_core::store::JsonFileStore` (temp + atomic rename).
The session record already carries the name:

```jsonc
{
  "id": "0e1c…-…-…",
  "worktree_dir": "feat-abc_login",   // absent/null ⇒ the project root ("Default")
  "title": "Fix the flaky login test", // ← this feature makes it routinely present
  "mode": "AiCli",
  "archived": false,
  "provider": "ClaudeCode"
}
```

**Guarantees**:

- `title` is `Option<String>`, `#[serde(default)]`. Absent ⇒ `SessionLabel::Pending`; present ⇒
  `SessionLabel::Named`.
- **No `schema_version` bump.** The field, its attributes, and both conversions already exist; this
  feature adds no key. A build that predates the feature reads a file written by one that follows it
  without special handling, and vice versa.
- `title` is never written as `""`. An empty name is not a name (data-model validation rules).
- Nothing else in the record changes meaning.

---

## 2. Daemon → client wire — **unchanged**

`SessionSummary.title: SessionLabel` (`micold-core/src/protocol/messages.rs`) already carries the
name on every `CatalogChanged` / attach snapshot. No message, field, or encoding changes, so **no
protocol version bump and no client change**.

What changes is only the *value* a client may now receive: a `Named` title for a session whose
`lifecycle` is `Idle` and which has no live process behind it. Clients must already handle that —
the discovery path (FR-014) has produced exactly that combination since feature 026 — and the
existing reconciler adopts `title` for known and unknown sessions alike.

**Guarantee to the client**: a session's title in a snapshot is the best name known to the daemon,
whether or not the session is running. A client must not treat `Pending` as "not loaded yet" and
must not substitute its own remembered value for what the snapshot says.

---

## 3. Daemon-internal interface — **what this feature adds**

Signatures are indicative; the behavioural clauses are the contract.

### 3.1 `Catalog::record_session_name(id: SessionId, name: &str) -> io::Result<bool>`

*(`micold-daemon/src/catalog.rs` — the catalog's single-writer surface)*

| Clause | Behaviour |
|---|---|
| C1 | Resolves the session by `id` through `Workspace::find_session_mut`, which also yields the owning project. Never by index or position. |
| C2 | Returns `Ok(false)` and writes nothing if the session's label already equals `Named(name)`, or if no session with that id is known. |
| C3 | Otherwise sets the label to `Named(name)` **in memory first**, then persists that project's state file, and returns `Ok(true)`. |
| C4 | An empty `name` is rejected as a no-op (`Ok(false)`) — it is not a name. |
| C5 | A persist failure returns the `io::Error`; the in-memory label stays updated. The caller logs and continues (FR-009). |
| C6 | Touches exactly one session's label. No other session, project, or field is read-modify-written. |

### 3.2 `DaemonState::drain_signals(&self) -> DrainedSignals`

*(`micold-daemon/src/state.rs` — existing method, return type widened)*

| Clause | Behaviour |
|---|---|
| C7 | Still lock-only and free of blocking I/O; still safe to call from the async supervisor task. |
| C8 | Reports the same "a projected summary changed" signal it reports today (the supervisor's broadcast trigger is unchanged). |
| C9 | Additionally returns the `(SessionId, String)` pairs whose stripped OSC-0 title differed from the previous drain — the same debounce that already gates `last_title`, surfaced rather than swallowed. |
| C10 | Reports each change once. A title that does not change reports nothing, however many ticks pass. |

### 3.3 `DaemonState::record_observed_names(&self, changes) -> ()`

*(`micold-daemon/src/state.rs` — called from the supervisor's blocking hop)*

| Clause | Behaviour |
|---|---|
| C11 | Called only with a non-empty set, and only from a blocking context (`spawn_blocking`), never on the async runtime. |
| C12 | Forwards each pair to `Catalog::record_session_name`, logging a failure at `warn` and continuing with the rest. One bad write does not abandon the others. |
| C13 | Surfaces nothing to the client: no `WireLifecycle::Failed`, no error message, no lifecycle change (FR-009). |

### 3.4 `DaemonState::recover_session_names(&self, project: &Path) -> usize`

*(`micold-daemon/src/state.rs` — the FR-006 pass, run beside `discover_external_sessions`)*

| Clause | Behaviour |
|---|---|
| C14 | **Blocking**: reads each provider's own store. Runs in the same `spawn_blocking` hop as the worktree refresh and the discovery pass, and reads the same worktree cache for its location list. |
| C15 | Considers only sessions of `project` whose label is `Pending`. A `Named` session is skipped **before** any filesystem access — this is what bounds the pass (research R4) and what protects FR-008. |
| C16 | For each candidate, asks that session's **own** provider (`session.provider.provider()`) for `read_title(config_dir, session.location.cwd(project), id)`. One hoisted provider would read one CLI's sessions out of the other's store and find nothing. |
| C16.1 | `read_title` answers with the name that CLI **currently** holds for the conversation, not the first or the most recently written of several (FR-005, BUG-002). For `ClaudeProvider`: the latest non-empty `{"type":"custom-title","customTitle":…}` if there is one — that kind alone ranks above position, because `claude` re-emits the pre-rename `ai-title` on every turn after a `/rename`, so the last line written is not the current name. Failing that, the latest non-empty `{"type":"agent-name","agentName":…}` **or** `{"type":"ai-title","aiTitle":…}` **by position**: the two agree on every observed transcript and `agent-name` is written last, so position gives today's answer, and it stays correct if a later `claude` stops emitting `agent-name` — where ranking that kind above position would pin an early `agent-name` for ever, which is this bug again. Unchanged for `PiProvider` and `CopilotProvider`, whose formats have no second name record. |
| C17 | A `None` — missing, unreadable, unparsable, or empty — leaves the label `Pending` and writes nothing. Never an error, never a wrong name. |
| C18 | A `Some(name)` is recorded through `Catalog::record_session_name`, so it is persisted and the pass does not repeat for that session (FR-007). |
| C19 | Returns how many names were recovered; `0` ⇒ no write happened. |
| C20 | A provider whose `config_dir()` is `None` contributes nothing and does not suppress the other provider's contribution. |

---

## 4. What this contract forbids

- **A second writer.** The client must not persist names; `micold-client` is unchanged by design,
  and a change to it in this feature's diff is a review failure.
- **Clearing a name.** No path may set a `Named` label back to `Pending` (data-model invariant 2).
- **Blocking I/O on the async runtime.** `drain_signals` stays lock-only (C7); the write is the
  caller's hop.
- **Per-conversation work on the live path.** The live path persists a value already in memory; it
  must not read the provider's store to confirm it.
- **Surfacing a storage failure as a session failure.** (C5, C13.)

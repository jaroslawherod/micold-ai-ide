# Phase 1 Data Model: Run a session on the Pi coding agent

**Feature**: 029-pi-cli-provider | **Date**: 2026-09-12 | **Spec**: [spec.md](./spec.md) |
**Research**: [research.md](./research.md)

The spec's Key Entities, resolved against the existing types. Almost everything here is an *existing*
entity gaining a third value rather than a new one — that is the result this feature was meant to
test, so the absence of new entities is the finding, not an omission.

---

## 1. `AiCli` — the AI CLI choice

`crates/micold-core/src/session.rs`. Persisted; the single declaration every list of CLIs derives
from (FR-021).

| Field | Change |
|---|---|
| variants | `ClaudeCode`, `Copilot`, **`Pi`** |
| `ALL` | `[AiCli; 2]` → `[AiCli; 3]`, still sorted: `[ClaudeCode, Copilot, Pi]` |
| `Default` | unchanged — `ClaudeCode`, so no existing session or setting is re-interpreted (FR-002, SC-003) |

Two name registers hang off it, neither allowed to leak into the other (FR-001a):

- `display_name()` → `"Pi Coding Agent"` — menus and sentences; also what `Display` renders.
- `command()` → `"pi"` — the sidebar row label and the terminal bar's pinned-tab text.

**Validation**: the serialized form must round-trip and must not collide with either existing value;
`crates/micold-core/tests/schema_hash.rs` and `protocol_roundtrip.rs` are the gates. The order of
`ALL` is user-visible (it builds the menus), so it is asserted, not incidental.

**No state transitions.** A session's recorded CLI never changes after creation — including when the
FR-012e switch is toggled, which must not alter the recorded CLI of any existing session.

---

## 2. `PiProvider` — the seam implementation

`crates/micold-core/src/provider.rs`, the one module allowed to name a concrete provider type. A
zero-sized struct like its two siblings, reached only through `AiCli::provider()`'s exhaustive match.
No method has a default, so all twelve are written out. Full derivations live in
[contracts/pi-cli.md](./contracts/pi-cli.md); the shape is:

| Capability | Derivation |
|---|---|
| `id` | `AiCli::Pi` |
| `display_name` / `command` | `"Pi Coding Agent"` / `"pi"` |
| `is_available` | existing `resolves_on_path("pi")` — no version floor, no probe (FR-003a) |
| `launch_args` | `["--session-id", <uuid>]` for **both** `LaunchMode` values (research R1) |
| `config_dir` | `$PI_CODING_AGENT_DIR` if set and non-empty, else `<home>/.pi/agent`; `None` ⇒ uncertain, never absent |
| `recorded_session_ids` | ids parsed from `*.jsonl` names in the per-cwd session directory |
| `has_recorded_conversation` | that directory contains a file for this id |
| `read_title` | bounded-prefix read of that file (research R5) |
| `mark_archived` / `is_archived` | empty `<session-id>.archived` marker in the same directory |
| `activity_source` | `ActivitySource::Extension { log: <base>/micold-activity/<id>.jsonl }` |

**Invariant**: every derivation is a pure function of `(config_dir, cwd, session_id)` plus
best-effort reads of Pi's own storage. Nothing is remembered; the correspondence between a session
and its conversation is derived (FR-005a).

---

## 3. `ActivitySource` — the one seam change

`crates/micold-core/src/provider.rs`. Gains a fourth variant:

```rust
pub enum ActivitySource {
    Hooks,                          // claude — daemon supplies per-session launch material
    EventLog { path: PathBuf },     // copilot — the provider writes a log we can find
    Extension { log: PathBuf },     // NEW — we supply the reporter; it writes this log
    None,
}
```

`Extension` means two things at once, and that is why it is a variant rather than a reuse of
`EventLog`:

1. **Spawn obligation** — the daemon must materialise the activity component and inject it into the
   launch (`-e <path>`, plus the environment variable that tells it where to write). `EventLog`
   carries no such obligation and must not acquire one, because Copilot needs none.
2. **Observation** — the daemon tails `log` with the existing `EventLogTail`, unchanged.

Keeping the two facts in one variant is what stops the daemon from asking *which CLI is this?* — the
conditional FR-019 and FR-020 forbid by name (Principle V).

**Relationship to the FR-012e switch**: when the switch is off, the provider still reports
`Extension { log }` — the provider is pure and does not read settings — and the *daemon* declines to
inject the component. With nothing writing the log, no event arrives and the badge stays `Unknown`,
which is exactly the FR-012d degradation, reached without a second code path.

---

## 4. Activity component record — the line format

Written by `crates/micold-daemon/assets/pi-activity.ts`, read by the daemon. Append-only JSONL, one
object per line:

```json
{"type":"turn_start","at":"2026-09-12T10:15:04.117Z"}
```

| Field | Rule |
|---|---|
| `type` | one of Pi's event names; unknown values are **ignored, not rejected** — Pi gains events between versions |
| `at` | RFC 3339 timestamp, informational; the daemon orders by arrival, not by this field |

**No conversation content is ever written** — no prompt, no response, no tool name, no file path,
no model name (FR-012b). The line above is the entire vocabulary, and that is what makes the
component reviewable in one place.

Mapping into the `ActivityEvent` vocabulary the daemon's state machine already consumes is in
[contracts/pi-cli.md](./contracts/pi-cli.md) §Activity signal. The state machine itself is unchanged.

---

## 5. Pi conversation record — owned by Pi, read best-effort

Not an entity this feature defines; it is an external format this feature *reads*, and every read of
it degrades rather than fails.

```
<base>/sessions/--<cwd with leading separator stripped, / \ : → ->--/<timestamp>_<session-id>.jsonl
```

Line 1 is the header — `{"type":"session","version":3,"id":…,"timestamp":…,"cwd":…}`. Later lines
carry `id`, `parentId`, `timestamp` and a `type`, forming a tree inside the one file.

What this feature takes from it, and nothing else:

| Read | For | Bound |
|---|---|---|
| the file's existence and name | discovery, recorded-conversation detection (FR-015) | one directory listing per location |
| the latest `session_info` name within a fixed prefix | the sidebar label (FR-011) | fixed byte budget |
| the first user message within that same prefix | the label's fallback, matching Pi's own picker | same budget |

What this feature **never** reads: `parentId` (so a forked conversation is an ordinary row and its
parentage is never surfaced — FR-015), the tree structure, message bodies beyond the first user
message, and anything at all for a session it is not labelling. Activity is **not** derived from this
file (FR-012).

**Ownership boundary**: Pi may change this format without notice. Every read is best-effort; a
missing, truncated or unparseable file contributes nothing and never fails a project open or a
session.

---

## 6. Application-owned artifacts inside Pi's storage

Two, both ours, neither read or written by `pi`:

| Artifact | Path | Shape | Lifetime |
|---|---|---|---|
| Close/remove marker (FR-016) | `<base>/sessions/--<cwd>--/<session-id>.archived` | empty file | permanent; survives loss of the application's own store |
| Activity log (FR-012) | `<base>/micold-activity/<session-id>.jsonl` | the lines of §4 | per supervised session |

The marker cannot be mistaken for a conversation: discovery filters on `.jsonl`, and Pi's own
listing does the same. The activity log is deliberately **outside** `sessions/`, so it is invisible
to both listings and cannot be misread as a conversation by either side.

Neither artifact deletes, truncates or modifies anything of Pi's. Closing a row in this application
is not permission to remove history from a store shared with the user's own `pi` (FR-016).

---

## 7. Settings — the activity-component switch

`crates/micold-core/src/settings.rs`, beside the existing default-CLI setting.

| Property | Value |
|---|---|
| Scope | **application-wide** — one switch, not per project and not per session (FR-012e) |
| Default | **on** (the badge works with no configuration) |
| Persistence | survives restart |
| Applies to | sessions started *after* it changes |
| Never alters | the recorded CLI of any existing session |
| Off ⇒ | Pi launches without the component; badge reads `Unknown`; nothing else about the session or its row differs, and it is not presented as a fault (FR-012d, FR-012f) |

**Validation**: adding it must not change how any existing setting serializes, and a settings file
written before this feature must load with the switch on.

---

## 8. `Session` and `SessionLabel` — unchanged

`Session` records which CLI backs it and resumes on that CLI; adding a third value changes nothing
about its shape. `SessionLabel` keeps its `Pending → Named(String)` lifecycle: a Pi conversation with
no name and no first user message yet stays `Pending` and shows the existing neutral placeholder,
exactly as an unsummarised Claude or Copilot conversation does.

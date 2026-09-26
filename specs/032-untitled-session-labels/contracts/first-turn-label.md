# Contract: deriving a session's label from its first turn

**Feature**: 032 · **Requirements**: FR-002, FR-003, FR-012, FR-014, FR-016 · **Research**: R4–R8, R10

This is the behaviour of `AiCliProvider::read_label` for `claude` and Copilot, the change to
`CopilotProvider::read_title`, and the precedence the daemon applies. Records are those of `claude`
2.1.2xx and Copilot 1.0.10–1.0.83 as read on 2026-09-19. Each numbered clause is a test.

## C1 — The seam

```rust
/// A label derived from the conversation's first typed turn (feature 032, FR-002), or `None`.
/// Best-effort like `read_title`: a missing, unreadable or unrecognised record yields `None`,
/// never an error (FR-011). Reads a bounded prefix only (FR-014).
fn read_label(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> Option<String>;
```

- C1.1 Required on every implementation; no default (feature 026 FR-021).
- C1.2 `PiProvider::read_label` returns `None` (its `read_title` already falls back to the first
  user message, 029-pi-cli-provider FR-011).
- C1.3 `FakeAiCliProvider::with_label(conversation, label)` mirrors `with_title`.
- C1.4 `read_label` never reads a title and `read_title` never returns a label: the daemon owns the
  precedence (C6).

## C2 — The bounded prefix (FR-014)

- C2.1 At most the first `LABEL_BUDGET_BYTES = 1 MiB` of the file are read.
- C2.2 Only lines ending in `\n` inside the prefix are parsed; a trailing partial line (half-written,
  or cut by the bound) is ignored, never an error.
- C2.3 A line that is not valid JSON is skipped.
- C2.4 A first turn that starts beyond the bound yields `None`; a later turn is never used instead
  of an unread first one.

## C3 — `claude` turns (FR-012)

File: `<config>/projects/<encoded cwd>/<session id>.jsonl` (the existing `transcript_path`).

A record is a **candidate** when all hold:
- C3.1 `type == "user"`;
- C3.2 `isMeta` is not `true`, `isCompactSummary` is not `true`, and `toolUseResult` is absent;
- C3.3 `message.content` is a string, or a list containing no `tool_result` part. Its **text** is
  the string, or the `text` parts joined with a space (image parts contribute nothing).

A candidate's text is classified by its first non-whitespace characters:
- C3.4 `<local-command-`, `<bash-`, `<task-notification>` or `<system-reminder>`: **not a turn**
  (output or text `claude` or the application inserted).
- C3.5 `<command-name>` or `<command-message>`: a **slash command**. Its name is the text inside
  `<command-name>…</command-name>` (with its leading `/`), its arguments the text inside
  `<command-args>…</command-args>` (absent or empty = none).
  - C3.5a Its **follower** is the next record in the prefix whose `type` is `user`, or `system`
    with `subtype == "local_command"`. It is **not a turn** (a command `claude` handled itself:
    `/model`, `/compact`, `/reload-plugins`, `/login`, `/usage`, `/clear`) when the follower's text
    (`message.content` for `user`, `content` for `system`) opens with `<local-command-stdout>` or
    `<local-command-stderr>`.
  - C3.5b It **is a turn** (a custom command or skill sends a prompt) when the follower is a `user`
    record with `isMeta: true` (the expanded prompt), or any other follower.
  - C3.5b′ With **no follower in the prefix yet** (the last line of a running transcript), the tag
    order decides: text opening with `<command-message>` is a turn, text opening with
    `<command-name>` is not.
  - C3.5c Its label source is the arguments when they are non-empty after C5, else the name.
  - C3.5d A `<command-…>` record with no parsable `<command-name>` is not a turn.
- C3.6 Anything else is a **prompt turn**; its label source is the text.
- C3.7 The label is the first turn whose label source is non-empty after C5 (FR-002: an empty turn
  is skipped).

Fixture cases (synthetic, shaped on the real records): the four reported openings (a bare
`/speckit-autopilot`; `/speckit-bugfix-report` with arguments), a bare `/model` followed by
`<local-command-stdout>` and then a prompt, a `/reload-plugins` followed by a
`system`/`local_command` stdout record and then a prompt, each command kind as the last line with no
follower, a `<local-command-caveat>` meta record, a
`<task-notification>`, a tool-result list, an `[Image #1] text` + image list, a compact summary, a
whitespace-only prompt followed by a real one, a truncated last line, and a first turn past 1 MiB.

## C4 — Copilot turns (FR-012)

File: `<config>/session-state/<session id>/events.jsonl` (the existing `events_path`).

- C4.1 A **turn** is a record with `type == "user.message"` whose `data.source` is absent or null
  and whose `data.isAutopilotContinuation` is not `true`.
- C4.2 Its label source is `data.content` (a string), never `data.transformedContent`.
- C4.3 The label is the first turn whose `content` is non-empty after C5.
- C4.4 No slash-command handling: Copilot records the text after a command (`/plan <text>` →
  `<text>`, `/fleet <text>` → `Fleet deployed: <text>`) and records no turn for a bare command, so the
  recorded `content` is the label as it stands.

## C5 — Shaping (FR-003)

- C5.1 Every run of Unicode whitespace, line breaks included, becomes one space; leading and
  trailing whitespace is removed.
- C5.2 If the result is empty, the turn has no label source (C3.7, C4.3).
- C5.3 If it has at most 80 extended grapheme clusters it is the label unchanged.
- C5.4 Otherwise the label is its first 79 grapheme clusters followed by `…` (U+2026): exactly 80.
- C5.5 Shaping never splits a grapheme cluster (emoji ZWJ sequences, combining marks, CJK).

## C6 — Precedence in the daemon (FR-005, FR-006, FR-008, FR-011)

- C6.1 **Discovery** (`discover_external_sessions`): `read_title` → `Named`; else `read_label` →
  `Derived`; else `Pending`.
- C6.2 **Recovery** (`recover_session_names`, `recover_live_session_names`): candidates are the
  non-archived sessions whose label is not `Named`. For each, off the lock, `read_title`; if it is
  `None` and the candidate was `Pending`, `read_label`.
- C6.3 Under the lock: a title is recorded (`Catalog::record_session_name`) unless the session is
  now `Named`; a label is recorded (`Catalog::record_session_label`) only if the session is still
  `Pending`.
- C6.3a `record_recovered_names` (and so `recover_session_names` / `recover_live_session_names`)
  returns the number of titles **plus labels** recorded. The supervisor broadcasts the catalog when
  it is > 0 (`server.rs`, unchanged), so a label derived for a running session reaches the row on the
  same tick.
- C6.3b `drain_signals` sets `name_stale` whenever it changes a session's activity (the
  `SpinnerObserved` route), as `note_activity` already does, so the first prompt re-arms the live
  lookup whichever of the spinner and the prompt hook arrives first (FR-010).
- C6.3c `note_activity` sets `name_stale` for a `UserPromptSubmit` hook **whether or not the signal
  changed**. C6.3b alone is not enough: `SpinnerObserved` can only move `Unknown → Working`, so a
  spinner drawn while the CLI starts up — before anything is typed — spends that one transition on a
  tick with an empty conversation, and the prompt hook that follows then changes nothing. The prompt
  is the event that puts the first turn on disk, so it is the one signal that must always re-arm the
  lookup. At most once per turn, so SC-006's bound is unmoved.
- C6.4 `Catalog::record_session_label(id, label) -> io::Result<bool>`: `Ok(false)` for an unknown
  id, an empty label, or a session that is not `Pending`; otherwise sets `Derived`, persists, and
  returns `Ok(true)`. A persist error leaves the in-memory label set and is logged by the caller,
  never surfaced as a session failure.
- C6.5 `Catalog::record_session_name` replaces a `Derived` label with `Named` (unchanged code; now a
  tested guarantee).
- C6.6 Nothing ever turns `Named` into `Derived` or `Pending`, or `Derived` into `Pending`.
- C6.7 A failed read (`None`) changes nothing.

## C7 — Copilot title (FR-016)

- C7.1 `CopilotProvider::read_title` returns the `name:` scalar of `workspace.yaml` when readable,
  else the `summary:` scalar when readable, else `None`.
- C7.2 `read_yaml_scalar` returns `None` for a value whose first character is `|` or `>` (a block
  scalar), for both keys.
- C7.3 An empty `name:` does not hide a readable `summary:`.

## C8 — Storage and wire (FR-007, FR-008)

- C8.1 `StoredSession.label: Option<String>` — `#[serde(default, skip_serializing_if = "Option::is_none")]`.
  `Named(t)` stores `title: t`; `Derived(l)` stores `label: l`; `Pending` stores neither.
- C8.2 On load, `title` present ⇒ `Named`; else `label` present and non-empty ⇒ `Derived`; else
  `Pending`.
- C8.3 A file written before this feature (no `label`) loads exactly as before.
- C8.4 `SessionSummary.title` carries `Derived` unchanged; `PROTOCOL_VERSION` is 14.
- C8.5 The client adopts a summary's label when it is `Named` or `Derived`; a `Pending` summary never
  clobbers a label the client already shows.

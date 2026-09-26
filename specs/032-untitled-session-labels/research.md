# Research: A session the AI CLI never titled still gets a label

**Feature**: [spec.md](./spec.md) · **Plan**: [plan.md](./plan.md) · **Date**: 2026-09-19

Every decision below was taken against the code on `origin/main` at `3c58945e` and against the
records on the development machine (87 `claude` transcripts under `~/.claude/projects/*/`, 142
Copilot sessions with an `events.jsonl` under `~/.copilot/session-state/`), read on 2026-09-19.

## R1 — Where the label is modelled: a third `SessionLabel` variant

**Decision**: `micold_core::session::SessionLabel` gains `Derived(String)` beside `Pending` and
`Named(String)`. `display()` renders it exactly as `Named` (spec *Assumptions*, D7).

**Rationale**: FR-005/FR-006/FR-008 are a *precedence* between two kinds of text, and the daemon
decides it in three places (discovery, recovery, the live terminal-title path). A variant makes
"which kind is this" a property of the value, so every match on a label has to answer it; 029 chose
`{Pending, Named}` over `Option<String>` for the same reason (029 plan, Principle V). The one
variant covers the in-memory catalog, the wire summary and the client model with no parallel flag
that could disagree.

**Alternatives considered**:
- *A `derived: bool` beside `Named`*: two fields that can contradict (`Pending` + `derived`), and every
  existing `matches!(label, Named(_))` silently treats a label as a title — the exact FR-005 bug.
- *Keep derived labels out of the catalog, compute them per snapshot*: violates FR-007 (a restart
  must show the label without reading the records again) and makes SC-006 pay a read per row per
  snapshot.

## R2 — Persisting it: a separate `label` field, not a tagged `title`

**Decision**: `StoredSession` (`crates/micold-core/src/store.rs`) gains
`#[serde(default, skip_serializing_if = "Option::is_none")] label: Option<String>`. `Named` writes
`title`, `Derived` writes `label`, `Pending` writes neither. On load a `title` wins over a `label`
(FR-005 holds even for a file that somehow carries both). No `schema_version` bump.

**Rationale**: FR-008. Additive and defaulted, by the argument `mode`, `archived` and `provider`
already carry: every file written before this feature has no `label` and loads unchanged. A
*downgrade* reads the new file correctly too: `StoredSession` does not `deny_unknown_fields`, so an
older build ignores `label` and sees `Pending`, which is exactly what it would have shown — never a
label mistaken for a title.

**Alternatives considered**:
- *Store the label in `title` with a `title_kind` tag*: an older build would read the label as a
  title, and then 029's recovery would never replace it (it skips `Named`) — FR-006 broken by a
  downgrade.
- *Bump `schema_version`*: nothing to migrate; a bump would make the new file unreadable to an
  older build for no gain.

## R3 — The wire: `SessionSummary.title` carries the variant; `PROTOCOL_VERSION` 13 → 14

**Decision**: `SessionSummary.title: SessionLabel` (`crates/micold-core/src/protocol/messages.rs`)
carries `Derived` as it carries `Named`. `PROTOCOL_VERSION` goes 13 → 14 with a doc line, and
`crates/micold-core/tests/schema_hash.rs`'s pinned `FEATURE_026_PROTOCOL_VERSION` follows it. The
client's `catalog_sync.rs` adopts a summary's label whenever it is not `Pending` (today: only
`Named`), so a `Derived` label reaches the row and a later `Named` replaces it.

**Rationale**: an older peer cannot decode the new variant, which is a wire-visible change by the
rule in `version.rs`. `SCHEMA_HASH` hashes only `messages.rs`, `grid.rs` and `envelope.rs`, and
`SessionLabel` lives in `session.rs`, so the hash does **not** move with `Derived`: the manual bump
is the only guard, and `schema_hash.rs`'s pinned constant is what makes forgetting it fail. The client needs no knowledge of precedence: the daemon has already decided
it, and the client only stops ignoring a non-`Pending` value.

**Alternatives considered**:
- *Send a derived label as `Named` on the wire, no bump*: works today, but makes the wire lie about
  what the catalog holds, and the next feature that wants the difference in the client (a tooltip, a
  style) would have to re-plumb it. It also leaves `SessionLabel` meaning two different things on the
  two sides of the socket.

## R4 — A new provider method, `read_label`, not a wider `read_title`

**Decision**: `AiCliProvider` (`crates/micold-core/src/provider.rs`) gains
`fn read_label(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> Option<String>`, required
(the trait has no defaults, feature 026 FR-021). `ClaudeProvider` and `CopilotProvider` implement the
first-turn rule (contract [first-turn-label.md](./contracts/first-turn-label.md)); `PiProvider`
returns `None` because its `read_title` already falls back to the first user message
(029-pi-cli-provider FR-011, out of scope here); `FakeAiCliProvider` gains `with_label`.

**Rationale**: the two reads have different costs and different callers. `read_title` looks for
the *latest* title and reads the whole transcript (`claude`) or one small YAML (Copilot); the label
is the *first* turn and reads a bounded prefix (FR-014). Keeping them apart keeps the precedence
(title first, label only when there is none) in one place in the daemon, and lets a test fake them
independently.

**Alternatives considered**:
- *`read_title` returns `enum { Title, Label }`*: every caller then has to split it again, and the
  title read would drag the label read along even for a session that is about to be `Named`.
- *Derive the label in the daemon from raw records*: puts `claude`'s and Copilot's record formats
  above the seam, which is what feature 026 took out.

## R5 — What a `claude` turn is (FR-012), from the records

**Decision**: the contract's rule, summarised: a `type: "user"` record that is not `isMeta`, not
`isCompactSummary`, has no `toolUseResult`, and whose content is not a `tool_result` list; its text
is the string content or the joined `text` parts; text opening with an injected wrapper
(`<local-command-…>`, `<bash-…>`, `<task-notification>`, `<system-reminder>`) is not a turn; text
opening with `<command-name>` or `<command-message>` is a slash command, and it is a turn **unless the
next `user` record opens with `<local-command-stdout>` or `<local-command-stderr>`**.

**Evidence** (all 87 transcripts, first 512 KiB each):
- Whole files, all 87 transcripts, following each command record to the next `user` or `system`
  record: 217 open with `<command-message>` and all are followed by an `isMeta` user record (the
  expanded skill or command). 103 open with `<command-name>` (`/model`, `/compact`,
  `/autocompact`) followed by a `user` `<local-command-stdout>`; 5 open with `<command-name>`
  (`/compact`, `/reload-plugins`) followed by a `type: "system", subtype: "local_command"` record
  carrying the stdout. (A first 512 KiB-only pass missed the `system` shape; the plan review caught
  it.) Tag order and follower agree on every record.
- 29 `<task-notification>` records and 4 `<local-command-stdout>` records sit among typed-looking
  user records; neither is typed.
- `origin: {"kind": "human"}` marks typed records only from 2.1.2xx on (87 of 465 user records), so
  it cannot be the rule for older transcripts.

**Rationale**: the follower is the meaning — `claude` answered the command itself (local stdout,
in a `user` or a `system/local_command` record) or sent a prompt (an `isMeta` expansion). It is
decided by the follower whenever one is in the prefix; the tag order, which agrees on every observed
record but is a serialisation detail, only breaks the tie for a command that is the last line of a
running transcript (contract C3.5a–C3.5b′). The follower's *type* is enough to classify it, so a
half-written 110 KB expansion still counts as a follower once its line is complete, and a
still-incomplete one falls back to tag order.

**Alternatives considered**:
- *Tag order (`<command-message>` first ⇒ prompt-sending)*: agrees on every record today, but it is
  a serialisation accident, not a meaning.
- *A list of CLI-handled command names*: grows with every `claude` release.

## R6 — What a Copilot turn is (FR-012)

**Decision**: the first `events.jsonl` record with `type: "user.message"` whose `data.source` is
absent, whose `data.isAutopilotContinuation` is not true, and whose `data.content` is non-empty after
FR-003; the label is `data.content`, never `data.transformedContent`.

**Evidence**: spec *Copilot evidence* (972 `user.message` records; 12 with a `source`, 28 autopilot
continuations with empty `content`). The first qualifying record is at record 2–10 and ends within
the first 125,751 bytes in all 141 sessions that have one.

**Alternatives considered**: *`transformedContent`* — wrapped in timestamps and reminders;
*`session.start`'s metadata* — carries no prompt; *Copilot's `session-store.db`* — a database read
for one string, rejected for titles already in 026 research R3.

## R7 — The bound (FR-014): the first 1 MiB of the record file

**Decision**: a label is read from at most the first **1 MiB** of the transcript / `events.jsonl`;
a last line not ended by `\n` inside that prefix is dropped (half-written, or cut by the bound).

**Evidence**: the first turn (plus the record after it, R5) ends by byte 380,149 in the worst of the
87 `claude` transcripts (a first prompt carrying a pasted image, inlined as base64) and by record 11;
the four reported sessions need 18–28 KB; Copilot needs ≤125,751 bytes. 1 MiB is 2.7× the worst case.

**Cost** (SC-006): once a label is found it is remembered and never re-derived, and only a session
with no title is read. 50 such sessions cost ≤50 MiB once. A session that stays `Pending` (no usable
turn in the prefix: never used, only notifications, a first turn past the bound) re-reads its prefix
on every project open and every `name_stale`, exactly as 029 already re-reads its whole transcript
for a title; remembering a negative result is not worth a second persisted state for the handful of
such sessions. Label writes are one state-file write per session, once, as 029's name writes are;
batching them is left as it is in 029. For
comparison, `ClaudeProvider::read_title` already reads *whole* transcripts (4 MB for `9a536c7e`) for
every untitled session on every project open, today; that cost is unchanged and bounds this one.

**Alternatives considered**:
- *64 KiB, as `pi` uses*: misses 2 of 87 `claude` first turns (images).
- *A record-count bound*: one record can be 380 KB, so a count bounds nothing.

## R8 — Shaping (FR-003): `unicode-segmentation`, 80 graphemes, `…`

**Decision**: collapse every whitespace run (including line breaks) to one space, trim, and if more
than 80 extended grapheme clusters remain keep the first 79 and append `…` (U+2026), so the result is
at most 80 user-perceived characters. Graphemes come from `unicode-segmentation`, added as a direct
dependency of `micold-core`.

**Rationale**: FR-003 says *user-perceived characters*, which is Unicode's grapheme cluster; a
`chars()` cut can split an emoji ZWJ sequence or a base + combining mark. The crate is already in
`Cargo.lock` (iced depends on it), is maintained by the `unicode-rs` org, and is MIT/Apache-2.0 —
vetted per the constitution's dependency rule at zero new download. It is new to the **daemon**
and the sandbox image (which do not build iced): a small pure-Rust crate with no dependencies of its
own. Added under `[workspace.dependencies]` and taken by `micold-core` with `{ workspace = true }`.

**Alternatives considered**: *`chars().take(80)`* — splits clusters; *hand-rolled segmentation* —
the Unicode tables are the whole difficulty.

## R9 — Where the daemon decides precedence

**Decision** (`crates/micold-daemon/src/state.rs`, `catalog.rs`):
- `discover_external_sessions`: `read_title` → `Named`; else `read_label` → `Derived`; else `Pending`.
- `recover_session_names` / `recover_live_session_names`: candidates are every non-archived session
  that is **not `Named`** (was: `Pending`), so a `Derived` session still looks for a title (FR-006,
  US2 #3). For each: `read_title`; if none **and the session is `Pending`**, `read_label`.
- Applied under the lock, re-checked: a title is recorded unless the session became `Named`
  meanwhile (029's rule); a label is recorded only while the session is still `Pending`
  (`Catalog::record_session_label`, new), so a label never replaces a title (FR-005) and a title and
  label racing for one session end on the title.
- The live terminal-title path (`drain_signals` → `record_observed_names` →
  `Catalog::record_session_name`) is unchanged: it already replaces any non-matching label with
  `Named`, which is FR-006 for a running session.
- `prunable_session_cwds` stays `Pending`-only: a `Derived` session has a conversation by
  definition, and one whose records were deleted keeps its label (spec edge case, 029 FR-008).

**Running sessions (US3, FR-010)**: `name_stale` is set at spawn and on every activity change
`note_activity` sees, and the supervisor tick (`SUPERVISION_INTERVAL`, 250 ms) runs
`recover_live_session_names`. Two gaps close it (plan review F3, F4):
- `drain_signals` moves a session to `Working` on a spinner glyph **without** setting `name_stale`;
  when the spinner is drained before the `UserPromptSubmit` hook arrives, the hook then changes
  nothing and the next re-arm is the turn's end. `drain_signals` now sets `name_stale` on every
  activity change it makes (C6.3b). The user record is on disk ~200 ms before the hook fires, so
  reading on either signal finds it.
- The supervisor broadcasts only when the recovery count is > 0, and today that counts titles only;
  labels now count too (C6.3a).
A third gap closes it, found by M3's review A: `SpinnerObserved` moves the FSM only from `Unknown`
to `Working`, so a session whose CLI drew a spinner while starting up has already spent that one
transition before the user typed. The prompt hook then changes no signal and C6.3b's re-arm never
fires, leaving the turn unread until the closing `Stop`. So `note_activity` re-arms on
`UserPromptSubmit` unconditionally (C6.3c) — the prompt is the event that wrote the turn, and it
fires at most once per turn.

So the label is derived and broadcast within a tick or two of the prompt, inside FR-010's 60 s.

**Cost of widening recovery to `Derived`**: the set of sessions re-read for a title is the same set
029 re-reads today (the untitled ones, then `Pending`, now `Derived`), so SC-006 is not moved.

## R10 — Copilot `summary:` (FR-016, D9)

**Decision**: `CopilotProvider::read_title` returns `name:`, else `summary:`, via the existing
`read_yaml_scalar`. `read_yaml_scalar` learns to reject a block-scalar indicator (a value starting
with `|` or `>`), returning `None` rather than the literal `|-`.

**Evidence**: 44 no-`name:` sessions, all with a one-line `summary:`; the two `summary: |-` values on
the machine sit beside a `name:`. Without the guard, a block `summary:` alone would be read as the
title `|-`.

**Alternatives considered**: *a YAML crate to read block scalars too* — rejected in 026 research R4
for one key, and block values are multi-line prompts, not titles; *`summary:` as a derived label*
rather than a title — the user decided it is Copilot's title (D9).

## R11 — Tests at each layer

| Requirement | Layer | Where |
|---|---|---|
| FR-002, FR-003, FR-012, FR-014 (rule, shaping, bound) | core unit over fixture records | `crates/micold-core/tests/first_turn_label.rs`, fixtures in `crates/micold-core/tests/fixtures/first_turn/` |
| FR-016 (`summary:`) | core, real provider over a temp `COPILOT_HOME`-shaped dir | `crates/micold-core/tests/copilot_provider.rs` |
| FR-007, FR-008 (persisted, distinct) | core store round-trip | `crates/micold-core/tests/session_name_round_trip.rs` |
| FR-001, FR-004, FR-005, FR-006, FR-009, FR-011 | daemon integration, real `Catalog`, real `ClaudeProvider`/`CopilotProvider` under a temp `CLAUDE_CONFIG_DIR`/`COPILOT_HOME` (the daemon has no provider injection; env serialised as in `session_name_recovery.rs`) | `crates/micold-daemon/tests/untitled_session_labels.rs` |
| FR-010 (running session) | daemon integration, `recover_live_session_names` after `note_activity` | same file |
| Client adoption of `Derived` | client render-free reducer | `crates/micold-client/tests/session_title_sync.rs` |
| Wire bump | core | `crates/micold-core/tests/schema_hash.rs` |
| SC-008, SC-009 (listed Copilot sessions, D10) | core provider + daemon integration over fixtures shaped on the 44 old sessions | `copilot_provider.rs`, `untitled_session_labels.rs` |
| SC-001, SC-005 (real stores) | ignored corpus probe + quickstart §B | `crates/micold-core/tests/first_turn_label_corpus.rs` (`#[ignore]`), [quickstart.md](./quickstart.md) |
| FR-013 (not user-editable) | daemon: no message sets a label; the only writers are C6 | `untitled_session_labels.rs` asserts `Derived` changes only to `Named` through the recovery and title paths; no new `ClientMsg` (protocol diff) |
| FR-015 | docs | `docs/user-guide/worktrees-and-sessions.md` |

No geometry gate or visual pass is needed: the row renders `SessionLabel::display()` as before, no
widget changes (Principle VIII not engaged); quickstart §B checks the text on a real row.

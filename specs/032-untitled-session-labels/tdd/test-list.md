---
feature: 032-untitled-session-labels
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 11
planned_at: fa19e553
updated_at: fa19e553
suite_baseline: green # 3333 passed, 0 failed, 6 ignored, 327 binaries at fa19e553
---

# Test List: A session the AI CLI never titled still gets a label

Derived from `spec.md` (US1–US3 acceptance scenarios, FR-001–FR-016, SC-001–SC-009) and `plan.md`
with `contracts/first-turn-label.md` (C1–C8). It was not derived from the code.

**Trace ids.** `US1.3` is User Story 1, acceptance scenario 3. `FR-`/`SC-` refer to `spec.md`;
`C3.5a` etc. to the contract.

**Milestones.** Behaviors are grouped by component. `tasks.md` carries each id on the tasks that
write and implement it; its `## Milestones` section says which PR ships them.

## Outer loop: acceptance behaviors

**Entry point.** The row text a client shows is `SessionSummary.title` from the daemon's catalog
snapshot. There is no end-to-end GUI runner (the profile's acceptance runner is the sandbox
real-runtime suite, which does not drive sidebars), so these are **integration tests of the composed
daemon**: a real `DaemonState` + `Catalog` over a temp data dir, calling the same passes the server
calls on project open (`discover_external_sessions`, `recover_session_names`) and on the supervisor
tick (`drain_signals`, `recover_live_session_names`), with providers reading real temp record files
(`ClaudeProvider` / `CopilotProvider` under a temp `CLAUDE_CONFIG_DIR` / `COPILOT_HOME`; the daemon has
no provider injection, so the fake provider appears only in core seam tests). The env variables are
process-global: the daemon tests hold a file-local `static ENV: Mutex<()>` while set. A "restart" is a fresh `DaemonState` over the same data dir. The row's visible
text is covered by quickstart §B2.

`guard` rows are characterization tests: the behaviour already holds once the step before them lands
(e.g. `set_title` already replaces any label), so they may pass on arrival. Record them as guards in
the cycle log; a red is not required.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1  | A known untitled `claude` session whose transcript holds a typed prompt reads that first turn after project open, without being opened | US1.1, FR-001, FR-002, SC-002 | example | PENDING | |
| A2  | Several untitled `claude` sessions in one project each read their own first turn, all different | US1.2, SC-004, SC-005 | example | PENDING | |
| A3  | After a restart over the same data dir, the label is in the first snapshot, with the provider's records removed | US1.3, FR-007, FR-009 | example | PENDING | |
| A4  | A `claude` session with no typed prompt (only injected records) still reads "New session" | US1.4, FR-004, SC-004 | example | PENDING | |
| A5  | A listed Copilot session with a typed turn and neither `name:` nor `summary:` reads its first `user.message` content | US1.5, FR-012, SC-009 | example | PENDING | |
| A6  | A listed Copilot session with `summary:` and no `name:` reads the summary as its title, never a label | US1.6, FR-016, SC-008 | example | PENDING | |
| A7  | A titled session reads its title, running or not, and never a derived label | US2.1, FR-005, SC-003 | example | PENDING | |
| A8  | A session showing a label switches to the title the terminal reports, and still reads the title after a restart | US2.2, FR-006 | example | PENDING | |
| A9  | A stopped session showing a label reads the title its records gained, after the next project open | US2.3, FR-006 | example | PENDING | |
| A10 | A running untitled session reads its label within one supervisor pass of its first prompt, whether the prompt hook or the spinner arrives first, and the snapshot is broadcast | US3.1, FR-010, SC-007 | example | PENDING | |
| A11 | A running session showing a label switches to the title as the terminal reports it | US3.2, FR-006 | example | PENDING | |

## Inner loop: unit behaviors

### `crates/micold-core/src/session.rs` — `SessionLabel`, `Session`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1  | `Derived(text)` displays as `text`, like `Named` | FR-001, D7 | example | PENDING | |
| U2  | `set_derived_label` on a `Pending` session sets `Derived` and reports a change | FR-001 | example | PENDING | |
| U3  | `set_derived_label` on a `Named` session changes nothing and reports no change | FR-005 | example | PENDING | |
| U4  | `set_derived_label` on a `Derived` session changes nothing (a label is derived once) | FR-007 | example | PENDING | |
| U5  | `set_derived_label("")` changes nothing | FR-004 | example | PENDING | |
| U6  | `set_title` on a `Derived` session sets `Named` | FR-006 | example | PENDING | |

### `crates/micold-core/src/store.rs` — `StoredSession`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U7  | A `Derived` session saves `label` and no `title`, and loads back `Derived` | FR-007, FR-008, C8.1 | example | PENDING | |
| U8  | A record with both `title` and `label` loads `Named` | FR-005, C8.2 | example | PENDING | |
| U9  | A record with an empty `label` loads `Pending` | FR-004, C8.2 | example | PENDING | |
| U10 | A pre-032 record (no `label` key) loads as before; `Named`/`Pending` write no `label` key | FR-009, C8.3 | example | PENDING | |

### `crates/micold-core/src/protocol/version.rs` — wire

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U11 | `PROTOCOL_VERSION` is 14 (pinned) | C8.4 | example | PENDING | |
| U12 | A `SessionSummary` with a `Derived` title round-trips through the control codec | C8.4 | guard | PENDING | |

### `crates/micold-client/src/catalog_sync.rs` — client adoption

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U13 | A `Derived` summary title is adopted onto a `Pending` client session | FR-001, C8.5 | example | PENDING | |
| U14 | A `Named` summary replaces a `Derived` client label | FR-006, C8.5 | example | PENDING | |
| U15 | A `Pending` summary never clobbers a `Derived` client label | FR-007, C8.5 | example | PENDING | |

### `crates/micold-core/src/first_turn.rs` — prefix and shaping

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U16 | Whitespace and line-break runs collapse to one space; ends trimmed | FR-003, C5.1 | example | PENDING | |
| U17 | Whitespace-only text yields no label | FR-002, C5.2 | example | PENDING | |
| U18 | Exactly 80 graphemes pass unchanged | FR-003, C5.3 | example | PENDING | |
| U19 | 81 graphemes yield the first 79 plus `…`, 80 in total | FR-003, C5.4 | example | PENDING | |
| U20 | The cut never splits a ZWJ emoji, a base + combining mark, or CJK | FR-003, C5.5 | example | PENDING | |
| U21 | `read_prefix` returns at most 1 MiB (1,048,576 bytes; a file of 1 MiB + 1 is cut) | FR-014, C2.1 | example | PENDING | |
| U22 | `read_prefix` drops a trailing line without `\n` | FR-011, C2.2 | example | PENDING | |
| U23 | `read_prefix` of a missing file is `None` | FR-011 | example | PENDING | |

### `crates/micold-core/src/first_turn.rs` — `claude_first_turn`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U24 | A bare prompt-sending command (`/speckit-autopilot` + `isMeta` follower) yields its name with `/` | FR-002, C3.5b, C3.5c | example | PENDING | |
| U25 | A command with arguments yields the arguments | FR-002, C3.5c | example | PENDING | |
| U26 | `/model` followed by a user `<local-command-stdout>` is skipped; the next prompt is the label | FR-012, C3.5a | example | PENDING | |
| U27 | `/reload-plugins` followed by a `system`/`local_command` stdout record is skipped | FR-012, C3.5a | example | PENDING | |
| U28 | A last-line `<command-message>` command with no follower is a turn | FR-010, C3.5b′ | example | PENDING | |
| U29 | A last-line `<command-name>` command with no follower is not a turn | FR-012, C3.5b′ | example | PENDING | |
| U30 | `isMeta`, compact-summary, `toolUseResult` and tool-result-list records are never turns | FR-012, C3.2, C3.3 | example | PENDING | |
| U31 | `<local-command-caveat>`, `<bash-…>`, `<task-notification>`, `<system-reminder>` texts are never turns | FR-012, C3.4 | example | PENDING | |
| U32 | A list content's `text` parts form the label; image parts contribute nothing | FR-002, C3.3 | example | PENDING | |
| U33 | A whitespace-only prompt is skipped and the next turn is the label | FR-002, C3.7 | example | PENDING | |
| U34 | A non-JSON line is skipped without failing | FR-011, C2.3 | example | PENDING | |
| U35 | A `<command-…>` record with no parsable `<command-name>` is not a turn | FR-011, C3.5d | example | PENDING | |
| U36 | Records with only injected text yield no label | FR-004 | example | PENDING | |

### `crates/micold-core/src/first_turn.rs` — `copilot_first_turn`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U37 | The first `user.message` with no `source` yields its `content` | FR-012, C4.1, C4.3 | example | PENDING | |
| U38 | A `user.message` with a `source` (skill context, `instruction-discovery`) is skipped | FR-012, C4.1 | example | PENDING | |
| U39 | An autopilot continuation is skipped | FR-012, C4.1 | example | PENDING | |
| U40 | `content` is used, never `transformedContent` | FR-012, C4.2 | example | PENDING | |
| U41 | A whitespace-only `content` is skipped; the next turn is used | FR-002, C4.3 | example | PENDING | |
| U42 | `Fleet deployed: <text>` is kept verbatim | FR-012, C4.4 | example | PENDING | |
| U43 | A first turn at record 10 is found | FR-014, SC-009 | example | PENDING | |

### `crates/micold-core/src/provider.rs` — the seam

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U44 | The fake's `with_label` answers `read_label`; its `read_title` stays `None` | C1.3, C1.4 | example | PENDING | |
| U45 | `PiProvider::read_label` is `None` even when its `read_title` is the first message | C1.2 | example | PENDING | |
| U46 | `ClaudeProvider::read_label` returns the shaped first turn of the transcript at its derived path | FR-002, FR-012 | example | PENDING | |
| U47 | `ClaudeProvider::read_label` is `None` for a missing transcript | FR-011 | example | PENDING | |
| U48 | A `claude` first turn starting past 1 MiB yields `None` even when a later prompt exists | FR-014, C2.4 | example | PENDING | |
| U49 | `ClaudeProvider::read_title` still returns the latest `ai-title` for a file `read_label` reads | FR-005, C1.4 | example | PENDING | |
| U50 | `CopilotProvider::read_title` returns `name:` when present, even with a `summary:` | FR-016, C7.1 | example | PENDING | |
| U51 | `CopilotProvider::read_title` returns `summary:` when `name:` is absent | FR-016, C7.1 | example | PENDING | |
| U52 | `CopilotProvider::read_title` returns `summary:` when `name:` is empty | FR-016, C7.3 | example | PENDING | |
| U53 | A block `summary: |-` alone yields `None` | FR-016, C7.2 | example | PENDING | |
| U54 | `CopilotProvider::read_label` returns the first turn of `events.jsonl`; `None` when missing | FR-012, FR-011 | example | PENDING | |
| U55 | A Copilot first turn starting past 1 MiB yields `None` | FR-014, C2.4 | example | PENDING | |

### `crates/micold-daemon/src/catalog.rs` — `record_session_label`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U56 | Sets `Derived` on a `Pending` session, persists, and a reloaded catalog shows it | FR-007, C6.4 | example | PENDING | |
| U57 | Returns `Ok(false)` for an unknown id, an empty label, a `Named` and a `Derived` session | FR-005, C6.4 | example | PENDING | |
| U58 | A persist failure keeps the in-memory label and returns `Err` | FR-011, C6.4 | example | PENDING | |
| U59 | Labelling session A never changes session B in the same worktree | FR-002, Principle II | example | PENDING | |
| U60 | `record_session_name` replaces a `Derived` label with `Named` and persists | FR-006, C6.5 | guard | PENDING | |

### `crates/micold-daemon/src/state.rs` — precedence and passes

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U61 | Discovery adopts a titled session `Named`, an untitled one with a label `Derived`, one with neither `Pending` | FR-001, FR-005, C6.1 | example | PENDING | |
| U62 | `recover_session_names` turns `Pending` into `Derived` and counts it | FR-001, C6.2, C6.3a | example | PENDING | |
| U63 | `recover_session_names` turns `Derived` into `Named` when a title appears | FR-006, C6.2 | example | PENDING | |
| U64 | A `Derived` session is never re-labelled from its records (label read only for `Pending`) | FR-007, C6.2 | guard | PENDING | |
| U65 | A title arriving between the off-lock read and the label write wins; a label arriving after a title is dropped | FR-005, C6.3, C6.6 | guard | PENDING | |
| U66 | A `None` read and a failed label write change nothing and report no session failure | FR-011, C6.7 | example | PENDING | |
| U67 | A `Derived` session is not pruned; a `Pending` session with no conversation is pruned as before | FR-004, data-model inv. 4 | example | PENDING | |
| U68 | `recover_live_session_names` records a label after `note_activity` and returns > 0 | FR-010, C6.3a | example | PENDING | |
| U69 | A spinner-driven activity change in `drain_signals` sets `name_stale` | FR-010, C6.3b | example | PENDING | |
| U70 | A drain with no activity change leaves `name_stale` unset (idle tick reads nothing) | SC-006 | example | PENDING | |
| U71 | Nothing but the recovery and title paths writes a label (no client message sets one) | FR-013 | guard | PENDING | |

## Invariants and edge cases still to place

None. Two-window consistency (spec *Concurrency*) needs no own behavior: every client adopts the
same daemon snapshot (U13–U15 plus A1).

## Out of scope

- Discovering Copilot sessions missing from Copilot's index (D10; ledger follow-up).
- `pi` labelling (029-pi-cli-provider FR-011): only U45 pins that it is untouched.
- `custom-title` / `agent-name` records (spec *Out of scope*).
- Label styling or disambiguation (D6, D7): nothing to test; no widget change.
- SC-001, SC-005, SC-006 on real data: quickstart §B (corpus probe, sidebar, list timing), not a
  test; SC-006 has no automated timing test because the added cost is one bounded read per
  untitled session (research R7).

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact` (assert on the
  `N passed` count, not the exit code)
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`)
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets` (`mise run test-core`)
- Coverage: none (no `cargo-llvm-cov`)
- Mutation: none (no `cargo-mutants`)

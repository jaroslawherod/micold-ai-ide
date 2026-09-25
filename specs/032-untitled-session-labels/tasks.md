---

description: "Task list for feature 032 — untitled session labels"
---

# Tasks: A session the AI CLI never titled still gets a label

**Input**: Design documents from `/specs/032-untitled-session-labels/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/first-turn-label.md](./contracts/first-turn-label.md),
[quickstart.md](./quickstart.md)

**Tests**: Mandatory (Constitution Principle I). Every test task is written and seen failing for
the stated reason before the implementation task after it. Clause IDs (C1–C8) are the contract's.

**Documentation**: `docs/user-guide/worktrees-and-sessions.md` is updated in the milestone that
ships each user-facing behaviour (Principle VII, FR-015).

**Cross-platform**: No `cfg(target_os)` arm anywhere (plan, Principle VI); tests use temp
directories and `\n`-terminated fixture records, so they run unchanged on all three OSes.

**Organization**: phases follow delivery order (priority first): the `claude` half of US1
(scenarios 1–4) with US2, then the Copilot half of US1 (scenarios 5–6), then US3 (both providers),
then Polish. Task IDs keep their first-draft numbers, so the US3 IDs (T028–T030) sit after Copilot.

**Daemon tests**: the daemon has no provider injection (`DaemonState` builds its providers from the
environment), so every daemon test uses the real `ClaudeProvider` / `CopilotProvider` over a temp
`CLAUDE_CONFIG_DIR` / `COPILOT_HOME`, as `crates/micold-daemon/tests/session_name_recovery.rs` does.
Those variables are process-global: `untitled_session_labels.rs` holds a file-local
`static ENV: Mutex<()>` for the whole of each test that sets them. The fake provider is used only
in the core seam tests (T013).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1, US2, US3 from spec.md

## Path Conventions

Cargo workspace: `crates/micold-core/`, `crates/micold-daemon/`, `crates/micold-client/`, each with
`src/` and `tests/`. Iterate with `mise run test-core`; run `mise run gate` before a push.

---

## Phase 1: Setup

**Purpose**: the one new dependency and the fixture directory.

- [X] T001 Add `unicode-segmentation = "1.13"` to `[workspace.dependencies]` in `Cargo.toml` (under the render-free core deps) and `unicode-segmentation = { workspace = true }` to `[dependencies]` in `crates/micold-core/Cargo.toml` (research R8); confirm `Cargo.lock` gains no new package version
- [X] T002 [P] Create `crates/micold-core/tests/fixtures/first_turn/claude/` and `crates/micold-core/tests/fixtures/first_turn/copilot/` with a `README.md` stating the fixtures are synthetic records shaped on `claude` 2.1.2xx / Copilot 1.0.10–1.0.83 (contract C3, C4), not copies of real conversations

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: the third label kind end to end (memory, disk, wire, client), the shaping and prefix
helpers, the seam method, and the catalog write. Nothing here changes a row by itself:
`read_label` answers `None` for every provider until Phase 3.

- [X] T003 [P] [U1] [U2] [U3] [U4] [U5] [U6] Write failing tests in `crates/micold-core/tests/session_label_kinds.rs` (new): `SessionLabel::Derived("x").display() == "x"`; `Session::set_derived_label` sets `Derived` only from `Pending` and returns `true`, returns `false` and changes nothing on a `Named` or `Derived` session or an empty string; `Session::set_title` replaces a `Derived` label with `Named` (data-model *Transitions*)
- [X] T004 [U1] [U2] [U3] [U4] [U5] [U6] Add `SessionLabel::Derived(String)` and `Session::set_derived_label(&mut self, label: impl Into<String>) -> bool` in `crates/micold-core/src/session.rs`; `display()` renders `Derived` as `Named`; update the enum's doc comment (no longer "`claude`-provided only")
- [X] T005 [P] [U7] [U8] [U9] [U10] Write failing tests in `crates/micold-core/tests/session_name_round_trip.rs`: a `Derived` session saves `label` and no `title` and loads back `Derived` (C8.1); a record with both `title` and `label` loads `Named` (C8.2); a record with an empty `label` loads `Pending`; a pre-032 record (no `label` key) loads exactly as before (C8.3); a `Named` or `Pending` session writes no `label` key
- [X] T006 [U7] [U8] [U9] [U10] Add `label: Option<String>` to `StoredSession` in `crates/micold-core/src/store.rs` with `#[serde(default, skip_serializing_if = "Option::is_none")]`; map `Named(t)` → `title`, `Derived(l)` → `label`, `Pending` → neither in `from_session`, and "`title` ⇒ `Named`; else non-empty `label` ⇒ `Derived`; else `Pending`" in `into_session`; no `schema_version` bump (research R2)
- [X] T007 [P] [U11] In `crates/micold-core/tests/schema_hash.rs`, move the pinned `FEATURE_026_PROTOCOL_VERSION` to 14 with a doc line citing 032 `SessionLabel::Derived`, and note that `SCHEMA_HASH` does not cover `session.rs` (research R3); see it fail
- [X] T008 [U11] [U12] Bump `PROTOCOL_VERSION` 13 → 14 in `crates/micold-core/src/protocol/version.rs` with a "Bumped 13 → 14 for feature 032's `SessionLabel::Derived` on `SessionSummary.title`" doc line; add a round-trip case for a `Derived` summary title in `crates/micold-core/tests/protocol_roundtrip.rs`
- [X] T009 [P] [U13] [U14] [U15] Write failing tests in `crates/micold-client/tests/session_title_sync.rs`: a summary carrying `Derived` is adopted onto a `Pending` client session; a later `Named` summary replaces it; a `Pending` summary never clobbers a `Derived` or `Named` label already shown (C8.5)
- [X] T010 [U13] [U14] [U15] Adopt `summary.title` when it is `Named` or `Derived` (was `Named` only) in `crates/micold-client/src/catalog_sync.rs`, keeping the "never clobber with `Pending`" comment true
- [X] T011 [P] [U16] [U17] [U18] [U19] [U20] [U21] [U22] [U23] Write failing tests in `crates/micold-core/tests/first_turn_label.rs` (new) for `shape_label` (C5): whitespace and line-break runs collapse to one space and ends are trimmed; whitespace-only yields `None`; exactly 80 graphemes pass unchanged; 81 yield the first 79 plus `…` (80 total); a ZWJ emoji family, a base + combining mark and CJK text are never split at the cut; and for `read_prefix` (C2): reads at most `LABEL_BUDGET_BYTES` (1 MiB), drops a trailing line with no `\n`, a missing file yields `None`
- [X] T012 [U16] [U17] [U18] [U19] [U20] [U21] [U22] [U23] Create `crates/micold-core/src/first_turn.rs` (register it in `crates/micold-core/src/lib.rs`) with `pub const LABEL_BUDGET_BYTES: u64 = 1024 * 1024`, `read_prefix(&Path) -> Option<Vec<u8>>` (complete lines only) and `shape_label(&str) -> Option<String>` using `unicode_segmentation::UnicodeSegmentation::graphemes(.., true)`
- [X] T013 [P] [U44] [U45] Write failing tests in `crates/micold-core/tests/ai_cli_provider_seam.rs`: `FakeAiCliProvider::with_label(conversation, label)` makes `read_label` answer it and `read_title` stay `None` (C1.3, C1.4); `PiProvider.read_label` is `None` for a Pi conversation whose `read_title` is its first message (C1.2); `CopilotProvider.read_label` is `None` (until Phase 5; T036 deletes this assertion)
- [X] T014 [U44] [U45] Add the required `fn read_label(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> Option<String>` to `AiCliProvider` in `crates/micold-core/src/provider.rs` with the C1 doc comment; implement it on `PiProvider` and `CopilotProvider` as `None` (Copilot's comment names Phase 5/M2), on `ClaudeProvider` as `None` for now, on `FakeAiCliProvider` with a `labels` map and `with_label` builder, and on the test-only `MinimalProvider` in `crates/micold-core/tests/ai_cli_provider_seam.rs` as `None` (the trait has no default)
- [X] T015 [P] [U56] [U57] [U58] [U59] [U60] Write failing tests in `crates/micold-daemon/tests/untitled_session_labels.rs` (new; real `Catalog` over a temp data dir, as `session_name_recovery.rs` does; the `ENV` mutex and store helpers live here for T020–T034) for `Catalog::record_session_label` (C6.4): sets `Derived` on a `Pending` session and persists it (a reloaded catalog shows it); `Ok(false)` for an unknown id, an empty label, a `Named` session and a `Derived` session; a persist failure (a data dir path whose parent is a regular file, `tmp/file/data`, so the write fails on every OS) leaves the in-memory label set and returns `Err`; writing session A's label never touches session B in the same worktree (Principle II)
- [X] T016 [U56] [U57] [U58] [U59] [U60] Add `Catalog::record_session_label(&mut self, id: SessionId, label: &str) -> io::Result<bool>` in `crates/micold-daemon/src/catalog.rs` through `Workspace::find_session_mut` and `Session::set_derived_label`, persisting only on change

**Checkpoint**: `mise run test-core` and the daemon/client tests above are green; no row changes yet.

---

## Phase 3: User Story 1 — Tell my past untitled `claude` sessions apart (Priority: P1) 🎯 MVP

**Goal**: an untitled `claude` session with a typed prompt shows its first turn as its label, after
a restart, without being opened (US1 scenarios 1–4).

**Independent Test**: a `claude` transcript with typed prompts and no `ai-title`; restart the
service; the row reads the first turn (quickstart B2).

### Tests for User Story 1 ⚠️ (mandatory; each seen failing first)

Outside-in: T020's acceptance tests (A1–A4) are written first and stay red until T023; the unit tests T017–T019 drive the core underneath them.

- [X] T017 [P] [US1] Write the `claude` fixtures in `crates/micold-core/tests/fixtures/first_turn/claude/` (contract C3 *Fixture cases*): `bare_skill.jsonl` (bare `/speckit-autopilot` + `isMeta` expansion), `skill_with_args.jsonl` (`/speckit-bugfix-report` with arguments), `model_then_prompt.jsonl` (`<command-name>/model` + user `<local-command-stdout>` + prompt), `reload_plugins_system.jsonl` (`/reload-plugins` + `type:"system", subtype:"local_command"` stdout + prompt), `last_line_command_message.jsonl` and `last_line_command_name.jsonl` (no follower), `injected_only.jsonl` (`<local-command-caveat>` meta, `<task-notification>`, tool-result list, compact summary, no typed prompt), `image_prompt.jsonl` (`[Image #1] text` + image part), `whitespace_then_prompt.jsonl`, `truncated_last_line.jsonl`, and a test helper that builds a first turn beyond 1 MiB at run time (no multi-MB file in the repo)
- [X] T018 [US1] [U24] [U25] [U26] [U27] [U28] [U29] [U30] [U31] [U32] [U33] [U34] [U35] [U36] Write failing tests in `crates/micold-core/tests/first_turn_label.rs` for `claude_first_turn(&[u8])` over each T017 fixture, one test per clause C3.1–C3.7 (e.g. `bare_skill` → `/speckit-autopilot`, `skill_with_args` → the arguments, `model_then_prompt` and `reload_plugins_system` → the prompt, `last_line_command_message` → the name, `last_line_command_name` → `None`, `injected_only` → `None`, `image_prompt` → `[Image #1] text`, a non-JSON line skipped)
- [X] T019 [US1] [U46] [U47] [U48] [U49] Write failing tests in `crates/micold-core/tests/ai_cli_provider.rs` for `ClaudeProvider::read_label` over a temp `<config>/projects/<encoded cwd>/<id>.jsonl`: returns the shaped first turn; `None` for a missing file; `None` when the first turn starts past 1 MiB even if a later prompt exists (C2.4); `read_title` still returns the latest `ai-title` for the same file (C1.4)
- [X] T020 [US1] [A1] [A2] [A3] [A4] [U61] [U62] [U66] [U67] Write failing tests in `crates/micold-daemon/tests/untitled_session_labels.rs` (real `ClaudeProvider` under a temp `CLAUDE_CONFIG_DIR`, transcripts written with the T017 fixture shapes, `ENV` mutex held): discovery adopts an untitled session with a label as `Derived` and a titled one as `Named` (C6.1); `recover_session_names` turns a known `Pending` session into `Derived`, persisted, and returns 1 (C6.2, C6.3a); a session with no typed prompt stays `Pending` and is still pruned as before (FR-004, data-model invariant 4); two untitled sessions in one project each get their own label (US1 #2, SC-004); after a catalog reload the label is there with no provider read (US1 #3, FR-007); a `None` read, and a label persist failure (the `tmp/file/data` data dir of T015), change nothing else and report no session failure (FR-011)

### Implementation for User Story 1

- [X] T021 [US1] [U24] [U25] [U26] [U27] [U28] [U29] [U30] [U31] [U32] [U33] [U34] [U35] [U36] Implement `claude_first_turn(prefix: &[u8]) -> Option<String>` in `crates/micold-core/src/first_turn.rs` per C3 (candidate filter, injected-wrapper prefixes, slash-command name/args parsing, follower rule C3.5a–C3.5b′, first non-empty label source via `shape_label`)
- [X] T022 [US1] [U46] [U47] [U48] [U49] Implement `ClaudeProvider::read_label` in `crates/micold-core/src/provider.rs` as `first_turn::read_prefix(&self.transcript_path(..))` → `claude_first_turn`
- [X] T023 [US1] [A1] [A2] [A3] [A4] [U61] [U62] [U66] [U67] In `crates/micold-daemon/src/state.rs`: `discover_external_sessions` falls back `read_title` → `Named`, else `read_label` → `Derived`, else `Pending` (C6.1); `record_recovered_names` reads `read_title`, and when it is `None` and the candidate was `Pending`, `read_label`; under the lock records a title unless the session is now `Named`, and a label via `Catalog::record_session_label` only if still `Pending` (C6.3); returns titles **plus** labels recorded (C6.3a); logs a failed label write like a failed name write
- [X] T024 [US1] Update `docs/user-guide/worktrees-and-sessions.md` (*"New session" means…* and *Sessions from before this was true…*): a session the AI CLI never titled reads the first thing you typed in it (a skill's arguments, or its name when it has none), one line, cut at 80 characters with "…"; it still reads "New session" when nothing was typed; a title the CLI gives later replaces it (FR-015, FR-006)
- [X] T025 [US1] Write `crates/micold-core/tests/first_turn_label_corpus.rs` (new): an `#[ignore]` test gated on `MICOLD_LABEL_CORPUS=1` that walks `ClaudeProvider.config_dir()/projects/*/*.jsonl` read-only and prints per transcript title / label / neither, and counts, for quickstart B1 (SC-001, SC-002, SC-005); it asserts nothing about the machine when the variable is unset

- [X] T042 [US1] [A1] [A2] [A3] [A4] Outer loop green: the acceptance tests for US1 scenarios 1–4 in `crates/micold-daemon/tests/untitled_session_labels.rs` pass with the full suite
- [X] T046 [US1] Run quickstart §B1 (`claude` rows only) and §B2 (steps 1–4, incl. SC-006 by eye) on the development machine after T043, and record output and pass/fail per step in `specs/032-untitled-session-labels/evidence/quickstart-b.md` (new)

**Checkpoint**: past untitled `claude` sessions show their labels after a restart.

---

## Phase 4: User Story 2 — The AI CLI's own title still wins (Priority: P2)

**Goal**: a title always beats a label, and replaces it when it arrives, running or not (FR-005,
FR-006, FR-013).

**Independent Test**: a `Derived` session whose records gain an `ai-title`; restart; the row shows
the title (US2 #3).

### Tests for User Story 2 ⚠️ (mandatory; each seen failing first)

- [X] T026 [US2] [A7] [A8] [A9] [U63] [U64] [U65] [U71] Write failing tests in `crates/micold-daemon/tests/untitled_session_labels.rs` (real `ClaudeProvider`, `ENV` mutex): a titled session is never labelled by discovery or recovery (US2 #1, SC-003); a `Derived` session whose records gain a title becomes `Named` on the next `recover_session_names`, persisted, and stays `Named` after a reload (US2 #3, C6.2); an observed terminal title (`record_observed_names`) replaces a `Derived` label and persists (US2 #2, C6.5; guard, may pass on arrival); a label read that races a title for the same session ends `Named` (label applied first then title, and title first then label — C6.3, C6.6; guard); nothing ever moves `Named` → `Derived`/`Pending` or `Derived` → `Pending` (C6.6; guard); no client message writes a label (FR-013: assert the label changes only through the recovery and title paths; guard). Only the "records gain a title" case must be red before T027

### Implementation for User Story 2

- [X] T027 [US2] [A7] [A8] [A9] [U63] [U64] [U65] [U71] In `crates/micold-daemon/src/state.rs`, widen the candidates of `recover_session_names` and `recover_live_session_names` from `Pending` to "not `Named`" (C6.2); keep `prunable_session_cwds` in `crates/micold-daemon/src/catalog.rs` `Pending`-only and say why in its doc comment (a `Derived` session has a conversation; data-model invariant 4)

- [X] T043 [US2] [A7] [A8] [A9] Outer loop green: the US2 acceptance tests in `crates/micold-daemon/tests/untitled_session_labels.rs` pass with the full suite

**Checkpoint**: labels never outrank titles; a later title replaces a label, before and after restart.

---

## Phase 5: User Story 1 (Copilot) — Copilot rows: `summary:` titles and first-turn labels (Priority: P1, scenarios 5–6)

**Goal**: a listed Copilot session shows `name:`, else `summary:`, as its title (FR-016, US1 #6,
SC-008), and a first-turn label when it has neither (US1 #5, SC-009). A running Copilot session gets its label
through the M1 live path; the spinner-first case lands with US3 (Phase 6). D10:
listed sessions only; discovery unchanged.

**Independent Test**: a listed Copilot session fixture with `summary:` and no `name:` reads the
summary; one with neither reads its first `user.message` content.

### Tests for User Story 1 (Copilot) ⚠️ (mandatory; each seen failing first)

Outside-in: T034's acceptance tests (A5, A6) are written first and stay red until T036.

- [X] T031 [P] [US1] Write the Copilot fixtures in `crates/micold-core/tests/fixtures/first_turn/copilot/`, shaped on the 44 old sessions (spec *Copilot evidence*): `events.jsonl` files with a `session.start` record, a skill-context `user.message` carrying `data.source`, an `instruction-discovery` one, an autopilot continuation (`isAutopilotContinuation: true`, empty `content`), a whitespace-only `content`, a `/fleet`-shaped `Fleet deployed: …` content, a first turn at record 10, and a `transformedContent` differing from `content`; `workspace.yaml` files with `name:` only, `summary:` only, both, `summary: |-` block only, empty `name:` + `summary:`, and neither
- [X] T032 [US1] [U37] [U38] [U39] [U40] [U41] [U42] [U43] Write failing tests in `crates/micold-core/tests/first_turn_label.rs` for `copilot_first_turn(&[u8])` over the T031 events fixtures, one per clause C4.1–C4.4 (sourced and continuation records skipped, `content` not `transformedContent`, first non-empty wins, `Fleet deployed: …` kept verbatim)
- [X] T033 [US1] [U50] [U51] [U52] [U53] [U54] [U55] Write failing tests in `crates/micold-core/tests/copilot_provider.rs`: `read_title` returns `name:` when present, else `summary:` (C7.1, SC-008 fixture), `None` for a block `summary: |-` alone and for neither (C7.2), `summary:` when `name:` is empty (C7.3); `read_label` returns the first turn over a temp `session-state/<id>/events.jsonl`, `None` for a missing file and for a first turn past 1 MiB (C2.4, SC-009 fixture)
- [X] T034 [US1] [A5] [A6] Write failing tests in `crates/micold-daemon/tests/untitled_session_labels.rs` with the real `CopilotProvider` under a temp `COPILOT_HOME` holding a per-cwd index that **lists** the fixture sessions (D10): after `discover_external_sessions` + `recover_session_names`, the `summary:`-only session is `Named(summary)` and never `Derived` (US1 #6, SC-008), the neither session is `Derived(first content)` (US1 #5, SC-009), and a `Derived` Copilot session whose `workspace.yaml` gains `name:` becomes `Named` (US2 for Copilot)

### Implementation for User Story 1 (Copilot)

- [X] T035 [US1] [U37] [U38] [U39] [U40] [U41] [U42] [U43] Implement `copilot_first_turn(prefix: &[u8]) -> Option<String>` in `crates/micold-core/src/first_turn.rs` per C4, shaping with `shape_label`
- [X] T036 [US1] [U50] [U51] [U52] [U53] [U54] [U55] In `crates/micold-core/src/provider.rs`: `CopilotProvider::read_label` = `read_prefix(events_path)` → `copilot_first_turn` (replacing the Phase 2 `None`); `read_title` = `name:` else `summary:` (C7.1); `read_yaml_scalar` returns `None` for a value starting with `|` or `>` (C7.2); delete T013's "`CopilotProvider.read_label` is `None`" assertion; update the doc comments that say "exactly one key is ever read"
- [X] T037 [US1] Update `docs/user-guide/worktrees-and-sessions.md`: GitHub Copilot sessions follow the same rule, and a session from an older Copilot (1.0.36 and earlier) shows the summary Copilot wrote for it as its name
- [X] T038 [US1] Extend `crates/micold-core/tests/first_turn_label_corpus.rs` to walk `CopilotProvider.config_dir()/session-state/*/` read-only and print title (`name:`/`summary:`) / label / neither per session, as evidence for quickstart B1 (not the SC-008/SC-009 verdict, which T033/T034 give; D10)

- [X] T045 [US1] [A5] [A6] Outer loop green: the US1 scenario 5–6 acceptance tests in `crates/micold-daemon/tests/untitled_session_labels.rs` pass with the full suite

**Checkpoint**: listed Copilot sessions show `name:`, `summary:` or a first-turn label, in that order.

---

## Phase 6: User Story 3 — A session I am working in gets its label too (Priority: P3)

**Goal**: a running untitled session shows its label within 60 s of the first prompt, whichever of
the spinner and the prompt hook arrives first (FR-010, SC-007).

**Independent Test**: start a session, type a prompt, keep it untitled; the row shows the label
without reopening (quickstart B3).

### Tests for User Story 3 ⚠️ (mandatory; each seen failing first)

- [X] T028 [US3] [A10] [A11] [U68] [U69] [U70] Write failing tests in `crates/micold-daemon/tests/untitled_session_labels.rs`, copying the `register_cat` / `cat` PTY harness from `crates/micold-daemon/tests/activity_pipeline.rs` so the session is both catalog-known and live: a live untitled session, `note_activity` busy → the next `recover_live_session_names` records the label and returns > 0 (C6.3a); **spinner first**: `drain_signals` applies `SpinnerObserved`, then the prompt hook arrives with no activity change, and the next `recover_live_session_names` still records the label (C6.3b, research R9); an idle tick with no change reads nothing

### Implementation for User Story 3

- [X] T029 [US3] [A10] [A11] [U68] [U69] [U70] In `crates/micold-daemon/src/state.rs` `drain_signals`, set `live.name_stale = true` whenever the `SpinnerObserved` branch changes the session's activity, with a comment citing 032 FR-010 and research R9
- [X] T030 [US3] Add to `docs/user-guide/worktrees-and-sessions.md` that a session you are working in gets the same label within a minute of your first prompt, and switches to the CLI's title when it has one

- [X] T044 [US3] [A10] [A11] Outer loop green: the US3 acceptance tests in `crates/micold-daemon/tests/untitled_session_labels.rs` pass with the full suite

**Checkpoint**: running `claude` and Copilot sessions agree with what they will show after a restart.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T039 Update the module doc of `crates/micold-core/src/provider.rs` (seam method list, contract links: add `specs/032-untitled-session-labels/contracts/first-turn-label.md`) and the doc comments of `recover_session_names` / `record_recovered_names` in `crates/micold-daemon/src/state.rs` that still say "unnamed = `Pending`"
- [ ] T040 Run quickstart §B1 for Copilot rows and §B3 (running session, `claude` and Copilot) on the development machine and append the output and pass/fail per step to `specs/032-untitled-session-labels/evidence/quickstart-b.md` (created by T046)
- [ ] T041 Run `mise run gate` and record the passing SHA in the milestone PR body

---

## Dependencies & Execution Order

### Phase Dependencies

- Setup (T001–T002) → Foundational (T003–T016) → US1 `claude` (T017–T025, T042) → US2 (T026–T027,
  T043, T046) → US1 Copilot (T031–T038, T045) → US3 (T028–T030, T044) → Polish (T039–T041). T042–T045
  are the outer-loop-green tasks that close their phases; T046 records quickstart §B1/§B2 for M1.
- US2 needs US1's label step (there is nothing to replace before a label exists). The Copilot phase
  needs Foundational and the US1/US2 daemon logic, which are provider-agnostic. US3 needs US2's
  widened live candidates for its title half and US1 for its label half; it follows Copilot only
  because P1 ships before P3, not because of a code dependency.

### Within Each Phase

- Each test task precedes the implementation task(s) listed after it and is seen failing for the
  stated reason first (Principle I).
- `first_turn.rs` (T012 → T021 → T035) and `provider.rs` (T014 → T022 → T036) are edited in order,
  never in parallel.

### Parallel Opportunities

- T002 with T001; T003, T005, T007, T009, T011, T013, T015 (distinct test files) in parallel, then
  each implementation after its test.
- T017 with T019's temp-dir helper; T031 with T032's scaffolding.

## Parallel Example: Foundational

```text
Tests together: T003 (session_label_kinds.rs), T005 (session_name_round_trip.rs), T007 (schema_hash.rs),
T009 (session_title_sync.rs), T011 (first_turn_label.rs), T013 (ai_cli_provider_seam.rs),
T015 (untitled_session_labels.rs)
```

## Implementation Strategy

### MVP first

Setup + Foundational + US1 `claude` + US2 is the MVP: the reported rows get labels, and a later
title still replaces them. US2 ships with it because a label that no title could replace would be a
regression on `main` (FR-006).

### Incremental delivery

1. MVP (M1) → the reporter's four untitled `claude` rows read four different labels.
2. Copilot (M2) → listed Copilot sessions read `name:`, `summary:` or a label.
3. US3 + Polish (M3) → running sessions get their label within a minute of the first prompt, spinner
   first or not; quickstart §B3 recorded.

## Notes

- `[P]` tasks touch different files. Commit after each task or red/green pair.
- The fixtures are synthetic; never commit a real user transcript.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). Order follows
priority: both P1 halves (M1 `claude`, M2 Copilot) ship before the P3 story (M3).

### M1 — Untitled `claude` sessions read their first turn; a title still wins 🎯 MVP

- **Tasks**: T001–T027, T042, T043, T046
- **Deliverable**: after a restart, a past `claude` session with typed prompts and no `ai-title` shows its first turn on its row (the reporter's four rows read four different labels), a session with no typed prompt still reads "New session", and a title the CLI records later replaces the label, before and after a restart; the user guide says so.
- **Satisfies**: US1 acceptance scenarios 1–4; US2 acceptance scenarios 1–3; FR-001–FR-009, FR-011–FR-015; SC-001–SC-005 for `claude`; SC-006 (by the research R7 bound, checked by eye in quickstart §B2 step 4)
- **Verify**: `mise run gate`; `scripts/build-lock.sh cargo test --test untitled_session_labels` (A1–A4, A7–A9 pass); `scripts/build-lock.sh cargo test --test first_turn_label`; quickstart §B1 (`claude` rows: `9a536c7e` reads `/speckit-autopilot`, four distinct labels) and §B2, recorded by T046 in `evidence/quickstart-b.md`
- **Depends on**: —
- **Why US2 rides in M1** (milestones.md rule 2/3 exception): without T026–T027 a `Derived` session is never re-read for a title, so a title arriving in the records would never replace the label: an FR-006 regression on `main`. Setup + Foundational (T001–T016) have no deliverable of their own, and every `claude` scenario (US1 #1–4) needs all of them. US1 #6 (Copilot `summary:`) could ship alone, but splitting it out does not make M1 smaller, so no acceptance-scenario split yields two deliverables.

### M2 — Copilot rows: `summary:` titles and first-turn labels

- **Tasks**: T031–T038, T045
- **Deliverable**: a listed Copilot session reads `name:`, else `summary:`, as its title, and its first typed turn when it has neither; a later `name:` replaces a label; the user guide covers Copilot.
- **Satisfies**: US1 acceptance scenarios 5–6; US2 for Copilot; FR-012 (Copilot), FR-016; SC-008, SC-009 (listed sessions, fixtures; D10)
- **Verify**: `mise run gate`; `scripts/build-lock.sh cargo test --test copilot_provider` and `--test untitled_session_labels` (A5, A6 pass); `MICOLD_LABEL_CORPUS=1` corpus probe prints the Copilot rows
- **Depends on**: M1

### M3 — A running session shows its label within a minute of the first prompt (+ Polish)

- **Tasks**: T028–T030, T044, T039–T041
- **Deliverable**: a running untitled `claude` or Copilot session shows its label within one supervisor pass of its first prompt, whether the prompt hook or the spinner is seen first, and switches to the title when the CLI reports one; the user guide says so; quickstart §B3 is recorded in `evidence/quickstart-b.md`.
- **Satisfies**: US3 acceptance scenarios 1–2 (both providers); FR-010; SC-007
- **Verify**: `mise run gate`; `scripts/build-lock.sh cargo test --test untitled_session_labels` (A10, A11 pass, incl. the spinner-first case); quickstart §B3 steps 1–3 recorded in `specs/032-untitled-session-labels/evidence/quickstart-b.md`
- **Depends on**: M1, M2
- **Polish folded in**: T039–T041 are small (doc comments, quickstart §B record, gate) and depend on all behaviour, so they ship with the last behavioural milestone rather than as a deliverable-less milestone.

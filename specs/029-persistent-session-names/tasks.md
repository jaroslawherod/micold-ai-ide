---
description: "Task list for feature 029: a session keeps its name when nothing is running it"
---

# Tasks: A session keeps its name when nothing is running it

**Input**: Design documents from `/specs/029-persistent-session-names/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/session-name-persistence.md](./contracts/session-name-persistence.md), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are MANDATORY. Every user story includes failing tests written and reviewed BEFORE its implementation tasks (Red-Green-Refactor). Two places depart from Red-first, and say so where they appear rather than pretending otherwise: T002 is a **characterization gate** over a mapping that already exists and that this feature starts depending on, and US3's gates (T017-T018) pin invariants US1 and US2's implementations are expected to satisfy already. Both are still written before the code they guard.

**Documentation**: Per Constitution Principle VII, both user-facing stories carry a user-guide task in this same change (T011, T016).

**Cross-platform**: Per Constitution Principle VI, nothing here is platform-conditional. The one test that could accidentally become Unix-only — FR-009's "a failed write changes nothing" — is written against `Catalog::ephemeral` rather than a `chmod`-ed directory (research R8).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Exact file paths in every description

## Path Conventions

Three-crate Cargo workspace. `crates/micold-core/` (render-free core + persistence), `crates/micold-daemon/` (catalog single-writer + live session registry), `crates/micold-client/` (**not touched by this feature**). Integration tests live in each crate's `tests/`.

---

## Phase 1: Setup

**Purpose**: Establish that any Red observed later belongs to a new test and not to a pre-existing failure.

- [X] T001 Run `mise run test` (the whole-workspace task in `mise.toml`) and confirm it is green before any edit; note the HEAD sha in the first test commit's message so later Red is attributable to a new test

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The single durable write path both US1 and US2 record names through. **No user story can start until T005 is green.**

### Tests (write first, observe Red)

- [X] T002 [P] Add `crates/micold-core/tests/session_name_round_trip.rs` pinning the existing `SessionLabel` ↔ `StoredSession::title` mapping: `Named(s)` saves and loads as `Named(s)`, `Pending` round-trips as `Pending`, and a session record written **without** a `title` key loads as `Pending` rather than erroring — the claim that lets this ship with no `schema_version` bump (contract §1). **Characterization gate**: this behaviour already exists in `crates/micold-core/src/store.rs`, so it will be green on arrival; it is here to fail if anyone changes the mapping underneath the feature
- [X] T003 Add `crates/micold-daemon/tests/session_name_persistence.rs` with failing tests for `Catalog::record_session_name` (contract C1, C2, C4, C6): it resolves the session by `SessionId` through `Workspace::find_session_mut` and sets `SessionLabel::Named`; recording the name the session already has returns `Ok(false)` and writes nothing; an empty name is a no-op returning `Ok(false)`; an unknown id returns `Ok(false)`; and exactly one session's label changes — every other session in that project and in a second project is untouched (Red: the method does not exist)
- [X] T004 Extend `crates/micold-daemon/tests/session_name_persistence.rs` with the FR-009 clause (contract C5): against a `Catalog::ephemeral()` the in-memory label still becomes `Named`, and nothing is reported as a session failure (Red: same missing method)

### Implementation

- [X] T005 Implement `Catalog::record_session_name(&mut self, id: SessionId, name: &str) -> io::Result<bool>` in `crates/micold-daemon/src/catalog.rs`, beside `remember_foreground` and following its compare-before-write shape: look the session up via `self.workspace.find_session_mut(id)`, return `Ok(false)` when the name is empty / unknown / already recorded, otherwise `session.set_title(name)` **in memory first**, then `self.persist()`, returning `Ok(true)`. Doc-comment why the comparison is load-bearing (research R3: the file holds every one of that project's session records). Run T002–T004 green

**Checkpoint**: the durable write exists and is covered. US1 and US2 can now proceed — in either order, or in parallel by two people.

---

## Phase 3: User Story 1 - Find my session again after a restart (Priority: P1) 🎯 MVP

**Goal**: A name the user has already seen stays on its row when the daemon restarts and nothing is running the session.

**Independent Test**: Name a session, stop and restart the daemon, and read the session list without opening anything — the name is on the row (quickstart §B1).

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [X] T006 [P] [US1] Extend `crates/micold-daemon/tests/activity_pipeline.rs`: the stripped OSC-0 title that becomes the live session title is **also** returned by `drain_signals` as an observed `(SessionId, String)` change, exactly once, and is not re-reported on the following drain when the title has not changed (contract C9, C10). Reuses the existing real-PTY `printf` fixture in that file (Red: `drain_signals` returns `bool`)
- [X] T007 [US1] Extend `crates/micold-daemon/tests/session_name_persistence.rs`: a name recorded for a session is still present after building a **fresh** `Catalog` over the same data directory — the in-process stand-in for a daemon restart (FR-001, SC-001) — and three sessions each keep their own distinct name across that reload, none of them reading as `Pending` (SC-001, SC-006)

### Implementation for User Story 1

- [X] T008 [US1] Widen `DaemonState::drain_signals` in `crates/micold-daemon/src/state.rs` to return the existing "a projected summary changed" signal **plus** the debounced `(SessionId, String)` title changes it currently swallows into `LiveSession::last_title`. Keep it lock-only and free of blocking I/O (contract C7) and say so in the doc comment — the reason the write is the caller's job, not this method's (research R2). Update the existing call sites in `crates/micold-daemon/tests/activity_pipeline.rs` that ignore the return value
- [X] T009 [US1] Add `DaemonState::record_observed_names` to `crates/micold-daemon/src/state.rs`: forward each observed pair to `Catalog::record_session_name`, log a failure at `warn` and continue with the rest, and surface nothing to the client — no `WireLifecycle::Failed`, no lifecycle change (contract C12, C13; the convention `adopt_discovered_sessions` already follows)
- [X] T010 [US1] Wire it into the supervisor tick in `crates/micold-daemon/src/server.rs` (`spawn_supervisor`, ~line 243): take the changes from `drain_signals`, and when the set is non-empty hand it to `record_observed_names` inside a `tokio::task::spawn_blocking` hop — never on the async runtime (contract C11). No change to when `broadcast_catalog` fires

### Documentation for User Story 1

- [X] T011 [P] [US1] Add a "The name on a session row" subsection to `docs/user-guide/worktrees-and-sessions.md` under *What the sidebar shows* (~line 428): the name comes from the conversation, not from you; it is remembered, so it is on the row whether or not the session is running; a row reads "New session" only when the conversation has never been named. Verify with `mise run site-check`

**Checkpoint**: US1 is independently shippable — the reported bug is fixed for every session named from here on.

---

## Phase 4: User Story 2 - My existing sessions get their names back (Priority: P2)

**Goal**: A session the application has no recorded name for shows the name the AI CLI already holds, without the user opening it.

**Independent Test**: Remove the `"title"` key from one session record whose AI CLI records do hold a name, open the project, and read the row without opening the session (quickstart §B4).

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T012 [P] [US2] Add `crates/micold-daemon/tests/session_name_recovery.rs` with failing tests for `DaemonState::recover_session_names(project)`: a known session labelled `Pending` whose provider records hold a name is recovered and the count reflects it; a session with no recorded name stays `Pending` and writes nothing (contract C17, FR-004); a provider whose `config_dir()` is `None` contributes nothing and does not suppress the other provider's contribution (contract C20). Model the fixtures on `crates/micold-daemon/tests/session_discovery.rs` (Red: the method does not exist)
- [X] T013 [US2] Extend `crates/micold-daemon/tests/session_name_recovery.rs`: the recovered name is **persisted**, so a second pass over the same state recovers `0` and a freshly loaded `Catalog` still has it (FR-007); a session already `Named` is skipped, and deleting its provider records afterwards does not cost it its name (FR-008, contract C15); and each session is read through **its own** provider, so a Copilot session's name is not looked for in `claude`'s store (contract C16)

### Implementation for User Story 2

- [X] T014 [US2] Implement `DaemonState::recover_session_names(&self, project: &Path) -> usize` in `crates/micold-daemon/src/state.rs`, beside `discover_external_sessions` and reading the same worktree cache for its location list: collect under the lock the project's sessions whose label is `Pending` (filtering **before** any filesystem access — contract C15, the bound on the pass), then off the lock ask each session's own `session.provider.provider()` for `read_title(config_dir, session.location.cwd(project), id)`, and record each `Some(name)` through `Catalog::record_session_name`. A `None` writes nothing. Doc-comment the cost argument from research R4 — FR-007 is what stops this repeating, not a per-location rule
- [X] T015 [US2] Call it from `refresh_worktrees_off_runtime` in `crates/micold-daemon/src/server.rs` (~line 1516), inside the **same** `spawn_blocking` hop that already refreshes worktrees and runs `discover_external_sessions`, after the discovery pass so a just-adopted session is not probed twice. Log the recovered count at `info` the way the discovery count is logged

### Documentation for User Story 2

- [X] T016 [US2] Extend the T011 subsection in `docs/user-guide/worktrees-and-sessions.md` with the recovery behaviour: sessions from before this existed get their names back the next time you open the project, read from the CLI's own records, once. Verify with `mise run site-check`

**Checkpoint**: the reporter's existing sessions are named without them touching anything.

---

## Phase 5: User Story 3 - The name stays true to the conversation (Priority: P3)

**Goal**: Remembering a name does not mean freezing it, and an unnamed session still says so.

**Independent Test**: Re-title a running session's conversation, restart the daemon, and confirm the row shows the new name and never the old one (quickstart §B6).

**Note on Red**: US1 and US2's implementations are expected to satisfy these already — `set_title` replaces unconditionally and no path clears a label. That is exactly why the gates belong here: they are the invariants a later change would break silently. Write them, run them; if either is Red, the fix lands in T019.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [X] T017 [P] [US3] Extend `crates/micold-daemon/tests/session_name_persistence.rs`: recording a second, different name replaces the first, the replacement survives a catalog reload, and the previous name is not readable anywhere afterwards (FR-005, SC-005)
- [X] T018 [US3] Extend `crates/micold-daemon/tests/session_name_persistence.rs`: a session that has never been named is `Pending` before and after a reload and its `SessionLabel::display()` is `"New session"` (FR-004); and a named and an unnamed session sharing a project and a location keep their own labels across a reload — neither inherits the other's (FR-012, SC-006)

### Implementation for User Story 3

- [X] T019 [US3] Run T017–T018. If green, record in `specs/029-persistent-session-names/quickstart.md` §A that they passed without new implementation and why (data-model invariants 2 and 3 hold by construction). If Red, fix in `crates/micold-daemon/src/catalog.rs` or `crates/micold-daemon/src/state.rs` — the defect is a path that clears or first-write-wins a label, and it must not be fixed by special-casing the test

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T020 [P] Run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` over the changed files (`crates/micold-daemon/src/{catalog,state,server}.rs` and the new `tests/`); `mise.toml` has no task for either, and CI stops at `fmt` before every other job
- [X] T021 [P] Run `mise run site-check` to confirm the documentation build passes with the T011/T016 edits (Principle VII) — run as `site/build.sh --no-media` (2026-09-12): the theme emit, the stage and the mdBook render all pass with the edits, and `site/checks/links.sh` resolves every internal link (its only 20 errors are the media `--no-media` deliberately did not capture). The media-presence check and `page-checks.mjs` (axe-core in a real browser) are the two steps `--no-media` cannot satisfy — they need the capture harness and a `playwright install --with-deps`, which the publishing workflow does.
- [X] T022 Run `mise run test` for the whole workspace, matching CI (Principle I), and `cargo check --target aarch64-apple-darwin` to confirm the change builds for macOS as well as this host (Principle VI)
- [X] T023 Confirm the diff touches **no** file under `crates/micold-client/` — a client change here would be a second writer of the catalog, which the contract forbids (contract §4). `git diff --name-only main...HEAD | grep micold-client` must be empty
- [X] T024 Run the quickstart §B manual pass (B1–B7) against a release build with a real AI CLI, and append the dated record to `specs/029-persistent-session-names/quickstart.md` — including any step that could not be run and why. §B1 is the only proof of the central claim; no automated test restarts the daemon process — **run 2026-09-12** (record in quickstart.md §B). Two departures from this line, both recorded there: a **debug** build, not a release one (the pair was pinned to a private directory and verified to carry this feature, which is what the line is protecting), and a **stand-in `claude`** emitting the two interfaces the daemon reads (OSC-0 title, `ai-title` transcript record) rather than the real CLI. B7's busy/idle indicator and worktree rows were not reached and are not marked passed

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)** → no dependencies
- **Phase 2 (Foundational)** → depends on Phase 1. **Blocks every user story**: T005 is the write path all three stories record through
- **Phase 3 (US1)** → depends on Phase 2
- **Phase 4 (US2)** → depends on Phase 2. Independent of US1
- **Phase 5 (US3)** → depends on Phase 2; its gates are meaningful only once US1 (and ideally US2) has landed, since they assert invariants over those write paths
- **Phase 6 (Polish)** → depends on every story that is being shipped

### User Story Dependencies

- **US1 (P1)**: needs only Foundational. Ships alone as the MVP
- **US2 (P2)**: needs only Foundational. Does **not** need US1 — recovery reads the provider's records, not the live title. Two people can take US1 and US2 at once
- **US3 (P3)**: no new implementation expected; verifies invariants over US1 and US2's paths

### Within Each User Story

Tests first (observe Red) → implementation → green → refactor under green → documentation.

Within US1 specifically: T008 before T009 before T010 — each is the previous one's caller, and T008 changes a signature T010 consumes.

### Parallel Opportunities

- T002 is `micold-core`, T003/T004 are `micold-daemon`: T002 runs alongside either
- **US1 and US2 are fully parallel after T005** — different test files, and their only shared source file (`state.rs`) is touched in different, non-overlapping regions (`drain_signals`/`record_observed_names` vs. a new `recover_session_names` beside `discover_external_sessions`)
- T006 (`activity_pipeline.rs`) and T007 (`session_name_persistence.rs`) are different files
- T011 (docs) is parallel with all of US1's code once the behaviour is settled
- T020 and T021 are independent of each other

Tasks touching the **same** file are deliberately not marked `[P]`: T003/T004/T007/T017/T018 all extend `session_name_persistence.rs`, and T008/T009 both edit `state.rs`.

## Parallel Example: after the Foundational checkpoint

```text
Developer A (US1):  T006 → T007 → T008 → T009 → T010 → T011
Developer B (US2):  T012 → T013 → T014 → T015 → T016
Both converge on:   T017 → T018 → T019 → Phase 6
```

## Implementation Strategy

### MVP First (User Story 1 only)

Phase 1 → Phase 2 → Phase 3. That is T001–T011: eleven tasks, and at the end of them the reported
bug is fixed for every session named from that point on. Stop, run quickstart §B1–B3, and ship if
you want the fix out before the recovery pass exists.

### Incremental Delivery

1. **Foundational** (T001–T005) — the write path exists, nothing calls it yet, suite green
2. **+US1** (T006–T011) — names survive a restart. **Shippable**
3. **+US2** (T012–T016) — existing sessions get their names back. **Shippable**
4. **+US3** (T017–T019) — the invariants are pinned against future change
5. **Polish** (T020–T024) — gates, docs build, and the manual pass on record

### What is deliberately not here

- **Removing `SessionMsg::TitleUpdated`.** It is unreached in production (research R0) but it is the
  client's own reducer message, with tests of its own to delete. Cleaning it up here would put
  `micold-client` in this diff for no behaviour change and cost T023 its meaning. Separate change.
- **A time-based write debounce.** Rejected in research R3 — the two existing comparisons already
  bound the write rate, and a coalescing window would add a gap in which a crash loses a name change.
- **Naming a session by hand.** Out of scope by the spec's own assumption; FR-011 requires the name
  stay derived from the conversation.

## Notes

- `[P]` = different files, no dependency on an incomplete task
- Every task names its exact file path; each is a Red-Green-Refactor step or a verification step
- Commit after each task; the Red commit and the Green commit stay separate so the failing test is
  reviewable, per Principle I
- `mise run test-core` while iterating on T002; `cargo test -p micold-daemon session_name` for the
  daemon gates; `activity_pipeline` spawns real PTYs, so run it on its own when it is the thing
  under change

## Phase 7: Convergence

- [X] T025 Stop recording (and projecting as `Named`) an AI CLI's own placeholder terminal title — `claude` emits OSC-0 `"✳ Claude Code"` at startup, which `strip_status_glyph` turns into `"Claude Code"` and `drain_signals` → `record_observed_names` persists as the name of a session that has never had one, after which `recover_session_names` skips it as `Named` (C15); have each provider declare its placeholder title, drop it in `drain_signals` (`crates/micold-daemon/src/state.rs`), and write the failing gate first in `crates/micold-daemon/tests/activity_pipeline.rs` (a PTY emitting `"✳ Claude Code"` leaves the session `Pending` on disk and on the wire) per FR-004 (contradicts)
- [X] T026 Record a session's name only from its primary AI CLI process — `drain_signals` reads `live.procs.get(&live.attached)`, and `attached` is a `SessionProcess::Shell` once a shell tab is attached (`attach_process`, `open_shell`), or a plain shell for a `TerminalMode::Regular` session, so a bash prompt's OSC-0 `"user@host: ~/dir"` (default `PS1` under `TERM=xterm-256color`) is now persisted over the conversation's name and survives restarts; take the title from `SessionProcess::Primary` of an `AiCli` session only, with a failing gate first (a shell tab emitting a title leaves the recorded name unchanged) per FR-011 (contradicts)
- [X] T027 Re-run quickstart §B1 and §B7 against the real `claude` CLI rather than the stand-in, including opening a shell tab on a named session and a never-named session left untouched across a restart, and append the dated result to `quickstart.md` per SC-001 / SC-006 (partial)

## Phase 8: Convergence

- [X] T028 Stop `prune_empty_sessions` from archiving a `Named` session whose AI CLI transcript is gone. Today `Catalog::prunable_session_cwds` in `crates/micold-daemon/src/catalog.rs` offers every non-archived session, so when the project is next attached (`server.rs` `prune_empty_off_runtime`) a named session loses its row, and its name with it. Write the failing test first: a `Named` session with no transcript survives `prune_empty_sessions` and is still listed under its name, while a `Pending` one is still pruned. Then exclude `Named` sessions from the candidates per FR-008 / Edge Case 1 / research R5 (partial)
- [X] T029 Correct `docs/user-guide/worktrees-and-sessions.md` *The name on a session row*: a session that was created but never talked to does **not** keep reading "New session" across restarts, because it has no conversation and is tidied away when the project is next opened (observed in T027). Rebuild the docs with `site/build.sh --no-media` per Constitution VII / FR-004 (contradicts)

## Phase 9: Integration with `main`

- [X] T030 Carry the FR-004 title rule to the Pi provider that landed on `main` while this branch was open: `pi` has no fixed startup title (`π - <folder>` unnamed, `π - <name> - <folder>` named), so replace `AiCliProvider::startup_title` with `name_in_terminal_title(title, cwd)` in `crates/micold-core/src/provider.rs`, call it from `drain_signals`, and add the Pi case to `an_ai_clis_own_startup_title_is_not_a_name` (failing first) plus a parser test in `crates/micold-core/tests/pi_provider.rs` per FR-004 (partial)

# Tasks: Choose which AI CLI a session runs on

**Input**: Design documents from `/specs/026-multi-provider-sessions/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

> ## Status, 2026-08-20
>
> **1924 tests, 0 failures.** `micold-core` + `micold-daemon` and `micold-client` were run as two
> `cargo test --no-fail-fast` invocations rather than one `--workspace`, and that is a machine
> constraint rather than a choice: the development disk is at 100%, and linking every test binary in
> one pass runs out of room (`ld terminated with signal 7 [Bus error]`, then `No space left on
> device`). Per package it fits. Both returned 0.
>
> Two things a reader should know before trusting the list:
>
> - **T049 was pulled forward into Phase 2, deliberately.** T026 staged
>   `CopilotProvider::has_recorded_conversation`, `mark_archived` and `is_archived` to US2 as
>   "conservative interim bodies". They are not conservative: the client's boot prune and the
>   daemon's attach prune both **archive or drop** a session whose provider reports no recorded
>   conversation, so a `false` there deletes every Copilot session at startup — the exact
>   silent-data-loss path T008a and T008b exist to catch. They are implemented for real. Only
>   `recorded_session_ids` (empty), `read_title` (`None`) and `activity_source` (`None`) were ever
>   genuinely safe as interim answers, and those were staged as written.
> - **T042a's file is `micold-daemon/tests/session_discovery.rs`**, not the name the task text
>   guesses at.

**Tests**: Per Constitution Principle I (NON-NEGOTIABLE), test tasks are mandatory and come first in
every phase. Every provider path derivation and parser here is pure, so the whole Copilot
implementation is testable **without `copilot` installed** — that property is load-bearing for CI on
all three platforms and every test task below must preserve it.

**Documentation**: Per Principle VII, `docs/user-guide/settings.md` and
`docs/user-guide/worktrees-and-sessions.md` are updated inside the stories that make them true, not
deferred to Polish.

**Cross-platform**: Per Principle VI, no `cfg(target_os)` in either provider. Copilot's Windows base
directory is unverified (research R2) — T081 is the task that closes it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1–US4)

## Path Conventions

Three-crate Rust workspace, no new crate:

- `crates/micold-core/{src,tests}/` — the seam, both providers, session/store/settings/protocol
- `crates/micold-daemon/{src,tests}/` — supervision, catalog, activity
- `crates/micold-client/{src,tests}/` — `Capabilities`, features, UI
- `docs/user-guide/` — Principle VII deliverables

Build and test through `mise run test` (whole workspace, matching CI) and `mise run test-core` while
iterating; do not invoke `cargo` directly.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: The fixture corpus everything else is tested against, captured once from the real CLI so
no later task needs `copilot` installed — plus the two decisions the rest of the feature is blocked
on: the watch dependency (T003a) and where FR-014's discovery pass runs (T003b, now settled as
R15 — the daemon, in the attach arm).

- [X] T001 Capture a Copilot fixture corpus from CLI 1.0.62 into `crates/micold-core/tests/fixtures/copilot/`: a `sidebar-sessions-state/<sha256>.json` index, a `session-state/<uuid>/workspace.yaml` both with and without a `name:` key, and a `session-state/<uuid>/events.jsonl` covering a full turn (`user.message` → `assistant.turn_start` → `tool.execution_start` → `tool.execution_complete` → `assistant.turn_end`), a permission prompt, a shutdown, and at least two event types not in the contract's table
- [X] T002 [P] Add a `copilot_home()` temp-directory helper to `crates/micold-core/tests/support/mod.rs` that materialises the T001 corpus under a scratch base directory and sets `COPILOT_HOME` for the duration, so no test can touch a developer's real `~/.copilot`
- [X] T003 [P] Record the byte-for-byte `sha256_hex(cwd)` vector for one known working directory in `crates/micold-core/tests/fixtures/copilot/README.md`, cross-referenced from `specs/026-multi-provider-sessions/contracts/copilot-cli.md`, so a future change to the hashing helper is caught rather than silently orphaning every session
- [X] T003a Add the cross-platform filesystem-watch dependency (`notify`) to `crates/micold-daemon/Cargo.toml`, and record the vetting the Dependencies constraint requires — maintenance health, license, and why FR-019's no-polling rule leaves no in-workspace alternative — in `specs/026-multi-provider-sessions/research.md` as R14. **Blocks T064 and T060** (the latter asserts what is and is not in the watch path, so it cannot be written before the path exists); nothing else in the feature depends on it
- [x] T003b **Settled — R15.** FR-014 discovery runs in the **daemon**, inside the existing
  `ClientMsg::AttachProject` arm (`crates/micold-daemon/src/server.rs:378-397`), in the same
  `spawn_blocking` hop that already refreshes the project's worktrees. No new RPC, no protocol
  change, no client round trip. The premise this task was written on was false: the daemon *does*
  enumerate worktrees — `State::refresh_worktrees` (`state.rs:587`) runs
  `worktree::discover(&GitCli::new(), &repo)` and caches the result where the catalog snapshot reads
  it (US3/T053) — so the client-sends-the-list option existed only on paper. Attach already runs
  attach → prune → send `Attached` → refresh worktrees; discovery is a fourth step between the
  enumeration and the snapshot, on the one process allowed to write `projects.json`. FR-014's
  proportionality rule holds because the per-location cost is one `recorded_session_ids` per provider
  and the archived check is ordered *after* subtracting the catalog's own ids, so it touches only
  genuinely unknown ones. Full reasoning, including what was rejected, in **R15**. Unblocks T042a and
  T050
**Checkpoint**: Fixtures exist, the watch crate is vetted, and R15 has named the process that owns
discovery — the daemon, in the attach arm. The Copilot layout can be asserted against without the
CLI.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Make the seam substitutable. Until this phase lands, `ClaudeProvider` is reached
concretely from four files outside the seam — `micold-daemon/src/{catalog,supervisor,state}.rs` and
`micold-core/src/terminal.rs` — and two more places hide behind an abstraction rather than a name:
the client's boot prune takes a single provider for the whole workspace (T015b), and the launch path
never reaches the seam at all, because `LaunchSpec` has no provider field and `spawn_claude` is named
for the CLI it spawns (T016, T016a). No story can be built on the seam until all six are gone.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests (MANDATORY — write first, observe RED)

- [X] T004 Extend `crates/micold-core/tests/ai_cli_provider_seam.rs` so its fake implements **every
  method `contracts/ai-cli-provider.md` lists** — not just the four new ones. Four that the fake
  inherits today lose their defaults in the reshape (`has_recorded_conversation`, `read_title`,
  `mark_archived`, `is_archived`), so a fake written against the new-methods list alone will not
  compile. Assert object safety via `&dyn AiCliProvider`; delete the module doc's "It is not, yet"
  claim only once the file proves otherwise
- [X] T005 [P] Add a test to `crates/micold-core/tests/ai_cli_provider_seam.rs` asserting the trait provides **no layout-specific default** — a provider that implements only the required methods compiles and inherits nothing that assumes a `*.jsonl`-per-cwd directory (FR-021)
- [X] T006 [P] Lock `ClaudeProvider`'s existing behaviour in `crates/micold-core/tests/ai_cli_provider.rs`: launch args, `config_dir` from `CLAUDE_CONFIG_DIR`, transcript path, title parsing, archived marker and per-cwd listing must be byte-identical after the reshape — the regression gate on not breaking provider one while adding provider two
- [X] T007 [P] Extend `crates/micold-client/tests/no_concrete_implementations.rs` to scan `crates/micold-daemon/src/` and `crates/micold-core/src/terminal.rs` in addition to the client, asserting that only `crates/micold-client/src/shell/capabilities.rs` names a concrete provider type (FR-022, SC-007). This test MUST fail on the current tree —
  the name appears in four files outside `capabilities.rs`: `catalog.rs` and `supervisor.rs` (once
  each), `state.rs` (**twice**, T015a) and `terminal.rs`. Note what this scan cannot see: the client's
  boot prune (T015b) reaches one provider for the whole workspace through `Capabilities`, so it names
  nothing concrete and passes this gate while being the same defect. **T011a's registry is an
  explicit exemption**, not a quiet pass: `AiCli::provider` lives in `micold-core/src/provider.rs`,
  which is where both types are *defined*, so it names them by necessity. List it by name — an
  exemption the test states is a decision; one it happens to miss is a hole
- [X] T007a [P] Extend `crates/micold-core/tests/terminal_backend.rs` so the argument vector for a
  launch is built from the **spec's own provider**: a `LaunchSpec` naming Copilot yields Copilot's
  argv, one naming Claude Code yields today's argv byte-for-byte. This MUST fail on the current tree
  — `LaunchSpec` has no provider field and `claude_args` hard-codes `ClaudeProvider`. It is the gate
  on T016, and `terminal_backend.rs` is the file T013's signature sweep will hit anyway
- [X] T008 [P] Add `crates/micold-core/tests/ai_cli_registry.rs` asserting `AiCli` is `Copy + Eq + Hash + Ord`, that `AiCli::default()` is `ClaudeCode`, and that iterating the variants is deterministic (it orders the UI choices)
- [X] T008a [P] Add `crates/micold-daemon/tests/set_wide_provider_decisions.rs` driving `State::prune_empty_sessions` and `State::present_interrupted_resumable_at_startup` with a **mixed** set of sessions and two fake providers that disagree about which conversations exist. Assert: a session is judged **only** by its own provider; a session whose provider reports a conversation is never archived by the prune and *is* presented as resumable; and one provider's `config_dir()` returning `None` leaves the other provider's sessions fully judged rather than zeroing the whole set. This test MUST fail on the current tree — both call sites judge every session with one hard-coded `ClaudeProvider`. It is the gate on T015a, and on the only silent-data-loss path in this feature
- [X] T008b [P] Add `crates/micold-client/tests/boot_prune_is_per_provider.rs` (or extend
  `tests/app_state.rs` if that is the cheaper home) driving the client's boot prune —
  `micold-client/src/main.rs::prune_empty_sessions` and `session_has_conversation`, reached at
  startup from `crates/micold-client/src/shell/startup.rs:106` — with a **mixed** workspace and two
  fake providers that disagree. Assert a session is judged only by its own provider, and that one
  provider's `config_dir()` returning `None` keeps **only that provider's** sessions rather than
  sparing or condemning the whole workspace (the rule `main.rs:520` states today, now held
  per provider). This MUST fail on the current tree: the prune takes one `&dyn AiCliProvider` and
  applies it to every session in every project. It is the gate on T015b
- [X] T017 [P] Add `crates/micold-core/tests/copilot_provider.rs` asserting `CopilotProvider::launch_args`: `Fresh` → `["--session-id", "<uuid>", "--no-remote"]`, `Resume` → `["--resume=<uuid>", "--no-remote"]`, and that `--allow-all-tools`/`--allow-all` never appear (contract "Launch")
- [X] T017a [P] In `crates/micold-core/tests/copilot_provider.rs`, assert **FR-011**: a fresh launch and a resume both leave the base config directory byte-identical apart from the app-owned `micold.archived` sentinel — no `config.json` write, no `trustedFolders` edit, no user-level state touched. Every per-launch need is an argument, not a file
- [X] T018 [P] In `crates/micold-core/tests/copilot_provider.rs`, assert `config_dir()` resolves `COPILOT_HOME` when set and non-empty, falls back to `~/.copilot`, treats an empty `COPILOT_HOME` as absent, and returns `None` (uncertain, not absent) when the home directory is unresolvable

### Implementation

- [X] T009 Add the `AiCli` enum (`#[default] ClaudeCode`, `Copilot`) to `crates/micold-core/src/session.rs` per `data-model.md`, with no behaviour attached
- [X] T010 Reshape the trait in `crates/micold-core/src/provider.rs` to exactly the method set
  `contracts/ai-cli-provider.md` lists. That is four changes, not one, and only the first was in an
  earlier draft of this task: (a) **replace** `transcript_dir` + the `discover_transcript_session_ids`
  default with a required `recorded_session_ids(&self, config_dir, cwd) -> Vec<Uuid>`; (b) **remove**
  `transcript_path`, `parse_title` and `archived_marker_path` from the seam entirely — all three
  encode `claude`'s one-file-per-session layout, and `archived_marker_path`'s default builds
  `{id}.archived` *inside* `transcript_dir`, which is precisely the layout-specific default T005
  fails on; (c) **promote** `has_recorded_conversation`, `read_title`, `mark_archived` and
  `is_archived` from defaulted to required, since each default was written on top of the two paths
  being removed; (d) **add** `id`, `display_name`, `is_available`, `activity_source`, and the
  `ActivitySource` enum — `Hooks` (payload-free) / `EventLog { path }` / `None`. T005 is the gate on
  (b) and (c). This task and T011 are together what FR-020 and FR-021 ask for: every CLI-specific
  detail behind the seam, and no layout assumption left in front of it
- [X] T011 Move everything T010 removed or promoted **into `ClaudeProvider` itself**, in
  `crates/micold-core/src/provider.rs`: the per-cwd directory listing becomes its own
  `recorded_session_ids` with the `*.jsonl`-stem filter and `.archived` exclusion byte-identical;
  the transcript-path arithmetic, the marker path and the JSONL title parser become private helpers
  of the impl rather than trait surface; and the four promoted methods get concrete bodies built on
  those helpers. Behaviour must not move — T006 is the gate, and it is the only thing standing
  between this task and a quiet regression in the provider that already works. **Also give
  `ClaudeProvider` the methods T010 *added* rather than promoted** — `id`, `display_name`,
  `is_available`, `activity_source` — because T010 leaves the trait with no defaults at all, so
  Phase 2 does not compile without them. `is_available` gets its real body here, the `PATH` lookup
  of `command()` that T074 describes: the method is required from Phase 2 onward and stubbing it now
  only means writing it twice
- [X] T026 Add `CopilotProvider` to `crates/micold-core/src/provider.rs`. Five methods get
  their real bodies here: `id`, `display_name` ("GitHub Copilot"), `command` ("copilot"),
  `config_dir` (`COPILOT_HOME` → `~/.copilot`) and `launch_args` including `--no-remote` (research
  R12, Principle IV). **T010 leaves the trait with no defaults, so all twelve must exist before T027
  can register this type as `Arc<dyn AiCliProvider>`** — and the other seven are staged to later
  stories. Six get an explicit interim body here, each chosen so the MVP degrades to "Copilot starts
  but is not discoverable yet" rather than to a wrong answer: `recorded_session_ids` → empty (T048),
  `has_recorded_conversation` → `false` (T049), `is_archived` → `false` (T049), `read_title` →
  `None` (T061), `mark_archived` → `Ok(())` (T049), `activity_source` → `ActivitySource::None`
  (T062). Not `todo!()`: a registered provider is reachable from the sidebar, so a panic there is a
  crash, not a reminder. The seventh, `is_available`, gets its **real** body — the `PATH` lookup
  T074 describes — because T033's split affordance branches on it inside this same phase, and a stub
  would have the MVP's one availability-dependent decision answering from a lie.
  **Moved out of US1 into Phase 2**: T011a's lookup is an exhaustive match over `AiCli`, so this type has to be constructible before the seam reshape finishes — which it is, since every method already has a body here. T017, T017a and T018 came with it, so the gate still precedes the code (Principle I)
- [X] T011a **The registry the daemon and core can actually reach.** `Capabilities` lives in
  `crates/micold-client/src/shell/capabilities.rs`, and `micold-daemon` depends on `micold-client`
  only as a **dev-dependency** (`micold-daemon/Cargo.toml:46,57`) while `micold-core` cannot depend
  on it at all — so four Phase 2 tasks on the critical path currently read from a registry they
  cannot see: T015 (`catalog.rs`), T015a (`state.rs` ×2), T016 (`micold-core::terminal::claude_args`,
  which its own task says to "drive from the registry") and T016a (`supervisor.rs`). Put the lookup
  in **`crates/micold-core/src/provider.rs`** — the one crate all three see — as
  `AiCli::provider(self) -> &'static dyn AiCliProvider`, an **exhaustive match**, so it is total by
  construction and no call site has an `Option` to mishandle. `Capabilities` keeps its role as the
  client's assembly point but delegates rather than owning a second map. Record the guard nuance:
  `no_concrete_implementations.rs` derives real implementations *from* `micold-core` and asserts
  only the shell names them, so this function is the definition site and needs an **explicit**
  exemption in T007's list — not a quiet pass. Depends on T011 and T026; blocks T015, T015a, T016
  and T016a
- [X] T012 Add `provider: AiCli` to `Session` in `crates/micold-core/src/session.rs` and give it to
  **both** constructors: `Session::start_new(location)` → `start_new(location, provider)`, and
  `Session::restored(id, location, label, mode)` (line ~273) → one that takes a provider too.
  `restored` is not an afterthought — it is the constructor for every session that comes back from
  disk (`store.rs:207`) or from the daemon (`micold-client/src/main.rs:2303`), which between them are
  FR-012, FR-013 and FR-016. Provide no setter and no mutation path (FR-001, FR-005 by shape), and
  note that this is what forces the constructor change rather than the easier repair: `into_session`
  already writes `session.archived` after construction and `reconcile_catalog` already assigns
  `s.lifecycle` and `s.activity` **from another crate**, so `Session`'s fields are `pub` and a
  `provider` set the same way would satisfy the field list while contradicting the sentence above it
- [X] T013 Update **every** call site of **both** constructors T012 changed. Two sweeps, not one —
  `Session::restored` has **41** of its own (39 in tests; the two production ones are
  `crates/micold-core/src/store.rs:207`, the load path, and `crates/micold-client/src/main.rs:2303`,
  where a daemon-reported session becomes a client one). The `start_new` half —
  **101 of them**, not the 28 an earlier draft counted; 28 is the number of *files*, which is not
  the same question. They break down as 95 in `crates/*/tests/` across 25 files (`micold-core` 38,
  `micold-client` 54, `micold-daemon` 3) and 6 inside `src/`. There is exactly **one production call
  site** — `crates/micold-daemon/src/catalog.rs:240`, inside `Catalog::create_session`, which is why
  T031's plumbing has exactly one destination. The other five sit in **three `#[cfg(test)]` modules
  inside `src/`** that a `tests/` sweep will miss: `crates/micold-client/src/main.rs` (line 2700),
  `crates/micold-client/src/ui/sidebar.rs` (563) and `crates/micold-daemon/src/supervision.rs` (69)
  — that last one an earlier draft filed as production; it is not. Find
  them with a workspace-wide search rather than a list — a list is what went stale here. Separately,
  the two test files that name `ClaudeProvider` and appear in no other task —
  `crates/micold-daemon/tests/mutation_semantics.rs` and
  `crates/micold-client/tests/worktree_delete.rs` — need the same treatment: T007's guard does not
  scan test files, so these fail at compile time rather than as a caught assertion
- [X] T014 **Half of this was written and then deleted — see the note at the end.** Give `Capabilities` (`crates/micold-client/src/shell/capabilities.rs`) the same lookup
  the rest of the workspace gets from T011a, by **delegating to it** rather than owning a second
  map: `provider(which)` forwards to `AiCli::provider(which)`, and `available_providers() ->
  Vec<AiCli>` filters the variants by `is_available()`. The single `Arc<dyn AiCliProvider>` field
  goes. Delegation is what keeps "each implementation is chosen in exactly one place" literally true
  once there are two crates that need the answer — a client-side map would be a second place.
  **The delegating `provider(which)` has no caller and is gone.** Every consumer in the client
  already holds a `Session`, and `session.provider.provider()` is both shorter and more honest — it
  says the provider comes from the record. A forwarding method nobody calls is not a seam; it is a
  second name for one, and `capabilities.rs`'s own module doc makes that argument about
  `from_parts`. `available_providers()` stays, and the difference is instructive: it is not a lookup
  but an I/O probe, which is exactly what that struct exists to own
- [X] T014a Carry the availability set into the render-free layer — a `Vec<AiCli>` field on
  `State` in `crates/micold-client/src/app.rs`, populated at the I/O boundary from
  `Capabilities::available_providers()` (T014), the way `worktrees: Vec<Worktree>` already is — a
  `Vec`, matching the contract, and already ordered because the variants are.
  Without it there is no route to any of the three places that consume the set. (a)
  `crates/micold-client/src/features/` imports nothing from `shell::` and `Capabilities` is a shell
  type, so T075 cannot filter where an earlier draft said it did. (b) `ui/settings_form.rs`'s
  `dialog` is dispatched through `crate::ui::DialogView`, a single fn-pointer type
  `(&State, ColorScheme, &EnvIncludeOutcome)` shared by all nine registered surfaces
  (`overlay/registry.rs:113,231`), so T032's select can read only `&State` short of a nine-site
  signature change. (c) `ui/sidebar.rs::view` does take `&State`, but `row_actions_cluster`
  (line ~370) takes four narrow arguments, so T033 needs the set threaded in as a fifth rather than
  reaching for it. Refresh on a named event — the Settings overlay opening and the override menu
  opening, which is what research R11 means by "when the choice is offered" — and **never per
  frame**: that would be a `PATH` probe per render, which is the scheduled work SC-006 forbids.
  Blocks T032, T033 and T075
- [X] T015 Take the provider from the session record instead of naming `ClaudeProvider` in
  `crates/micold-daemon/src/catalog.rs` (the `mark_archived_durable` helper, line ~41) — a
  per-session path where the record is already in hand, and the one genuinely mechanical
  substitution in the daemon. `supervisor.rs` is **not** mechanical and moved to T016a: its
  `ClaudeProvider.command()` sits inside `spawn_claude`, whose name, signature and
  `settings_file` argument are all Claude-shaped. **T044 is the gate**, and it sits in US2 rather
  than here for a reason: the substitution only asserts anything once
  `CopilotProvider::mark_archived` exists (T049), so foundational has nothing to check beyond
  T006's byte-identical Claude behaviour
- [X] T015a **The set-wide decisions in `crates/micold-daemon/src/state.rs`** — `prune_empty_sessions` (line ~722) and `present_interrupted_resumable_at_startup` (line ~761). Neither fits "take the provider from the session record": each judges a *set* of candidates with one provider hoisted outside the loop, and `prune_empty_sessions` **archives** what it judges empty. Left as-is with a registry, every Copilot session looks empty to `ClaudeProvider` and is silently archived on attach, and none is ever presented as resumable at startup (FR-012, FR-014, FR-015, SC-002). Restructure both so the provider is looked up **per candidate id**, resolve each provider's `config_dir()` independently, and preserve the existing "never drop a session on uncertainty" rule **per provider** — one provider's unresolvable config dir must not condemn or spare another's sessions. T008a is the gate
- [X] T015b **The client's own set-wide decision** — `crates/micold-client/src/main.rs`'s
  `prune_empty_sessions` (line ~488) and `session_has_conversation` (line ~514), called at boot from
  `crates/micold-client/src/shell/startup.rs:106` as `prune_empty_sessions(caps.provider(), ...)`.
  This is T015a's defect on the other side of the socket, and it **drops sessions from the
  workspace**: one hoisted provider judges every session in every project, so with a registry every
  Copilot session is pruned at startup. T014 removes the no-arg `Capabilities::provider()` this
  depends on, which makes it a compile error — and invites exactly the wrong repair
  (`caps.provider(AiCli::default())`), which is the silent-loss bug wearing a green build. Look the
  provider up **per session** and keep the "cannot determine the config dir → keep the session"
  rule **per provider**. T008b is the gate
- [X] T016 Give `crates/micold-core/src/terminal.rs`'s `LaunchSpec` (line ~24) a `provider: AiCli`
  field and make the argument builder read it. The function is called **`claude_args`**, not
  `launch_args`, and today it ignores the spec entirely — `ClaudeProvider.launch_args(spec.session_id,
  spec.mode)`. Rename it and drive it from the registry. Also update the doc comment that cites
  `ClaudeProvider::config_dir`'s empty-is-absent convention as if it were the only one. T007a is the
  gate
- [X] T016a **The spawn path** — `crates/micold-daemon/src/supervisor.rs`'s `spawn_claude` (line ~88)
  and the two `LaunchSpec { … }` construction sites in `crates/micold-daemon/src/state.rs` (~838,
  ~1031). This is where FR-007 is actually decided, and no earlier draft of this list assigned it:
  each site builds a spec, calls `hook_settings_file(id)` unconditionally for every
  `TerminalMode::AiCli` session, and hands both to `spawn_claude`. Rename the function off the CLI,
  populate the new `provider` field from the session record, and make the `settings_file` argument
  follow `activity_source` — supplied for a `Hooks` provider, **absent for an `EventLog` one**. Left
  undone, a Copilot session is spawned with `claude`'s argv and a `--settings` file `copilot` has no
  flag for, which is what makes this the launch half of the seam rather than a substitution
  (T024 is the gate)

**Checkpoint**: The seam is substitutable, the guard proves it, and the lookup that makes it usable
from all three crates exists (T011a). `mise run test` is green. Both provider *types* now exist —
`CopilotProvider` had to, for T011a's match to be exhaustive — but only its launch and path
arithmetic are real; discovery, titles and activity are the conservative bodies T026 lists, so no
Copilot session is startable from the UI yet. That is US1.

---

## Phase 3: User Story 1 - Start a session on the CLI I choose (Priority: P1) 🎯 MVP

**Goal**: A user with both CLIs installed picks which one runs in a session — a default in Settings
and a per-session override — and the chosen CLI actually starts, in the session's worktree, under an
id the application owns.

**Independent Test**: With both CLIs installed, start a session choosing Copilot: its terminal shows
a running Copilot CLI in the right working directory. Start another choosing Claude Code in the same
project; both run at once without interfering (quickstart §B B1, B2).

### Tests for User Story 1 (MANDATORY — write first, observe RED)

- [X] T019 [P] [US1] Add `crates/micold-core/tests/settings_ai_cli.rs`: `default_ai_cli` round-trips through the settings file, a settings file written without the field loads as `ClaudeCode` (FR-003), and a default naming an uninstalled CLI is **kept, not rewritten** (research R11)
- [X] T020 [P] [US1] Extend `crates/micold-core/tests/schema_hash.rs` to assert the protocol schema hash moves **exactly once for this whole feature** — one bump in `crates/micold-core/src/protocol/version.rs` covering both wire changes together (`SessionCreate`'s `provider` inbound and `CatalogSnapshot`'s outbound, T029). A hash move without the bump, or a second bump later in the feature, fails
- [X] T021 [P] [US1] Extend `crates/micold-core/tests/protocol_roundtrip.rs` so `ClientMsg::SessionCreate` round-trips with each `AiCli` variant
- [X] T022 [P] [US1] Extend `crates/micold-client/tests/features_session.rs`: a session-start with no override uses `Settings::default_ai_cli`; an override uses the override and leaves the setting untouched (FR-004); changing the default afterwards changes no existing session's provider (FR-005); no message mutates a session's provider (FR-001). **Also the split affordance's decision** — a press-target (primary vs secondary) plus the availability set resolves to start-with-default, open-the-list, or nothing-to-offer, including the fewer-than-two-CLIs case where the secondary half does not exist (FR-006, SC-001). This file owns every render-free session decision in US1, so the branching assertions live here and not in an implementation task; it is the gate on T032a
- [X] T023 [P] [US1] Extend `crates/micold-client/tests/features_settings.rs` so changing the Default
  AI CLI preference sends a `SettingsSet` and re-renders the form from the `SettingsChanged` the
  daemon echoes back — the service-owned path (T028), not `persist_settings`
- [X] T023a [P] [US1] Add a test in `crates/micold-daemon/tests/` that a `SettingsSet` changing the
  **scrollback limit** leaves `default_ai_cli` intact. This is the whole reason the preference is
  service-owned: the daemon saves its boot-time `Settings` struct wholesale, so a client-owned field
  would be reverted by an unrelated setting change. Assert it once, here, where it is cheap
- [X] T024 [P] [US1] Extend `crates/micold-daemon/tests/session_start.rs` so a `SessionCreate` carrying `Copilot` spawns `copilot` with the contract's argv, and one carrying `ClaudeCode` spawns `claude` exactly as today
- [X] T025 [P] [US1] Extend `crates/micold-daemon/tests/session_isolation.rs` (and `crates/micold-client/tests/session_isolation.rs`) so two sessions in the same project on different providers keep separate argv, cwd, config directory and terminal state (FR-009, US1 scenario 4)

### Implementation for User Story 1

- [X] T027 [US1] Nothing to register: T011a's exhaustive match already answers for both providers
  and T014 made `Capabilities` delegate to it, so `real()` names no provider type at all any more.
  What is left here is deleting the `provider: Arc::new(ClaudeProvider)` line and its import, and
  confirming `no_concrete_implementations.rs` still passes with `capabilities.rs` naming nothing —
  the guard's expectation moves from "the shell is the one place" to "core's definition site is the
  one place" (T007)
- [X] T028 [US1] Add `#[serde(default)] default_ai_cli: AiCli` to `Settings` in
  `crates/micold-core/src/settings.rs` and make it **service-owned**, alongside scrollback and
  environment-include rather than alongside `theme`. `settings.json` has two writers and the split is
  by field: the daemon's `Catalog` loads it at boot (`catalog.rs:71`) and calls itself its single
  writer, while the client's `persist_settings` (`micold-client/src/main.rs:527`) writes the whole
  struct for `theme` alone. Client-owned would be the smaller diff and the wrong one: `set_scrollback`
  and `set_env_include` each `store.save(&self.settings)` from the **boot-time** copy, so any
  scrollback change silently reverts whatever the client wrote since. (That is already true of
  `theme`; it is a pre-existing defect, out of scope here, and T023a will observe it in passing.) So
  `default_ai_cli` joins `DaemonSettings`, `SettingsSet` and `SettingsChanged` — three additions
  folded into T029's single hash move. `settings_version` deliberately does **not** move: the
  `#[serde(default)]` argument that spares `schema_version` (research R8) spares this third version
  number for the same reason
- [X] T029 [US1] Make **every** wire change for this feature in one edit to
  `crates/micold-core/src/protocol/messages.rs`, with a single bump in
  `crates/micold-core/src/protocol/version.rs`, recorded in `specs/*/contracts/protocol.md` §4:
  `provider: AiCli` on `ClientMsg::SessionCreate` (inbound); the same on `SessionSummary`
  (outbound); and `default_ai_cli` on `DaemonSettings`, `SettingsSet` and `SettingsChanged` (T028's
  service-owned preference). Model the outbound session field on **`worktree_dir` / `title`**, which
  are the real precedents — *not* on `mode`, which an earlier draft of this task cited: `mode` does
  not travel at all, as `micold-client/src/main.rs` says in as many words ("There is no `SetMode`
  RPC, so the mode simply does not persist across restarts"). The **store** precedent T047 cites is
  the sound one (`StoredSession.mode` and `archived`, both `#[serde(default)]`, no `schema_version`
  bump); it is only the wire analogy that was wrong. Doing all five together is what lets T020 assert
  one hash move; US3 then consumes the outbound field without touching the wire again
- [X] T030 [US1] Resolve default-or-override in **`crates/micold-client/src/features/session.rs`**
  and put the answer on `Message::SessionStartRequested { location, provider }`
  (`crates/micold-client/src/app.rs:309`). The resolution is a branch, so Principle I puts it in a
  render-free module a test can link — T022's territory
- [X] T030a [US1] Carry the resolved provider onto the wire in **`crates/micold-client/src/main.rs`**,
  which is where `SessionStartRequested` is actually handled (line ~1424) and where
  `ClientMsg::SessionCreate` is built. `app.rs:1676` classifies this message deliberately —
  *"Performed by the binary at the I/O boundary … no pure reducer effect"* — so this half **must
  decide nothing**: it copies the field across and no more. That boundary is why T030 and T030a are
  separate tasks rather than one: `main.rs` is the GUI binary, no integration test can link it (the
  constraint that produced `session_reconciliation.rs`'s mirror, T003b), and a default-vs-override
  branch that drifts into here becomes the feature's second untestable mirror. **Both** construction
  sites need the new field — the handler at ~1424 and the guarded auto-start at ~347, which starts a
  `SessionLocation::Default` session and today has no reason to name a provider
- [X] T030b [US1] The daemon reads the settings file at boot but is **not** asked to
  resolve the choice — R9's point survives, its stated reason does not: `Catalog::new` does
  `settings_store.load()` (`crates/micold-daemon/src/catalog.rs:71`), so "the daemon never reads
  settings" was simply false. What holds is that the *resolution* of default-vs-override happens
  client-side and arrives as an explicit `provider` on the wire, so no launch depends on the two
  processes agreeing about a file. Nothing to build here — it is the note that keeps a later reader
  from "fixing" the client-side resolution by moving it to the daemon
- [X] T031 [US1] Carry the new field down all **three** hops, not two: the `SessionCreate` arm
  (`crates/micold-daemon/src/server.rs:514`) → `State::create_session(project, worktree_dir)`
  (`state.rs:572`, the thin forwarder an earlier draft skipped) → `Catalog::create_session`
  (`catalog.rs:234`), which is where `Session::start_new` is finally called and the provider lands on
  the record
- [X] T032 [US1] Add the **Default AI CLI** select to `crates/micold-client/src/ui/settings_form.rs`,
  reusing the shared `crates/micold-client/src/ui/material/select.rs` component (Principle VIII — no
  bespoke control). The options are named by `display_name()` — "Claude Code", "GitHub Copilot" — the
  human-readable form, and the list it offers comes from **T014a's `State` field**, not from
  `Capabilities` — `settings_form`'s view is dispatched through the shared `DialogView` fn-pointer
  and sees `&State` and nothing else. A menu is not a label in a width budget, so it does not use the command name
  FR-016 puts on rows (Clarifications 2026-08-18)
- [X] T032a [US1] Put the split affordance's **decision** in a render-free module: `crates/micold-client/src/features/session.rs` resolves a press-target (primary vs secondary) plus the availability set into an intent — start-with-default, open-the-list, or nothing-to-offer — turning T022's failing assertions green. Principle I's GUI exception does **not** cover branching, so this must exist before T033 and `ui/sidebar.rs` must only dispatch to it (T022 is the gate)
- [X] T033 [US1] Add the per-session override as a **split affordance**, as a shared component in `crates/micold-client/src/ui/material/` with a chainable builder terminating in `.into()` (Principle VIII — not assembled inline in the sidebar), consumed from `crates/micold-client/src/ui/sidebar.rs::row_actions_cluster`: pressing the primary half starts the default in one interaction exactly as today, the secondary half opens the list of available providers **named by
  `display_name()`**, not by command name (FR-006, Clarifications 2026-08-18), and the secondary half
  is absent entirely when fewer than two CLIs are available — counted from **T014a's field**,
  threaded into `row_actions_cluster` as an argument rather than reached for. Keep it within the
  location row's existing height: that row is already one line or two depending on tags
  (`features/sidebar.rs::row_heights`), and a control taller than the icon buttons beside it puts the
  scroll arithmetic out of step with what is drawn. It renders T032a's intent and decides nothing itself (SC-001, Clarifications 2026-08-16)
- [X] T033a [US1] Register the T033 component in the component showcase (`crates/micold-client/src/showcase/`) and satisfy `crates/micold-client/tests/showcase_completeness.rs` and `crates/micold-client/tests/material_builder_api.rs` — **and
  `crates/micold-client/tests/showcase_captions.rs`**, which reads the same catalogue and fails in
  both directions on `interactive` vs `live`, so the entry must declare which states are live rather
  than being added bare. The component's anatomy assertions go in
  `crates/micold-client/src/ui/material/anatomy_size.rs`, not `tests/`, because `material` is
  `pub(crate)`. Shares the catalogue and both gates with **T067**, which registers US3's components —
  never `[P]` with it
- [X] T034 [P] [US1] Document the **Default AI CLI** preference in `docs/user-guide/settings.md`, including that a default naming an uninstalled CLI is kept rather than silently repaired
- [X] T035 [P] [US1] Document choosing a CLI per session in `docs/user-guide/worktrees-and-sessions.md`, including that the default applies when nothing is chosen and that changing it leaves open sessions alone

**Checkpoint**: US1 is independently testable — both CLIs start from the application, from a default
and from an override. This is the MVP.

---

## Phase 4: User Story 2 - The choice sticks (Priority: P1)

**Goal**: A session is a Copilot session for as long as it exists — across application restart,
daemon restart and reboot — resuming its own conversation; pre-feature sessions load unchanged; and
Copilot sessions started outside the application are discovered, while closed ones stay closed.

**Independent Test**: Start a Copilot session, have a short conversation, quit, reopen, select it —
the Copilot conversation is there (quickstart §B B3, B5).

### Tests for User Story 2 (MANDATORY — write first, observe RED)

- [X] T036 [P] [US2] Extend `crates/micold-core/tests/store_roundtrip.rs`: `provider` survives save → load; a session file written **without** the field loads as `ClaudeCode` (FR-013); and `schema_version` does **not** move (research R8). This pair is SC-003
- [X] T037 [P] [US2] Extend `crates/micold-core/tests/store_roundtrip.rs` so an unknown provider string is a **load error** for that project file, not a silent fallback to `ClaudeCode` (data-model round-trip table) — and that the store's existing malformed-file recovery covers it
- [X] T038 [P] [US2] In `crates/micold-core/tests/copilot_provider.rs`, assert index parsing against the T001 fixture: ids listed in order; a `schemaVersion` other than `1` contributes nothing; a truncated, empty or absent file contributes nothing — **never an error** (contract "Sessions recorded for a working directory")
- [X] T039 [P] [US2] In `crates/micold-core/tests/copilot_provider.rs`, assert `recorded_session_ids` derives `sidebar-sessions-state/<sha256_hex(cwd)>.json` purely — no I/O in the derivation — and that the hash matches the T003 recorded vector
- [X] T040 [P] [US2] In `crates/micold-core/tests/copilot_provider.rs`, assert `has_recorded_conversation` is true exactly when `session-state/<uuid>/events.jsonl` exists — a session directory without it was opened and never used (contract "Recorded-conversation detection")
- [X] T041 [P] [US2] In `crates/micold-core/tests/copilot_provider.rs`, assert `mark_archived` writes `session-state/<uuid>/micold.archived`, `is_archived` reads it, and a failure to write is swallowed rather than propagated (FR-015)
- [X] T042 [P] [US2] Extend `crates/micold-core/tests/session_reconciliation.rs` so its reconcile
  helper runs over **every** registered provider: a Copilot id in the per-cwd index with no
  application record becomes a listed Copilot session (FR-014), one with a `micold.archived` marker
  is never reconstructed (FR-015), and an index of ~250 ids proves **nothing is capped or dropped by
  age** (FR-014 as amended 2026-08-16). Be honest about what this file is: a **mirror**, by its own
  module doc, of a function that no longer exists. It is a cheap place to pin the *rules*; it is
  **not** the gate on FR-014, and it must not be counted as one. T042a is the gate
- [X] T042a [US2] Gate FR-014/FR-015 on the **real** entry point — R15 settled it, so this is
  `crates/micold-daemon/tests/`, driving the discovery function T050 adds and the attach arm that
  calls it, against a fixture store. Not the mirror in `session_reconciliation.rs`. Assert it runs on
  a **reopen** and not only a first open, since a first-open-only rule would never surface the second
  session a user starts outside the application; asserting a previously-unknown Copilot id in a location's index is listed as a Copilot
  session, an archived one never returns, a Claude Code id is judged only by `ClaudeProvider`, and
  ~250 ids all survive. Assert the **ordering R15 depends on** too: the catalog's own ids are
  subtracted before any `micold.archived` stat, so a location holding hundreds of already-known
  conversations does no per-conversation filesystem work. And assert **idempotence** — a second open
  adds nothing, because a discovered session's `SessionId` is the CLI's own uuid. This MUST fail on
  the current tree because the function does not exist yet. It is the gate on T050
- [X] T043 [P] [US2] Add a test to `crates/micold-core/tests/session_reconciliation.rs` for the spec's colliding-id edge case: the same uuid present in both providers' stores must resolve by the session's **persisted** provider, and must never re-derive a known session's provider from disk (data-model invariant 3)
- [X] T044 [P] [US2] Extend `crates/micold-daemon/tests/session_archive_durable_marker.rs` so closing a Copilot session writes its marker through the seam rather than through `ClaudeProvider`

  **Done 2026-08-25.** A fourth scenario inside the file's single `#[test]`; `COPILOT_HOME` joins
  `CLAUDE_CONFIG_DIR` as a process-global this file owns, which is why the scenarios keep sharing
  one function. Only one of the three archive paths needed the new provider — all three funnel into
  `catalog.rs::mark_archived_durable`, and what genuinely differs between them is the *cwd*, which
  scenario 2 already covers. The negative is asserted twice: `!ClaudeProvider.is_archived` for the
  one path `claude` probes, and a recursive listing of the whole `claude` store before and after
  for everything else.

  `mark_archived_durable` already took the provider from the session record (T047's work), so this
  **passes on arrival**. Two probes say it is load-bearing rather than decorative:

  - provider reverted to a hardcoded `ClaudeProvider` → fails at the first assertion, the marker
    missing from Copilot's own store.
  - seam left correct, but a stray file written into the `claude` store under a name
    `ClaudeProvider::is_archived` does not probe → the first two assertions pass and the
    before/after listing fails, which is exactly the case it exists for.
- [X] T045 [P] [US2] Extend `crates/micold-daemon/tests/catalog_adoption.rs` so a restored Copilot session resumes with `--resume=<uuid>` and not a fresh `--session-id` when a conversation is recorded (FR-008)
- [X] T045a [P] [US2] Extend `crates/micold-daemon/tests/session_survival.rs` so a Copilot session survives a **daemon** restart on the right provider and resumes its own conversation — the leg of FR-012 that neither the store round-trip (T036) nor the application-restart pass (quickstart B3) reaches

  **Done 2026-08-25.** The gap turned out to be narrower and sharper than "a restart": T036,
  `catalog_adoption.rs` and `set_wide_provider_decisions.rs` all start from a `projects.json` a
  *test* hand-wrote, so none of them says the **daemon's own write** carries the provider — and
  `create_session` is what runs when the user clicks "+". So the test creates both sessions through
  the daemon, drops it, and loads a second one from the same store paths: providers preserved, both
  presented `InterruptedResumable` because each was asked of its own CLI, and the argv each reloaded
  record implies is its own (`--resume=<uuid>` beside `--resume <uuid>`).

  Probe: `Catalog::create_session` reverted to naming no provider (`AiCli::ClaudeCode` regardless of
  the caller) → `present_interrupted_resumable_at_startup()` returns 1 instead of 2, which is the
  failure described — the Copilot session comes back as a Claude one, `claude` has never seen its
  id, and it stays `Idle`, indistinguishable from created-and-never-used.

  Two limits, both deliberate: the restart is a dropped `DaemonState` reloaded from the same files,
  not a forked binary (`daemon_singleton.rs` and `version_recovery.rs` own that, and neither needs a
  CLI installed); and the resume is asserted as argv rather than as a spawn, because this suite
  never spawns either CLI — `session_start.rs` states why.
- [X] T046 [P] [US2] Add a test to `crates/micold-core/tests/copilot_provider.rs` and
  `crates/micold-daemon/tests/session_start.rs` for the spec's removed-store-entry edge case:
  resuming an id Copilot no longer has reports **`WireLifecycle::Failed { reason, attempts }`** with
  a reason naming the CLI and the lost conversation, **starts nothing**, and does not begin a fresh
  conversation under the old session's identity (Clarifications 2026-08-16). Assert against the wire
  type, not `session::SessionLifecycle` — the domain enum's `Failed` is a **unit variant** meaning
  "auto-restart gave up after repeated quick failures" and has nowhere to put a message

  **Done 2026-08-25.** Two tests, one per layer. The core one characterises what the edge case
  leaves on disk — a per-cwd index still listing an id whose `session-state/<uuid>` directory is
  gone, so `has_recorded_conversation`, `read_title` and `is_archived` all answer "no" while
  discovery still names it. The daemon one stages the removal in the order it happens: record a
  conversation, let `present_interrupted_resumable_at_startup()` offer the session (the state a user
  clicks Start from), *then* delete it. Staging matters — the case is an entry that was **removed**,
  and the refusal is gated on the session having been offered.

  Read from `catalog_snapshot()`, not `sessions_for()`: the reason is runtime state that
  `overlay_live_summaries` projects onto the durable record on the way out, and `sessions_for` is
  documented as the one snapshot path that does not run it, so it reported `InterruptedResumable`
  and the first version of the test failed against a working implementation.

  Probes, all against the committed code:
  - the refusal deleted → the test fails on `result.is_err()`, so it does drive the new code;
  - the `Err` kept but `start_failures.insert` dropped → still fails: erroring silently is not
    reporting, and the row would go red with nothing to read;
  - `display_name()` → `command()` in the reason → fails on the register assertion, which is the
    difference between "GitHub Copilot no longer has this conversation" and something that reads
    like a shell error.
  - **and one that did not fail.** Dropping `plan.resumable` from the condition — pre-checking every
    `LaunchMode::Resume` — passed the entire workspace. The prediction that `catalog_join.rs` would
    catch it was wrong: the session it resumes is a **shell**, so the AI-CLI branch never runs. That
    gate was carrying real behaviour on nothing but an argument, so T046 also adds
    `a_session_that_never_recorded_a_conversation_is_not_told_its_conversation_is_gone`, and the
    re-probe fails there with the message that made the case: a session created and never used being
    told a conversation it never had is gone.

  What it does not model: the client's rendering of the reason (T032a's), and any *fresh* start
  under the same id — `LaunchMode::Fresh` is untouched, so closing the session and starting a new
  one remains the way out.
- [X] T046a [P] [US2] Extend `crates/micold-daemon/tests/session_start.rs` for the already-attached
  case: resuming a conversation another process holds attempts the resume like any other, and if the
  CLI refuses or exits immediately the session reports `WireLifecycle::Failed { reason, .. }` and
  starts nothing (FR-008 as amended 2026-08-18). Assert the **negative** just as hard: no liveness
  probe, no process scan, no lock file — nothing that tries to work out beforehand whether a
  conversation is in use. Neither CLI exposes a marker to test against, so such a check would be a
  guess, and a wrong "in use" blocks a resume the user is entitled to

  **Done 2026-08-25.** No implementation followed, and that is the finding: the daemon already has
  no liveness detection to remove, so the tests exist to keep it that way. "Another terminal holds
  it" is not a state that can be set up, because nothing on disk records it — a held conversation
  and a free one are the same bytes — so the setup is an ordinary offered Copilot session and the
  assertions are the negatives.

  The stub `copilot` now records the argv it is handed, which turns the negatives into things a test
  can actually say: run **once** (not a probe run first), with `--resume=<uuid> --no-remote` (not a
  different command because the conversation might be busy), and Copilot's whole store byte-for-byte
  unchanged across the start (not a lock, not an in-use sentinel). The refusal half makes the stub
  exit immediately with a message, which is what a CLI that will not attach twice does, and asserts
  it is treated as any other immediate exit: supervised, retried inside the budget, then `Failed`
  with the process dropped.

  Probes: a pre-flight "looks busy" refusal → the attempt assertion fails; a lock file written into
  the store before the spawn → the whole-store comparison fails (the reason it is a comparison and
  not a path check); the AI-CLI respawn removed → the refusal test fails to settle.

  That last probe turned up something outside this task and worth writing down: when a **respawn**
  fails to spawn at all, `respawn_primary` counts one crash and then drops the session from the live
  registry, so the next supervision cycle has nothing to observe and the record stays `Restarting`
  forever — no process, no report, and a client that renders "restarting" indefinitely. Not
  reachable through this feature's paths (the CLI is checked at `start_session`, and a respawn that
  fails needs the binary to vanish mid-loop), so it is recorded rather than fixed here.

  `PATH` and `COPILOT_HOME` are process-global and this binary threads its tests, so the stub now
  holds a mutex for its lifetime — two of these tests running at once would restore each other's
  `PATH` and leave the survivor pointing at a deleted directory.

### Implementation for User Story 2

- [X] T047 [US2] Add `#[serde(default)] provider: StoredAiCli` and the `StoredAiCli` mirror enum to `crates/micold-core/src/store.rs`, deliberately without `#[serde(other)]`, and map it to/from `AiCli` on load and save (FR-013,
  SC-003). "On load" is `StoredSession::into_session` (`store.rs:207`), which is where the mapped
  value is handed to `Session::restored` — as a **constructor argument**, alongside the other
  identity fields, not assigned afterwards the way `archived` is on the line below it (T012)
- [X] T048 [US2] Implement `CopilotProvider::recorded_session_ids` in `crates/micold-core/src/provider.rs`: derive the index path from `sha256_hex(cwd)` using `crates/micold-core/src/protocol/hashing.rs`, parse `sessionIds`, and contribute nothing on any read or parse failure
- [X] T049 [P] [US2] Implement `CopilotProvider::has_recorded_conversation` and the `mark_archived` / `is_archived` pair in `crates/micold-core/src/provider.rs`, per the contract's marker section
- [X] T050 [US2] **Write** the discovery pass FR-014 requires, in the place R15 settled: a new
  function in `crates/micold-daemon/`, called from the `ClientMsg::AttachProject` arm
  (`server.rs:378-397`) between `refresh_worktrees_and_send`'s enumeration and the catalog snapshot,
  and running inside that same `spawn_blocking` hop rather than adding a second one. This is new
  code, not an edit to an existing provider-agnostic path, because none exists. For each
  location, ask **every** registered provider for the ids it has recorded there, list an id the
  application has no record of as a session of the provider whose store it came from, skip anything
  that provider reports archived (FR-015), and **never re-derive the provider of a session already
  known** (data-model invariant 3). It runs on **every** project open and reopen, and its work is
  **per location** — one index read or one directory listing each — never per conversation, so a
  worktree with hundreds of recorded conversations costs what one with three costs. Hold that by
  **ordering**: subtract the catalog's known ids for the location first, then check
  `is_archived` only on what is left, so the per-id filesystem probe never runs over conversations
  the application already knows about (R15). Give each discovered session the CLI's own uuid as its
  `SessionId`, so a reopen is a no-op rather than a duplicate. Resolve each provider's `config_dir()`
  independently — one returning `None` must not suppress the other's contribution. It ends in a
  `Catalog` write, since the daemon is `projects.json`'s single writer. T042a is the gate
- [X] T050a [US2] Implement the removed-store-entry refusal T046 drives, in
  `crates/micold-daemon/src/state.rs::start_session`: after the FR-010 availability check and before
  the spawn, ask the session's own provider whether the conversation is still recorded, and on "no"
  record a reason and return `Err` instead of spawning. Gated on the record's lifecycle being
  `SessionLifecycle::InterruptedResumable` — carried on `SpawnPlan` as `resumable`, read under the
  same lock as the rest of the plan — so it fires only where absence is news, and reported through
  the path FR-010 already uses (`start_failures` → `overlay_live_summaries` → `WireLifecycle::Failed
  { reason, attempts: 0 }`). An unresolvable `config_dir()` is ignorance, not evidence: attempt the
  resume and let the CLI answer

  **Done 2026-08-25.** Missing from this section until now — US2's implementation list went from the
  discovery pass (T050) straight to documentation, so the behaviour T046 tests had no task. Nothing
  else on the resume path changed: `respawn_primary` spawns directly and never enters
  `start_session`, so crash-restart is untouched, and `server.rs` maps `SessionCreate` to `Fresh`,
  so only an explicit resume is pre-checked

- [X] T051 [P] [US2] Document in `docs/user-guide/worktrees-and-sessions.md` that a session's CLI is fixed for its lifetime, survives restart with its conversation, that sessions started outside the application are discovered per CLI, and that closing one is durable

**Checkpoint**: US1 + US2 both work. A Copilot session is a first-class, persistent, rediscoverable
session.

---

## Phase 5: User Story 3 - I can see which CLI a session is running (Priority: P2)

**Goal**: The sidebar names each session's CLI in text, labels a Copilot row with Copilot's own title
once it has one, and shows a busy/idle badge — identical for both CLIs. An open session names its
CLI on its own terminal bar, on the pinned AI tab (FR-016a).

**Independent Test**: With one session of each kind in a project, the sidebar distinguishes them
without hovering or opening; opening each shows its CLI named on the terminal bar; and a Copilot
session mid-response reads as working **within one second** (quickstart §B B4, SC-004, SC-005).

### Tests for User Story 3 (MANDATORY — write first, observe RED)

- [X] T052 [P] [US3] In `crates/micold-core/tests/copilot_provider.rs`, assert `read_title` against the T001 fixtures: `name:` present → `Some(title)`; absent → `None` (label stays `Pending`); plain, quoted and colon-containing values all parse; an unreadable file yields `None` and never an error (FR-017)
- [X] T053 [P] [US3] In `crates/micold-core/tests/copilot_provider.rs`, assert `activity_source`
  returns `ActivitySource::EventLog { path }` at `session-state/<uuid>/events.jsonl`, and that
  `ClaudeProvider` returns the **payload-free** `ActivitySource::Hooks`. Assert the asymmetry
  deliberately, because it is the part a reader will want to "tidy": the daemon writes the hook
  settings file itself (`State::hook_settings_file` → `receiver.prepare_settings(id)`, embedding a
  port chosen at daemon start and a per-session token), so no pure `(config_dir, cwd, id)`
  derivation in `micold-core` can produce that path
- [X] T054 [P] [US3] Add `crates/micold-daemon/tests/copilot_activity.rs` mapping the T001 fixture log to signals: `user.message`/`assistant.turn_start`/`tool.execution_start` → `Working`, `tool.execution_complete` → no change, `assistant.turn_end` → `AwaitingInput`, `permission.requested` → `AwaitingInput`, `session.shutdown`/`session.error` → `Ended { reason }`
- [X] T055 [P] [US3] In `crates/micold-daemon/tests/copilot_activity.rs`, assert **unknown event types are ignored, not rejected** (the two off-contract types from T001), and that a malformed JSONL line is skipped rather than ending the tail
- [X] T056 [P] [US3] In `crates/micold-daemon/tests/copilot_activity.rs`, assert a dangling
  `assistant.turn_start` from a dead process does not leave the badge `Working` forever, and that an
  absent `events.jsonl` yields `Unknown` rather than a guess (FR-018's conservatism clause)
- [X] T056a [P] [US3] Assert the badge's **scope**: a session this application discovered (FR-014)
  but is not supervising reads `Unknown` — never idle, never working — and **no watch is opened for
  it**, however many such sessions a project holds (FR-018 and SC-006 as amended 2026-08-18). The
  second half is the one that matters: a project with hundreds of discovered sessions must schedule
  no observation work at all, so assert the absence of a watcher per discovered session, not just the
  signal value
- [X] T057 [P] [US3] Extend `crates/micold-daemon/tests/activity_pipeline.rs` to prove the `Activity`
  state machine's transitions are unchanged. Correct the framing while you are here: it is not that
  "only the event source differs" — a Copilot session has **two** sources, because the braille-spinner
  path is shared and not provider-conditional. `micold-daemon/src/terminal.rs:141` scans every PTY
  session's OSC-0 titles for any codepoint in U+2800..=U+28FF and raises `SpinnerObserved`, and a
  Copilot TUI drawing a spinner will trip it. That is harmless by construction — `SpinnerObserved`
  only ever moves `Unknown → Working` and is a no-op from every other state (H1a/A1a) — so the two
  sources cannot contradict each other. Assert that, rather than asserting a single-source claim that
  is not true

  The FSM's own transitions needed nothing here: `micold-daemon/src/activity.rs::mod tests` already
  walks the whole table event-by-event from every state, including `h1a_spinner_from_unknown_only`
  from Unknown, Working, AwaitingInput and Ended. Re-asserting that in an integration test would
  have duplicated it in a slower place. What was untested is the *pipeline* around it for a Copilot
  session, and that is where the wrong framing lived, so that is what the two new tests in
  `activity_pipeline.rs` pin.

  `a_copilot_session_is_watched_by_its_event_log_and_scanned_for_spinners_like_any_other` asks the
  provider for the session's activity source (an `EventLog`, source one), then drives a braille OSC-0
  title through a real PTY on that same Copilot session and asserts the badge reaches `Working` —
  source two, reached without anything asking which CLI is running.
  `a_copilot_event_log_and_the_shared_spinner_scan_cannot_contradict_each_other` puts both live on
  one session in the order that could produce a contradiction: `DaemonState::open_event_log_tail`
  watches a real `events.jsonl`, an appended `assistant.turn_end` moves the badge to `AwaitingInput`
  through the tail, and only *then* is the spinner drained. It stays `AwaitingInput`. The test
  asserts the title actually landed as well, because without that the activity assertion would pass
  for free on a spinner that never arrived.

  `catalog_with_session` now takes the CLI as a parameter — it hardcoded `AiCli::ClaudeCode`, which
  is exactly the single-source assumption this task is correcting. `COPILOT_HOME` is process-global,
  so the two new tests take a `CopilotHome` guard that serialises them and clears the variable on
  drop.

  Probed, both from a green tree, each mutation reverted before the next:

  - **P1 — the spinner scan becomes provider-conditional.** `drain_signals` gated on the session's
    provider being `ClaudeCode`. Only
    `a_copilot_session_is_watched_by_its_event_log_and_scanned_for_spinners_like_any_other` failed
    (5 passed, 1 failed) — the existing claude spinner tests cannot see this, which is the whole
    point of adding a Copilot one.
  - **P2 — the event-log source is severed.** An early `return` at the top of
    `open_event_log_tail`. Only
    `a_copilot_event_log_and_the_shared_spinner_scan_cannot_contradict_each_other` failed, on "the
    event log's turn_end must reach the activity machine". `copilot_activity.rs` stayed green
    through it (11 passed): its tests drive `EventLogTail::open` directly and its watch-site test
    greps the source text, so *nothing else in the suite notices the daemon no longer opening a
    watch*. That test is the only end-to-end coverage of that wiring.

  The framing was corrected in both places that carried it: this file's module doc now states the
  two sources and why they cannot disagree, and `copilot_activity.rs`'s "a second source" paragraph
  now says second, not only, and points here for the proof.
- [X] T058 [P] [US3] Extend `crates/micold-client/tests/features_sidebar.rs` and `crates/micold-client/tests/sidebar_tree.rs` so each session row carries a **short text label**
  naming its CLI by its **command name** — `claude`, `copilot`, not "Claude Code"/"GitHub Copilot",
  and not a colour, a glyph alone or a tooltip (FR-016 as amended 2026-08-16 and 2026-08-18) — and so the activity badge is rendered identically for both providers, with no per-provider styling and no "less certain" variant (FR-018). **Also assert the row is still one line.**
  `crates/micold-client/src/features/sidebar.rs::row_heights` hardcodes "a session row is always one
  line" and its own doc calls that the one place in the sidebar where a wrong answer is silent — the
  computed scroll target drifts from the rendered rows and nothing complains. This file is already
  the only test over `row_heights`/`scroll_target`, so the assertion costs a line here and has
  nowhere else to live
- [X] T058a [P] [US3] Add a test to `crates/micold-client/tests/terminal_bar_stability.rs` (or a
  sibling) asserting the bar's **pinned AI tab** carries the session's **command name** (`claude`,
  `copilot`) as text beside its glyph, in both of a session's panes (FR-016a, US3 scenario 4).
  Landed as two halves, because neither alone is enough: `ui/terminal.rs`'s unit test
  `the_bar_reads_the_session_own_cli_by_command_name` proves `session_provider` answers with the
  session's own CLI and falls back to the default for a session the projection has not caught up
  with, and `terminal_bar_stability.rs::the_pinned_ai_tab_names_the_sessions_cli` proves the tab
  still *asks* it and still asks in the `command()` register. A label rewritten to a literal or to
  `display_name()` leaves the first green — that is the whole reason the second is a source gate.
  Retargeted from the mode toggle when feature 027 deleted it; see FR-016a's amendment
- [X] T058b [P] [US3] Extend `crates/micold-daemon/tests/copilot_activity.rs` so the badge reaches
  `Working` within **1 second** of a `user.message` line being appended, on the event-driven path
  (SC-005 as tightened 2026-08-16) — and prove it was the **log** that moved it. The shared
  spinner-title path (T057) can drive the same `Unknown → Working` transition from a Copilot TUI's
  own animation, so a test that only watches the badge would go green with the watch path dead. Drive
  the event path in isolation, with no title traffic, or assert the transition's provenance
- [X] T058c [P] [US3] **SC-009** — extend `crates/micold-client/tests/sidebar_tree.rs` with a
  location holding ~250 discovered sessions — **in `sidebar_tree.rs`, not `layout_snapshot.rs`**: a snapshot state
  built inline is exactly what `crates/micold-client/tests/layout_coverage_registry.rs` fails on, so
  putting it there would mean registering a new covered state rather than writing a test: the tree builds, every row is present, and the pass costs no more **per
  row** than a location holding three — no per-session I/O, no per-session watcher, and nothing that
  grows faster than the list itself. Structural, not timed: SC-009 is deliberately not a stopwatch
  bound, for the reason SC-006 isn't one either. This is the other half of the "long CLI history"
  edge case, whose uncapped half T042 covers, and with per-open discovery (2026-08-18) it is a
  routine size rather than an outlier
- [X] T058d [P] [US3] Extend `crates/micold-client/tests/layout_text_overflow.rs` so a narrow sidebar row degrades in the declared order — actions and CLI label hold, the **title** ellipsizes first — and the CLI label is never the thing that disappears, since FR-016 makes it the identification (G4's width budget)

  `layout_text_overflow.rs` reports what *spills*, and a row under width pressure does not spill —
  it shortens. `Ellipsized` rewrites its own content before shaping, so a title that gave way
  arrives at the renderer as a different string rather than as an overflow, and the existing gate
  cannot see it at all. So `support::layout` gained `painted_text`: the same measurement
  `text_overflows` filters, unfiltered. `text_overflows` is now a call to it, and the
  `LAYOUT_OVERFLOW_DEBUG=1` escape hatch is that same flag.

  `a_narrow_session_row_shortens_the_title_and_never_the_cli_label` draws one Copilot session row at
  `SIDEBAR_DEFAULT_WIDTH` and again at `SIDEBAR_MIN_WIDTH` and compares what was painted. Measured:
  the title falls from `"rewriting the provider s…"` to `"r…"`, `"copilot"` stays whole at both
  widths (36.5px wanted, 36.5px allowed), the same 33 strings are painted at both, and nothing at
  the minimum width exceeds its clip. That is the declared order holding — and worth writing down
  plainly, since at 180px the name is one character and an ellipsis. That is the width budget G4
  describes, not a defect this task introduced: the annotation is 36.5px of a 172px column, and the
  alternative — letting the identification ellipsize alongside the name — is what FR-016 rules out.

  Probed from a green tree, each mutation reverted before the next:

  - **P1 — the annotation degrades with the name.** `tree_view.rs` pushes the annotation as an
    `Ellipsized` instead of a natural-width `Text`. The row then paints `"re…"` and `"co…"`: the
    identification is exactly what the narrow row drops. Only this test caught it as such.
    `features_sidebar.rs` and `sidebar_tree.rs` passed — they assert the label is *in* the row, not
    that it survives width pressure — and `no_text_is_drawn_wider_than_its_clip` passed too, since
    nothing spilled. `layout_snapshot`'s fixture parity did fail, but it fails for any geometry
    change and says only "the layout differs"; regenerating it is the normal response, which is
    precisely how this regression would pass unnoticed.
  - **P2 — the name stops ellipsizing.** The row label pushed as a plain `Text`. The full title is
    painted, and this test fails on "the title must ellipsize rather than spill or vanish".
    `no_text_is_drawn_wider_than_its_clip` passed through it: no covered state carries a session
    title long enough to spill, so the gate this file was built around cannot see it either.
- [X] T058e [P] [US3] Assert the two naming registers stay apart, in whichever of
  `crates/micold-client/tests/features_sidebar.rs` / `features_settings.rs` is the cheaper home: rows
  and the terminal bar carry `command()`, the Settings select and the override list carry
  `display_name()`, and a failure message names the CLI by `display_name()` (FR-006, FR-010, FR-016,
  FR-016a). One register leaking into the other is the drift this feature is most likely to produce,
  since both strings hang off the same provider
- [X] T059 [P] [US3] Extend `crates/micold-client/tests/session_title_sync.rs` so a Copilot title reaches the row through the seam, and a missing one falls back to the existing label without failing the session
- [X] T060 [P] [US3] Add `crates/micold-daemon/tests/copilot_activity_is_event_driven.rs` asserting **structurally** that *this
  application's* watch path schedules no polling timer, no periodic wakeup, no per-idle-session work
  and **no per-discovered-session work** — the absence of a timer in our code, not a measurement of one, and explicitly **not** an assertion about the watch crate's internals, whose fallback on a filesystem with no native notification FR-019 permits (SC-006, Clarifications 2026-08-16). Also assert no debouncer wrapper is in the path, since that would be a timer of ours by adoption
- [X] T060a [P] [US3] Gate the client's half of T065: extend `crates/micold-client/src/main.rs`'s
  own `#[cfg(test)]` module — which already drives `reconcile_catalog` four times — so a
  `SessionSummary` carrying `Copilot` materialises as a **Copilot** session, and one carrying
  `ClaudeCode` as a Claude one. This is the only path a daemon-reported session takes into the client
  model: every session discovered by R15's pass, and every session at all after a client restart. It
  has to live in that module because `reconcile_catalog` is a free function in the GUI binary and no
  integration test can link it — the same constraint T030a records. Without it the provider silently
  defaults and the row label, the AI tab and the split affordance all read `claude` for a
  Copilot session, with every other test still green (FR-012, FR-016, FR-016a)

### Implementation for User Story 3

- [X] T061 [US3] Implement `CopilotProvider::read_title` in `crates/micold-core/src/provider.rs` with a purpose-built single-scalar reader for `name:` in `workspace.yaml` — plain and quoted forms only, no YAML dependency (research R4)
- [X] T062 [US3] Implement `CopilotProvider::activity_source` in `crates/micold-core/src/provider.rs`
  and give `ClaudeProvider` the payload-free `Hooks` arm. Feature 010's mechanism is preserved
  byte-for-byte precisely *because* the variant carries nothing: the daemon keeps choosing, writing
  and passing the settings file exactly as it does today, and the provider only names which
  mechanism applies
- [X] T063 [US3] Add the Copilot event → **`ActivityEvent`** mapping to
  `crates/micold-daemon/src/activity.rs` as a pure function over parsed lines, leaving the state
  machine itself untouched. `ActivityEvent`, not `HookKind`: the turn events map to
  `Hook(HookKind::{UserPromptSubmit, PreToolUse, PostToolUse, Stop, Notification})`, but
  `session.shutdown` / `session.error` map to `Ended { reason }` (`activity.rs:47`), which is a
  sibling variant of `ActivityEvent` and has no `HookKind` to express it. A mapping typed at
  `HookKind` cannot satisfy T054, which asserts exactly that terminal state
- [X] T064 [US3] Tail `events.jsonl` for running `ActivitySource::EventLog` sessions in `crates/micold-daemon/src/supervisor.rs`, driven by the T003a watch crate — woken by a change notification and by nothing else, with **no timer of our own** — and drop the watch when the process ends. Open a watch **only** for sessions this
  application is supervising: a discovered-but-unsupervised session gets none, whatever its event log
  contains (FR-018, SC-006, Clarifications 2026-08-18). T056a is the gate. Cap the watcher's coalescing latency at **250 ms** so SC-005's one-second bound holds on macOS, where the default FSEvents latency is otherwise high enough to breach it; do not use a debouncer wrapper, which reintroduces a timer. The crate's own poll fallback on a filesystem with no native notification is permitted by FR-019 as scoped (depends on T003a)
- [X] T065 [US3] Populate and consume `CatalogSnapshot`'s provider field — the wire shape itself landed in T029, so this is the daemon filling it from the session record in
  `crates/micold-daemon/src/catalog.rs` (one line in `session_summary`, line ~560) and the client
  reading it in `reconcile_catalog` (`main.rs:2303`) where the session is constructed — T060a is the
  gate — with **no change to `protocol/messages.rs` and no second version bump** (T020 fails if the hash moves again)
- [X] T066 [US3] Render the CLI's **command name** as the row's short text label in
  `crates/micold-client/src/ui/sidebar.rs::session_tree_item` — `command()`, not `display_name()`,
  which stays reserved for menus and messages (FR-006, FR-010, Clarifications 2026-08-18); if the existing `icon_label`/`tag` primitives do not suffice, add the new one to `crates/micold-client/src/ui/material/` with a chainable builder terminating in `.into()` (Principle VIII), not inline in the sidebar. Declare the row's width budget explicitly: actions and CLI label are fixed, the title takes the remainder and ellipsizes via `ui/material/ellipsized.rs` — the identification must not be what a narrow row drops (T058d is the gate). The label changes the
  row's *content*, never its *height* — `features/sidebar.rs::row_heights` says a session row is one
  line and the scroll arithmetic believes it (T058)
- [X] T066a [US3] Give the bar's bottom-right control its CLI's **command name** in `crates/micold-client/src/ui/terminal.rs`.
  **Re-landed on the pinned AI tab.** As originally written this task named the AI-CLI mode toggle,
  and it was built as `material::LabelledToggle` — a labelled toggle that satisfies the note below.
  Feature 027 then deleted the toggle (its FR-001) and the pinned AI tab took that corner, so the
  name moved with it: `pinned_ai_tab` now labels itself `material::IconLabel::new(Icon::AiCli,
  session_provider(..).provider().command(), ..)`, which is the existing shared component for a
  glyph and a word (and the only permitted one — `tests/composite_call_sites.rs` forbids a feature
  module assembling `row![Glyph, Text]` itself). `LabelledToggle` outlives its call site in the
  showcase; it is a correct component with nothing currently posing it, not a mistake to undo here.
  The original note, which still holds for any labelled *button*: `IconButton` is not merely icon-only by
  habit — its module contract says so and `crates/micold-client/src/ui/material/anatomy_size.rs`
  asserts a disabled one *is sized by its glyph* and a compact one *is sized by its glyph, not by the
  room it is given*. So **do not add `.label(...)` to `IconButton`**; compose
  `ui/material/icon_label.rs` inside the button, or introduce a distinct labelled-toggle component.
  Either way it is a **shared component with a chainable builder**, not an inline one-off
  (Principle VIII), its anatomy tests belong in `src/ui/material/anatomy_size.rs` (the `material`
  module is `pub(crate)`, so `tests/` cannot construct one), and it goes in the showcase (FR-016a)
- [X] T067 [US3] Register whatever shared component T066 and T066a added or changed in the component showcase (`crates/micold-client/src/showcase/`) and satisfy `crates/micold-client/tests/showcase_completeness.rs`
  **and `crates/micold-client/tests/showcase_captions.rs`** (declare the entry's `interactive` flag
  and its live states; a bare entry fails). Same catalogue and same gates as **T033a** — sequence
  them, do not run them in parallel
- [X] T067a [US3] Regenerate the layout parity fixture for the states T066 and T066a move —
  `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client layout_snapshot` — and review the diff
  deliberately. A row that gains a CLI label and a terminal bar whose pinned AI tab gains text both
  change resolved geometry that `crates/micold-client/tests/layout_snapshot.rs` asserts against a
  **committed** fixture, and `layout_snapshot_regeneration.rs` exists precisely so the gate cannot
  heal itself. Without this the phase ends on a red gate that reads like a mystery; with it, the
  diff is the evidence the label landed where it was meant to (depends on T066, T066a)
- [X] T068 [P] [US3] Document what the sidebar shows per CLI — the row's CLI label, titles, and the activity badge — and that an open session names its CLI on its own terminal bar, on the pinned AI tab (FR-016a), in `docs/user-guide/worktrees-and-sessions.md`

- [X] T067b [US3] Look at the pinned AI tab in the real GUI, for both CLIs, both schemes, marked and
  unmarked. T067a regenerates a fixture of *rectangles*; a word inside a tab that was sized for a
  16dp label is a question about ink — crowding, truncation, glyph/label spacing, whether the label
  takes the marked tint with its glyph — and `layout_snapshot` is structurally blind to all of it.
  Run it with the repo's `visual-pass` skill (Xvfb + lavapipe, a seeded project, a private display),
  not by asking a person (depends on T066a, T067a)

  **Ran 2026-08-24**, Xvfb 1600×1400 + lavapipe (software Vulkan), against pinned binaries built
  from this branch — *not* a real display or a real GPU. Evidence:
  `evidence/FR-016a-ai-tab-names-its-cli.png`, four strips cropped at identical geometry: marked
  `claude` (dark), unmarked `claude` beside three numbered tabs (dark), marked `copilot` with a
  stopped badge (dark), unmarked `claude` beside three numbered tabs (light).

  Nothing wrong found. What the frames actually show, beyond "it fits":

  - The label takes the marked tint **with** its glyph and the muted one with it — `label_tint`
    doing what `tint` alone could not. An unmarked tab's word is clearly subordinate to the marked
    numbered tab beside it in both schemes.
  - The word sits on the numbered tabs' baseline, not near it.
  - **A stopped badge does not move the label.** The leading slot is a fixed 48dp whether or not it
    holds a badge, so the ring appears beside an unshifted word — the anatomy's promise, confirmed
    against the two frames rather than assumed.
  - The tab's real extent, which only the active indicator and the hover state layer reveal, is
    ~156dp for `claude` and ~4dp more for `copilot`; both **anchor their trailing edge**, so the
    longer name grows leftward and the strip's right edge does not move. The content centres inside
    it, because both 48dp slots are reserved.

  Not covered, and not claimable from this: mid-transition frames and perceived smoothness (a
  software rasteriser says nothing about frame pacing), and every §B step other than B4's
  terminal-bar half.

**Checkpoint**: US1–US3 work. A mixed project is legible at a glance.

---

## Phase 6: User Story 4 - Sensible behaviour when a CLI is not there (Priority: P2)

**Goal**: A CLI that is not installed is never offered and never silently substituted; a session on a
missing CLI is still listed and correctly identified, and says what is missing rather than appearing
to start.

**Independent Test**: With only one CLI installed, the session-creation surface and Settings do not
offer the other, and an existing session on the missing CLI reports the CLI by name (quickstart §B
B7, SC-008).

### Tests for User Story 4 (MANDATORY — write first, observe RED)

- [X] T069 [P] [US4] In `crates/micold-core/tests/copilot_provider.rs` and
  `crates/micold-core/tests/ai_cli_provider.rs`, assert `is_available()` is a live `PATH` resolution
  of `command()`: the **method** memoises nothing, is re-evaluated between calls when `PATH` changes,
  and persists nothing (research R11). Keep the claim scoped to the method. The client legitimately
  holds an in-memory snapshot of the answer (T014a), refreshed when the choice is offered, because a
  view cannot run a `PATH` probe per frame and SC-006 forbids scheduling one. "Never persisted" is
  R11's rule; "never held in memory anywhere" is not, and asserting the latter here would contradict
  T014a
- [X] T070 [P] [US4] **Not writable as specified — the requirement is covered elsewhere; decide
  whether to close it.** `available_providers()` cannot be driven by a fake any more: it filters
  `AiCli::ALL` through `AiCli::provider(which).is_available()`, and that registry is a static
  exhaustive match with no injection point. That is T011a's whole property — the lookup is total by
  construction — so the seam this task wanted to substitute at is deliberately gone.
  FR-006 is held in two places instead, both green: `micold-core/tests/copilot_provider.rs`'s
  `availability_is_a_live_path_lookup_that_remembers_nothing` drives the real predicate over a
  scratch `PATH` (installing and removing the binaries), and
  `micold-client/tests/features_session.rs` asserts that an unavailable CLI is never offered, from
  the availability set on `State`. What is *not* covered is the wiring between them —
  `Capabilities::available_providers()` itself — which is four lines with no branching

  Written rather than closed, and that last sentence is why: the two suites above hold the ends,
  and `available_providers()` returning `AiCli::ALL` unconditionally leaves both of them green.
  Confirmed, not assumed — under that mutation `copilot_provider.rs` passed 22/22 and
  `features_session.rs` 28/28. The requirement was covered; the wiring was not.

  The seam is gone, but the test does not need one. Availability *is* a `PATH` probe, so the honest
  way to drive these four lines is to give the process a scratch `PATH` and read back what the real
  `Capabilities::real()` offers. That also keeps the assertion pointed at the real thing: a
  substituted registry would have shown the filter forwards *a* predicate, not that the client's
  offer follows what is installed — the same argument `capabilities.rs`'s own module doc makes
  about the `from_parts` constructor it deleted.

  `the_offer_is_exactly_what_is_installed_now` walks four states over one scratch `PATH`: nothing
  installed → an empty offer (not a default guess); `copilot` alone → `[Copilot]`, the *second*
  variant, so a stub returning a prefix of `ALL` does not pass; every command in `AiCli::ALL`
  installed → all of `AiCli::ALL`, written over the array rather than over today's two names so a
  third CLI is covered the day it joins the registry; then `claude` removed → the offer shrinks,
  which is R11's "computed, never stored" holding at this level and not only at the provider's.
  A fifth step installs the variants backwards across two `PATH` directories and asserts the offer
  still comes back in the declared order — the settings select renders the list as it arrives, so
  its order is user-visible and must not depend on the shape of the user's machine.

  It is a unit test in the bin crate because `shell/` is declared in `main.rs`, not in the library,
  so `tests/` cannot reach `Capabilities` at all — the first attempt was an integration test and
  did not compile. One test function, because `PATH` is process-global and Rust runs tests on
  threads: the same arrangement, for the same reason, as `shell/persist.rs`'s boot-prune test.

  Probed from a green tree, each mutation reverted before the next:

  - **D1 — the offer ignores `PATH`.** `available_providers` returns `AiCli::ALL.to_vec()`. Fails
    at the first assertion ("an empty PATH must offer no CLI at all"), while both suites named
    above stay green — which is the whole case for this task not being closed.
  - **D2 — the offer comes back in `PATH` order.** The filtered list sorted by which `PATH` entry
    resolved it. The first four states pass unchanged (they install into one directory), and only
    the order assertion fails: `[Copilot, ClaudeCode]` where `[ClaudeCode, Copilot]` was declared.
- [X] T071 [P] [US4] Extend `crates/micold-client/tests/features_settings.rs` and `crates/micold-client/tests/features_session.rs` so neither the Settings select nor the per-session override offers an unavailable CLI; and so starting a session when the stored default is unavailable **says so and offers the available CLIs to choose from**, starts nothing until one is chosen, and leaves the stored default unrewritten (FR-002, FR-004 scenario 4, Clarifications 2026-08-16)

  Neither named file was the right home, and the reasons differ. The FR-004 half was *already*
  there: `features_session.rs` holds `an_unavailable_default_offers_the_choice_rather_than_starting_or_substituting`,
  which asserts all three of `start_intent(Primary) == OfferChoice`, that nothing starts, and that
  `default_ai_cli` is unrewritten on the way past — landed with T032a and green. And
  `features_settings.rs` builds no `State` at all; its own module doc says so.

  What was left uncovered is the other end. FR-006 is a rule about what the user is *shown*, and
  both surfaces read the availability set correctly today — so a test over `State` restates T075's
  own function and guards nothing. The risk is `settings_form.rs` reaching for `AiCli::ALL`: one
  token, every pure test still green, an uninstalled CLI in front of the user. That is only
  observable in what is drawn, so the tests are in a new render file,
  `crates/micold-client/tests/provider_choice_surfaces.rs` — open the control the way a person
  would and assert on the strings the renderer painted.

  Three tests, each claim asserted in **both** directions, because "GitHub Copilot was not painted"
  is also what a list that never opened looks like: the Settings select with only `claude`
  installed and then with both; the session start list the same way; and a third that pins the two
  surfaces against *each other* rather than against `AiCli::ALL`, since "both are filtered" and
  "both are filtered the same way" are different claims — one reads `state.available_providers`,
  the other `State::offered_providers()`.

  One deliberate detail in the fixture: the settings draft's default is always set to a CLI that
  *is* installed. A select paints its current value on the trigger as well as its options in the
  list, and the trigger is not an offer — it is a report of what the setting says. A draft pointing
  at an uninstalled CLI puts that name on screen for a reason FR-006 has nothing to say about, and
  a test reading painted strings cannot tell the two apart. The third test failed exactly that way
  before the fixture was fixed.

  The harness needed two additions, both in `tests/support/layout.rs`, and the second is the find:

  - `painted_text_pressing` presses a node and then draws the **overlay** as well as the base tree.
    `resolve_pressing` already grew an overlay pass for the geometry fixture; what is *drawn* needs
    the same two answers, because `material::Select`'s list is not in the element tree at all.
  - The overlay needs its **own** settle. The frames pumped before the press go through
    `Widget::update`, and that never reaches an overlay — the runtime dispatches to those
    separately. So a list opened by a press arrives with its entrance still at zero and paints its
    panel's geometry and none of its rows' text. Geometry never showed this because the entrance is
    layout-neutral by construction, which is why `resolve_pressing` has never needed a settle and
    this does. `painted_text_settled` is the same fix for a surface the *state* opens rather than a
    press, which is how the session start list is built.

  `Before` replaces the `Option<&[usize]>` the paint pass took, so the overflow gate's own pass is
  unchanged: `Before::Mounted` skips the overlay draw entirely rather than adding a measurement
  whose result is always empty. `cargo test --workspace` is green.

  Probed from a green tree, each mutation reverted before the next:

  - **D1 — the Settings select offers `AiCli::ALL`.** `settings_form::modal` passes `&AiCli::ALL`
    instead of the availability set. Two of the three fail; `features_settings.rs` stays green
    6/6, which is the case for the tests being here rather than there.
  - **D2 — the start list offers `AiCli::ALL`.** `session_start_menu_items` maps over the array
    instead of `state.offered_providers()`. The other two fail — including the agreement test, from
    the opposite side — while `features_session.rs` stays green 28/28.
  - **D3 — the harness stops settling the overlay.** The pump reduced to zero frames. Only
    `the_settings_select_lists_only_the_installed_clis` fails, and that asymmetry is the point: the
    start list is in the base tree and settles with it, so the overlay pump is load-bearing for
    exactly the surface that floats. Without it that test would have passed while asserting
    nothing.

  **Left undone, and not this task's to do:** nothing dispatches through `start_intent`. It has no
  caller in `crates/micold-client/src/` — `ui/sidebar.rs`'s primary half emits
  `SessionStartRequested { provider: state.provider_for_start(None) }` unconditionally, so pressing
  it with an uninstalled default asks the daemon to spawn a binary the application already knows is
  missing. The decision is written and tested; the wiring is not. T075 claims that wiring in its
  own text ("when a start is attempted on an unavailable default, present the available-CLI list
  … instead of starting anything") and is marked done, so this is T075 left half-finished rather
  than a task nobody wrote. Opened as T085.
- [X] T072 [P] [US4] Extend `crates/micold-daemon/tests/session_start.rs` so launching a session
  whose CLI is absent reports **`WireLifecycle::Failed { reason, attempts }`** with a reason naming
  the CLI, and the session is **not** presented as started (FR-010). Two things to hold apart, both
  of which an earlier draft of this task blurred by naming the domain enum: the message has a home
  only on the wire variant (`micold-core/src/protocol/messages.rs:576`, whose doc already covers
  "spawn failed"), and a missing binary must **not** consume the crash-loop budget — assert
  `attempts` does not climb toward `MAX_RESTART_ATTEMPTS` for a CLI that was never there. Retrying a
  `PATH` problem three times is noise, and it makes the reason arrive late

  Two tests, and a guard that is half the work. `NoCliOnPath` is the inverse of the file's existing
  `StubOnPath` and takes the same `ENV_LOCK`, since `PATH` is process-global and a stub installed by
  another test on another thread would put the CLI back halfway through this one. It removes the
  directories that hold an AI CLI rather than emptying `PATH`, because the shell-session tests beside
  these spawn a real interactive shell and inherit whatever `PATH` is set when they do; narrowing it
  is enough for `is_available` to say no. Its constructor then asserts that every provider in
  `AiCli::ALL` reports unavailable — without that, the whole file would keep passing on a developer
  machine with `claude` installed while asserting nothing at all, which is the failure mode a
  "the CLI is missing" test is most exposed to.

  The first test runs for every CLI in `AiCli::ALL`, because the message is the provider's own and a
  check written against one of them would pass on the other by accident. It reads the snapshot a
  client receives, not the durable record: the reason has a home only on the wire variant, since
  `session::SessionLifecycle::Failed` is a unit variant with nothing to carry a sentence in. Four
  assertions, the last two the ones this task exists for — the name appears in the **display**
  register and the executable register does not (`"claude"` is not a substring of `"Claude Code"`,
  so the negative is real rather than decorative), and `attempts` is `0` **and** below
  `MAX_RESTART_ATTEMPTS`. Both, deliberately: `== 0` pins today's value, and `< MAX_RESTART_ATTEMPTS`
  says why the value matters, so a future change that starts counting something here still trips the
  rule rather than only the constant.

  The second test is the other direction, and it is not decoration. What keeps the launch-time check
  off a Regular session is one `plan.mode == TerminalMode::AiCli` guard; dropping it leaves the first
  test green and refuses every shell session on a machine with no `claude` — a terminal that runs
  nothing but `/bin/sh`, turned away for the absence of something it never launches.

  Four probes, each from a green tree and each reverted: neutering the launch-time
  `is_available()` check, swapping `display_name()` for `command()` in the reason, setting `attempts`
  to `MAX_RESTART_ATTEMPTS` in the overlay, and widening the mode guard to `if true`. The first three
  fail the missing-CLI test; the fourth fails **only** the shell test and leaves the missing-CLI one
  passing, which is the asymmetry that says the two tests are covering different things.
  `cargo test --workspace` is green (218 `test result: ok`).
- [X] T073 [P] [US4] Extend `crates/micold-client/tests/features_sidebar.rs` so a session on an uninstalled CLI is still listed and still identified as that CLI (US4 scenario 3)

### Implementation for User Story 4

- [X] T074 [US4] Finish `is_available()` for both providers in `crates/micold-core/src/provider.rs`.
  The `PATH` lookup of `command()` already landed in T011 and T026 — T010 makes the method required
  from Phase 2, and US1's split affordance branches on its answer — so what US4 owns is the rest:
  platform-neutral resolution, with Windows `.exe`/`.cmd` handled by the lookup rather than by a
  `cfg`, and T069 green on both providers
- [X] T075 [US4] Filter the offered choices by the availability set **T014a put on `State`** — not
  by `Capabilities`, which `crates/micold-client/src/features/` cannot see and must not learn to.
  Nor is `features/settings.rs` the place an earlier draft assumed: it holds a `SettingsDraft` of
  strings and its overlay impl, and its own module doc records that the settings logic lives in
  `main.rs`'s `SettingsSaved` arm. So the filter is a pure function over `State` in
  `crates/micold-client/src/features/session.rs`, read by the Settings select through T014a's field.
  Keep a stored-but-unavailable default visible and marked rather than rewritten; when a start is
  attempted on an unavailable default, present the available-CLI list (the same one T033's secondary
  control opens) instead of starting anything
- [X] T076 [US4] Check availability again at launch in `crates/micold-daemon/src/supervisor.rs` and
  route the absence into the existing failure path — still not a new mechanism, but name the right
  one: the reason travels as `WireLifecycle::Failed { reason, attempts }` at the domain↔wire
  boundary, since `session::SessionLifecycle::Failed` carries no payload. Report it without spending
  restart attempts (T072). Research R11 records why the launch-time check is not redundant with the
  offer-time one
- [X] T077 [P] [US4] Document what happens when a CLI is missing — not offered, existing sessions still listed, a clear failure rather than a dead terminal — in `docs/user-guide/settings.md` and `docs/user-guide/worktrees-and-sessions.md`

**Checkpoint**: All four stories are independently functional.

---

## Phase 7: Polish & Cross-Cutting Concerns

> Per-story user-guide docs shipped inside their own phases (Principle VII). This phase is only for
> cross-cutting review and the verification the feature cannot claim without running.

- [X] T078 [P] Cross-cutting docs review: index/navigation in `docs/user-guide/` and any statement elsewhere in the repo that says or implies the application runs `claude` specifically (including `README.md`)

  Five files said it and now do not: `README.md` (headline and features bullet), `docs/README.md`,
  `docs/user-guide/project-selection.md`, `docs/user-guide/worktrees-and-sessions.md` (~18
  substitutions across the terminal, resume/restart, shell, AI-tab, background-project and
  keyboard-input sections) and `docs/daemon.md`. The rule applied throughout: say "AI CLI" where the
  sentence is true of either, and name both where it is about a difference between them — a doc that
  says "the AI CLI" everywhere is neutral but tells a Copilot user nothing about why their session
  behaves differently.

  `docs/daemon.md`'s "How the daemon knows" could not be substituted, only rewritten. It described
  the hook receiver as *the* mechanism, and FR-019 gave the daemon a second one: it is now two
  bullets — Claude Code posts to the loopback listener, GitHub Copilot writes an event log the
  daemon tails — with the property that survived both, "nothing is polled on a timer, and no work at
  all is scheduled for an idle session", stated once for both rather than once inside the hook
  bullet. Two claims were added that the doc never made and a reader would otherwise get wrong:
  SC-006/SC-009's rule that a session the app merely *discovered* on disk is watched by neither, and
  that a discovered session's title comes from `claude`'s transcript or `copilot`'s session state
  rather than from the OSC-0 title a supervised session sets.

  Outside `docs/`, the grep found two live statements and both were packaging metadata, which is
  user-facing on a `.deb` install and in a desktop search where nobody will read a correction:
  `cargo deb`'s `extended-description` said "managing Claude Code worktrees and sessions", and the
  `.desktop` keywords listed `claude` with no `copilot`. Everything else that matched is either
  history that should stay history (`CHANGELOG.md`, `specs/**`), a type name (`ClaudeProvider` in
  `docs/development/architecture.md`), a path (`.claude/worktrees/`), or a fixture label in tests
  and the design showcase.

  `docs/user-guide/settings.md` needed no change at all — it was written two-CLI in T072 and already
  states what T085 later made true, that "when you start a session the app tells you what is missing
  and offers the CLIs that are available instead of substituting one". The session guide is where
  that had gone missing, so it gains a bullet for it: the offer is made from the set checked against
  `PATH` at press time, nothing is created until you pick, and your stored default is left alone
  (FR-002).

  `scripts/tests/documentation-set.test.sh` and `classify-change.test.sh` both green, which is the
  only thing here a test can speak to — that the files changed are still classified as documentation
  and that `packaging/` and `Cargo.toml` are not, so CI runs the full build for this commit.

  Out of scope but found and left: `README.md`'s build instructions still say `cargo run --features
  gui` and `cargo test --no-default-features`, which are neither the workspace's feature layout nor
  the `mise run` tasks CLAUDE.md names as canonical. That is stale against feature 010's split, not
  against this feature, and correcting it here would hide it in a docs commit about providers.
- [X] T079 Remove the now-false module docs in `crates/micold-core/tests/ai_cli_provider_seam.rs` and `crates/micold-client/tests/no_concrete_implementations.rs` that state the seam is not substitutable and that `Capabilities` is where to fix it — the files now prove the opposite

  Both docs were rewritten in `6993fbb`, in the same commit that made them false — the seam file now
  opens with what it *used* to say and why it no longer says it, and the guard file's provider
  section states that the "one place" is `micold-core/src/provider.rs` and explicitly not the shell.
  So this task closed itself, and what was left was to check that rather than to assume it. Every
  surviving mention of the old position is in the past tense and framed as history, which is worth
  keeping: a doc that says only what is true now leaves the next reader to rediscover why the
  arrangement is what it is.

  Verified rather than read: `ClaudeProvider` appears in both files only in that historical framing
  and in the guard's own expectation list, which now names `CopilotProvider` beside it; the seam
  doc's claim that `discover_transcript_session_ids` and `archived_marker_path` are off the trait
  holds — the two survivors are inherent methods on `ClaudeProvider`, with a comment saying they are
  `claude`'s layout and nothing else's; and the four consumers the doc names as taking the provider
  from the session record (`catalog.rs`, `state.rs`, `supervisor.rs`, `terminal.rs`) all call
  `.provider()`.

  One line was actually wrong and is fixed: the guard file pointed at "`main.rs`'s own
  `boot_judges_each_session_by_its_own_provider`", and that test moved to `shell/persist.rs` with
  the boot prune. A pointer to a test that is no longer where it is named is the same defect this
  task is about, one file further on. Both suites are green (13 and 8).
- [X] T080 Satisfy `cargo clippy` and `cargo fmt` across the workspace for the new code, matching what CI enforces

  The three commands `.github/workflows/ci.yml` runs, in its order: `cargo fmt --all -- --check`,
  `cargo clippy -p micold-core --all-targets -- -D warnings`, and the same clippy across the
  workspace. Both clippy passes were already clean — including the render-free one, which is a
  separate job precisely because `micold-core` must stay buildable without the GUI.

  `fmt` found two diffs, both in code this feature added and both mine: a trailing blank line at the
  end of `tests/session_start_press.rs`, and an `assert_eq!(attempts, 0, …)` in
  `tests/session_start.rs` that rustfmt splits one-argument-per-line once the message pushes it over
  the width. Applied with `cargo fmt --all`; nothing else in the workspace moved, which is the
  useful part of the result — the formatting drift this task was hedging against did not exist.

  `cargo test --workspace` green afterwards (219 `test result: ok`), since a formatting pass that
  reflows an assertion is still an edit to a test.
- [X] T081 Verify Copilot's base directory on Windows (research R2's one unverified row) and correct `CopilotProvider::config_dir` and `contracts/copilot-cli.md` if it is not `%USERPROFILE%\.copilot`; if it cannot be verified, record that explicitly rather than leaving the table implying it was

  Verified, and `config_dir` needed no correction. There is no Windows machine here and the task's
  escape hatch was to say so — but "unverifiable" was the wrong answer, because the thing to verify
  is on this disk. The `copilot` binary is a Node launcher that unpacks the real CLI into
  `~/.cache/copilot/pkg/<platform>/<version>/app.js`, and that bundle is the CLI's own answer to
  the question, not a document about it.

  Three findings from 1.0.80, each independent of the others:

  - The canonical resolver is called as `resolveCopilotHome(configDir, $COPILOT_HOME, homedir())`.
    It is handed the home directory and **neither the platform nor `%LOCALAPPDATA%`** — so it
    cannot branch on Windows even if it wanted to. The contrast is the decisive part rather than
    the absence: `copilotCacheHome(process.platform, homedir(), $COPILOT_CACHE_HOME,
    $LOCALAPPDATA, $XDG_CACHE_HOME)` sits immediately beside it with all three. The cache home is
    deliberately platform-aware and the config home deliberately is not; that is a design decision
    visible in two adjacent signatures, not an oversight one release could quietly correct.
  - All five `.copilot` literals in `app.js` are `join(homedir(), ".copilot")`, none behind a
    `win32` branch, and the launcher's own SEA bundle does the same for its package cache while
    its cache-home helper branches on `win32`/`darwin` — the same split, one layer up.
  - One of those five is a migration that **moves** `session-state`, `session-store.db`,
    command history and the config files *out of* `$XDG_STATE_HOME/.copilot` and
    `$XDG_CONFIG_HOME/.copilot` into `homedir()/.copilot`. R2 recorded XDG as "not honoured"; it
    is more than that — the home-relative path is the destination those variables are being retired
    in favour of. A base directory a vendor is actively migrating *towards* is not one that varies
    by platform.

  Joined to Node's documented `os.homedir()` — `%USERPROFILE%` when defined on Windows — the row is
  `%USERPROFILE%\.copilot`, which is what `CopilotProvider::config_dir` already computes.

  The one real divergence this turned up, recorded in `contracts/copilot-cli.md` because it is the
  kind of thing that becomes a bug report years later: the application does **not** resolve home the
  way Node does. `directories::UserDirs` calls `SHGetKnownFolderPath(FOLDERID_Profile)` and ignores
  `%USERPROFILE%`; Node prefers the variable. They name the same directory unless something has
  redefined it for the process tree, and if that ever happens `ClaudeProvider` is wrong in exactly
  the same way — it resolves its own base identically, and has since feature 021. So this is a
  property of the seam, not of Copilot, and `$COPILOT_HOME` overrides both. Not fixed here: making
  the two agree is a change to `ClaudeProvider`'s behaviour on a platform nobody has run this on,
  which is a worse trade than recording it.

  Caveat kept honestly: the package inspected is `linux-x64`, so this is the JS the Linux build
  ships. The argument does not rest on the platform of the bundle — a resolver with no platform
  parameter has none on any build — but a Windows package that shipped *different* JS would defeat
  it, and nothing here can rule that out. `research.md`'s table says "verified (T081)" and names the
  method, so a future reader can weigh it rather than inherit it.

  `plan.md`'s Principle VI risk is marked retired rather than deleted, keeping the mitigation it
  named. `mise run test-core` green (64 `test result: ok`), `cargo fmt --all -- --check` clean —
  which is all a comment-and-docs change can be asked to prove.
- [X] T082 Confirm `mise run test` is green on Linux, macOS and Windows in CI (Principle VI), with every Copilot test passing on a runner that has no `copilot` installed

  Green on all three, for commit `6564e1c`: `build + test (ubuntu-latest)` 4m40s, `(macos-latest)`
  1m20s, `(windows-latest)` 4m56s, alongside `classify change`, `docs check`, `assertion freeze`,
  `fmt + clippy` and `ci complete` — eight checks, no failures.

  The second half of the task is the one worth writing down, because a green matrix does not by
  itself prove it: **no runner has `copilot`.** Nothing in `.github/workflows/` installs it or
  mentions it, so every Copilot test in the suite passed on three machines where the binary is
  absent. That is the shape the feature was built to: availability is probed, never assumed, and a
  test that needed the real CLI would have had to be `#[ignore]`d to get here. There is no live
  `#[ignore]` anywhere in `crates/` — the four matches are all prose in doc comments recording that
  two tests *used* to be ignored (`service_capability_fakes.rs`, `no_concrete_implementations.rs`)
  and are not any more. So the Windows job in particular exercises Copilot's base-directory
  resolution (T081) against fixtures on the platform that resolution is about.
- [X] T083 Run quickstart.md §A and confirm every gate in the table has a corresponding green test

  Every gate is present and green, and seven of the table's pointers were wrong about where. The
  suite itself was never in doubt — `mise run test` is green, which is what the task's second half
  asks. What the walk was for is the first half: that each row's *left column* names something.

  The drift is three different failures, which is why walking the column matters more than counting
  the rows:

  - `micold-daemon/tests/copilot_activity_is_event_driven.rs` was never created. FR-019's two
    assertions live in `copilot_activity.rs` as `this_applications_watch_path_schedules_no_timer_of_its_own`
    and `a_quiet_session_costs_nothing_between_appends`.
  - Three rows named files that exist and hold something else — `sidebar_tree.rs` for SC-009
    (it is `features_sidebar.rs`, T058c), `service_capability_fakes.rs` for FR-006's "an
    unavailable CLI is not offered" (`provider_choice_surfaces.rs`, T071, with the `PATH` probe
    itself in `shell/capabilities.rs`'s own tests, T070), and `main.rs` for the catalog fold.
  - Three were never updated from planning at all, still reading "the test T023a adds": they are
    `settings_default_ai_cli.rs`, `session_discovery.rs` and `shell/persist.rs`'s own tests. This
    is the same class T079 fixed in `plan.md`, one document over.

  The `main.rs` row is the finding worth keeping. Walking it, a search of `crates/*/tests/` for a
  test that builds a `SessionSummary` carrying `Copilot` returned nothing, and the row's own text —
  *"miss it and the provider defaults silently while every other test stays green"* — read like a
  description of a hole rather than of a gate. So a test was written into `catalog_sync.rs` beside
  the fold, and then probed the way anything here is probed: replace `summary.provider` with a
  constant `AiCli::ClaudeCode` and run the workspace.

  Three tests failed, not one. `shell/daemon_sync.rs::a_daemon_reported_session_keeps_the_cli_the_daemon_named`
  and `an_existing_sessions_cli_is_not_rewritten_by_a_later_snapshot` — T060a's, covering the adopt
  branch *and* the deliberate non-adoption on update, which the new test did not. The gate existed;
  it sits in a `#[cfg(test)]` module in the **binary** crate, which is precisely where a search of
  `crates/*/tests/` cannot see it. The row was right in substance and stale only in filename. The
  duplicate was deleted and the row now points at `shell/daemon_sync.rs`.

  So the probe's real yield here was not that an assertion is load-bearing but that a *conclusion*
  was wrong, and it is worth naming why the mistake was available: "no test names this type" is
  evidence about a search, not about the suite, and this repo keeps a third of its client tests
  inside `src/`. The corrected table now says which rows are `#[cfg(test)]` modules for that reason.

  Everything the mutated run does say, it says cleanly: 1792 tests pass with the fold defaulted, and
  the only three that fail are the ones written to watch it. Nothing else in the workspace notices —
  which is the row's claim, now demonstrated rather than asserted.

  Two merge fallouts fixed on the way, both from `origin/main` landing while this branch was open
  and neither visible locally until CI built the merge commit: `labelled_toggle.rs` had not been
  given `text_button`'s new `host` argument (`None` — it draws on a neutral surface, and a `Host` is
  only for a control on an accent fill, FR-004a), and a test main brought in called `start_new` with
  one argument.

  `cargo test --workspace` green: 2167 tests over 220 targets, nothing failing. One caveat worth
  recording rather than smoothing over: an earlier run of the same tree failed
  `frame_coalescing::a_flood_is_coalesced_to_at_most_one_frame_per_frame_interval` with *"only 1
  frame(s) — nothing was streamed"*, and passed on re-run. That test is feature 010's and this
  branch does not touch its path; the assertion needs the flood to arrive in more than one frame,
  which a machine running four `rustc` jobs and another worktree's build alongside it cannot
  guarantee. It is a load flake, not a regression — but it is one this repository's own build
  discipline makes likely, so it is named here rather than left for the next person to rediscover.
- [X] T084 Run quickstart.md §B B1–B8 against a real Copilot CLI with `COPILOT_HOME` pointed at a scratch directory, and fill in the "Recording the pass" table with the date, platform, and any step that did not behave as written — including B6, the untrusted-worktree behaviour no probe could confirm

  Run on 2026-08-25 with the repository's `visual-pass` skill rather than by a person at a display:
  Xvfb `:91` at 1600x1400 with Mesa lavapipe, a pinned matched client/daemon pair, a private
  `XDG_RUNTIME_DIR`/`XDG_DATA_HOME`, three scratch git repos, and Copilot CLI 1.0.80 against
  `COPILOT_HOME=/tmp/micold-copilot-probe`. The table in `quickstart.md` holds the per-step results;
  what follows is what the pass *found*, since that is the part a green table hides.

  **Seven of the eight steps pass, including the three §A cannot substitute for** — B2's click
  budget (one press, no menu, on the default), B3's restart with the conversation intact, and B5's
  externally-started sessions. B4 passes except for its badge-timing clause. **§B found four
  defects, none of them in a path §A covers**, which is the answer to whether a manual pass was
  worth running against a suite of 2167 tests:

  1. **The badge never moves on its own (SC-005, FR-018)** — eight frames from 0.24 s to 0.76 s
     after Return are byte-identical, and so is a frame taken long after the reply finished. It
     changes only when some *other* broadcast happens to run. `micold-daemon/src/state.rs` calls
     `note_activity` without the `broadcast_catalog()` beside it. True for `claude` as well as
     `copilot`, so it is not a Copilot defect — it is one this feature's own success criterion is
     the first thing to measure. Opened as T086.
  2. **Restart on a session whose CLI is missing does nothing visible (FR-010)** —
     `spawn_session_start` in `micold-daemon/src/server.rs` passes `reply: None` on the resume path,
     so the failure is computed and dropped. Opened as T087.
  3. **A failed start never says why (FR-010)** — the reason string the daemon builds ("GitHub
     Copilot isn't installed. Install it, or start this session on another AI CLI.") reaches no
     surface in the UI, not even a tooltip. Opened as T088.
  4. **The session-start list opens at the window origin**, over the sidebar header, from both the
     `+` and the secondary control — `evidence/session-start-menu-anchors-at-origin.png`.
     `ContextArea` publishes the anchor on `ButtonPressed`; the iced button publishes its own
     message on *release*; `start_menu_toggled` therefore always runs last and writes `(0, 0)`.
     `features/session.rs`'s doc comment asserts the opposite ("the same press publishes both
     messages … so the panel is never rendered at the origin") and is simply wrong about the event
     phases. T085's `tests/session_start_press.rs` cannot see it: its assertion is
     `SessionStartMenuAnchored(_)`. Opened as T089.

  Two findings that are not defects but would have cost the next person the same time they cost
  this pass, so they are written into the quickstart's steps rather than left in a log:

  - **Copilot's folder trust is per-root and inherited by subdirectories.** B6's first attempt
    produced no prompt at all, which read like the contract being wrong; it was the repo root
    already sitting in `trustedFolders`. With this application's `.claude/worktrees/` layout that
    means the trust question appears only for a project's *first* session, never for a new worktree
    — which is a better outcome than research R12 dared assume, and the reason B6 needed a
    brand-new project to be observable. When it does appear it appears in the session's own
    terminal, and the sidebar shows the session running, not failed, while it waits.
  - **A hand-started `copilot -p` writes no per-cwd index**, so B5's discovery has nothing to find
    and the step fails for a reason that has nothing to do with the application. B5 needs a real
    interactive `copilot`, driven through a pty.

  And one artefact of running this *inside* Claude Code, recorded so it is not mistaken for
  FR-007a misbehaving: the application inherits `CLAUDE_CODE_CHILD_SESSION` from the session driving
  it, which turns off claude's transcript saving, so a `claude` session started under it looks empty
  and is pruned on the next open. B8's resume half only became observable once every `CLAUDE*`
  marker was unset from the launch environment.

  What the pass cannot answer is unchanged from the skill's own limits: lavapipe says nothing about
  frame pacing on the user's GPU, and no screenshot pipeline catches a chosen frame mid-animation.
  Neither is a claim §B makes. The badge defect is not one of those limits — it is not late, it does
  not happen at all.
- [X] T085 Dispatch the primary start press through `State::start_intent` (FR-004 scenario 4), which
  T075 decided and left unwired. `ui/sidebar.rs` currently emits `SessionStartRequested` with
  `provider_for_start(None)` on every press, so an uninstalled stored default reaches the daemon as
  a spawn of a missing binary — FR-010's failure, arriving after the fact, where FR-004 wants the
  list offered instead. Two things the wiring needs that the secondary half already has and the
  primary does not: the availability set is only refreshed at `SessionStartMenuOpened` and
  `SettingsOpened` (research R11's two named events), and `SplitAction` reports a press point only
  for the chevron (`on_secondary_anchor`) — a menu opened from the primary half has nowhere to hang
  until it reports one too, which `tests/context_menu_anchor_call_sites.rs` governs. The test is a
  view-level one: pressing the primary half with an unavailable default must publish no
  `SessionStartRequested`

  `start_press` in `ui/sidebar.rs` names the message each `StartIntent` answer arrives as, and
  decides nothing itself — the branch is `State::start_intent`'s, which `features_session.rs` has
  covered since T032a. What was missing was only the dispatch, which is why every pure test stayed
  green while a press on an uninstalled default went to the daemon as a spawn of a binary that is
  not there.

  The `OfferChoice` arm publishes `SessionStartMenuOpened` — the chevron's own message, not one of
  its own. That is what makes the offered list current rather than merely correct-at-launch: the
  availability set is re-probed on that message and on `SettingsOpened`, research R11's two named
  events, so the very press that opens the list refreshes what it contains. A message of its own
  would have opened the list on the set as it stood at the last of those events, which for a user
  who has installed nothing since launch is the set from launch. The third test pins exactly this,
  since it is a property of *which* message is published and nothing about the list's contents would
  show it.

  The set that decision reads can still be a press out of date, and both directions are safe and
  neither is silent: a default uninstalled since the last probe reaches the daemon, whose
  launch-time check names the missing CLI (FR-010, T076); one installed since then opens a list
  that — refreshed by this press — now contains it, one press away. `NothingAvailable` keeps
  today's behaviour and sends the stored default, because FR-006 forbids opening a
  present-and-empty list and an inert `+` answers nothing; the daemon's report is the only thing
  that can tell that user anything. No test covers that arm — the probe that would have caught it
  is P5 below, and it is recorded as uncovered rather than implied.

  `SplitAction::on_primary_anchor` is the other half of the wiring: the primary half can open a
  list now, and a list hangs from the press point because a sidebar row's position is not something
  the view holds (018 BUG-008). It reports the point on **every** primary press, including the ones
  that start a session — what a press means is the state's to decide and the control does not know
  which answer it just published; an anchor with no list open is a no-op at the reducer, the same
  contract the chevron already relies on for the press that closes the list.

  The test presses the real `+`, found by the glyph it paints rather than by a node path, so a
  rearranged sidebar says so instead of quietly pressing something else. Finding it needed one
  harness addition: `Overflow` now reports the paragraph's draw `origin`. `node_path` answers a
  different question — attribution is by clip, so that it can report the width the text was allowed
  — and a glyph inside the sidebar's scrollable attributes to the scrollable. Pressing that node's
  centre presses empty list, which publishes nothing, which is indistinguishable from the very
  thing the negative assertion looks for. That is P4, and it fails all three tests including the
  negative one, because the negative test also asserts the press *did* something.

  Five probes, each from a green tree and each reverted. P1, the `OfferChoice` arm restored to an
  unconditional `SessionStartRequested`: the two uninstalled-default tests fail and the installed
  one passes. P2, `on_primary_anchor` removed: **only** the anchor assertion fails, the rest of the
  first test passing on the same press. P3, the `Start` arm made to open the list too: only
  SC-001's test fails — one press, the stored default, started. P4 above. P5, the
  `NothingAvailable` arm switched to `SessionStartMenuOpened`: **nothing fails**, which is the gap
  named two paragraphs up; covering it needs a state with an empty availability set and is not in
  this task's scope. `cargo test --workspace` is green (219 `test result: ok`).

### Defects found by the §B manual pass (T084)

Four, opened as tasks rather than written up as notes, because each is a behaviour the feature
claims and does not have. None is reachable from §A: every one of them is either a timing property,
a path only a second press exercises, or a pixel position.

- [X] T086 Broadcast after `note_activity`, so the badge moves when the event lands (SC-005, FR-018).
  `state.rs`'s `open_event_log_tail` callback calls `state.note_activity(id, event)` and **discards
  its return value** — the `bool` that says the projected signal changed. `drain_signals`, the other
  producer of the same kind of change, returns the same `bool` to the supervisor tick, which pushes
  a `CatalogChanged`; the event-log path has no tick behind it, so nothing pushes. The badge
  therefore only ever changes when some unrelated broadcast happens to run — which §B measured: eight
  frames from 0.24 s to 0.76 s after Return are byte-identical, and so is one taken long after the
  reply finished. True for `claude` as well as `copilot`, so the defect is in the fold, not in
  Copilot's reporting. The gate has to be about the *push*, not about the state: a test that asserts
  `note_activity` changed the signal passes today. Assert a subscriber receives a catalog after a
  synthesised event-log append, and probe it by deleting the new broadcast — nothing else in the
  suite should fail, which is the measure of how alone that assertion is.

  **Done.** `open_event_log_tail`'s callback now reads the `bool` and calls `broadcast_catalog()`
  when it is `true`, which is what the hook receiver two files over has always done with the same
  value. The guard is not decoration: `events.jsonl` is mostly lines the machine deliberately
  ignores, and an unconditional push would send a full catalog snapshot per line for a badge that
  never moves.

  The gate is `activity_pipeline.rs::an_event_log_append_pushes_the_new_badge_to_connected_clients`,
  and it is written against the trap the task named — it registers a client and reads *what that
  client was sent*, never `catalog_snapshot()`, because a snapshot assertion passes against the
  defect. Nothing else in the test can broadcast: no `drain_signals`, no supervisor tick, no start,
  so the tail is the only thing that can speak. Its second half proves the `if` rather than the
  call, and does it without a sleep: a no-op line is appended *between* two real ones, so when the
  second push arrives the first has certainly been delivered too, and the assertion is that exactly
  two arrived.

  Probed on `11bd5b2` by restoring the discard and running `cargo test --workspace --no-fail-fast`
  — the flag matters, because a fail-fast run stops at the first failing binary and 42 of the 220
  never execute, which cannot distinguish "alone" from "first". 219 binaries pass, 1 fails, and the
  only failing assertion is this one. The whole suite is otherwise blind to whether the daemon ever
  tells anybody the badge moved.

  Timing is deliberately not asserted. SC-005's budget is one second and the run takes 0.24 s, but
  a deadline tight enough to *be* the measurement would fail on a loaded runner for a reason that is
  not the defect; what is under test is that a push happens at all. §B's frames remain the timing
  evidence.

- [X] T087 Report a failed **resume** to the client (FR-010). `ClientMsg::SessionStart` — the restart
  a user presses on an existing session — calls `spawn_session_start(…, LaunchMode::Resume, None, …)`
  in `micold-daemon/src/server.rs`. That `None` is the reply channel, and the whole of the outcome
  handling sits behind `if let Some((client, req)) = reply`: the start failure is logged at `warn`
  and then dropped, with no `OperationError` and no `broadcast_catalog()`. So pressing restart on a
  session whose CLI is no longer installed does *nothing visible at all* — the check
  `state.rs` performs is correct and lands nowhere. The fresh path passes `Some((id, req))` and is
  fine, which is why every test of the failure message passes. Note the asymmetry is deliberate for
  the *success* case (a resume has no `SessionCreated` to send); the fix is to broadcast and report
  the error regardless, not to invent a reply for the success.

  **Done 2026-08-26.** The fix is one line in each failure arm of `spawn_session_start`:
  `task_state.broadcast_catalog()`, outside the `if let Some(reply)` that held every other outcome
  path. Neither of the two obvious alternatives is right. An `OperationError` cannot be addressed to
  this message at all — `ClientMsg::SessionStart` carries no `req` — and inventing one would be a
  wire change for a report the catalog already carries; and the missing reply is *correct* for the
  success case, which is the trap the diagnosis above warns about. What was actually asymmetric was
  the announcement, not the reply: `start_session` records the sentence in `start_failures`,
  `overlay_live_summaries` projects it as `Failed { reason, attempts: 0 }`, and the only
  `broadcast_catalog` on the success path sits *after* `mark_session_running` — which a failed start
  returns before reaching. The `SessionCreate` path has been relying on exactly this broadcast for
  the same case all along, which is why every test of the failure *message* passed.

  The gate (`crates/micold-daemon/tests/resume_failure_reported.rs`) drives `SessionStart` through a
  real `serve_connection` and asserts on the frames the client receives. That is deliberate and it is
  the whole difficulty of the task: the state half is already correct and already gated, so a test
  that reads `welcome_payload()` — or calls `start_session` directly, as every existing test of this
  refusal does — passes against the defect. Nothing else in the test can broadcast: the supervisor
  tick belongs to the daemon binary rather than to `serve_connection`, and the `Welcome` snapshot is
  drained before the start is sent, so a `CatalogChanged` arriving afterwards came from the start.
  One provider (Copilot) rather than a sweep over `AiCli::ALL` — the per-provider wording is already
  gated in `session_start.rs`; what was missing here is delivery.

  Probed on `c96af25` by deleting the two broadcasts and running `cargo test --workspace
  --no-fail-fast`: 221 binaries, **2168 pass and 1 fails**, and the failure is
  `a_resume_that_fails_reaches_the_client` alone. `--no-fail-fast` again for the reason recorded
  under T086 — a fail-fast run stops at the first failing binary and cannot distinguish "alone" from
  "first". The red run before the fix failed at the ten-second receive timeout rather than on a
  wrong value, which is the defect's actual shape: not a bad announcement, no announcement.

  The ten seconds is a timeout and not a measurement — the start is a single `PATH` lookup and the
  green run takes 0.00 s — so nothing here asserts latency; a deadline tight enough to *be* the
  measurement would fail on a loaded runner for a reason that is not the defect.

  What this does **not** do is put the reason in front of the user: the client now receives
  `Failed { reason }` for a resume as it already did for a create, and T088 is where that sentence
  gets somewhere readable. Restart on a missing CLI moves the bar to `failed` after this; it does
  not yet say why.

- [ ] T088 Surface the reason a start failed, somewhere a user can read it (FR-010). The daemon
  already computes the sentence — "GitHub Copilot isn't installed. Install it, or start this session
  on another AI CLI." — and it reaches `WireLifecycle::Failed { reason, attempts: 0 }`. §B found it
  displayed nowhere: the bar reads "failed", and the reason is not in the row, not in the terminal,
  not on hover. FR-010 asks for the CLI to be *named*, which "failed" does not do. Decide one
  surface and gate it there; a tooltip is enough if that is what the sidebar can carry, but the
  choice belongs in the task rather than in whatever is convenient at the call site.

- [ ] T089 Anchor the session-start list to the press that opened it. Both halves of `SplitAction`
  open the CLI list at the window origin, over the sidebar header, whatever row was pressed —
  `evidence/session-start-menu-anchors-at-origin.png`. The cause is an event-phase mismatch:
  `ContextArea::on_primary_press`/`on_secondary_anchor` publish the anchor from `reported_press`,
  which fires on `ButtonPressed`, while the iced button inside publishes its own message on
  **release**. `start_menu_toggled` therefore always runs *after* `start_menu_anchored` and
  overwrites the anchor with `(0, 0)`. The doc comment on `features/session.rs`'s
  `start_menu_anchored` asserts the opposite in so many words — "the same press publishes both
  messages, and iced applies every queued message before it draws, so the panel is never rendered at
  the origin" — and is wrong about which phase each message comes from; fix the comment with the
  code. The overflow menu and the Settings dropdown anchor correctly, so this is not a
  `clamp_menu_anchor` problem. T085's `tests/session_start_press.rs` cannot see it because its
  assertion is `SessionStartMenuAnchored(_)`: the gate must assert the *point*, and the probe is to
  restore the `(0, 0)` write and watch that one assertion fail.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies — start immediately
- **Foundational (Phase 2)**: depends on Phase 1 for fixtures; **blocks every user story**
- **US1 (Phase 3)**: depends on Phase 2 only. Smaller than it was: T017, T017a, T018 and T026 moved
  into Phase 2, because T011a's exhaustive match needs `CopilotProvider` to exist before the seam
  reshape can finish. Their IDs did not change, so Phase 2 runs four IDs out of numeric order
- **US2 (Phase 4)**: depends on Phase 2. **T003b is settled** (R15: the daemon, in the attach arm),
  so T042a and T050 — still net-new discovery work — are unblocked and both land in
  `crates/micold-daemon/`. Shares `provider.rs`
  and `catalog.rs` with US1, so
  interleaving the two on one branch means file contention, not logical coupling — US2 is
  independently testable (a session record and a fixture store are enough; nothing has to be started
  from the UI)
- **US3 (Phase 5)**: T067a depends on T066 and T066a — it regenerates the layout fixture those two
  move, and until it runs the phase ends on a red gate that looks like a mystery. Otherwise depends
  on Phase 2, and on **T029 for the wire** — the outbound provider field
  lands there so the schema hash moves once for the whole feature (T020). T064 additionally depends
  on **T003a** (the watch crate). Otherwise independent of US1 and US2 in logic
- **US4 (Phase 6)**: depends on Phase 2 and on **T014a** for the availability set. **Not
  independent**, despite touching one provider method: T074 edits `provider.rs`, shared with T026,
  T048, T049, T061 and T062; T075 edits `features/session.rs`, which is T032a's; T071 extends
  `features_session.rs` and `features_settings.rs`, which are T022's and T023's. The logical
  entanglement runs one way only — T033's "no secondary half when fewer than two CLIs are available"
  branches on `is_available()`, which is why T026 gives that one method a real body in US1 instead
  of waiting for here
- **Polish (Phase 7)**: depends on all desired stories. T084 additionally needs a real `copilot`

### Critical path

T001 → T004/T007/T007a/T008a/T008b (RED) → T009 → T010 → T011 → **T026 → T011a** → T012 → T013 → T014 → T014a →
T015/T015a/T015b/T016/T016a (GREEN) → **US1**

T003b is settled (R15), so nothing runs alongside for it any more; US2's discovery work is
unblocked and lands in `crates/micold-daemon/`.

T007 is on the critical path deliberately: it is the test that fails on today's tree and stays
failing until T015, T015a and T016 land, which is what makes the seam reshape verifiable rather than
asserted. **T008a and T008b are there for the same reason and matter more**: they are the only gates
on the three set-wide provider decisions — two in the daemon's `state.rs` (one of which archives) and
one in the client's boot prune (which drops sessions from the workspace). A reshape that leaves those
hoisted lookups in place compiles, passes T007 — the client's names nothing concrete, so the seam
audit cannot see it at all — and silently loses every Copilot session. Both must be RED before T015a
and T015b are attempted. **T007a covers the third blind spot**: the launch path, which names
`ClaudeProvider` in `core/terminal.rs` (so T007 does see that line) but whose real defect is
structural — `LaunchSpec` cannot carry a provider, so there is nothing for the daemon's two spawn
sites to read even once every name is gone.

### Within each user story

- Tests are written and observed failing before implementation (Principle I)
- Provider methods before their consumers; the protocol change before either side reads it
- Shared UI components before the call sites that use them (Principle VIII)
- The story's user-guide docs ship with it, not after it

### Parallel Opportunities

- T002, T003 and T003a in Setup — three different files. T003a (a manifest edit plus its vetting
  note) blocks only T064 and T060. T003b is done
- T017, T017a and T018 are all `copilot_provider.rs`, so they are `[P]` with the rest of the
  foundational tests but **not with each other**
- T005, T006, T007, T007a, T008, T008a, T008b in Foundational — seven different test files, and each
  fails for its own reason. T008a is the only one in `crates/micold-daemon/tests/`, T008b the only
  one in `crates/micold-client/tests/`, T007a the only one that fails because a **struct** is the
  wrong shape rather than because a call site names a type
- Every `[P]` test task inside a story phase: they are distinct files with no shared state
- Across stories, once Phase 2 is done: **there is no cleanly parallel story.** An earlier draft
  named US4, on the grounds that it touches one provider method and two consumers; that is wrong —
  T075 edits `features/session.rs` and T071 both `features_*` test files, all three shared with US1.
  US1 and US2 both edit `provider.rs`, `catalog.rs` and `state.rs`, and US3 adds `catalog.rs` (T065)
  and `provider.rs` (T061, T062) to that list. Parallelise **within** a phase, where `[P]` already
  marks it, rather than across stories
- T034, T035, T051, T068 and T077 are five documentation edits across two files, and **four** of
  them land in `worktrees-and-sessions.md` (T035, T051, T068, T077), so they conflict textually if
  run at once. **T077 is the one to sequence**: it is the only task editing *both* files, so it also
  collides with T034 on `settings.md` and can be `[P]` with none of the other four

---

## Parallel Example: User Story 1

```bash
# All ten US1 test tasks are separate files — launch together, observe RED:
Task: "T019 default_ai_cli round-trip in crates/micold-core/tests/settings_ai_cli.rs"
Task: "T020 schema hash moves once in crates/micold-core/tests/schema_hash.rs"
Task: "T022 default-vs-override resolution in crates/micold-client/tests/features_session.rs"
Task: "T024 SessionCreate spawns the right binary in crates/micold-daemon/tests/session_start.rs"

# Then the two independent implementation tasks:
Task: "T028 Settings::default_ai_cli in crates/micold-core/src/settings.rs"
Task: "T034 Default AI CLI preference in docs/user-guide/settings.md"
```

---

## Implementation Strategy

### MVP First (Phases 1–3)

1. Phase 1: fixtures
2. Phase 2: the seam reshape — **the phase that carries the risk**, because it changes a trait seven
   call sites use and the whole point is that no behaviour moves with it. It now also lands
   `CopilotProvider` itself (T026) and the core-side lookup (T011a), which the reshape cannot be
   completed without
3. Phase 3: US1
4. **STOP and VALIDATE**: quickstart §B B1 and B2 by hand. A Copilot session that starts in the right
   worktree with an id we chose is the proof the feature is possible at all

### Incremental Delivery

1. Setup + Foundational → the seam is honest and reachable from every crate; `mise run test` green,
   with the second provider present but not yet wired to anything a user can press
2. + US1 → both CLIs start (MVP)
3. + US2 → the choice persists and rediscovers
4. + US3 → the sidebar tells the truth
5. + US4 → the one-CLI user is unharmed
6. Polish → verified on three platforms, §B recorded

### If something has to be cut

**Nothing here is optional any more.** The activity badge was the spec's one sanctioned droppable
slice, on the assumption that the signal would have to be inferred from a database. Research R5
disproved that — Copilot reports its own turn events — and the 2026-08-16 clarifications withdrew
the escape hatch and committed to a watch facility to read them. Dropping the badge from here is a
spec change with a reason recorded, not a fallback taken quietly under time pressure.

---

## Notes

- `[P]` = different files, no dependency on an incomplete task
- Verify each test fails before implementing it — the seam reshape in particular is only meaningful
  if T007 is observed red on the current tree
- Commit after each task or logical group; `mise run test-core` while iterating in `micold-core`,
  `mise run test` before pushing
- Expect to wait behind another worktree's build; do not point `CARGO_TARGET_DIR` somewhere private
- Every test touching Copilot's store must go through the T002 helper. A test that reads a real
  `~/.copilot` is a defect even when it passes

---

description: "Task list for 031 clickable terminal links"
---

# Tasks: Clickable Links in the Terminal

**Input**: Design documents from `/specs/031-clickable-terminal-links/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: MANDATORY (Constitution I). In every block, the tests come first and must fail before
their implementation tasks run. `[U#]` and `[A#]` markers name the behaviors of
[tdd/test-list.md](./tdd/test-list.md) each task writes or implements.

**Acceptance tests** (`[A#]`) live in `#[cfg(test)] mod acceptance` in
`crates/micold-client/src/shell/links.rs`, because `update_inner` and `base_app()` exist only in the
`micold-ai-ide` binary. Each drives the composed client: a `GridCache` holding the printed lines,
the real `TerminalPane` laid out on the headless tiny-skia renderer with real mouse and modifier
events, the published messages run through `crate::update_inner`, and recording `LinkOpener` and
clipboard capabilities. They are integration tests of the composed modules, not GUI end-to-end
tests; the visual pass covers the rendered window. Run them with
`scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide acceptance`.

**Documentation**: each story that users can see extends the "Links" subsection of
`docs/user-guide/worktrees-and-sessions.md` ("Interacting with the terminal") in its own milestone
(Constitution VII, FR-023).

**Cross-platform**: every platform decision in core is a parameter. The platform is read only in
`shell/link_opener.rs`, in the `shell/links.rs` fact gatherer, as `cfg!(windows)` values, and in
the `env_include.rs` builders (plan, Target Platform).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US4 from spec.md
- Paths are relative to the repository root.

---

## Phase 1: Setup

- [X] T001 Create the render-free module `crates/micold-core/src/link/`, with `mod.rs`, `detect.rs`, `line.rs`, `address.rs` and `resolve.rs` declared and empty, and add `pub mod link;` to `crates/micold-core/src/lib.rs`
- [X] T002 [P] Write `specs/031-clickable-terminal-links/scripts/links-fixture.sh`. It prints the quickstart §B.0 lines, emitting the OSC 8 runs with `printf '\e]8;;%s\e\\%s\e]8;;\e\\'`. It creates `/tmp/031-fixture/readme.txt`, `/tmp/031-fixture/run.sh` (`chmod +x`) and `/tmp/031-fixture/folder/`. It uses `$(hostname)` in the `file://` host line

---

## Phase 2: Foundational

- [X] T003 Define the shared core types from data-model §1. `LinkRows` and `Link` go in `crates/micold-core/src/link/mod.rs`; the rest go in `crates/micold-core/src/link/resolve.rs` and are re-exported from `link`.
  - `trait LinkRows { fn text(&self, row: i64) -> Option<&str>; fn wrapped(&self, row: i64) -> bool; fn hyperlink(&self, row: i64, col: u16) -> Option<&str>; fn spacer(&self, row: i64, col: u16) -> bool; }` (`spacer` added in M1, ledger decision 15). `row` is relative to the viewport top and may be negative.
  - `Link { address: String, origin: LinkOrigin, cells: Vec<CellSpan> }`, with `LinkOrigin { Detected, Declared }` and `CellSpan { row: i64, cols: Range<u16> }`. "Two `Link`s are the same link when `address`, `origin` and `cells` are equal" (derive `PartialEq, Eq`).
  - `LinkContext { host_names: Vec<String>, windows_host: bool, sandbox: Option<SandboxLinkContext> }`.
  - `SandboxLinkContext { host_names: Vec<String>, locations: Vec<SharedLocation>, denied: Vec<String> }`.
  - `SharedLocation { container: String, host: String }`.
  - `ResolvedLink { link: Link, display: String, target: Target, needs_confirmation: bool }`.
  - `Target { Url(String), HostPath(String), Unreachable(Reason) }`.
  - `LinkContext` and `ResolvedLink` derive `Clone, Debug, PartialEq, Eq`, because the pane's hover cache compares them.

**Checkpoint**: the link vocabulary compiles in `mise run test-core`.

---

## Phase 3: User Story 1 — Follow a web address printed in the terminal (Priority: P1) 🎯 MVP

**Goal**: an `http`, `https` or `mailto` address printed in any pane, including one the terminal soft-wrapped, is marked on hover and shows its address. Ctrl+click (Cmd+click on macOS) opens it in the default application, and selection is unchanged.

**Independent Test**: quickstart §B.1–B.6. Print `See https://example.com/docs/page.html for details.`, hover it, Ctrl+click it, and check that the browser receives exactly the address. Narrow the pane and repeat.

### Tests for US1: recognition in core ⚠️ write first, must fail

- [X] T004 [P] [US1] [U1] [U2] [U3] [U4] [U5] [U6] [U7] [U8] [U9] [U10] [U11] [U12] [U13] [U14] [U15] [U16] [U152] Unit tests for `detect(text) -> Vec<Range<usize>>` (char-index ranges) in `crates/micold-core/src/link/detect.rs`.
  - Every row of contract link-recognition §3.
  - Each rule of research R3:
    - A prefix preceded by `[A-Za-z0-9]` is rejected.
    - The scan stops at whitespace, control characters, `<`, `>`, `"`, `` ` ``, `{`, `}`, `|`, `\` and `^`.
    - "Trailing `.,;:!?'*` trimmed" repeatedly.
    - "closing bracket kept only if balanced inside".
    - The scan stops at an unbalanced closer.
    - "enclosing quote excluded".
    - A web host must be "`localhost`, dotted, a bracketed IPv6 literal, or carry a port".
    - `file` needs a path starting with `/`.
    - `mailto:` needs `@` with text on both sides.
- [X] T005 [P] [US1] [U17] [U18] [U19] [U20] [U21] [U22] [U23] [U24] [U25] [U26] [U27] [U151] [U153] [U154] Unit tests for `link_at(rows, row, col) -> Option<Link>` in `crates/micold-core/src/link/line.rs`, over a fake `LinkRows`.
  - Contract §2 L1–L7:
    - A declared run over the logical line.
    - A detected range mapped back to cells on every row.
    - `None` over plain text.
    - Rows join only across `wrapped = true`, "at most 64 rows each way", and a candidate reaching the cap is dropped.
    - A wide character's spacer cell belongs to the link.
    - Two same-URI runs separated by another cell are two links.
    - A candidate touching a `text = None` boundary is dropped.
  - Also: two rows separated by a real line break are never joined (FR-003). A declared URI wins over address-shaped visible text (US2 scenario 5).
- [X] T006 [P] [US1] [U28] [U29] [U30] [U33] [U34] [U46] Unit tests for `classify` and `resolve` in `crates/micold-core/src/link/address.rs` and `crates/micold-core/src/link/resolve.rs`.
  - C1–C3: `http`/`https`/`mailto` give `Target::Url(address)` with `display == address`. `vscode:`, `slack:`, `zoommtg:`, `javascript:`, `data:`, `vbscript:` and an unknown scheme give `None`.
  - The scheme compares ASCII case-insensitively.
  - `file:` gives `None` until T047 (the test is named so T042 replaces it).
  - The SC-006 invariant, checked over every case above (the profile has no property library): `display` equals the string the target carries.
- [X] T007 [P] [US1] [U49] A no-I/O test in `crates/micold-core/src/link/mod.rs`. It reads every `link/*.rs` source through `include_str!` and fails if any names `std::net`, `std::fs` or `std::process` (FR-019, quickstart §A.2). The test builds its needles at runtime (for example `["std", "fs"].join("::")`) so its own source never matches them. It also asserts that its `include_str!` list names exactly the modules `link/mod.rs` declares, so a module added later (T043's `runnable.rs`) cannot escape it.
- [X] T008 [US1] [U132] [U133] The SC-002 corpus.
  - `crates/micold-core/tests/fixtures/link_corpus.txt` holds at least 50 real lines of AI CLI and command-line output. A header documents the format: expected links are delimited inline, and groups of soft-wrapped rows are declared with a column width.
  - It includes punctuation, brackets, quotes, Markdown and angle forms, `localhost` ports, soft-wrapped addresses, scheme-less text, and addresses hard-broken across real line breaks.
  - `crates/micold-core/tests/link_corpus.rs` asserts:
    - 100% exact spans through `link_at`;
    - no scheme-less match;
    - a hard-broken address is recognised on its first row only;
    - the resolved `display` equals exactly the piece that opens.

### Implementation for US1: recognition in core

- [X] T009 [US1] [U1] [U2] [U3] [U4] [U5] [U6] [U7] [U8] [U9] [U10] [U11] [U12] [U13] [U14] [U15] [U16] [U152] [U132] [U133] Implement `detect` (research R3, rules 1–6, hand-written, no new crate) in `crates/micold-core/src/link/detect.rs`
- [X] T010 [US1] [U17] [U18] [U19] [U20] [U21] [U22] [U23] [U24] [U25] [U26] [U27] [U133] [U151] [U153] [U154] Implement `link_at` in `crates/micold-core/src/link/line.rs`, per research R4 and R5.
  - Build the logical line by walking back while the previous row is `wrapped`, then forward, capped at 64 rows each way.
  - Map char indices to cells, keeping wide-char spacers.
  - Take the declared maximal run first.
  - Otherwise map a `detect` range to cells, and drop candidates at the cap or at a `None` boundary.
- [X] T011 [US1] [U28] [U29] [U30] Implement `Address` and `classify` in `crates/micold-core/src/link/address.rs`: `Web(String)` for `http`/`https` verbatim, `Mail(String)` for `mailto` verbatim, and `NotFollowable` for everything else. `file` is also `NotFollowable` until T046.
- [X] T012 [US1] [U33] [U34] [U46] Implement `resolve(link: Link, ctx: &LinkContext) -> Option<ResolvedLink>` in `crates/micold-core/src/link/resolve.rs` for `Web` and `Mail` (C1–C2): `target = Url(address)`, `display = address`, `needs_confirmation = false`. Anything else gives `None`.

**Checkpoint (M1)**: `mise run test-core` passes `link::*` and `link_corpus`. No client change yet.

### Tests for US1: opening ⚠️ write first, must fail

- [X] T013 [P] [US1] [U73] [U74] [U75] [U76] Create `crates/micold-client/tests/features_session_links.rs` with T5: `SessionMsg::LinkActivated(r)` with `r.target = Url(u)` gives `Outcome::OpenLink(OpenRequest::Url(u))`. Also T12/T13: `LinkOpenFinished { address, result: Err(NoApplication) }` notifies `Couldn't open <address>: no application is set up to open it`, `Err(LaunchFailed(e))` notifies `Couldn't open <address>: <e>`, and `Ok(())` does nothing.
- [X] T014 [P] [US1] [U93] [U94] A `#[cfg(test)]` module in `crates/micold-client/src/shell/links.rs`, declared as `pub mod links;` in `crates/micold-client/src/shell/mod.rs` so it compiles and its red is seen, testing the route and SC-003.
  - `crate::update_inner` on `base_app()` with `caps.with_link_opener(recording)` given `Message::Session(SessionMsg::LinkActivated(url link))`: running the returned task calls `open` with the URL verbatim.
  - No timer or debounce sits between the outcome and the call.
- [X] T015 [US1] [U101] [U102] [U103] [U104] [U138] [U139] [U144] Tests in the `#[cfg(test)]` module of `crates/micold-client/src/shell/link_opener.rs`, declaring `pub mod link_opener;` in `crates/micold-client/src/shell/mod.rs` so the module compiles and its red is seen (after T014, which edits the same `mod.rs`). The launch-window cases are `#[cfg(unix)]` and drive the window helper with stub children; the classifier and reveal-fallback cases are not `cfg`-gated.
  - Exit 0 is `Ok`.
  - Exit 3 is `NoApplication`.
  - Another non-zero exit is `LaunchFailed`.
  - A child still sleeping after 2 s is `Ok` and is left running (plan Risk 10).
  - macOS classification through `classify_macos_open(exit_code, stderr) -> Result<(), OpenFailure>`, which is not behind a macOS `cfg` so the Linux gate runs it: non-zero with stderr naming no application is `NoApplication`, otherwise `LaunchFailed` (contract link-opening §2).
  - Windows classification through `classify_shell_execute(ret: isize) -> Result<(), OpenFailure>`, not `cfg`-gated: 33 is `Ok`; 31 (`SE_ERR_NOASSOC`) and 27 (`SE_ERR_ASSOCINCOMPLETE`) are `NoApplication`; 32 and 2 are `LaunchFailed`.
  - Linux reveal through `reveal_linux(runner: &dyn CommandRunner, path)`, also not `cfg`-gated: when the stub runner fails the `dbus-send` `ShowItems` call, the next command is the opener with the parent folder (FR-013).
- [X] T016 [P] [US1] [U105] Add a named client-side check to `crates/micold-client/tests/no_concrete_implementations.rs` (its derivation reads only `micold-core`, so it cannot see a client type): scanning `crates/micold-client/src/`, `SystemLinkOpener` occurs outside `use` lines only in `shell/link_opener.rs` (its definition) and in `Capabilities::real()` in `shell/capabilities.rs`. A vacuity assertion requires the definition to exist, so the test is red until T019 and T020 land

### Implementation for US1: opening

- [X] T017 [US1] [U73] Add the effect vocabulary.
  - `crates/micold-client/src/features/mod.rs` gains `Outcome::OpenLink(OpenRequest)`, with `OpenRequest = Url(String) | Path { path: String, address: String }`, and `OpenFailure { NoApplication, LaunchFailed(String), NotFound }`, both deriving `Clone, Debug, PartialEq, Eq`. `OpenFailure` lives here so that render-free features can name it; `shell/link_opener.rs` re-exports it.
  - Add an empty `Outcome::OpenLink(_)` arm to `app::interpret` in `crates/micold-client/src/app.rs`.
  - Add `Outcome::OpenLink(_)` to the no-op arm of `interpret` in `crates/micold-client/src/shell/clipboard.rs`. `tests/clipboard_request.rs` must still pass.
- [X] T018 [US1] [U73] [U74] [U75] [U76] In `crates/micold-client/src/features/session.rs`, add `SessionMsg::LinkActivated(ResolvedLink)` and `SessionMsg::LinkOpenFinished { address: String, result: Result<(), OpenFailure> }`, with reducers: T5 emits `OpenLink(Url(u))`, T12 calls `notify_error` with the §5 texts, and T13 does nothing. Update any gate that enumerates `SessionMsg` variants (for example `tests/features_are_render_free.rs` or `tests/outcome_termination.rs`, if they list them).
- [X] T019 [US1] [U101] [U102] [U103] [U104] [U105] [U138] [U139] [U144] Implement `crates/micold-client/src/shell/link_opener.rs` (declared by T015). Every argument is a single argv element and no shell is involved.
  - `trait LinkOpener: Send + Sync { fn open(&self, target: &str) -> Result<(), OpenFailure>; fn reveal(&self, path: &Path) -> Result<(), OpenFailure>; }`.
  - `SystemLinkOpener` implements it, with one arm per platform from contract link-opening §2:
    - **Linux**: `open` is `xdg-open`, with null stdio and the 2 s launch window. `reveal` is `dbus-send … org.freedesktop.FileManager1.ShowItems array:string:file://<path> string:""`, falling back to `open(<parent>)`.
    - **macOS**: `open`, and `open -R` to reveal.
    - **Windows**: `ShellExecuteW(null, "open", target, null, null, SW_SHOWNORMAL)`, its return mapped by `classify_shell_execute`: `SE_ERR_NOASSOC` and `SE_ERR_ASSOCINCOMPLETE` map to `NoApplication`, and any other return ≤ 32 to `LaunchFailed`. Reveal runs `explorer.exe` with `raw_arg(format!("/select,\"{path}\""))`.
  - The testable seams are plain functions compiled on every OS: `classify_macos_open(exit_code: i32, stderr: &str) -> Result<(), OpenFailure>`, `classify_shell_execute(ret: isize) -> Result<(), OpenFailure>`, and `reveal_linux(runner: &dyn CommandRunner, path: &Path)` over a private `trait CommandRunner { fn run(&self, program: &str, args: &[&OsStr]) -> Result<(), OpenFailure>; }` that `SystemLinkOpener` implements with the launch window.
  - `#[cfg(test)] pub(crate) struct NoopLinkOpener`, whose methods return `Ok(())`, for `base_app()` (T020).
  - `crates/micold-client/Cargo.toml` gains `windows-sys = { workspace = true }` under `[target.'cfg(windows)'.dependencies]`.
  - The workspace `Cargo.toml` `windows-sys` entry gains `Win32_UI_Shell` and `Win32_UI_WindowsAndMessaging`.
- [X] T020 [US1] [U93] [U105] Add `link_opener: Arc<dyn LinkOpener>` to `Capabilities` in `crates/micold-client/src/shell/capabilities.rs`. `real()` constructs `SystemLinkOpener`. Add `#[cfg(test)] pub(crate) fn with_link_opener(mut self, opener: Arc<dyn LinkOpener>) -> Self`, modelled on `without_settings`. `base_app()` in `crates/micold-client/src/main.rs` becomes `Capabilities::real().with_link_opener(Arc::new(NoopLinkOpener))`, so no test can reach the system opener, and link tests replace it with a recording opener.
- [X] T021 [US1] [U93] [U94] Implement `crates/micold-client/src/shell/links.rs` (declared by T014).
  - `on_link_message(app: &mut App, msg: SessionMsg) -> Task<Message>` runs the session reducer, then splits its outcomes:
    - `OpenLink` goes to `perform`;
    - `ClipboardWrite` goes to `shell::clipboard::interpret`;
    - every other outcome goes through `app::drain`/`app::interpret`.
  - `perform(OpenRequest::Url(u))` is `Task::perform` over `tokio::task::spawn_blocking` calling `opener.open(&u)`, and maps the result to `LinkOpenFinished { address: u, result }`.
- [X] T022 [US1] [U93] In `update_inner` in `crates/micold-client/src/main.rs`, add a `Message::Session(SessionMsg::LinkActivated(_))` arm ahead of the general `Message::Session` arm, the way `RemoveConfirmed` is routed, calling `shell::links::on_link_message` (glue)

**Checkpoint (M2)**: a `LinkActivated` for a web or mail address reaches the system opener through `update_inner`, and a failure notifies. Nothing in the UI emits it yet; M3 wires the pane.

### Tests for US1: the pane ⚠️ write first, must fail

- [x] T082 [US1] [A1] [A2] [A3] [A4] [A5] [A6] [A13] Outer-loop acceptance tests in `#[cfg(test)] mod acceptance` of `crates/micold-client/src/shell/links.rs`, with the shared headless harness (a `GridCache` fed rows, a laid-out `TerminalPane`, events in, messages through `crate::update_inner` on `base_app()` with `caps.with_link_opener(recording)`). One test per behavior:
  - A1: hovering any character of `https://example.com/docs/page.html` in `See https://example.com/docs/page.html for details.` marks exactly the address's cells, and the mouse interaction is `Pointer`.
  - A2: a Ctrl/Cmd press and release on it calls the opener once with the address, and no `TerminalBytes` is published.
  - A3: a soft-wrapped address opens complete from either row.
  - A4: `(https://example.com/a_(b))` opens `https://example.com/a_(b)`; `"https://example.com"` opens `https://example.com`.
  - A5: a plain drag, double- or triple-click starting on a link selects and calls no opener.
  - A6: an address in scrollback, with the view scrolled to it, opens the same address.
  - A13: `mailto:team@example.com` reaches the opener verbatim.
  They must fail (no hover, no activation) before T028–T030, except A5, a guard that is green on arrival (selection already works and nothing opens yet): its red is shown by the mutant in `tdd/test-list.md`.
- [x] T083 [US2] [A7] [A8] [A9] [A10] [A11] Outer-loop acceptance tests in `mod acceptance` of `crates/micold-client/src/shell/links.rs`, over grid rows whose cells carry declared hyperlinks:
  - A7: hovering `docs` declared as `https://example.com/manual` marks the run, and the hint's text is `https://example.com/manual`.
  - A8: activating it calls the opener with `https://example.com/manual`.
  - A9: two adjacent runs with different declared addresses resolve to their own addresses.
  - A10: same-URI runs separated by plain text: hovering one marks only that run.
  - A11: text reading `https://a.example` declared as `https://b.example` shows and opens `https://b.example`.
  They must fail (no hover, no activation) before T028–T030. They sit in M3 because the pane's hover resolves declared runs from the start.
- [x] T037 [US2] [U129] Declared-link hover tests in the `#[cfg(test)]` module of `crates/micold-client/src/ui/material/terminal_pane.rs`, over a fake grid.
  - Hovering `docs` (declared `https://example.com/manual`) resolves that address as `display`, and the underline covers exactly the run.
  - Two adjacent runs with different URIs are two links.
  - Two same-URI runs separated by plain text mark only the hovered run.
  - Address-shaped visible text resolves to the declared URI (US2 scenarios 1, 3–5).
- [x] T023 [US1] [U106] [U107] [U108] [U109] [U110] [U111] [U112] [U113] [U114] [U115] [U116] [U142] `link_gesture` tests in the `#[cfg(test)]` module of `crates/micold-client/src/ui/material/terminal_pane.rs`.
  - Contract link-opening §1: G1–G9 and G3b, including G7: a middle click or wheel over a link behaves as today.
  - A step never writes to the PTY, changes the selection or scrolls (FR-014).
  - The SC-004 script: 100 scripted plain drag, double-click and triple-click selections starting on links produce 0 `LinkActivated`. A modifier double-click on a link produces exactly 1, and its second press counts as a double click because the link press recorded `last_click` through `Click::new`.
- [x] T024 [US1] [U125] [U126] [U127] [U128] `link_hint_rect(content, pointer_row, hint_size) -> Rectangle` tests in the same module.
  - Bottom-left by default.
  - Top-left when the pointer is within the hint's height of the bottom.
  - Always inside the content bounds.
  - The middle-elided label leaves `ResolvedLink.display` complete.
- [x] T025 [US1] [U118] [U119] [U120] [U121] [U122] [U123] [U124] Hover tests in the same module.
  - Hover is recomputed when the pointer cell changes, and on `window::Event::RedrawRequested` when `(generation, seq)` moved *and* the hash of the consulted rows changed. This includes an in-place redraw that keeps the same `LineId`.
  - It is reused, without re-running `link_at`, when the hash is unchanged.
  - It is recomputed when the session shown or the `LinkContext` changes, and on `ModifiersChanged` under mouse reporting.
  - `mouse_interaction` is `Pointer` over a followable link only while the link modifier (`command()`) is held, and back to the text pointer when it is released; under mouse reporting only while Shift and the link modifier are held (clarification 2026-09-16).
  - Two `PaneState`s: hovering and pressing in one leaves the other's `hover` and `link_press` `None`, and only the pressed pane emits `LinkActivated` (FR-022).
- [x] T026 [P] [US1] [U131] A new test module in `crates/micold-client/src/showcase/samples.rs`: the terminal sample's grid carries a detected `https://` address and a cell with a declared hyperlink
- [x] T027 [P] [US2] [U72] Create `crates/micold-daemon/tests/osc8_passthrough.rs` (research R13).
  - The test binary re-executes itself through the real PTY supervisor, as a child gated by an environment flag, and the child prints one OSC 8 link.
  - The test asserts that the resulting grid cell's hyperlink equals the URI.
  - It must run on all three CI OSes. If the Windows job shows ConPTY drops the sequence, mark the Windows arm `#[cfg_attr(windows, ignore = "<CI run URL>")]` and record the finding in research R13 and in T040.

### Implementation for US1: the pane

- [x] T028 [US1] [U118] [U119] [U120] [U121] [U122] [U123] [U124] [U129] [A1] [A7] [A9] [A10] [A11] Hover in `crates/micold-client/src/ui/material/terminal_pane.rs`.
  - A `LinkRows` adapter over `GridCache`: `line(LineId(viewport_top - display_offset + row))`, with `text`, `wrapped`, `CachedExtra.hyperlink`, and `spacer` from the cell's style-run flags (`WIDE_CHAR_SPACER | LEADING_WIDE_CHAR_SPACER`; ledger decision 15).
  - A row above the first line the terminal ever printed, or above the top of the alternate screen (`less`, `vim`), answers `text = Some("")`, unwrapped, not `None`: `None` means "may continue past here" (contract L7), so it would drop an address at column 0 of a session's first line. Rows trimmed from scrollback (older than `GridCache::oldest_available`) or not yet cached still answer `None` (M1 review B).
  - `PaneState.hover: Option<HoverCache { session, context, cell, grid_version: (u64, u64), rows_hash: u64, resolved: Option<ResolvedLink> }>`.
  - The pure invalidation function.
  - In `update`, recompute on `CursorMoved`, `ModifiersChanged` and `window::Event::RedrawRequested`, which reads `GridCache::generation()`/`seq()`. Request a redraw when `resolved` changes.
  - The pane builder takes a `LinkContext`.
- [x] T029 [US1] [U106] [U107] [U108] [U109] [U110] [U111] [U112] [U113] [U114] [U115] [U116] [U142] [A2] [A3] [A4] [A5] [A6] [A8] [A11] [A13] The gesture in `crates/micold-client/src/ui/material/terminal_pane.rs`.
  - `PaneState.link_press: Option<LinkPress { link: ResolvedLink, cell: (u16, u16) }>` and the pure `link_gesture`.
  - **Press**: `modifiers.command()`, routing is terminal (`press_routing`), single cadence, and `hover.resolved` is `Some`. The press records `link_press`, updates `last_click` through `Click::new`, focuses as today, and emits no `TerminalSelectStart`.
  - **Move off the press cell**: `TerminalSelectStart` at the press cell.
  - **Release on the press cell**: re-resolve and publish `Message::Session(SessionMsg::LinkActivated(r))` once.
- [x] T030 [US1] [U123] [U125] [U126] [U127] [U128] Drawing in `crates/micold-client/src/ui/material/terminal_pane.rs`.
  - An underline under the hovered link's cells inside the viewport, in each cell's own foreground colour, so text colour, cursor and selection stay visible (FR-009).
  - The hint label at `link_hint_rect`, middle-elided, using the existing surface and on-surface theme roles.
  - `mouse_interaction` returns `Pointer` over a link only while the link modifier is held (the pane keeps the modifiers from `ModifiersChanged`), and under mouse reporting only while Shift and the link modifier are held.
- [x] T031 [US1] [U134] In `crates/micold-client/src/ui/terminal.rs` and its call site in `crates/micold-client/src/ui/mod.rs`, build the pane's `LinkContext { host_names: Vec::new(), windows_host: cfg!(windows), sandbox: None }`; `ui::view`'s signature is unchanged. T050 fills in `host_names` and `sandbox` (glue).
- [x] T032 [US1] [U131] Give the terminal sample in `crates/micold-client/src/showcase/samples.rs` a detected `https://` address and a run of cells carrying a declared hyperlink, so that T026 passes
- [x] T033 [US1] Add a "Links" subsection under "Interacting with the terminal" in `docs/user-guide/worktrees-and-sessions.md`. It covers:
  - hover marking and the address hint;
  - Ctrl+click on Linux and Windows, Cmd+click on macOS;
  - plain clicks, drags and double or triple clicks still select;
  - Shift+Ctrl/Cmd+click under mouse-reporting programs;
  - web and mail addresses open in the default browser or mail client;
  - a link a program declared behind its text is marked as that text and shows, and opens, the address it declares;
  - the "Couldn't open …" notification.
- [x] T034 [US1] Verify the milestone: `mise run gate`, then `cargo check --target aarch64-apple-darwin -p micold-client`, then the `visual-pass` skill for quickstart §B.1–B.7 and §B.12 using `scripts/links-fixture.sh`. Record rows in `specs/031-clickable-terminal-links/visual-pass.md`. US1 and US2's pane scenarios are complete only when the T082 and T083 acceptance tests A1–A11 and A13 are green. It confirms acceptance rows A1–A11 and A13 and carries no markers, so it is not ticked by them.

**Checkpoint (M3)**: US1, and US2's declared links in the pane, work end to end on Linux, and CI is green on all three OSes.

---

## Phase 4: User Story 2 — Follow a hyperlink a program declared behind its text (Priority: P2)

**Goal**: declared (OSC 8) links are marked as maximal runs and show their declared address before opening (built with the pane in M3: T083, T037). Sessions no longer inherit another terminal's identity, and `FORCE_HYPERLINK=1` from the include script is the documented opt-in.

**Independent Test**: quickstart §B.7, §B.14 and §B.17.

### Tests for US2 ⚠️ write first, must fail

- [ ] T035 [P] [US2] [U64] [U65] [U66] [U67] Unit tests for `is_inherited_terminal_identity(key, value, keys_case_insensitive)` in `crates/micold-core/src/env_include.rs`.
  - Contract session-terminal-identity §1, every row: the nine keys are always true, and `COLORTERM` is true unless its value is `truecolor` or `24bit` (ASCII case-insensitive).
  - Keys compare exactly when the flag is false and case-insensitively when it is true.
  - The Unix `bash` builder, and both Windows `powershell.exe` builders (built on every OS without running them), take the inherited environment as a parameter. Given a fixed set holding `TERM_PROGRAM=WezTerm`, `FORCE_HYPERLINK=1`, `COLORTERM=truecolor` and `HOME=/h`, each records exactly the `env_remove`s for `TERM_PROGRAM` and `FORCE_HYPERLINK`, checked through `Command::get_envs()`, so the test does not depend on the CI runner's environment.
- [ ] T036 [P] [US2] [U68] [U69] [U70] [U71] Create `crates/micold-daemon/tests/session_identity_env.rs`, against a real spawned session via `spawn_shell` and `spawn_ai_cli`. Set the inherited environment on a re-executed test child, never with `std::env::set_var`. It asserts every row of contract §2's table:
  - inherited `TERM_PROGRAM=WezTerm` and `FORCE_HYPERLINK=1` are absent;
  - include-script `FORCE_HYPERLINK=1` is present, including when it is also inherited;
  - `COLORTERM=truecolor` is kept;
  - `TERM=xterm-256color` is unchanged.
### Implementation for US2

- [ ] T038 [US2] [U64] [U65] [U66] [U67] Implement `is_inherited_terminal_identity` in `crates/micold-core/src/env_include.rs`. Call `env_remove` for every matching variable of an `inherited: impl IntoIterator<Item = (OsString, OsString)>` parameter at the three include-shell `Command` construction sites, whose callers pass `std::env::vars_os()`: the Unix `bash` builder shared by `baseline_env`/`attempt_env`, and the two Windows `powershell.exe` builders. Pass `cfg!(windows)`.
- [ ] T039 [US2] [U68] [U69] [U70] [U71] Add `strip_inherited_terminal_identity(&mut CommandBuilder)` in `crates/micold-daemon/src/supervisor.rs`. It calls `env_remove` for each inherited variable the predicate matches. Call it in both `spawn_ai_cli` and `spawn_shell` **before** `TERM` and `spec.env` are applied.
- [ ] T040 [US2] Extend "Links" in `docs/user-guide/worktrees-and-sessions.md`.
  - Programs decide whether to declare links, and micold does not advertise support.
  - Opt in with `export FORCE_HYPERLINK=1` in the session environment-include script.
  - AI CLIs that break their own lines show a long address as a truncated first-row link, so check the hint.
  - The Windows caveat, if T027 found one.
- [ ] T041 [US2] Verify the milestone: `mise run gate`, then the `visual-pass` skill for quickstart §B.14 and §B.17, recorded in `visual-pass.md`. `tests/session_identity_env.rs` and the `env_include` predicate tests are green. (A7–A11 were made green in M3 by T034.)

**Checkpoint (M4)**: US1 works, US2 works except scenario 6 (a `file` link, M5), and every session's identity is micold's own.

---

## Phase 5: User Story 3 — Open non-web links in their related application (Priority: P3)

**Goal**: `file` links open documents and folders in their applications, runnable files are revealed and never run, other hosts are not links, and a sandboxed session's file links are translated to host paths and confirmed first.

**Independent Test**: quickstart §B.8–B.10, §B.13, §B.15 and §B.16.

### Tests for US3: host file links ⚠️ write first, must fail

- [ ] T084 [US3] [A12] [A14] [A15] [A16] [A17] [A18] Outer-loop acceptance tests in `mod acceptance` of `crates/micold-client/src/shell/links.rs`, over real files in a `tempfile` directory and `app::State.host_names` set to a known name: A18 is a guard, green on arrival (after T011 those addresses are already `NotFollowable`); its red is shown by the mutant in `tdd/test-list.md`.
  - A12: a declared `file://<host name><dir>/readme%20a.txt` link calls `opener.open` with the decoded existing path.
  - A14: a `file://` link to an existing document calls `opener.open` with its path.
  - A15: a `file://` link to an existing folder calls `opener.open` with the folder path.
  - A16: a `file://` link to a runnable file (the execute bit on Unix, a `%PATHEXT%` extension on Windows) calls `opener.reveal` and never `opener.open`.
  - A17: a link to a missing file calls no opener and raises `Couldn't open <address>: the file doesn't exist on this machine`.
  - A18: `file://otherhost/x`, `javascript:alert(1)` and `vscode://x` are not marked on hover, and activating them calls no opener.
- [ ] T087 [US3] [U140] [U141] Unit tests in `crates/micold-core/src/link/resolve.rs` for two pure helpers.
  - `host_names_from(raw: &str) -> Vec<String>`: `build.example.com` gives `["build.example.com", "build"]`; `devbox` gives `["devbox"]`; an empty name gives `[]`; a name whose first label equals the whole name is listed once.
  - `container_host_names(id: &str) -> Vec<String>`: a 64-character id gives its 12-character prefix and the full id; a 12-character id gives it once; an 8-character id gives it once.
- [ ] T042 [P] [US3] [U31] [U32] [U35] [U36] [U37] [U41] [U46] Replace the `file:` → `None` test from T006 in `crates/micold-core/src/link/address.rs` and `crates/micold-core/src/link/resolve.rs` with contract link-recognition C4–C10 and C18, plus the rules below. Extend the SC-006 property to `HostPath`.
  - C5 compares hosts ASCII case-insensitively.
  - C6 covers `file://otherhost/…` and `file://server/share/…` (US3 scenario 6).
  - C9 percent-decodes `%20`.
  - C10: "Percent-decoding failure or non-UTF-8 ⇒ `NotFollowable`".
  - U32 and U36 are guards: `file:` is `NotFollowable` and resolves to `None` until T046–T047, so they pass when written. Their red is shown after T046–T047 by the mutants in `tdd/test-list.md`.
  - With `ctx.sandbox = Some` and no locations, a file link gives `Target::Unreachable`, with `display = "<path> — not reachable from this machine"` (C12).
- [ ] T043 [P] [US3] [U50] [U51] [U52] [U53] [U54] [U55] Unit tests for `action_for(platform, name, facts, pathext) -> FileAction` in the new `crates/micold-core/src/link/runnable.rs`, declared as `pub mod runnable;` in `crates/micold-core/src/link/mod.rs` so it compiles and its red is seen. Add `runnable.rs` to T007's `include_str!` list in the same change.
  - Every row of contract link-opening §3 for Linux, macOS and Windows, run on every OS.
  - Extensions compare ASCII case-insensitively on the last extension, and `AppImage` matches.
  - A macOS bundle directory (by extension or `is_bundle`) reveals. Other folders open.
  - Windows `pathext` entries reveal, and `any_exec_bit` is ignored on Windows.
- [ ] T044 [P] [US3] [U77] [U78] [U79] Extend `crates/micold-client/tests/features_session_links.rs`.
  - T6: `HostPath(p)` with `!needs_confirmation` gives `OpenLink(Path { path: p, address: r.link.address })`.
  - T8: `Unreachable` notifies `Couldn't open <address>: the sandbox doesn't share that location with this machine`.
  - `LinkOpenFinished` with `Err(NotFound)` notifies `Couldn't open <address>: the file doesn't exist on this machine`.
- [ ] T045 [P] [US3] [U95] [U96] [U97] [U146] Extend the `#[cfg(test)]` module of `crates/micold-client/src/shell/links.rs` with T11 using the recording opener.
  - A missing path gives `LinkOpenFinished { address, result: Err(NotFound) }`.
  - A document calls `open(p)`, and a runnable file calls `reveal(p)`.
  - SC-007 real files, on each CI OS, through the real fact gatherer:
    - every runnable kind for that OS reveals 100%: Unix execute bit, and a symlink to a file with it; on Linux a `.desktop` and an `.AppImage` file without the bit; on macOS a `.app` directory, a directory without a bundle extension that holds `Contents/Info.plist`, and a `.command` file without the bit; on Windows `%PATHEXT%` extensions; each also inside a directory whose name contains a space;
    - `.txt`, `.png`, `.pdf` and `.html` open 100%.

### Implementation for US3: host file links

- [ ] T046 [US3] [U31] [U32] In `crates/micold-core/src/link/address.rs`, add `Address::File { host: String, path: String }` for `file://host/path`, with a hand-written percent-decoder (about 20 lines, no new crate). An invalid escape or non-UTF-8 gives `NotFollowable`.
- [ ] T047 [US3] [U35] [U36] [U37] [U41] [A12] [A14] [A15] [A16] [A17] [A18] The `File` branch of `resolve` in `crates/micold-core/src/link/resolve.rs`.
  - The host must be "empty, `localhost`, or in `host_names` / `sandbox.host_names` (ASCII case-insensitive)", otherwise `None`.
  - Not sandboxed: `windows_host` turns `/C:/…` into `C:\…`, and a path without a drive gives `None`. Otherwise `HostPath(path)` with `display = path` and `needs_confirmation = false`.
  - Sandboxed: `Unreachable` for now. T062 adds translation.
- [ ] T048 [US3] [U50] [U51] [U52] [U53] [U54] [U55] Implement `HostPlatform { Linux, MacOs, Windows }`, `FileFacts { kind: Kind{File, Dir}, any_exec_bit: bool, is_bundle: bool }`, `FileAction { Open, Reveal }` and `action_for`, with FR-013's lists verbatim, in `crates/micold-core/src/link/runnable.rs` (declared by T043).
- [ ] T088 [US3] [U140] [U141] Implement `host_names_from` and `container_host_names` in `crates/micold-core/src/link/resolve.rs`, re-exported from `link`.
- [ ] T049 [US3] [U136] Add `app::State.host_names: Vec<String>` (empty by `Default`) in `crates/micold-client/src/app.rs`. `crates/micold-client/src/main.rs` fills it at boot with `link::host_names_from(&gethostname().to_string_lossy())` (glue). Add `gethostname = "1.1"` to `crates/micold-client/Cargo.toml` (already in `Cargo.lock`).
- [ ] T050 [US3] [U134] In `crates/micold-client/src/ui/terminal.rs`, build the `LinkContext` with `host_names` from `state.host_names`. While `sandbox.state` is `Running(id)` or `Stale(id)`, set `sandbox = Some(SandboxLinkContext { host_names: link::container_host_names(id), locations: vec![], denied: vec![] })`; otherwise `None`. So every sandboxed file link is "not reachable" until T066 (glue).
- [ ] T051 [US3] [U95] [U96] [U97] [U146] [A12] [A14] [A15] [A16] `perform(OpenRequest::Path { path, address })` in `crates/micold-client/src/shell/links.rs`, in one `spawn_blocking` task.
  - `std::fs::metadata(path)` follows symlinks. `NotFound` gives `LinkOpenFinished { address, result: Err(NotFound) }`.
  - Otherwise gather `FileFacts`:
    - `#[cfg(unix)]`: `PermissionsExt::mode() & 0o111 != 0`;
    - macOS: `is_bundle` by extension or `Contents/Info.plist`;
    - `#[cfg(windows)]`: `any_exec_bit = false`, with `PATHEXT` read from the environment.
  - Then call `action_for` with the `cfg`-selected `HostPlatform`, followed by `opener.open(path)` or `opener.reveal(path)`.
- [ ] T052 [US3] [U77] [U78] [U79] [A17] Reducers T6 and T8 for `LinkActivated`, and the `NotFound` notification text, in `crates/micold-client/src/features/session.rs`
- [ ] T053 [US3] Extend "Links" in `docs/user-guide/worktrees-and-sessions.md`.
  - `file://` links: documents open in their application, and folders in the file manager.
  - Runnable files (an execute bit, launchers, installers, per platform) are revealed, never run.
  - A `file` link naming another machine is not a link.
  - A missing file shows a notification.
  - A sandboxed session's file links show as not reachable. T069 replaces this.
- [ ] T054 [US3] Verify the milestone: `mise run gate`, `cargo check --target aarch64-apple-darwin -p micold-client`, then the `visual-pass` skill for quickstart §B.8–B.10 and §B.13. This part of US3 is complete only when the T084 acceptance tests A12 and A14–A18 are green. It confirms acceptance rows A12 and A14–A18 and carries no markers, so it is not ticked by them.

**Checkpoint (M5)**: host `file` links open, and runnable files are revealed.

### Tests for US3: sandbox translation and confirmation ⚠️ write first, must fail

- [ ] T085 [US3] [A19] Outer-loop acceptance test in `mod acceptance` of `crates/micold-client/src/shell/links.rs`: with `App.sandbox` `Running` and locations sharing `/work/<project>` from a `tempfile` host directory holding `readme.txt`, activating `file:///work/<project>/readme.txt` opens the `confirm_link_open` surface naming the host path and calls no opener; publishing **Open** (`LinkOpenConfirmed`) then calls `opener.open` with that host path.
- [ ] T055 [P] [US3] [U40] [U42] [U43] [U44] [U48] [U145] Unit tests for `reverse(locations, denied, container_path, windows_host) -> Option<String>` in `crates/micold-core/src/sandbox/pathmap.rs`.
  - Most-specific-first nesting (C15).
  - "A sandbox path is normalised lexically; climbing above `/` ⇒ `Unreachable`" (C13). Normalisation happens before the location match: with location `/work/proj`, `/work/proj/../../etc/passwd` maps to nothing, and `/work/proj/a/../b` maps to `<host>/b`.
  - Whole path components only: `/work/projector` is not under `/work/proj`.
  - Windows host pairs join with `\` (C14).
  - A result equal to or under a `denied` path gives `None` (C16b).
- [ ] T056 [P] [US3] [U56] [U57] [U58] [U59] [U60] [U61] [U62] [U63] Unit tests in `crates/micold-core/src/sandbox/parse.rs`, `lifecycle.rs` and `mod.rs`.
  - `ContainerFacts.mount_destinations` parsed from `Mounts[].Destination` in Docker and Podman inspect fixtures, and empty when `Mounts` is missing.
  - `Started.mounted` is `None` on `Create` and `Replace`, and `Some(destinations)` on `Attach` and `Start`.
  - `MountSet::shared_locations(mounted)` lists projects, state, home and credentials, sorted by container-path component count (descending). "The secret mount is never a shared location", and "A location the running container does not mount is not shared" (C16, C16c).
  - The denied list holds the secret mount's host path (`state_dir.join("sandbox.token")`).
- [ ] T057 [P] [US3] [U38] [U39] [U41] [U47] Replace T042's "sandboxed → `Unreachable`" case in `crates/micold-core/src/link/resolve.rs` with C11–C17, extending the SC-006 property.
  - The container-id prefix and the full id are accepted as hosts.
  - `HostPath` with `needs_confirmation = true`.
  - C12 `/tmp/x` gives `Unreachable`.
  - C16b: the token is denied.
  - C16c: an unmounted project gives `Unreachable`.
  - C17: `host_names` still translates.
- [ ] T058 [P] [US3] [U90] [U91] [U92] Tests in the `#[cfg(test)]` module of `crates/micold-client/src/features/sandbox.rs`.
  - T18: `started(started, locations)` stores them, for a created and for an attached container.
  - T19: every transition out of `Running`/`Stale` (observe `Stopped`, `failed`, `container_lost`, `accept_fallback`) and `for_placement` make `locations()` `None`.
  - A `Stale` sandbox still returns its locations.
- [ ] T059 [P] [US3] [U80] [U81] [U82] [U83] [U84] Extend `crates/micold-client/tests/features_session_links.rs`. Every case clears `pending_link_open`.
  - T7: `needs_confirmation` sets `pending_link_open = Some { active session, r }`, and the `ConfirmLinkOpenDialog` surface is open showing `r.display`.
  - T9, via `link_open_confirmed(state, sandbox_live)`:
    - the session is present and `sandbox_live` is true: `OpenLink(Path)`;
    - the session is gone: `Couldn't open <host path>: the session has closed`;
    - `sandbox_live` is false: `Couldn't open <host path>: the sandbox has stopped`.
  - T10: `LinkOpenDeclined` opens nothing.
- [ ] T060 [P] [US3] [U98] Extend the `#[cfg(test)]` module of `crates/micold-client/src/shell/links.rs`. `update_inner` with `LinkOpenConfirmed` reads `sandbox_live` from `App.sandbox`: when it is `Running`, the recording opener is called; when it is stopped, a notification and no call.
- [ ] T061 [P] [US3] [U137] Add `ConfirmLinkOpenDialog` rows to `crates/micold-client/tests/overlay_registry.rs`, `overlay_dispatch_ordering.rs`, `overlay_dismissal_delta.rs` and `overlay_transition_identity.rs`, and change `DIALOGS` from 9 to 10 in `crates/micold-client/tests/popover_displacement.rs`

### Implementation for US3: sandbox translation and confirmation

- [ ] T062 [US3] [U38] [U39] [U40] [U42] [U43] [U44] [U47] [U48] [U145] Implement `reverse` in `crates/micold-core/src/sandbox/pathmap.rs`, and call it from `resolve`'s sandboxed `File` branch in `crates/micold-core/src/link/resolve.rs`: a match gives `HostPath` with `needs_confirmation = true`, and no match gives `Unreachable`.
- [ ] T063 [US3] [U56] [U57] [U58] [U59] [U60] [U61] [U62] [U63] The mount-fact chain in core.
  - `crates/micold-core/src/sandbox/parse.rs`: `ContainerFacts.mount_destinations`.
  - `crates/micold-core/src/sandbox/lifecycle.rs`: `Started.mounted: Option<Vec<String>>`, set in `bring_up` per `adopt`'s decision.
  - `crates/micold-core/src/sandbox/mod.rs`: `MountSet::shared_locations(mounted) -> Vec<SharedLocation>`, plus the denied host paths.
- [ ] T064 [US3] [U90] [U91] [U92] Carry the locations to the client state.
  - `crates/micold-client/src/shell/sandbox.rs`: `Ready { started, locations: SandboxLocations }`, where `boot()` keeps both instead of `.map(|ready| ready.started)`.
  - `crates/micold-client/src/features/sandbox.rs`: define `SandboxLocations { shared: Vec<SharedLocation>, denied: Vec<String> }` (deriving `Clone, Debug, PartialEq, Eq`), `SandboxMsg::Started(Box<(Started, SandboxLocations)>)`, the `Sandbox.locations` field, `started(started, locations)`, and `locations()`, which returns it only while `Running` or `Stale`. `for_placement` and every other transition drop it.
  - Update every `SandboxMsg::Started` constructor and match, in `tests/features_sandbox.rs` and elsewhere.
- [ ] T065 [US3] [U80] [U81] [U82] [U83] [U84] [U137] [A19] The confirmation state in `crates/micold-client/src/features/session.rs`, built like `confirm_session_remove`.
  - `pending_link_open: Option<PendingLinkOpen { session: SessionId, link: ResolvedLink }>`.
  - Messages `LinkOpenConfirmed` and `LinkOpenDeclined`.
  - Reducers T7 and T10.
  - `pub fn link_open_confirmed(state, sandbox_live: bool)` for T9.
  - `ConfirmLinkOpenDialog` (`SurfaceId::new("confirm_link_open")`), implementing `FloatingSurface` and `Registered`; Escape and scrim-click decline.
- [ ] T066 [US3] [U134] [U137] The confirm view.
  - Create `crates/micold-client/src/ui/confirm_link_open.rs`: title **Open a file from the sandbox?**, body `The sandboxed session linked to <host path>. Files the sandbox wrote can contain scripts or macros.`, and actions **Open** (publishes `LinkOpenConfirmed`) and **Cancel** (glue).
  - Register it in `crates/micold-client/src/overlay/registry.rs`.
  - In `crates/micold-client/src/ui/terminal.rs`, build `LinkContext.sandbox = Some` iff `sandbox.locations()` is `Some`, from its `shared` and `denied` and `link::container_host_names(id)`.
- [ ] T067 [US3] [U98] [A19] Route the confirmation.
  - `crates/micold-client/src/main.rs` `update_inner`: a `LinkOpenConfirmed` arm ahead of `Message::Session`, calling `shell::links::on_link_message` (glue).
  - `crates/micold-client/src/shell/links.rs`: for `LinkOpenConfirmed`, compute `sandbox_live` from `app.sandbox.state` (`Running | Stale`) and call `session::link_open_confirmed`.
- [ ] T069 [US3] Replace the "not reachable" sentence in the "Links" section of `docs/user-guide/worktrees-and-sessions.md`.
  - A sandboxed session's file links open from the host location that holds the same file, after a confirmation naming that path.
  - Paths the sandbox does not share are reported as not reachable.
  - A confirmation pending when the sandbox stops opens nothing.
- [ ] T068 [US3] Verify the milestone: `mise run gate`, `cargo check --target aarch64-apple-darwin -p micold-client`, then the `visual-pass` skill for quickstart §B.15–B.16 under sandbox placement. US3 is complete only when the T085 acceptance test A19 is green. It confirms acceptance rows A19 and carries no markers, so it is not ticked by them.

**Checkpoint (M6)**: US3 is complete on host and sandbox placements.

---

## Phase 6: User Story 4 — Copy or open a link from the right-click menu (Priority: P3)

**Goal**: right-clicking a link offers **Open Link** and **Copy Link Address**, both acting on the link captured when the menu opened.

**Independent Test**: quickstart §B.11.

### Tests for US4 ⚠️ write first, must fail

- [ ] T086 [US4] [A20] [A21] [A22] Outer-loop acceptance tests in `mod acceptance` of `crates/micold-client/src/shell/links.rs`. A20 and A21 must fail before T073–T075; A22 is a guard, green on arrival, whose red is shown by the mutant in `tdd/test-list.md`:
  - A20: a right press over a link opens the terminal menu with **Open Link** and **Copy Link Address** ahead of today's items.
  - A21: choosing **Copy Link Address** on a declared link writes its declared address to the recording clipboard, and on a soft-wrapped detected link the complete unwrapped address.
  - A22: a right press over plain text opens the menu with no link items.
- [ ] T070 [P] [US4] [U85] [U86] [U87] [U88] [U89] [U143] Extend `crates/micold-client/tests/features_session_links.rs`.
  - T14: `TerminalContextMenuOpened { x, y, link: Some(r) }` sets `menu_link` and the menu lists **Open Link** and **Copy Link Address** first (contract §6 M1). With `link: None`, today's items only (contract M2).
  - T15: `ContextMenuOpenLink` acts as `LinkActivated(captured)` and closes the menu (contract M3).
  - T16: `ContextMenuCopyLinkAddress` gives `Outcome::ClipboardWrite(captured.link.address)`, the declared URI or the trimmed, unwrapped detected text (contract M4).
  - T17: closing the menu clears `menu_link`.
  - `link_menu_items(Option<&ResolvedLink>) -> Vec<LinkMenuItem>`: `[OpenLink, CopyLinkAddress]` for `Some`, empty for `None`.
- [ ] T071 [P] [US4] [U99] [U100] Extend the `#[cfg(test)]` module of `crates/micold-client/src/shell/links.rs`: `update_inner` with `ContextMenuCopyLinkAddress` returns the clipboard write task rather than dropping it, and with `ContextMenuOpenLink` on a URL it reaches the recording opener
- [ ] T072 [P] [US4] [U117] A test in the `#[cfg(test)]` module of `crates/micold-client/src/ui/material/terminal_pane.rs`: a right press over a link publishes `TerminalContextMenuOpened` with `link` equal to the link resolved at the press, and over plain text with `link: None` (FR-017)

### Implementation for US4

- [ ] T073 [US4] [U85] [U86] [U87] [U88] [U89] [U143] [A20] [A21] [A22] Menu state and reducers in `crates/micold-client/src/features/session.rs`: `TerminalContextMenuOpened` gains `link: Option<ResolvedLink>`, plus `menu_link`, the pure `link_menu_items`, `ContextMenuOpenLink` and `ContextMenuCopyLinkAddress`, and reducers T14–T17. Update every existing constructor or match of `TerminalContextMenuOpened` in `src/` and `tests/`.
- [ ] T074 [US4] [U117] In `crates/micold-client/src/ui/material/terminal_pane.rs`, resolve the link at the right press and include it in `TerminalContextMenuOpened`
- [ ] T075 [US4] [U99] [U100] [A20] [A22] In `crates/micold-client/src/ui/terminal.rs`, render one leading `MenuItem` per entry of `link_menu_items(menu_link)`, with no divider (glue). In `crates/micold-client/src/main.rs` `update_inner`, add `ContextMenuOpenLink` and `ContextMenuCopyLinkAddress` arms ahead of `Message::Session`, calling `shell::links::on_link_message` (glue).
- [ ] T076 [US4] Extend "Links" in `docs/user-guide/worktrees-and-sessions.md`: right-click a link for Open Link and Copy Link Address, which copies the complete address (the declared one for a declared link)
- [ ] T077 [US4] Verify the milestone: `mise run gate` (including `tests/gates/context_menu_anchor.rs` and `tests/context_menu_anchor_call_sites.rs` with the two new items), then the `visual-pass` skill for quickstart §B.11. US4 is complete only when the T086 acceptance tests A20–A22 are green. It confirms acceptance rows A20–A22 and carries no markers, so it is not ticked by them.

**Checkpoint (M7)**: all four stories work.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T078 [P] Write `specs/031-clickable-terminal-links/scripts/stream-links.sh`. It prints 10,000 lines, each holding an address, with one in ten 1,000 characters long; `--plain` replaces the addresses with words.
- [ ] T079 Run the SC-005 measurement from quickstart §A.3 on a release build (`mise run build`), and record both frame-probe p95 figures under "The pass" in `specs/031-clickable-terminal-links/quickstart.md`. A result more than 10% apart is a finding against research R2.
- [ ] T080 Run quickstart §B.18 through the `visual-pass` skill. Read the complete "Links" subsection against FR-023's list, fix any gap in `docs/user-guide/worktrees-and-sessions.md`, and complete `specs/031-clickable-terminal-links/visual-pass.md` with every §B row.
- [ ] T081 Confirm CI is green on Linux, macOS and Windows for the final tree (Principle VI), including `osc8_passthrough` on Windows or its recorded ignore

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (Phase 1)** comes first, then **Foundational (Phase 2)**. Both block every story.
- **US1 (Phase 3)** runs in three blocks, in this order:
  1. core recognition (T004–T012);
  2. opening (T013–T022), which needs only the `ResolvedLink` type from T003;
  3. the pane (T023–T034), which needs both.
- **US2 (Phase 4)** depends on US1: the pane and opener must exist for declared links to be seen and opened. T027 (in the pane block, so ConPTY's behaviour is known early, plan Risk 2) and T035, T036, T038 and T039 are independent of US1's code.
- **US3 (Phase 5)** depends on US1. The host-file tasks (T042–T054) block the sandbox tasks (T055–T069).
- **US4 (Phase 6)** depends on US1 (`LinkActivated`, `on_link_message`). It does not depend on US3: "Open Link" on a `file` link simply follows whatever US3 has shipped.
- **Polish (Phase 7)** depends on everything.

### Within each story

The tests block marked ⚠️ comes before its implementation block. Core comes before the client, reducers before the shell, and the shell before `main.rs` glue. Docs and verification close each milestone.

### Parallel opportunities

- T004–T007 together (different files in `link/`). T008 is written alongside, but its fixture needs T004's rules settled.
- T013, T014 and T016 together (feature test, `shell/links.rs`, gate); T015 after T014, since both declare a module in `shell/mod.rs`.
- T026 and T027 together, alongside the `terminal_pane.rs` tests. T023, T024, T025 and T037 share that file's test module, so they run in sequence.
- T035 and T036 together: core and daemon test files.
- T087 then T042 (both edit `link/resolve.rs`), alongside T043–T045 together; and T055–T061 together.
- T070–T072 together.

## Parallel Example: User Story 1 (core)

```bash
Task: "detect unit tests in crates/micold-core/src/link/detect.rs (T004)"
Task: "link_at unit tests in crates/micold-core/src/link/line.rs (T005)"
Task: "classify/resolve unit tests in crates/micold-core/src/link/address.rs + resolve.rs (T006)"
Task: "no-I/O test in crates/micold-core/src/link/mod.rs (T007)"
```

---

## Implementation Strategy

### MVP first

Setup and Foundational, then US1 in three milestones:
- M1: recognition;
- M2: opening;
- M3: the pane.

Stop and validate with quickstart §B.1–B.7. This alone removes the select, copy, switch and paste routine for web addresses.

### Incremental delivery

Each later milestone adds a story, or half of one, without breaking the earlier ones:

1. M4 (US2): declared links, and micold's own identity.
2. M5 (US3): host files.
3. M6 (US3): the sandbox.
4. M7 (US4): the menu.
5. M8: the close.

---

## Notes

- `[P]` = different files and no unfinished dependency. `[USn]` = traceability to spec.md.
- Every test must be seen to fail before its implementation (the `tdd.run` hook records the red).
- Gates that enumerate messages, outcomes or overlay surfaces are updated in the task that adds the variant, never in a later task.

---

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

US1 is 34 tasks, so it is split, by layer rather than by scenario: a deliberate departure from rule 3. The smallest scenario half (a single-row web link, scenarios 1, 2, 4 and 5) still needs nearly all of recognition, opening and the pane, so a scenario split would not shrink the first PR; soft wrap (3) and scrollback (6) are a few lines inside `link_at` and the `LinkRows` adapter. Recognition and opening each ship core behaviour with tests, and they stay unreachable from the UI until M3 (rule 6).

The outer-loop acceptance tasks T082–T086 and the helper tasks T087–T088 were added after the task list was cut, so they carry later ids. Each sits where it runs in the file and is listed in its milestone. T083 and T037 are US2 tasks placed in M3, because the pane's hover resolves declared runs from the start and their tests must precede it.

### M1 — Link recognition core 🎯 MVP

- **Tasks**: T001–T012
- **Deliverable**: `micold_core::link` finds exactly the addresses a user reads as links, detected or declared and across soft wraps, in a 50-line corpus of real CLI output. `cargo test -p micold-core link` shows it. Nothing is reachable from the UI yet; M3 wires it.
- **Satisfies**: FR-001, FR-002, FR-003, FR-005, FR-011 (web and mail schemes only), FR-019; SC-002, SC-006 (web and mail)
- **Verify**: `mise run test-core` (`link::*`, `tests/link_corpus.rs`); quickstart §A.2 no-I/O row
- **Depends on**: —

### M2 — Opening pipeline

- **Tasks**: T013–T022
- **Deliverable**: a `SessionMsg::LinkActivated` for a web or mail address, sent through `update_inner`, reaches the operating system's opener with exactly that address, and a failure raises a "Couldn't open …" notification. There is a `SystemLinkOpener` for Linux, macOS and Windows. Nothing in the UI emits the message yet; M3 wires the pane. No user-guide change, because nothing is visible, so the PR carries the `docs-not-needed` label, and its body says why: T018 touches `features/**`, which `.gitattributes` marks user-facing, so `scripts/check-user-guide-updated.sh` needs the label.
- **Satisfies**: FR-010, FR-015 (no application, launch failure); SC-003; contract link-opening §2, §3 O1, §5 (no application and launch failure rows)
- **Verify**: `tests/features_session_links.rs` (T5, T12, T13); the `shell::links` route test; the `shell::link_opener` launch-window tests; `tests/no_concrete_implementations.rs`; `cargo check --target aarch64-apple-darwin -p micold-client`
- **Depends on**: M1

### M3 — Clickable web, mail and declared links in the pane

- **Tasks**: T082, T083, T037, T023–T034
- **Deliverable**: in any session pane, hovering a web or mail address underlines it and shows the address, and Ctrl+click (Cmd+click on macOS) opens it in the default browser or mail client. A program's declared hyperlink is marked as its whole run and shows, and opens, its declared address. Selection, double-click and mouse-reporting programs behave as before. The user guide has a "Links" section.
- **Satisfies**: US1 acceptance scenarios 1–6; US2 acceptance scenarios 1–5; US3 acceptance scenario 1 (mailto); FR-002 (pane), FR-004, FR-007, FR-008, FR-009, FR-014, FR-016, FR-017 (gesture part), FR-022, FR-023 (US1 part); SC-001, SC-004; `osc8_passthrough` for US2's premise (research R13)
- **Verify**: quickstart §B.1–B.7 and §B.12 (visual-pass); acceptance tests A1–A11, A13; `mise run gate`; `crates/micold-daemon/tests/osc8_passthrough.rs` on all three CI OSes
- **Depends on**: M2

### M4 — Session terminal identity and the hyperlink opt-in

- **Tasks**: T035, T036, T038–T041
- **Deliverable**: sessions no longer inherit another terminal's identity (`TERM_PROGRAM`, `FORCE_HYPERLINK` and the rest are absent from a new session's environment), and `FORCE_HYPERLINK=1` in the include script is the documented opt-in that makes AI CLIs declare links.
- **Satisfies**: FR-006, FR-023 (US2 opt-in part)
- **Verify**: quickstart §B.7, §B.14, §B.17; `tests/session_identity_env.rs`; `env_include` predicate tests
- **Depends on**: M3

### M5 — File links on the host

- **Tasks**: T084, T087, T042–T048, T088, T049–T054
- **Deliverable**: a `file://` link to this machine opens documents in their application and folders in the file manager. Runnable files are revealed and never run, a missing file notifies, and a file link naming another machine is not a link. A sandboxed session's file links show as not reachable, which is the safe direction; M6 translates them.
- **Satisfies**: US3 acceptance scenarios 2–6; US2 acceptance scenario 6; FR-010 (`file`), FR-011 (`file`), FR-012, FR-013, FR-015 (not found), FR-023 (US3 host part); SC-006 (file), SC-007
- **Verify**: quickstart §B.8–B.10, §B.13; `link::runnable` tests; `shell::links` SC-007 real-file test on all three CI OSes
- **Depends on**: M3

### M6 — Sandboxed file links, translated and confirmed

- **Tasks**: T085, T055–T067, T069, T068
- **Deliverable**: in a sandboxed session, a file link under a location the sandbox shares opens the same file on the host, after a confirmation naming the host path. Unshared paths and the sandbox token are reported as not reachable. A confirmation pending when the sandbox stops opens nothing.
- **Satisfies**: US3 acceptance scenario 7; FR-018, FR-018a, FR-023 (sandbox part); contract link-recognition C11–C17, link-opening §4
- **Verify**: quickstart §B.15–B.16 under sandbox placement; `sandbox::pathmap` reverse tests; `features::sandbox` T18/T19; the overlay list tests
- **Depends on**: M5
- **Kept whole** (16 tasks): translation without the confirmation would open sandbox-written files unconfirmed (FR-018a), and the confirmation without translation has nothing to confirm.

### M7 — Link context menu

- **Tasks**: T086, T070–T077
- **Deliverable**: right-clicking a link offers **Open Link** and **Copy Link Address**, which act on the link under the pointer when the menu opened.
- **Satisfies**: US4 acceptance scenarios 1–3; FR-017 (menu part), FR-020, FR-021, FR-023 (US4 part)
- **Verify**: quickstart §B.11; `tests/features_session_links.rs` T14–T17; `tests/gates/context_menu_anchor.rs`
- **Depends on**: M3

### M8 — Close: performance and full walkthrough

- **Tasks**: T078–T081
- **Deliverable**: quickstart §A.3 records the SC-005 streaming frame-time comparison, and `visual-pass.md` records every quickstart §B row, including the B.18 user-guide review.
- **Satisfies**: SC-005; FR-023 (complete); Principle VI
- **Verify**: quickstart §A.3 figures; `visual-pass.md`; green CI on all three OSes
- **Depends on**: M4, M6, M7

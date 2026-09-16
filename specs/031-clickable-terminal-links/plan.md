# Implementation Plan: Clickable Links in the Terminal

**Branch**: `feat/links-in-terminal-should-be-clickable` | **Date**: 2026-09-14 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/031-clickable-terminal-links/spec.md`

## Summary

Addresses in a terminal pane become links. The pane recognises two kinds of link under the pointer.
A **detected** link is plain text with an explicit `http://`, `https://`, `mailto:` or `file://`
prefix, including text the terminal soft-wrapped. A **declared** link is text a program marked with
OSC 8. While the pointer is over either kind, the link is underlined, the pointer becomes a hand while Ctrl (Cmd) is held, and
the pane shows the address it will open. Ctrl+click (Cmd+click on macOS) hands the address to the
operating system:

- A web or mail address opens in the default browser or mail client.
- A document or folder opens in its application.
- A runnable file is revealed in the file manager, never run.
- A `file` link from a sandboxed session is translated to its host path and opens only after the
  user confirms.

The right-click menu gains "Open Link" and "Copy Link Address" over a link. Sessions stop inheriting
another terminal's identity variables, so programs decide on hyperlinks from micold's own identity.

Most of the plumbing already exists. OSC 8 URIs already reach the client in each cell's extras, and
each row already carries its soft-wrap flag (research R1), so **the wire does not change**. The new
work is:

- a render-free `micold_core::link` module that recognises, classifies and resolves;
- a hover and gesture layer in the pane, as pure functions;
- one new platform capability, `LinkOpener`;
- a confirm surface reusing `FloatingSurface`;
- a small environment change at session spawn in the daemon.

## Technical Context

**Language/Version**: Rust (stable, pinned by `rust-toolchain.toml`)

**Primary Dependencies**: `iced` (client), `alacritty_terminal` (the client grid), `tokio`, `serde`.
Two direct dependencies are added to `micold-client`, and both are already in `Cargo.lock`, so no
new crate enters the build:

- `gethostname` 1.1 reads this machine's name for FR-012 (research R8). On Linux it is already
  compiled, through `x11rb`. macOS and Windows builds compile it for the first time.
- `windows-sys` becomes a `cfg(windows)` workspace dependency of the client, for `ShellExecuteW`
  (research R11). The workspace entry gains the `Win32_UI_Shell` and `Win32_UI_WindowsAndMessaging`
  features. Cargo unifies features, so core and daemon compile those modules too on Windows. It
  adds binding code only, and no new crate.

Rejected: `url`, `linkify`, `regex`, `open`, `opener`, `webbrowser` (research R3, R8, R11).

**Storage**: None. Nothing about links is persisted (spec Assumptions). The running sandbox's shared
locations are kept in memory while the container runs (research R10).

**Testing**: `mise run test-core` while iterating on `link`, `pathmap` and `env_include`.
`mise run gate` before each push. `cargo check --target aarch64-apple-darwin -p micold-client`
whenever `shell/link_opener.rs` or the fact gatherer changes. The layers are listed in research R15
and the obligations in [quickstart.md §A.2](./quickstart.md).

**Target Platform**: Linux, macOS, Windows desktop (Constitution VI). Every platform branch in
core is a parameter. The platform is read in the client and daemon shells and passed down:

- the opener arms in `shell/link_opener.rs`;
- the execute-bit read in `shell/links.rs`;
- `cfg!(windows)` for `LinkContext.windows_host`, `HostPlatform` and the environment key flag;
- the existing `env_include.rs` Unix/Windows builders, which gain the removal.

**Project Type**: Desktop application, a three-crate workspace (`micold-core`, `micold-client`,
`micold-daemon`).

**Performance Goals**:

- SC-003: the open request reaches the OS within 1 s. It is one `spawn_blocking` hop with no timer,
  so the bound is met by construction and checked in the shell tests.
- SC-005: marking appears on the next frame, and streaming output renders within 10% of the
  no-address baseline. This is met by resolving only the pointer's logical line, only when its cell
  or line ids change (research R2), and by never scanning output that nobody is pointing at. It is
  measured via `frame_probe`, not gated (quickstart §A.3).

**Constraints**:

- FR-014: activation writes nothing to the PTY.
- FR-016: mouse reporting keeps priority unless Shift is held, which the existing `press_routing`
  already decides.
- FR-019: no network access of any kind, including no DNS lookup to decide whether a host is "this
  machine".
- Principle VIII: the hint and underline are drawn by the existing `TerminalPane`, the confirm
  surface reuses `FloatingSurface`, and the menu items reuse the existing menu. No new component.

**Scale/Scope**: A new core module of about six files. One new client capability, one new shell
module, and about eight new messages. One daemon spawn change, four new test binaries, and one user
guide section.

## Constitution Check

*GATE: passed before Phase 0 research; re-checked after Phase 1 design (see the re-check below).*

- [x] **I. Test-First (NON-NEGOTIABLE)**: every decision is a pure function preceded by a failing
  test.
  - Recognition, classification, resolution, runnability, reverse path mapping and the identity
    predicate live in `micold-core` and are unit-tested there, with the SC-002 corpus as an
    integration test.
  - The pane's decisions are pure functions tested in `terminal_pane.rs`'s own `#[cfg(test)]`
    module, the way `press_routing` and `grid_at` already are: `link_gesture`, `link_hint_rect`,
    hover invalidation and pointer interaction.
  - The session reducers are tested in `tests/features_session_links.rs`.
  - `shell/links.rs` is **not** claimed as glue; it is tested against a recording `LinkOpener`.
  - The daemon's spawn change is tested against a real PTY.
  - Only the `main.rs` routing arms, the `ui/terminal.rs` menu wiring and the showcase sample fall
    under the GUI-glue exception, and each invokes tested logic without branching of its own.
- [x] **II. Multi-Session Support**: hover and press state are widget-local `PaneState`, so they are
  scoped to one pane by construction (FR-022). A pending confirmation records its `SessionId` and
  is re-checked on confirm, so closing that session, or any other, cannot open the wrong thing.
- [x] **III. Worktree Integration**: not affected. No git operation is added.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: nothing is stored and nothing leaves the device
  (FR-019). Host matching compares against `gethostname` and the container id; it never resolves a
  name. Opening a web link hands it to the user's browser, which is the requested behaviour.
- [x] **V. Rust + iced Stack**: Rust and iced only. The platform openers are the operating systems'
  own launchers (`xdg-open`/`dbus-send`, `open`, `ShellExecuteW`/`explorer.exe`), reached through
  `std::process::Command` with single argv elements and no shell, or through `windows-sys`.
- [x] **VI. Cross-Platform Parity**: every platform difference in core is a value:
  - `windows_host: bool` for path forms;
  - `HostPlatform` for runnable kinds;
  - a case-insensitivity flag for environment keys.

  So all three platforms' rules are tested on every CI runner. The platform is read only in the shells
  (see Target Platform): one opener arm per OS, the Unix execute-bit read, `cfg!(windows)` values
  passed into core, and the existing include-shell builders. The capability sits behind
  `Capabilities`, guarded by `no_concrete_implementations.rs`. OSC 8 through ConPTY is verified on
  the Windows runner rather than assumed (research R13).
- [x] **VII. Documentation First-Class**: `docs/user-guide/worktrees-and-sessions.md` gains a
  "Links" subsection under "Interacting with the terminal" (FR-023). It covers:
  - the gesture, and the Shift rule under mouse reporting;
  - the hint, the menu items, what opens where, and runnable files being revealed;
  - the sandbox confirmation;
  - the `FORCE_HYPERLINK=1` include-script opt-in;
  - the Windows declared-link caveat, if R13 finds one.

  It lands with the first milestone that makes links visible, and grows with each later one.
- [x] **VIII. Reusable UI Component Foundation**: **no new component.**
  - The underline and hint are drawn by `TerminalPane` itself, with colours from the existing theme
    tokens, the same way it already draws selection and the cursor.
  - The confirm is a `FloatingSurface` built like `confirm_session_remove`, with its registry entry
    and the overlay list gates.
  - The menu items are two more `MenuItem`s in the existing context menu, with no divider
    (research R14).
  - The showcase's terminal sample gains a detected and a declared link, so the marking can be seen
    in the gallery, and a new test module in `showcase/samples.rs` checks for them.

**Post-design re-check**: all eight still pass. The design added:

- no component and no protocol change;
- no persisted state;
- no new crate in the lock.

Two points deserve naming:

- The client gains its first `cfg(windows)` FFI call (`ShellExecuteW`). It is confined to
  `shell/link_opener.rs` and never built on this machine, so its failure mode is a red Windows CI
  job, not a silent gap (Risk 1).
- Stripping inherited variables is a behaviour change for **every** session, not only ones showing
  links. Contract [session-terminal-identity.md](./contracts/session-terminal-identity.md) bounds
  it: `TERM` is untouched, `COLORTERM=truecolor` is kept, and include-script values still win.

## Project Structure

### Documentation (this feature)

```text
specs/031-clickable-terminal-links/
├── plan.md              # This file
├── spec.md              # The specification (clarified)
├── research.md          # Phase 0: R1..R15
├── data-model.md        # Phase 1: core values, client state, transitions T1..T19
├── quickstart.md        # Phase 1: Part A automated / Part B by eye
├── contracts/
│   ├── link-recognition.md          # core surface: link_at, detect, classify, resolve
│   ├── link-opening.md              # gesture, LinkOpener, confirm, notifications, menu
│   └── session-terminal-identity.md # the spawn environment rule
├── checklists/
│   └── requirements.md  # Spec quality checklist
├── autopilot.md         # The autopilot ledger
└── tasks.md             # Phase 2, not created by /speckit-plan
```

### Source Code (repository root)

```text
crates/
├── micold-core/
│   ├── src/
│   │   ├── lib.rs                   # + pub mod link
│   │   ├── link/                    # NEW, render-free
│   │   │   ├── mod.rs               #   Link, LinkOrigin, CellSpan, LinkRow, re-exports
│   │   │   ├── detect.rs            #   detect(text): scheme scan, trimming, brackets, well-formedness
│   │   │   ├── line.rs              #   link_at: logical line (≤64 rows each way), declared runs, cell mapping
│   │   │   ├── address.rs           #   classify, percent-decode
│   │   │   ├── resolve.rs           #   LinkContext, ResolvedLink, Target, resolve
│   │   │   └── runnable.rs          #   HostPlatform, FileFacts, FileAction, action_for
│   │   ├── sandbox/pathmap.rs       # + reverse(locations, denied, path, windows_host)
│   │   ├── sandbox/mod.rs           # + MountSet::shared_locations(mounted)
│   │   ├── sandbox/parse.rs         # + ContainerFacts.mount_destinations (inspect Mounts[].Destination)
│   │   ├── sandbox/lifecycle.rs     # + Started.mounted: None on Create/Replace, Some on Attach/Start
│   │   └── env_include.rs           # + is_inherited_terminal_identity; baseline/attempt shells strip it
│   └── tests/
│       ├── link_corpus.rs           # NEW: SC-002
│       └── fixtures/link_corpus.txt # NEW: ≥50 real lines, expected spans inline
├── micold-daemon/
│   ├── src/supervisor.rs            # spawn_ai_cli / spawn_shell: env_remove before TERM + include values
│   └── tests/
│       ├── session_identity_env.rs  # NEW: FR-006 against a real spawn
│       └── osc8_passthrough.rs      # NEW: R13, all three CI OSes
└── micold-client/
    ├── Cargo.toml                   # + gethostname; + windows-sys (cfg(windows))
    ├── src/
    │   ├── ui/material/terminal_pane.rs  # + hover cache, link_press, link_gesture, link_hint_rect,
    │   │                                 #   underline + hint drawing, Pointer interaction,
    │   │                                 #   publishes SessionMsg::LinkActivated, link in context-menu press
    │   ├── features/mod.rs          # + Outcome::OpenLink(OpenRequest)
    │   ├── app.rs                   # interpret: + empty OpenLink arm; the link messages never reach it (data-model §3)
    │   ├── features/session.rs      # + menu_link, pending_link_open, LinkActivated /
    │   │                            #   LinkOpenConfirmed / LinkOpenDeclined / LinkOpenFinished /
    │   │                            #   ContextMenuOpenLink / ContextMenuCopyLinkAddress,
    │   │                            #   ConfirmLinkOpenDialog (FloatingSurface + Registered)
    │   ├── overlay/registry.rs      # + ConfirmLinkOpenDialog => ui::confirm_link_open::dialog
    │   ├── ui/confirm_link_open.rs  # NEW: the confirm dialog view (glue)
    │   ├── shell/link_opener.rs     # NEW: LinkOpener trait, OpenFailure, SystemLinkOpener (cfg arms)
    │   ├── shell/links.rs           # NEW: LinkContext assembly, on_link_message (runs the reducer,
    │   │                            #   splits OpenLink / ClipboardWrite / the rest), sandbox_live,
    │   │                            #   OpenLink execution, file facts; #[cfg(test)] module incl. the route
    │   ├── shell/clipboard.rs       # interpret: + Outcome::OpenLink(_) in the no-op arm; now also fed by on_link_message
    │   ├── shell/capabilities.rs    # + link_opener: Arc<dyn LinkOpener>
    │   ├── shell/sandbox.rs         # Ready { started, locations }; boot() keeps both (was .map(|r| r.started))
    │   ├── features/sandbox.rs      # SandboxMsg::Started(Box<(Started, SandboxLocations)>); Sandbox.locations,
    │   │                            #   locations() only while Running|Stale (tests: T18–T19)
    │   ├── app.rs (State)           # + host_names: Vec<String> (Default: empty)
    │   ├── shell/mod.rs             # + mod links, link_opener
    │   ├── main.rs                  # host_names read at boot into State; update_inner arms for
    │   │                            #   LinkActivated / LinkOpenConfirmed / ContextMenuOpenLink /
    │   │                            #   ContextMenuCopyLinkAddress ahead of Message::Session (glue)
    │   ├── ui/terminal.rs           # pane(..) builds the LinkContext from state + sandbox (ui::view's
    │   │                            #   signature is unchanged); menu items (glue)
    │   ├── ui/mod.rs                # the pane(..) call site
    │   └── showcase/samples.rs      # terminal sample with a detected and a declared link
    └── tests/
        ├── features_session_links.rs      # NEW: reducers T5–T10, T14–T17
        ├── overlay_registry.rs, overlay_dispatch_ordering.rs,
        │   overlay_dismissal_delta.rs, overlay_transition_identity.rs  # + confirm_link_open rows
        ├── popover_displacement.rs        # DIALOGS 9 → 10
        ├── no_concrete_implementations.rs # + SystemLinkOpener
        └── (showcase/samples.rs #[cfg(test)]) # + the sample holds a detected and a declared link

specs/031-clickable-terminal-links/scripts/
├── links-fixture.sh                 # NEW: quickstart §B.0 output
└── stream-links.sh                  # NEW: quickstart §A.3 SC-005 stream

docs/user-guide/
└── worktrees-and-sessions.md        # + "Links" under "Interacting with the terminal"
```

**Structure Decision**: the existing three-crate workspace, unchanged. Everything that decides lives
in `micold-core::link`, so it is render-free and runs in `test-core`; the client never scans text
itself.

The client follows the feature-and-shell split established by feature 028:

- `features/session.rs` owns the messages, the pending state and the confirm surface.
- `shell/links.rs` owns the conversation with the filesystem and the OS, and reads the binary's
  `App` (sandbox liveness) on the reducer's behalf.
- `shell/link_opener.rs` owns the platform arms.
- The pane owns hover and gesture as widget-local state.

The daemon changes only the environment it spawns sessions with. Wire types are untouched.

## Implementation Phases

The order is chosen so the riskiest external unknowns are met early:

- Windows ConPTY and OSC 8 (R13) runs on CI from the first push.
- The platform openers are exercised on real OSes before the file and sandbox work depends on them.

Each phase ends with something testable.

**Phase A — recognition in core.** `link::detect`, `link::line` (`link_at`), `link::address`
(`classify`) and `link::resolve` for `Url` targets. Includes the SC-002 corpus fixture and test.
Pure, no client change. Contract link-recognition §2–§3 and C1–C3 are green.

**Phase B — the pane (US1 core).** Hover cache, underline, hint placement and drawing, the pointer
interaction, `link_gesture`, and publishing `SessionMsg::LinkActivated`. Add
`Outcome::OpenLink`, its route through `main.rs` to `shell/links.rs` (data-model §3), the `LinkOpener` capability with all three OS arms, and `shell/links.rs` for
`Url` targets. Add `osc8_passthrough.rs` here, because a pane that renders declared links is
pointless if Windows drops them, and CI answers that question on this PR. **Then run the macOS
cross-check and the visual pass for B.1–B.7 immediately.** Legibility in both themes (FR-009) is the
kind of finding that reshapes drawing code, so it must not wait for Phase E.

**Phase C — terminal identity (US2).** `is_inherited_terminal_identity`, stripping in both
include-script shells and in `supervisor.rs`, and `session_identity_env.rs`. Independent of B's code,
and it could land first. It is sequenced after B only so that declared links are visible when it
lands.

**Phase D — non-web links (US3).**

- `classify`/`resolve` for `file` and `mailto`, with C4–C10.
- `runnable::action_for` and the fact gatherer.
- `pathmap::reverse`, `MountSet::shared_locations(mounted)` and `ContainerFacts.mount_destinations`.
- `Started.mounted`, `Ready.locations` and `Sandbox::locations`, with the sandbox context (C11–C17).
- The confirm surface and its T7–T10 transitions.
- The notification texts.

This is the largest phase and the one most likely to be split into two milestones: host files, then
sandbox translation and confirmation.

**Phase E — the right-click menu (US4).** `TerminalContextMenuOpened { link }`, `menu_link`, the two
items, and T14–T17. The context-menu anchor gates are re-run.

**Phase F — close.** Complete the user guide section (FR-023) and run the SC-005 measurement
(quickstart §A.3). Finish the remaining quickstart §B steps (B.8–B.17) through `visual-pass`.

## Risks

| # | Risk | Mitigation |
|---|------|-----------|
| 1 | Windows code (`ShellExecuteW`, `explorer.exe /select,`, `%PATHEXT%`, case-insensitive env keys) is never compiled on this machine | Every Windows rule that is a *decision* is a core parameter tested on Linux. The FFI arm is small and lands in Phase B, so the Windows CI job reports it on the first milestone PR, not the last |
| 2 | Inbox ConPTY may drop OSC 8 (R13) | `osc8_passthrough.rs` answers it on CI in Phase B. If it drops, `cfg_attr(windows, ignore)` citing the run, a user-guide caveat, and the spec's own degradation edge case: detected links still work |
| 3 | Linux has no universal "reveal" | `dbus-send` to `org.freedesktop.FileManager1.ShowItems`, falling back to opening the parent folder. Either satisfies FR-013, since the file is shown and never run. The quickstart B.10 row accepts both |
| 4 | The hint overlaps output the user is reading, or an elided hint weakens SC-006 | The hint flips to the top when the pointer is near the bottom (`link_hint_rect`, tested). Elision is visual only: `ResolvedLink.display` stays full and is what the SC-006 property compares. The full address is always one "Copy Link Address" away |
| 5 | Streaming output under a resting pointer re-resolves every frame (SC-005) | A new frame costs a hash over the at most 129 rows under the pointer. `link_at` re-runs only when that hash, the cell, the session or the context changes (R2), so an in-place redraw is caught and an unrelated streamed line is not. Measured in §A.3; a regression is a finding against R2 |
| 6 | A container hostname is not its id prefix under every runtime or configuration | C11 accepts the empty host, `localhost`, the id prefix and the full id. A tool that prints an unrecognised container hostname yields a non-followable link, the safe direction. That is recorded as a follow-up rather than guessed at |
| 7 | A client attaches to a container that was created with other projects, and its own `MountSet` would describe mounts that do not exist | `bring_up` records the attached container's inspected mount destinations (`Started.mounted`), and only those locations are shared (R10, C16c). A project the container lacks resolves as `Unreachable` with the "not reachable" hint, the safe direction. If a runtime's inspect omits `Mounts`, every location is dropped, which is also safe |
| 8 | Stripping identity variables changes a program's colour or feature detection in every session | The list is closed and names only emulator identity. `COLORTERM=truecolor` and `TERM` are kept. `session_identity_env.rs` pins the table in contract §2 |
| 9 | Phase D is too large for one reviewable PR | Milestone cutting splits it at the host/sandbox boundary (tasks.md `## Milestones`). Neither half leaves half-wired UI. In the first half, `resolve` returns `Target::Unreachable` for every `file` link from a sandboxed session, with a test, so a container path is never opened as a host path. The link shows as not reachable until the second half lands |
| 10 | `xdg-open` in generic mode (Xvfb, tiling window managers) runs the handler in the foreground, so a plain wait could hang the blocking task, and its exit status would be the handler's | Null stdio and a 2 s launch window (contract link-opening §2): exit codes are mapped only inside the window, and a still-running child counts as launched. The shell tests drive it with a stub that sleeps. Quickstart §B runs on Xvfb, which exercises exactly this mode |

## Complexity Tracking

No Constitution Check violations.

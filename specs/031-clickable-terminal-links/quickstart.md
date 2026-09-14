# Quickstart: Clickable Links in the Terminal

**Feature**: 031-clickable-terminal-links | **Plan**: [plan.md](./plan.md)

Validation splits the way Principle I splits the code. **Part A** covers every decision:
recognition, resolution, runnability, the gesture state machine, the hint's placement, the
reducers, the shell's use of the opener, and the session environment. Part A is automated. **Part
B** is what an assertion cannot see: whether the underline and hint read well in both themes, and
whether a real browser, mail client, document viewer and file manager actually open. Part B runs
through the repo's `visual-pass` skill.

Run Part A first. Part B means nothing until it passes.

**Prerequisites**: `mise trust` once per fresh worktree. All commands run from the repository root.

---

## Part A — automated

### A.1 The whole gate

```bash
mise run gate          # fmt --check, clippy (core, workspace), test --workspace, scripts/tests
cargo check --target aarch64-apple-darwin -p micold-client   # the macOS cfg arms (link_opener.rs)
```

The Windows arms are built and tested only by CI's Windows job. A PR is not green until that job
is.

### A.2 The gates that must contain new coverage

Each row is an obligation: the row's test must fail against the tree without this feature's change.

| Gate | Covers | Requirement |
|---|---|---|
| `micold-core` `link::detect` unit tests | contract link-recognition §3, every row | FR-001, FR-005 |
| `micold-core/tests/link_corpus.rs` + `tests/fixtures/link_corpus.txt` **(new)** | ≥ 50 real lines: 100% exact spans; no scheme-less match; a hard-broken address on its first row only; each hint text equals the piece that opens | SC-002 |
| `micold-core` `link::line` unit tests | contract link-recognition §2 L1–L7, over a fake `LinkRows` | FR-002, FR-003, FR-007 |
| `micold-core` `link::resolve` unit tests | contract link-recognition §4 C1–C18, and the SC-006 invariant as a property over all of them | FR-008, FR-011, FR-012, FR-018, SC-006 |
| `micold-core` `link` has no I/O | C18 decides an unresolvable host with no lookup; `link/` imports nothing from `std::net`, `std::fs` or `std::process` (a grep in the module's test, like `no_concrete_implementations`) | FR-019 |
| `micold-core` `link::runnable` unit tests | contract link-opening §3 table for all three platforms, run on every OS | FR-013, SC-007 (classification) |
| `micold-core` `sandbox::pathmap::reverse` unit tests | nesting, `..` escape, Windows pairs, secret excluded, the token path denied through the state mount (C16b) | FR-018 |
| `micold-core` `sandbox::parse` + `lifecycle` unit tests | `Mounts[].Destination` parsed from Docker and Podman inspect fixtures; `Started.mounted` is `None` on `Create`/`Replace` and the inspected list on `Attach`/`Start`; `shared_locations(mounted)` drops an unmounted project (C16c) | FR-018 |
| `micold-core` `env_include` unit tests | contract session-terminal-identity §1; baseline/attempt shells run without the variables | FR-006 |
| `micold-daemon/tests/session_identity_env.rs` **(new)** | a real spawned session: inherited `TERM_PROGRAM`/`FORCE_HYPERLINK` absent; include-script `FORCE_HYPERLINK=1` present; `COLORTERM=truecolor` kept | FR-006 |
| `micold-daemon/tests/osc8_passthrough.rs` **(new)** | an OSC 8 link printed through the real PTY arrives as a cell hyperlink, on all three CI OSes | spec Assumptions, research R13 |
| `terminal_pane.rs` test module: `link_gesture` | contract link-opening §1 G1–G9; G1 is the single activation step that SC-001 counts | FR-004, FR-014, FR-016, FR-017, SC-001 |
| `terminal_pane.rs` test module: SC-004 script | 100 scripted plain drag/double/triple selections starting on links → 0 `LinkActivated`; and a modifier double-click → exactly 1 (G3b) | SC-004 |
| `terminal_pane.rs` test module: `link_hint_rect` | bottom-left by default, top-left when the pointer is within the hint's height of the bottom, always inside the content bounds | FR-008 |
| `terminal_pane.rs` test module: hover | hover recomputed on `RedrawRequested` when the text under a still pointer changes, including an in-place redraw that keeps the same `LineId`; when the session shown or the `LinkContext` changes; and on `ModifiersChanged` (pressing or releasing Shift under mouse reporting); pointer interaction is `Pointer` over a link, and under mouse reporting only with Shift. Two `PaneState`s: hovering and pressing in one leaves the other's `hover` and `link_press` `None`, and only the pressed pane emits `LinkActivated` | FR-007, FR-008, FR-016, FR-022, SC-006, edge case "output moving" |
| `micold-client/tests/features_session_links.rs` **(new)** | data-model T5–T10, T14–T17: outcomes, confirm/decline, session gone, sandbox stopped, menu items present/absent, copied text | FR-015, FR-018a, FR-020, FR-021 |
| `micold-client` `shell/links.rs` test module | the route: `update_inner` with `LinkActivated` of a URL reaches the recording opener, and `ContextMenuCopyLinkAddress` yields the clipboard task; `sandbox_live` is read from `App.sandbox` (live and stopped); T11–T13 with a recording `LinkOpener`: URL passed verbatim; document → `open`; runnable → `reveal`; missing file → notification; `NoApplication` → notification text; the opener is called with no timer or debounce between `Outcome::OpenLink` (or the confirmation) and the call | FR-010, FR-013, FR-015, SC-003 |
| `micold-client` `shell/links.rs` test module: real files | on each CI OS, temp files of every runnable kind for that OS (Unix: execute bit, bundle directory on macOS; Windows: `%PATHEXT%` extensions), each also in a directory whose name contains a space, gathered through the real fact gatherer → 100% `reveal`; `.txt`, `.png`, `.pdf`, `.html` → 100% `open`, with the recording opener | SC-007 |
| `micold-client` `features/sandbox.rs` tests | T18 `started` stores the locations, for a created and for an attached container; T19 every transition out of `Running`/`Stale`, and `for_placement`, makes `locations()` `None` | FR-018, edge case "Pending opens" |
| `overlay_registry.rs`, `overlay_dispatch_ordering.rs`, `overlay_dismissal_delta.rs`, `overlay_transition_identity.rs`, `popover_displacement.rs` (`DIALOGS` 9 → 10) | each lists `ConfirmLinkOpenDialog`, so the confirm is registered, dismissable and ordered like the other confirms | FR-018a, Principle VIII |
| `micold-client/tests/no_concrete_implementations.rs` | `SystemLinkOpener` named only in `Capabilities::real()` | Principle VI isolation |
| `showcase/samples.rs` test module **(new assertion)** | the terminal sample's grid carries a detected address and a cell with a declared hyperlink. `showcase_completeness.rs` still checks only that the pane is listed | Principle VIII |
| `micold-client/tests/gates/context_menu_anchor.rs`, `tests/context_menu_anchor_call_sites.rs` | the menu still anchors at the press with the two new items | FR-020 |
| `scripts/check-user-guide-updated.sh` (CI) | the user guide changed in each `feat` PR that changes `src/ui/**` or `src/features/**` | FR-023, Principle VII |

### A.3 SC-005, measured, not gated

The frame-time comparison is reported for trend and never gates a build, following feature 018's
precedent (`micold-core/src/frame_probe.rs`):

1. Build release: `mise run build`.
2. In a Regular Terminal, with the pointer resting on a link in the pane, run
   `specs/031-clickable-terminal-links/scripts/stream-links.sh` (10,000 lines, each with an address,
   one in ten 1,000 characters long). Record the frame probe's p95.
3. Repeat with `--plain`, where the addresses are replaced by words. The first p95 must be within
   10% of the second.
4. Record both figures under **The pass** below.

---

## Part B — by eye (the `visual-pass` skill)

### B.0 Fixture

A private Xvfb display and a private pin directory (see memory "Xvfb displays collide"). The client
is launched with a seeded `projects.json`, since the client binary has no CLI. The fixture file
`specs/031-clickable-terminal-links/scripts/links-fixture.sh` prints:

```text
See https://example.com/docs/page.html for details.
(https://en.wikipedia.org/wiki/Rust_(programming_language))
mailto:team@example.com
<OSC 8 "https://example.com/manual">docs<OSC 8 end>  <OSC 8 "https://example.com/other">other<OSC 8 end>
file://$(hostname)/tmp/031-fixture/readme.txt
file:///tmp/031-fixture/run.sh        (chmod +x)
file:///tmp/031-fixture/folder
a line long enough that the terminal soft-wraps https://example.com/a/very/long/path/…
```

### B.n Steps

| # | Do | See | Requirement |
|---|---|---|---|
| B.1 | Hover the first address, light theme | underline under exactly the address, not the full stop; pointer is a hand; hint bottom-left shows the address | FR-005, FR-007, FR-008 |
| B.2 | Same, dark theme; then select part of the address, and put the cursor on it (`printf` the address after the prompt, move the cursor back over it) | underline and hint legible; text colour, the selection highlight and the cursor all still visible | FR-009 |
| B.3 | Ctrl+click it | the default browser receives exactly the address, in one action and within 1 s; the terminal gets no input | FR-004, FR-010, FR-014, SC-001, SC-003 |
| B.4 | Drag from inside it; double-click it; triple-click it | selections only; nothing opens | FR-004, SC-004 |
| B.5 | Hover the Wikipedia line | the inner `)` is kept, the outer is not | FR-005 |
| B.6 | Narrow the pane until the long address wraps; hover the second row | both rows underlined; Ctrl+click opens the whole address | FR-003 |
| B.7 | Hover `docs` and `other` | each is its own link; the hint shows the declared address | FR-002, FR-007, FR-008 |
| B.8 | Ctrl+click `mailto:` | the default mail client opens a message | FR-010 |
| B.9 | Ctrl+click `readme.txt`, then `folder` | the text editor opens it; the file manager opens the folder | FR-010 |
| B.10 | Ctrl+click `run.sh` | the file manager shows it selected, or the folder opens; it does not run | FR-013 |
| B.11 | Right-click a link; choose Copy Link Address and paste into a text field; right-click plain text | the complete address; the menu over plain text has no link items | FR-020, FR-021 |
| B.12 | Run `vim` with `:set mouse=a`; Ctrl+click a link; then Shift+Ctrl+click it | first click goes to vim; second opens | FR-016 |
| B.13 | Move `/tmp/031-fixture/readme.txt` away; Ctrl+click it | a notification naming the address and "doesn't exist" | FR-015 |
| B.14 | Start the client from a shell with `TERM_PROGRAM=WezTerm FORCE_HYPERLINK=1`, a fresh daemon; in a session run `env \| grep -E 'TERM_PROGRAM\|FORCE_HYPERLINK'` | nothing printed. Add `export FORCE_HYPERLINK=1` to the include script, restart the session, run again: `FORCE_HYPERLINK=1` | FR-006 |
| B.15 | Sandbox placement (feature 027): in a session, `printf 'file://%s/%s/x.md\n' "$(hostname)" "$PWD"` for a file in the project | hint shows the host path; Ctrl+click asks to confirm naming that path; Open opens it; a `/tmp/x` link says "not reachable" | FR-012, FR-018, FR-018a |
| B.16 | Open a sandbox confirm, then stop the sandbox, then press Open | a notification that the sandbox stopped; nothing opens | Edge case "Pending opens" |
| B.17 | In an AI CLI session, ask the agent to print a long URL | the first-row piece is a link and its hint shows the truncated piece; with `FORCE_HYPERLINK=1` (B.14), the declared link opens in full | Edge case "hard-broken", FR-006 |
| B.18 | Read `docs/user-guide/worktrees-and-sessions.md` "Interacting with the terminal" | it names: the gesture per platform (Ctrl/Cmd), Shift under mouse reporting, runnable files revealed, the sandbox confirmation, the `FORCE_HYPERLINK` include-script opt-in, and what a failure looks like | FR-023 |

macOS and Windows parts of B.3, B.8–B.10 cannot be run on this machine. They are covered by the CI
build and test on those runners, and by `cargo check --target aarch64-apple-darwin`. If there is a
difference in behaviour, record it under **Recording** and do not tick it.

### Recording

Screenshots go to `specs/031-clickable-terminal-links/images/`, and the step table goes to
`visual-pass.md`, with one row per B step: pass/fail, screenshot, and a note.

## The pass — *(date, filled when run)*

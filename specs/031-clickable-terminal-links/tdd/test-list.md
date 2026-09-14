---
feature: 031-clickable-terminal-links
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 22
planned_at: 411711c1
updated_at: 411711c1
suite_baseline: red # locally only: 2 out-of-flow daemon `pi` tests; CI on main (b27ffe62) green — see cycle-log.md
---

# Test List: Clickable Links in the Terminal

Derived from `spec.md` (US1–US4 acceptance scenarios, FR-001–FR-023, SC-001–SC-007) and `plan.md`
with its contracts. It was not derived from the code.

**Trace ids.** spec.md numbers acceptance scenarios inside each story, so `US2.5` means User Story 2,
acceptance scenario 5. `FR-`, `SC-` and contract row ids (`C4`, `G3b`, `T7`, `L6`) refer to
`spec.md` and `contracts/`.

**Milestones.** Inner behaviors are grouped by component, not by milestone. `tasks.md` carries each
id on the tasks that write and implement it, and its `## Milestones` section says which PR ships
them.

## Outer loop: acceptance behaviors

**Entry point.** The desktop client has no end-to-end GUI runner; the profile's acceptance runner is
the sandbox real-runtime suite. So these are **integration tests of the composed client**, the
highest level the repository can test:

- a `GridCache` holding the printed lines;
- the real `TerminalPane` widget driven headless (tiny-skia, as the pane's existing `dispatch`
  harness does), with real mouse and modifier events;
- the published `Message`s run through `crate::update_inner` on `base_app()`;
- recording `LinkOpener` and clipboard capabilities.

They live in `crates/micold-client/src/shell/links.rs` `#[cfg(test)] mod acceptance`, which is in
the binary crate beside `update_inner`. What they cannot see — the real browser, file manager and
rendered pixels — is covered by the quickstart §B visual pass.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| A1  | Hovering any character of `https://example.com/docs/page.html` in `See https://example.com/docs/page.html for details.` marks exactly the address's cells (not "See", the space or the full stop) and the pane's pointer is `Pointer` | US1.1, FR-001, FR-005, FR-007 | example | PENDING | |
| A2  | A Ctrl/Cmd press and release on that address calls the opener once with `https://example.com/docs/page.html` and publishes no `TerminalBytes` | US1.2, FR-004, FR-010, FR-014, SC-001 | example | PENDING | |
| A3  | An address the terminal soft-wrapped across two rows opens complete when activated on either row | US1.3, FR-003 | example | PENDING | |
| A4  | Activating `(https://example.com/a_(b))` opens `https://example.com/a_(b)`, and `"https://example.com"` opens `https://example.com` | US1.4, FR-005 | example | PENDING | |
| A5  | A plain drag, double-click or triple-click starting on a link selects as today and calls no opener | US1.5, FR-004 | example | PENDING | |
| A6  | An address scrolled into scrollback, with the view scrolled up to it, opens the same address as on the live screen | US1.6 | example | PENDING | |
| A7  | Hovering `docs` declared as `https://example.com/manual` marks the run and the hint shows `https://example.com/manual` | US2.1, FR-002, FR-008 | example | PENDING | |
| A8  | Activating that run calls the opener with `https://example.com/manual` | US2.2, FR-002 | example | PENDING | |
| A9  | Two adjacent runs with different declared addresses each resolve to their own address when hovered | US2.3 | example | PENDING | |
| A10 | With the same declared address on two runs separated by undeclared text, hovering one marks only that run | US2.4 | example | PENDING | |
| A11 | Declared text that reads `https://a.example` but declares `https://b.example` shows and opens `https://b.example` | US2.5, FR-008 | example | PENDING | |
| A12 | A `file://<this host>/<tmp>/readme%20a.txt` declared name, as `ls --hyperlink=always` prints it, calls `opener.open` with the decoded existing path | US2.6, FR-012 | example | PENDING | |
| A13 | Activating `mailto:team@example.com` calls the opener with `mailto:team@example.com` | US3.1, FR-010 | example | PENDING | |
| A14 | Activating a `file://` link to an existing document calls `opener.open` with its host path | US3.2, FR-010 | example | PENDING | |
| A15 | Activating a `file://` link to an existing folder calls `opener.open` with the folder path | US3.3, FR-010 | example | PENDING | |
| A16 | Activating a `file://` link to a runnable file calls `opener.reveal` and never `opener.open` | US3.4, FR-013 | example | PENDING | |
| A17 | Activating a `file://` link to a missing file calls no opener and raises `Couldn't open <address>: the file doesn't exist on this machine` | US3.5, FR-015 | example | PENDING | |
| A18 | `file://otherhost/x`, `javascript:alert(1)` and `vscode://x` are not marked on hover and activating them calls no opener | US3.6, FR-011, FR-012 | example | PENDING | |
| A19 | In a sandboxed session with a shared project, activating `file:///work/<project>/readme.txt` opens a confirmation naming the host path; **Open** calls `opener.open` with that host path | US3.7, FR-018, FR-018a | example | PENDING | |
| A20 | A right press over a link opens the terminal menu with **Open Link** and **Copy Link Address** ahead of today's items | US4.1, FR-020 | example | PENDING | |
| A21 | Choosing **Copy Link Address** on a declared link writes its declared address to the clipboard, and on a wrapped detected link the complete unwrapped address | US4.2, FR-021 | example | PENDING | |
| A22 | A right press over plain text opens the menu with no link items | US4.3, FR-020 | example | PENDING | |

Guards, green on arrival, each with the deliberate mutant that shows its red:

- A5: selection already works and nothing opens before T029. Mutant: `link_gesture` activates on a press without the Ctrl/Cmd modifier.
- A18: after T011 these addresses are already `NotFollowable`. Mutants: `classify` returns `Web` for `javascript:`; the `File` branch of `resolve` accepts any host.
- A22: the menu has no link items before T073. Mutant: `link_menu_items(None)` returns both items.

## Inner loop: unit behaviors

### `crates/micold-core/src/link/detect.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U1  | Finds `https://example.com/docs/page.html` inside a sentence, without the final full stop | FR-001, FR-005 | example | PENDING | |
| U2  | Trims trailing `.,;:!?'*` repeatedly (`…page.html?!.` ends at `html`) | FR-005 | example | PENDING | |
| U3  | Keeps a closing bracket balanced inside the address (`https://example.com/a_(b)`) | FR-005, US1.4 | example | PENDING | |
| U4  | Stops before an unbalanced closing bracket (`(https://example.com/a)` excludes `)`) | FR-005, US1.4 | example | PENDING | |
| U5  | Excludes an enclosing quote (`"https://example.com"`) | FR-005, US1.4 | example | PENDING | |
| U6  | In `[text](https://x.example)` only the address is found | FR-005 | example | PENDING | |
| U7  | In `<https://x.example>` only the address is found | FR-005 | example | PENDING | |
| U8  | Rejects a scheme preceded by a letter or digit (`xhttps://a.example`) | FR-001 | example | PENDING | |
| U9  | Stops the scan at whitespace, control characters and each of `< > " \` { } \| \ ^` | FR-001 | example | PENDING | |
| U10 | Accepts web hosts `localhost`, `a.example`, `[::1]` and `intranet:8080`; rejects `http://intranet` (no dot, no port) | FR-001 | example | PENDING | |
| U11 | Rejects `https://` alone and `http://exa mple` | FR-001 | example | PENDING | |
| U12 | Finds `mailto:team@example.com`; rejects `mailto:@example.com` and `mailto:team@` | FR-001 | example | PENDING | |
| U13 | Finds `file:///tmp/x`; rejects a `file:` address whose path does not start with `/` | FR-001 | example | PENDING | |
| U14 | Never finds scheme-less text (`example.com/docs`, `www.example.com`) | FR-001, SC-002 | example | PENDING | |
| U15 | Never finds `javascript:…`, `data:…` or `vbscript:…` | FR-011 | example | PENDING | |
| U16 | Matches the scheme ASCII case-insensitively (`HTTPS://EXAMPLE.COM`) | FR-001 | example | PENDING | |

### `crates/micold-core/src/link/line.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U17 | Any cell of a maximal same-URI declared run returns that whole run as one link | FR-002, L1 | example | PENDING | |
| U18 | Two same-URI runs separated by an undeclared cell are two links | US2.4, L6 | example | PENDING | |
| U19 | Adjacent runs with different URIs are two links with their own addresses | US2.3 | example | PENDING | |
| U20 | A declared URI wins over address-shaped visible text in the same cells | US2.5 | example | PENDING | |
| U21 | A detected address over rows joined by `wrapped = true` returns cells on every row it covers | FR-003, US1.3, L2 | example | PENDING | |
| U22 | Rows separated by a real line break are never joined; only the first row's well-formed piece is a link | FR-003 | example | PENDING | |
| U23 | A wrapped logical line within 64 rows each way is joined; a candidate reaching the 64-row cap is dropped | FR-003, L4 | example | PENDING | |
| U24 | A candidate touching a row whose `text` is `None` is dropped | L7 | example | PENDING | |
| U25 | A wide character's spacer cell belongs to the link | FR-007, L5 | example | PENDING | |
| U26 | A plain-text cell returns `None` | L3 | example | PENDING | |
| U27 | A negative (scrollback) row resolves exactly as a viewport row with the same content | US1.6 | example | PENDING | |

### `crates/micold-core/src/link/address.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U28 | `http`/`https` classify as `Web` and `mailto` as `Mail`, verbatim | FR-010, C1, C2 | example | PENDING | |
| U29 | Scheme classification is ASCII case-insensitive | FR-011 | example | PENDING | |
| U30 | `vscode:`, `slack:`, `zoommtg:`, `javascript:`, `data:`, `vbscript:` and an unknown scheme classify as `NotFollowable` | FR-011, C3 | example | PENDING | |
| U31 | `file://host/path` classifies as `File { host, path }` with `%20` decoded to a space | FR-012, C9 | example | PENDING | |
| U32 | An invalid percent escape, or a decoding that is not UTF-8, classifies as `NotFollowable` | FR-012, C10 | example | PENDING | |

U32 is a guard: `file:` is `NotFollowable` until T046. Its red is shown after T046 by a decoder that passes an invalid escape through literally.

### `crates/micold-core/src/link/resolve.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U33 | A web or mail link resolves to `Url(address)` with `display == address` and no confirmation | FR-008, C1, C2 | example | PENDING | |
| U34 | A declared non-followable URI resolves to `None` | FR-011, C3 | example | PENDING | |
| U35 | A `file` link with host empty, `localhost` or one of `host_names` (any ASCII case) resolves to `HostPath(path)` | FR-012, C4, C5 | example | PENDING | |
| U36 | A `file` link naming another host, including `file://server/share/…` and an unresolvable `file://build-host.invalid/…`, resolves to `None` with no lookup | FR-012, US3.6, C6, C18 | example | PENDING | |
| U37 | With `windows_host`, `/C:/Users/x` resolves to `C:\Users\x`, and a path with no drive resolves to `None` | FR-012, C7, C8 | example | PENDING | |
| U38 | In a sandboxed context, the container id's 12-character prefix and the full id are accepted as hosts | FR-018, C11 | example | PENDING | |
| U39 | A sandboxed path under a shared location resolves to that location's host path, with `needs_confirmation` | FR-018, FR-018a, C11 | example | PENDING | |
| U41 | A sandboxed path under no shared location resolves to `Unreachable`, displayed as `<path> — not reachable from this machine` | FR-018, C12 | example | PENDING | |
| U46 | For every resolved link, `display` equals the string its target carries (`Url` and `HostPath`), sampled over every case above | SC-006 | example | PENDING | |
| U47 | In a sandboxed context, this machine's `host_names` still translate through the shared locations | FR-018, C17 | example | PENDING | |
| U140 | `host_names_from` lists the full name and its first DNS label (`build.example.com` → both), a dotless name once, an empty name as none | FR-012 | example | PENDING | |
| U141 | `container_host_names` gives a 64-character id's 12-character prefix and the full id, and a 12- or 8-character id once | FR-018, C11 | example | PENDING | |

U36 is a guard: `file:` resolves to `None` until T047. Its red is shown after T047 by a `File` branch that accepts any host.

U40, U42–U44 and U48 were placed on `sandbox/pathmap.rs` below; U46 is sampled by hand because the
profile has no property library.

### `crates/micold-core/src/link/mod.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U49 | No source in `link/` names `std::net`, `std::fs` or `std::process` | FR-019, US3.6 | example | PENDING | |

U49 is a guard: it is green on arrival. Its red is shown by adding `use std::fs;` to a `link/` file.

### `crates/micold-core/tests/link_corpus.rs`

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U132 | Every expected link in the ≥50-line corpus is found with its exact span, and no scheme-less text is found | SC-002 | approval | PENDING | |
| U133 | A corpus address hard-broken across real line breaks is found on its first row only | SC-002, FR-003 | example | PENDING | |

`approval` here is a committed fixture with inline expected spans, not a regenerated snapshot.

### `crates/micold-core/src/link/runnable.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U50 | Linux and macOS: a file with any execute bit reveals; the same file with none opens | FR-013 | example | PENDING | |
| U51 | Linux: each listed launcher extension (`AppImage` in any case) reveals; `.txt` opens | FR-013 | example | PENDING | |
| U52 | macOS: a bundle directory (by extension or `is_bundle`) reveals; an ordinary folder opens | FR-013 | example | PENDING | |
| U53 | macOS: each listed extension reveals | FR-013 | example | PENDING | |
| U54 | Windows: `%PATHEXT%` entries and each listed extension reveal; the execute bit is ignored, so `.txt` with it opens | FR-013 | example | PENDING | |
| U55 | Only the last extension counts: `x.exe.txt` opens and `x.txt.exe` reveals on Windows | FR-013 | example | PENDING | |

### `crates/micold-core/src/sandbox/pathmap.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U40 | The most specific of nested shared locations maps the path | FR-018, C15 | example | PENDING | |
| U42 | A path whose `..` climbs above `/` maps to nothing | FR-018, C13 | example | PENDING | |
| U43 | Locations match whole path components: `/work/projector` is not under `/work/proj` | FR-018 | example | PENDING | |
| U44 | A result equal to or under a denied host path maps to nothing | FR-018a, C16b | example | PENDING | |
| U48 | On a Windows host the remainder joins the host location with `\` | FR-018, C14 | example | PENDING | |
| U145 | Normalisation precedes the location match: with `/work/proj` shared, `/work/proj/../../etc/passwd` maps to nothing and `/work/proj/a/../b` maps to `<host>/b` | FR-018, C13 | example | PENDING | |

### `crates/micold-core/src/sandbox/parse.rs`, `lifecycle.rs`, `mod.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U56 | Mount destinations are read from `Mounts[].Destination` in Docker and in Podman inspect output | FR-018 | example | PENDING | |
| U57 | Inspect output with no `Mounts` gives no destinations | FR-018 | example | PENDING | |
| U58 | `Started.mounted` is `None` when the container was created or replaced | FR-018 | example | PENDING | |
| U59 | `Started.mounted` is the destinations when the container was attached or started | FR-018 | example | PENDING | |
| U60 | `shared_locations` lists projects, state, home and credentials, most path components first | FR-018, C15 | example | PENDING | |
| U61 | The secret mount is never a shared location | FR-018a, C16 | example | PENDING | |
| U62 | With `mounted = Some`, a location the container does not mount is not shared; with `None`, all are | FR-018, C16c | example | PENDING | |
| U63 | The denied list holds the secret mount's host path | FR-018a, C16b | example | PENDING | |

### `crates/micold-core/src/env_include.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U64 | Each of the nine inherited identity keys is matched whatever its value | FR-006 | example | PENDING | |
| U65 | `COLORTERM` is matched unless its value is `truecolor` or `24bit` (any ASCII case) | FR-006 | example | PENDING | |
| U66 | Keys compare exactly when `keys_case_insensitive` is false (`term_program` unmatched) and case-insensitively when true | FR-006 | example | PENDING | |
| U67 | Given a fixed inherited environment, the Unix `bash` builder and both Windows `powershell.exe` builders remove exactly the matched keys | FR-006 | example | PENDING | |

### `crates/micold-daemon/src/supervisor.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U68 | A session spawned by `spawn_shell` or `spawn_ai_cli` does not see inherited `TERM_PROGRAM=WezTerm` or `FORCE_HYPERLINK=1` | FR-006 | example | PENDING | |
| U69 | `FORCE_HYPERLINK=1` set by the include script is present, also when it was inherited too | FR-006 | example | PENDING | |
| U70 | Inherited `COLORTERM=truecolor` is kept | FR-006 | example | PENDING | |
| U71 | `TERM` is `xterm-256color` as before | FR-006 | example | PENDING | |

### `crates/micold-daemon/tests/osc8_passthrough.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U72 | An OSC 8 link printed by a child of the real PTY supervisor reaches the grid cell's hyperlink, on each CI OS (a Windows ignore is recorded with its CI run if ConPTY drops it) | FR-002, spec Edge Cases (terminal layer drops declared hyperlinks) | characterization | PENDING | |

It is a characterization test because it pins what the daemon already does. It must be green against
untouched code, and it is recorded as `BASELINE`.

### `crates/micold-client/src/features/session.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U73 | `LinkActivated` with `Url(u)` emits `OpenLink(Url(u))` | FR-010, T5 | example | PENDING | |
| U74 | `LinkOpenFinished` with `NoApplication` notifies `Couldn't open <address>: no application is set up to open it` | FR-015, T12 | example | PENDING | |
| U75 | `LinkOpenFinished` with `LaunchFailed(e)` notifies `Couldn't open <address>: <e>` | FR-015, T12 | example | PENDING | |
| U76 | `LinkOpenFinished` with `Ok` notifies nothing | FR-015, T13 | example | PENDING | |
| U77 | `LinkOpenFinished` with `NotFound` notifies `Couldn't open <address>: the file doesn't exist on this machine` | FR-015, US3.5 | example | PENDING | |
| U78 | `LinkActivated` with `HostPath(p)` and no confirmation emits `OpenLink(Path { path: p, address })` | FR-010, T6 | example | PENDING | |
| U79 | `LinkActivated` with `Unreachable` notifies `Couldn't open <address>: the sandbox doesn't share that location with this machine` and emits no open | FR-015, FR-018, T8 | example | PENDING | |
| U80 | `LinkActivated` needing confirmation records the pending open and shows the confirm surface with the host path | FR-018a, T7 | example | PENDING | |
| U81 | Confirming while the session exists and the sandbox is live emits `OpenLink(Path)` | FR-018a, T9 | example | PENDING | |
| U82 | Confirming after the session closed notifies `Couldn't open <host path>: the session has closed` and opens nothing | FR-018a, T9 | example | PENDING | |
| U83 | Confirming after the sandbox stopped notifies `Couldn't open <host path>: the sandbox has stopped` and opens nothing | FR-018a, T9 | example | PENDING | |
| U84 | Declining opens nothing and clears the pending open | FR-018a, T10 | example | PENDING | |
| U85 | A menu opened with a link lists **Open Link** and **Copy Link Address** first | FR-020, T14 | example | PENDING | |
| U86 | A menu opened without a link lists today's items only | FR-020, T14 | example | PENDING | |
| U87 | **Open Link** acts as `LinkActivated` on the captured link and closes the menu | FR-017, FR-020, T15 | example | PENDING | |
| U88 | **Copy Link Address** emits `ClipboardWrite` with the captured link's address | FR-021, T16 | example | PENDING | |
| U89 | Closing the menu clears the captured link | FR-017, T17 | example | PENDING | |
| U143 | `link_menu_items` is Open Link then Copy Link Address for a link, and empty for none | FR-020, M1, M2 | example | PENDING | |

### `crates/micold-client/src/features/sandbox.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U90 | `started(started, locations)` makes `locations()` return them, for a created and for an attached container | FR-018, T18 | example | PENDING | |
| U91 | After each exit from `Running`/`Stale` (stopped, failed, container lost, fallback accepted) and after `for_placement`, `locations()` is `None` | FR-018, T19 | example | PENDING | |
| U92 | A `Stale` sandbox still returns its locations | FR-018 | example | PENDING | |

### `crates/micold-client/src/shell/links.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U93 | `LinkActivated` for a URL, through `update_inner`, calls `opener.open` with the URL verbatim | FR-010 | example | PENDING | |
| U94 | No timer or debounce stands between the `OpenLink` outcome and the opener call | SC-003 | example | PENDING | |
| U95 | An `OpenLink(Path)` for a missing path finishes with `NotFound` and calls no opener | FR-015, T11 | example | PENDING | |
| U96 | An existing document or folder is passed to `opener.open` | FR-010, T11 | example | PENDING | |
| U97 | On each CI OS, real runnable files of every kind that OS lists (including a symlink to an executable file, the extension-only kinds such as Linux `.desktop`/`.AppImage` and macOS `.command`, and inside a directory whose name has a space) are passed to `reveal` 100%, and `.txt`, `.png`, `.pdf`, `.html` to `open` 100% | FR-013, SC-007 | example | PENDING | |
| U146 | macOS: a real directory without a bundle extension that holds `Contents/Info.plist` is passed to `reveal` | FR-013 | example | PENDING | |
| U98 | `LinkOpenConfirmed` calls the opener when `App.sandbox` is `Running`, and notifies without calling it when the sandbox has stopped | FR-018a | example | PENDING | |
| U99 | `ContextMenuCopyLinkAddress` through `update_inner` returns the clipboard write task rather than dropping it | FR-021 | example | PENDING | |
| U100 | `ContextMenuOpenLink` on a URL link reaches the opener | FR-020 | example | PENDING | |

### `crates/micold-client/src/shell/link_opener.rs`, `shell/capabilities.rs`

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U101 | A launcher exiting 0 inside the 2 s window is success | FR-015 | example | PENDING | |
| U102 | Linux: a launcher exiting 3 inside the window is `NoApplication` | FR-015 | example | PENDING | |
| U103 | A launcher exiting with another non-zero code inside the window is `LaunchFailed` | FR-015 | example | PENDING | |
| U104 | A launcher still running at the end of the window is success and is left running | SC-003 | example | PENDING | |
| U138 | Linux: when the file manager's select-item call fails, reveal opens the containing folder instead | FR-013 | example | PENDING | |
| U139 | macOS: an `open` exiting non-zero inside the window is `NoApplication` when its stderr names no application, and `LaunchFailed` otherwise | FR-015 | example | PENDING | |
| U144 | Windows: a `ShellExecuteW` return above 32 is success, 31 and 27 are `NoApplication`, and 32 and 2 are `LaunchFailed` | FR-015 | example | PENDING | |
| U105 | `SystemLinkOpener` is named in client `src/` only at its definition and in `Capabilities::real()`, and the definition exists | Constitution (capability seams), FR-019 | example | PENDING | |

### `crates/micold-client/src/ui/material/terminal_pane.rs`

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U106 | A command-modified press and release on one cell over a link publishes exactly one `LinkActivated` | FR-004, G1 | example | PENDING | |
| U107 | Moving off the press cell before release publishes no activation and starts a selection at the press cell | FR-004, G2 | example | PENDING | |
| U108 | A plain double or triple click on a link selects and publishes no activation | FR-004, G3 | example | PENDING | |
| U109 | A command-modified double click on a link activates once, and its second press counts as a double click that selects | FR-004, G3b | example | PENDING | |
| U110 | A plain press on a link publishes no activation | FR-004, G4 | example | PENDING | |
| U111 | Under mouse reporting without Shift, a command-modified press is reported to the program and activates nothing | FR-016, G5 | example | PENDING | |
| U112 | Under mouse reporting with Shift, a command-modified press and release on a link activates it | FR-016, G6 | example | PENDING | |
| U113 | A command-modified click on an unfocused pane asks for focus and activates the link | FR-004, G8 | example | PENDING | |
| U114 | When the link under the pointer changed between press and release, the release-time link opens; when there is none, nothing | FR-017, G9 | example | PENDING | |
| U115 | No gesture step publishes `TerminalBytes`, a selection change or a scroll | FR-014 | example | PENDING | |
| U116 | 100 scripted plain drag, double- and triple-click selections starting on links publish 0 activations | SC-004 | example | PENDING | |
| U142 | A middle click or wheel over a link behaves as today and publishes no activation | FR-004, G7 | example | PENDING | |
| U117 | A right press over a link publishes `TerminalContextMenuOpened` carrying the link resolved at the press; over plain text, `None` | FR-017, FR-020 | example | PENDING | |
| U118 | Hover is recomputed when the pointer moves to another cell | FR-007 | example | PENDING | |
| U119 | Hover is recomputed on redraw when the grid version moved and the consulted rows' content changed, including an in-place redraw of the same line | FR-007, spec Edge Cases (output moving under the pointer) | example | PENDING | |
| U120 | Hover is reused without re-resolving when the grid version moved but the consulted rows are unchanged | SC-005 | example | PENDING | |
| U121 | Hover is recomputed when the session shown or the `LinkContext` changes | FR-007 | example | PENDING | |
| U122 | Under mouse reporting, a link is marked only while Shift is held, and pressing or releasing Shift updates it | FR-016 | example | PENDING | |
| U123 | The mouse interaction is `Pointer` over a followable link and unchanged elsewhere | FR-007 | example | PENDING | |
| U124 | Hovering and pressing in one pane leaves another pane's hover and press empty, and only the pressed pane activates | FR-022 | example | PENDING | |
| U125 | The address hint sits bottom-left of the content by default | FR-008 | example | PENDING | |
| U126 | The hint moves top-left when the pointer is within the hint's height of the bottom edge, and stays bottom-left one row above that | FR-008 | example | PENDING | |
| U127 | The hint always lies inside the content bounds | FR-008 | example | PENDING | |
| U128 | A long address is middle-elided in the label while `ResolvedLink.display` stays complete | FR-008, SC-006 | example | PENDING | |
| U129 | Hovering a declared run marks exactly that run and resolves the declared address | FR-002, FR-008, US2.1 | example | PENDING | |

### `crates/micold-client/src/showcase/samples.rs`

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U131 | The terminal sample's grid holds a detected `https://` address and a declared hyperlink run | Constitution (showcase covers visible states), FR-007 | example | PENDING | |

### `crates/micold-client/src/ui/terminal.rs`, `ui/mod.rs`, `main.rs` (glue)

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U134 | The pane's `LinkContext` is sandboxed exactly while the sandbox is `Running`/`Stale`, carrying its locations, denied paths and container-id host names; otherwise it has no sandbox part | FR-018 | example | PENDING | covered at the outer loop by A19 and A14 |
| U136 | Boot fills `app::State.host_names` from `host_names_from(gethostname())` | FR-012 | example | PENDING | glue; the branching is U140, the wiring is covered at the outer loop by A12 |

These are glue (Constitution: glue is covered by the composed tests), so their test is the named
acceptance behavior rather than a unit of their own. AI CLI and Regular Terminal panes are the same
`TerminalPane`, so A1–A6 driving one pane kind covers FR-001's "both pane kinds".

### `crates/micold-client/tests/overlay_*.rs`, `popover_displacement.rs`

| id   | behavior | traces | kind | state | test |
| ---- | -------- | ------ | ---- | ----- | ---- |
| U137 | `confirm_link_open` is a registered floating surface: listed in each overlay list, dismissed by Escape and scrim click as a decline, counted among the 10 dialogs | FR-018a | example | PENDING | |

## Invariants and edge cases still to place

None. Every edge case in spec.md maps to a behavior above or to "Out of scope" below:

| Edge case | Behavior |
|---|---|
| Accidental activation during selection | U107, U108, U116 |
| Click to focus | U113 |
| Mouse-reporting programs, and hover under them | U111, U112, U122 |
| Output moving under the pointer | U114, U119 |
| Trailing and enclosing punctuation | U2–U5 |
| Markdown and angle forms | U6, U7 |
| Addresses a program broke itself | U22, U133 |
| Malformed or unsupported addresses | U11, U15, U30, U34 |
| Very long addresses | U23 |
| No handler | U74, U75 |
| Sandboxed sessions | U38–U48, U56–U63 |
| Encoded and Windows paths | U31, U37 |
| Filesystems that mark every file executable, and symlinks | U50, U97 |
| Pending opens | U82, U83 |
| Platforms that drop declared hyperlinks | U72 |
| Multiple panes | U124 |
| Repeated activation | U106 |

## Out of scope

- **FR-009 legibility** (underline colour, cursor and selection still visible in both themes). This
  is a rendered-pixel property, checked by the quickstart §B visual pass, not by a unit.
- **SC-005 frame timing.** It is measured (T079, quickstart §A.3). The structural part, reuse of
  hover without re-resolving, is U120.
- **FR-023 user guide.** Documentation, checked by CI's user-guide gate and by quickstart §B.18.
- **The real browser, mail client and file manager launching.** Platform behaviour outside this
  process. The opener's contract to the OS is U101–U104, and the launch is seen in the visual pass.
- **`localhost` port publishing from a sandbox.** The spec says this feature does not translate or
  publish ports.
- **Characterization baselines for existing selection, focus and mouse reporting.** Not needed:
  `terminal_pane.rs` already carries 44 tests, including double- and triple-click selection, press
  routing under mouse reporting and focus on press, which pin today's behaviour before the gesture
  changes it. `env_include` (`crates/micold-core/tests/env_include*.rs`), the supervisor (11 tests),
  `features::sandbox` (`tests/features_sandbox.rs`) and `features::session`
  (`tests/features_session.rs`) are likewise covered.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` (detected at `cdc473ab`):

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact` (assert on the
  observed `N passed`: a filter matching nothing exits 0)
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`)
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets` (`mise run test-core`)
- Acceptance (real runtime): `scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1`
- Coverage: none (no `cargo-llvm-cov`)
- Mutation: none (no `cargo-mutants`); strength is checked by deliberate mutants by hand
- Property: none (no `proptest`)

In-source `#[cfg(test)]` modules run with `scripts/build-lock.sh cargo test -p <crate> --lib <path>`
(or `--bin micold-ai-ide` for `shell/` and `main.rs`).

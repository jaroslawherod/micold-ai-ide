# Phase 0 Research: Clickable Links in the Terminal

**Feature**: 031-clickable-terminal-links | **Date**: 2026-09-14

Every unknown the Technical Context raised is resolved below against the code as it stands, with
file:line evidence. The three product questions (gesture, advertising, application schemes) were
settled by the user in the spec's Clarifications and are not reopened here.

---

## R1 — What the terminal already knows about links

**Finding**: declared hyperlinks (OSC 8) already travel daemon → client intact.
`alacritty_terminal` 0.26 keeps them per cell, including in scrollback. The daemon's framer
interns the URI and drops the OSC 8 `id` parameter. It sends `CellExtras.hyperlink: Option<u16>`
plus `GridFrame.hyperlinks`. The client resolves those into `CachedExtra.hyperlink: Option<String>`
(`crates/micold-client/src/grid.rs:43-50`). `CachedLine` also carries `wrapped`, which says the
line soft-wraps into the next (`grid.rs:63-64`). It keeps wide-character spacer cells, so
`text.chars().nth(col)` is column-exact (`grid.rs:57-58`). Nothing reads `hyperlink` today:
`TerminalPane::draw` ignores it (`ui/material/terminal_pane.rs:587-765`).

Plain-text addresses are not detected anywhere. The workspace has no `regex`, `linkify` or `url`
crate (`Cargo.lock`).

**Decision**: no protocol change. Everything the feature needs from a session, the client already
has: the text, the soft-wrap flag and the declared URIs. It also knows the session's placement
(`App.placement`, `main.rs:123`). `PROTOCOL_VERSION` stays 12 and `SCHEMA_HASH` stays unmoved.

**Rejected.** Detecting links in the daemon and shipping spans on the wire. That would add a
protocol bump and per-frame work on every line, for something needed only under the pointer (R2).

---

## R2 — When links are computed (SC-005)

**Finding**: SC-005 bounds the frame time while 10,000 address-bearing lines stream. Computing
links for every line on every frame would put a text scan on the hot path. Its cost would grow with
the output, not with what the user is doing.

**Decision**: links are computed **only for the pointer's logical line**. That happens when the
pointer moves to a different cell, and again when the grid under a stationary pointer changes
(the edge case "output moving under the pointer"). The result is cached in the pane's widget state. A
`LineId` alone cannot key it: a program that redraws a row in place (a progress bar, a TUI) keeps the
row's id and changes its text. So the cache is keyed by:

- the session shown and the `LinkContext` (a sandbox stopping changes what a `file` link resolves to);
- the pointer cell;
- a 64-bit hash of the rows `link_at` consulted: their text, `wrapped` flags and hyperlink extras.

The hash is recomputed only on a frame whose grid version (read through the existing `GridCache::generation()` and
`GridCache::seq()`) differs from the cached one, and only over those at most 129 rows. When the
hash is unchanged, the cached result is reused and `link_at` does not run. The check runs in the
pane's `update` on `window::Event::RedrawRequested`, since `draw` receives `&Tree` and cannot write
`PaneState`; when `resolved` changes there, the pane requests another redraw. Drawing adds one underline per hovered cell and one hint. With the pointer away from the
pane, streaming costs nothing extra, so the SC-005 comparison measures the hover path only.

**Rejected.** Keying by `LineId`s: an in-place redraw keeps the ids, so the hint would show the old
address while the release re-resolves and opens the new one (SC-006). Keying by the grid version
alone: every streamed frame would re-run `link_at` under a resting pointer. Pre-computing spans per
`CachedLine` in `GridCache::apply`. That would cost a scan
per changed line per frame, which is exactly the SC-005 risk, and only to make a hover that happens
once a second faster.

---

## R3 — Recognising plain-text addresses (FR-001, FR-005, SC-002)

**Finding**: FR-001 needs an explicit `http://`, `https://`, `mailto:` or `file://` prefix, and
FR-001 forbids scheme-less matches. `linkify` finds bare domains and e-mail addresses by design, so
it would need filtering back out. A regex crate is a new dependency for about a hundred lines of
scanning.

**Decision**: a hand-written scanner in a new render-free module,
`micold_core::link::detect`. It is unit-tested against the SC-002 corpus, which is checked in as a
test fixture. The rules, in order:

1. A candidate starts at a case-insensitive scheme prefix that is not preceded by an ASCII letter or
   digit, so `xhttps://` does not match.
2. It extends over URL characters: RFC 3986 unreserved and reserved characters, `%`, and any
   non-ASCII printable character. It stops at whitespace, control characters, `<`, `>`, `"`, `` ` ``,
   `{`, `}`, `|`, `\` and `^`.
3. **Trailing punctuation** (`.` `,` `;` `:` `!` `?` `'` `*`) is trimmed from the end, repeatedly.
4. **Brackets**: a closing `)`, `]` or `}` at the end is kept only if it balances an opener inside
   the candidate, so `(https://example.com/a_(b))` yields `https://example.com/a_(b)`. The scan also
   stops at an unbalanced closer. That handles `[text](https://x.y)`, where the link begins after
   `(` and ends before the unbalanced `)`.
5. **Enclosing quotes**: a candidate opened right after `"` or `'` ends before the matching quote.
6. **Well-formedness**, checked after trimming:
   - `http` and `https` need a host that is `localhost`, contains a `.`, is a bracketed IPv6
     literal, or is followed by a `:port`. So `https://` alone is not recognised. Nor is
     `http://exa mple`: the scan stops at the space, and the host `exa` has no dot and no port. The
     spec lists both as malformed. `http://localhost:5173`, `http://devbox:8080` and
     `https://example.com` are recognised.
   - `file` needs `file://` followed by an optional host and a path starting with `/`.
   - `mailto:` needs a non-empty address containing `@` with text on both sides.

`javascript:`, `data:` and similar are never candidates, because rule 1 knows only four prefixes.

**Rejected.** `linkify` (scheme-less matches that FR-001 forbids). `regex` (a new dependency, and
bracket balancing is not a regular language anyway).

---

## R4 — Soft wraps, hard breaks and cells (FR-003, FR-007, SC-002)

**Finding**: `CachedLine.wrapped` separates a soft wrap from a real line break. Claude Code
hard-wraps its own output (verified against its 2.1.270 build during the spec phase), so its long
addresses arrive as separate, unwrapped lines. That is the spec's "hard-broken" edge case.

**Decision**: `micold_core::link::line` builds a **logical line**. It walks back from the pointer
row while the previous row has `wrapped`, then forward while the current row has it. The rows are
joined into one string, keeping a `Vec<(row, col)>` from char index back to cell, with wide-char
spacer cells skipped in the text but kept in the map. Detection runs on the joined string, and the
span maps back to cells on every row it covers. The walk is capped at **64 rows** in each
direction. That comfortably covers a 1,000-character address at 20 columns, and bounds the work on
pathological output. A candidate cut by the cap is not recognised, which is safer than recognising a
truncation.

Because only `wrapped` joins rows, a hard-broken address is detected on its first row alone, and its
continuation rows contain no scheme prefix. This gives the SC-002 behaviour with no special case.

Core reads rows through a trait, `LinkRows { text(row), wrapped(row), hyperlink(row, col), spacer(row, col) }`, with
rows relative to the viewport top, so it never sees `CachedLine`. The client implements it over
`GridCache::line(LineId)`. That reaches cached scrollback and lines below the viewport as well as
the visible screen. So an address that wraps past a viewport edge is joined whole when the cache holds the
rest. When it does not, the candidate touching that edge is dropped (contract L7), for the same
reason as the cap.

**Rejected.** Detecting per row: every soft-wrapped address would be cut (FR-003). alacritty's own
`WRAPLINE` flag on the client grid: the client renders from `CachedLine`, which already carries the
flag as `wrapped`, so a second source would be a second truth. An uncapped walk: a program printing
one 100,000-character line would make every pointer move scan all of it (SC-005).

---

## R5 — Declared links: runs and precedence (FR-002, FR-007, US2 scenarios 3–5)

**Finding**: the framer drops the OSC 8 `id`, so two separated runs with the same URI cannot be
told apart by `id`. The spec anticipated this. FR-007 defines a declared link as a **maximal run
of adjacent cells with the same address**, continuing across a soft wrap and stopping at a hard
break.

**Decision**: over the logical line of R4, `link::line` takes the pointer cell's `hyperlink`. If it
is `Some(uri)`, it extends left and right while neighbouring cells carry the same `uri`. **Declared
wins**: when the pointer cell carries a declared URI, detection is not consulted at all. That
satisfies US2 scenario 5, where the visible text looks like a different address. A declared link's
followability is judged on its URI (R8), never on its text.

**Rejected.** Letting detection override a declared URI when the text is itself an address: the
program chose the target, and US2 scenario 5 requires the declared one. Treating each declared cell
as its own link: the hint and the underline would cover one character. Reconstructing OSC 8 `id`s
by changing the framer: a protocol change (R1) for a distinction FR-007's run rule already makes.

---

## R6 — The gesture inside the pane (FR-004, FR-014, FR-016, FR-017, SC-004)

**Finding**: the pane handles presses at `terminal_pane.rs:868`, drags at `:950`, releases at
`:959` (auto-copy), middle clicks at `:973` and right clicks at `:998`. `press_routing(focused,
mouse_mode, shift)` (`:291`) already decides "report to the program" versus "the terminal handles
it". `PaneState.modifiers` is tracked from `ModifiersChanged` (`:792`). iced's
`Modifiers::command()` is Ctrl on Linux and Windows and ⌘ on macOS
(`iced_core-0.14.0/src/keyboard/modifiers.rs:80`). The platform link modifier therefore needs no
`cfg` in this codebase.

**Decision**: a new field, `PaneState.link_press: Option<LinkPress { link, cell }>`, drives a small
state machine. It is a pure function `link_gesture(event, state) -> GestureStep`, unit-tested in the
pane's test module.

| Event | Condition | Step |
|---|---|---|
| Left press | routing is *terminal* (not `MouseReport`), `modifiers.command()`, a followable link under the pointer, and click cadence is single | record `link_press`; **no** `TerminalSelectStart`; focus as today (`press_grants_focus`) |
| Left press | anything else | today's behaviour, unchanged |
| Cursor moved | `link_press` is set and the cell changed | drop `link_press`, then emit the `TerminalSelectStart` the press would have sent, anchored at the press cell. The user is now dragging a selection (FR-004) |
| Left release | `link_press` set and the cell is unchanged | **re-resolve** the link under the pointer from the current grid (FR-017). If it is still a link, emit `LinkActivated(link)` once; otherwise nothing. Clear `link_press` |
| Left press | cadence is double or triple | selection as today, never activation (SC-004). A link press also records `last_click` through `Click::new` (today only the local-selection branch does, `terminal_pane.rs:894-896`), so the second press of a modifier double-click is a double click, not a second activation (G3b) |

Under mouse reporting, `press_routing` sends Ctrl/⌘-clicks to the program exactly as today. The
gesture is reachable only with Shift held, which is today's override, so FR-016 falls out of the
existing router. Middle click and scroll are untouched. Activation emits a message only: no bytes
are written to the PTY and the selection does not change (FR-014).

**Rejected.** Opening on *press*: it could not be told apart from the start of a drag, which
SC-004 forbids. A time-based click threshold: "no motion" is observable and testable, while a
timer is neither.

---

## R7 — Showing the address while hovering (FR-007, FR-008, FR-009)

**Finding**: the shared `Tooltip` (`ui/material/mod.rs:231`) anchors to a widget, not to a cell,
and appears after a delay. FR-008 wants the display to change "in the same refresh" as the link
under the pointer. The overlay registry (`overlay/registry.rs:289`) hosts menus and dialogs, and
a hover surface there would be a new component with its own anchoring gates.

**Decision**: the **pane draws its own link hint**. It is a single-line label painted inside the
pane's content bounds, anchored to the bottom-left corner. It flips to the top-left when the
pointer is within the hint's height of the bottom edge, so it never covers the hovered link. It is
the browser status-bar convention. It is part of `TerminalPane`, which is already a shared
component with a showcase entry (`showcase/catalogue.rs:551-562`), so Principle VIII is met by
extending it, not by adding a widget. The hint text is `ResolvedLink.display` (data-model §1). The
placement is a pure function, `link_hint_rect(content, pointer_row, hint_size) -> Rectangle`, with
tests. Colours come from existing surface and on-surface roles, so FR-009 holds in both themes.

An address wider than the pane is **middle-elided** in the drawn label. The model's `display`
string, which is what SC-006 compares with the address opened, stays complete. The confirmation
dialog (FR-018a) and "Copy Link Address" carry the full text.

The underline is drawn for exactly the link's cells using the cell's own foreground colour, so
text colour, cursor and selection highlight are not hidden (FR-009). The pointer becomes
`mouse::Interaction::Pointer` (`mouse_interaction`, `terminal_pane.rs:1124`) only while the link
modifier (`modifiers.command()`) is also held, so it signals exactly when a click would open the link
(clarification 2026-09-16). Under mouse reporting the underline appears only while Shift is held, and
the pointer only while Shift and the link modifier are, so hover is also recomputed on
`ModifiersChanged`.

**Rejected.** `Tooltip`: it is delayed, anchored to a widget and a refresh late. A status field in
the session chrome: a message round trip per pointer move, a layout-snapshot change, and ambiguity
with split panes. A new overlay surface: a new component and gate for a label.

---

## R8 — Which addresses are followable, and what they open (FR-011, FR-012, FR-018)

**Finding**: nothing in the workspace parses URLs. `percent-encoding` 2.3.2 and `gethostname`
1.1.0 are in `Cargo.lock` only as transitive dependencies, and `gethostname` only through `x11rb`
on Linux.

**Decision**: `micold_core::link::address::classify(uri) -> Address`, a pure function:

- `http://…` and `https://…` become `Address::Web(uri)`, and `mailto:…` becomes
  `Address::Mail(uri)`. They are passed on verbatim, never re-encoded.
- `file://host/path` becomes `Address::File { host, path }`. The path is **percent-decoded** by a
  small hand-written decoder, about 20 lines. Invalid escapes or non-UTF-8 bytes make it not
  followable. Adding `percent-encoding` for one function would not pay.
- Anything else becomes `Address::NotFollowable`. Scheme comparison is ASCII case-insensitive.

`micold_core::link::resolve(link: Link, ctx: &LinkContext) -> Option<ResolvedLink>` then applies FR-012 and FR-018.
`LinkContext` carries the host's hostnames, whether the host is Windows, and for a sandboxed session
the sandbox's hostnames and its shared locations. It performs **no** I/O and **no** DNS lookup
(FR-012, FR-019):

- The host part is accepted when it is empty, `localhost`, or equals a hostname in the context,
  compared ASCII case-insensitively. The comparison includes both the full hostname and its first
  DNS label, since `ls --hyperlink` prints what `gethostname()` returns, which may be either form.
  Any other host makes the link not followable. So does a UNC form, whose `server` lands in the host
  part and fails the check.
- Host is Windows and the session is not sandboxed: `/C:/Users/u/a.txt` becomes `C:\Users\u\a.txt`,
  with forward slashes turned into backslashes. On a Windows host a path without a drive letter is
  not followable.
- Session is sandboxed: the path is a container path. It is translated by R10's reverse map. If no
  shared location contains it, the result is `Unreachable`, and FR-008 has the hint say so.

The host's hostname comes from **`gethostname` 1.1** as a direct client dependency. It is already
vetted into `Cargo.lock`, it is portable, and it keeps `cfg` out of this code. On Linux it is
already compiled (through `x11rb`), while macOS and Windows compile it for the first time. It is a
small crate with no build script. It is read once when
the pane's `LinkContext` is built.

**Rejected.** The `url` crate: WHATWG normalisation re-encodes addresses, so what opens would differ
from what the hint displays (SC-006). A DNS check of "is this host me?": FR-012 forbids network.

---

## R9 — Runnable files (FR-013, SC-007)

**Finding**: the classification depends on the platform and on the file. Principle VI forbids core
logic that branches on the host OS directly. `pathmap::map_for(host, windows_host)` shows the
codebase's answer: the platform is a **parameter** (`sandbox/pathmap.rs:39`), so Linux CI exercises
all three arms.

**Decision**: `micold_core::link::runnable::action_for(platform: HostPlatform, name: &str, facts:
FileFacts, pathext: &[String]) -> FileAction { Open, Reveal }` is pure. `name` is the final path
component. `FileFacts { kind: File | Dir,
any_exec_bit: bool, is_bundle: bool }` and `pathext: &[String]` are gathered by the shell right
before opening. The extension lists are FR-013's, verbatim, compared ASCII case-insensitively on
the final extension. A macOS directory is a bundle when its extension is `.app`, `.bundle`,
`.framework`, `.plugin`, `.kext`, `.prefPane`, `.appex` or `.xpc`, or when it contains
`Contents/Info.plist`. A bundle's action is `Reveal`. On Windows, `any_exec_bit` is always false and
`PATHEXT` is read from the environment at activation. Files are judged through symbolic links
(`std::fs::metadata` follows them), which is FR-013's "judged by the file it points to". A
filesystem mounted with every file executable yields `Reveal` for every file, as the spec accepts.

The facts are gathered, classified and acted on **in one blocking task**, back to back. That keeps
the window between check and open to the time of two syscalls.

`std::os::unix::fs::PermissionsExt` needs a `#[cfg(unix)]` arm in the fact gatherer in
`shell/links.rs`, with a `#[cfg(windows)]` arm returning `false`. It sits beside the other platform
reads in the shell (plan, Constitution Check VI).

**Rejected.** Judging by extension alone on Unix: a script named `build` with no extension would
open in an editor, or worse, run through a handler. Asking the desktop's MIME database whether a
type is executable: a different answer per desktop, and a process spawn per hover. Checking in core
with `std::fs`: I/O in core, against Principle I's render-free and deterministic core.

---

## R10 — Sandboxed sessions: shared locations and the sandbox hostname (FR-012, FR-018, FR-018a)

**Finding**: the sandbox is per daemon, not per session. `App.placement.kind` is `HostProcess` or
`LocalSandbox` (`main.rs:123`). Every session of a sandboxed daemon runs in the container. The
container's bind mounts are the `MountSet` built in `shell/sandbox.rs:123-133`, with projects, state,
home, secret and credentials (`micold-core/src/sandbox/mod.rs:434-445`). It is built at bring-up
and then dropped. `BootPlan.projects` can later move ahead of the running container, which is
`adopt_mount_set` in `daemon_sync.rs:394` marking it stale. The container is created with no
`--hostname` (`sandbox/argv.rs:99-153`). Docker and Podman then default the hostname to the first 12
characters of the container ID, which the client holds as
`SandboxState::Running(ContainerId)` or `Stale(ContainerId)`
(`sandbox/lifecycle.rs:37,105`). `pathmap::map_for` maps host to container only.

**Decisions**:

1. **Keep only locations the running container really mounts.** `bring_up` builds this client's
   `MountSet` on every start, but `adopt` (`lifecycle.rs:192`) attaches to, or starts, an existing
   container by image and fingerprint alone. That container may have been created with other
   projects. So:
   - `ContainerFacts` (`sandbox/parse.rs:103`) gains `mount_destinations: Vec<String>`, parsed from
     the inspect document's `Mounts[].Destination`, which Docker and Podman both emit. A missing
     `Mounts` parses as empty.
   - `lifecycle::Started` gains `mounted: Option<Vec<String>>`: `None` when this bring-up created
     the container from the spec (`Create`, `Replace`), `Some(facts.mount_destinations)` when it
     attached or started an existing one.
   - `MountSet::shared_locations(mounted)` keeps a location only when `mounted` is `None` or lists
     its container path. A project the container does not mount is then not a shared location, and
     its `file` links resolve as `Unreachable`. Container destinations are enough: every host source
     is derived from its destination and this client's state directory, so a matching destination
     names the same host path.
   - `shell::sandbox::start` returns `Ready { started, locations }`; `boot()` keeps both instead of
     `.map(|ready| ready.started)` (`shell/sandbox.rs:196`); `SandboxMsg::Started` carries
     `Box<(Started, SandboxLocations)>` (`features/sandbox.rs:41`); and `Sandbox::started` stores them.
     `Sandbox::locations()` returns them only while `Running` or `Stale`, and `for_placement` starts
     without them, so a placement change or a stop can never leave stale locations applied to host
     sessions. That is the true map for the running container, stale
     or not. Rebuilding it from `BootPlan` would describe the *next* container.
2. **`shared_locations` lists projects, state, home and credentials, and excludes the secret mount.**
   It is sorted by container-path component count, descending, so the most specific location wins.
   That covers credentials nested inside home. Excluding the secret mount is not enough on its own:
   the host token is `state_dir/sandbox.token` (`protocol/auth.rs:114`), and `state_dir` is itself
   mounted at `/var/lib/micold-ai-ide`. So `SandboxLinkContext.denied` holds the secret mount's host
   path, and `reverse` rejects a result equal to or under any denied path (C16b).
3. **`pathmap::reverse(locations, denied, container_path, windows_host) -> Option<String>`** normalises the
   container path lexically. It rejects any `..` that would climb above `/`, so `/project/../etc`
   cannot escape into a shared prefix. It matches whole path components and re-joins the remainder
   with the host's separator, given as a parameter as `map_for` does. It is pure, and it is tested
   with Windows host pairs on Linux CI.
4. **Sandbox hostnames** are `id.get(..12).unwrap_or(id)` and the full `id`, as
   `LinkContext.sandbox.host_names`.
5. **Confirmation**: every resolved `file` target from a sandboxed session carries
   `needs_confirmation = true` (FR-018a). It is shown through a floating confirm surface built the
   way `confirm_session_remove` is (`features/session.rs:769-795`), naming the host path. The
   pending open lives in `features::session` state as `PendingLinkOpen { session, link }`. The
   sandbox state lives on the binary's `App`, which the reducer cannot read, so `shell/links.rs`
   calls `session::link_open_confirmed(state, sandbox_live)` when the dialog's `LinkOpenConfirmed` arrives. The reducer then checks the session still exists and
   `sandbox_live`. Otherwise it emits a notification and opens nothing (edge case "Pending opens").

**Rejected.** Storing this client's whole `MountSet` on attach: it can list a project the container
does not mount, and on Linux and macOS a container path equals its host path, so a container-local
file would open as a same-named host file. Storing nothing on attach: a sandbox outlives the
application, so almost every start attaches and file links would never work. Not sharing the state
directory: feature 027's daemon needs it. Setting `--hostname` on the container: that changes feature 027's container contract
and would need a runtime migration, for a value the client already knows. Translating by string
prefix without normalisation: `/project/../../etc/passwd` would pass.

---

## R11 — Opening with the operating system (FR-010, FR-013, FR-015, FR-019, SC-003)

**Finding**: the client assembles every external capability in `shell/capabilities.rs:87-120`.
`tests/no_concrete_implementations.rs` enforces that no concrete implementation is named elsewhere.
`features::Outcome` is how a reducer asks the shell to act (`features/mod.rs:36`). The client
depends on neither `libc` nor `windows-sys` today; both are workspace dependencies already used by
core and daemon.

**Decision**: a `LinkOpener` capability.

```text
trait LinkOpener { fn open(&self, target: &str) -> Result<(), OpenFailure>;
                   fn reveal(&self, path: &Path) -> Result<(), OpenFailure>; }
```

`SystemLinkOpener` is the real implementation, with one `cfg` arm per platform in
`shell/link_opener.rs`. Every argument is passed as **one argv element, never through a shell**.

| Platform | Open (URL, document, folder) | Reveal |
|---|---|---|
| Linux | `xdg-open <arg>` with null stdio, waited for within a 2 s launch window. Exit status 3 or 4 inside the window means "no application" or "launch failed"; still running at its end means launched | `dbus-send --session --dest=org.freedesktop.FileManager1 --type=method_call /org/freedesktop/FileManager1 org.freedesktop.FileManager1.ShowItems array:string:<file-uri> string:""`. If that fails, `xdg-open <parent dir>` (FR-013's fallback) |
| macOS | `open <arg>`, same launch window. A non-zero exit inside it means no application or a failure | `open -R <path>` |
| Windows | `ShellExecuteW(NULL, "open", arg, …)` through `windows-sys` (`Win32_UI_Shell`). A return ≤ 32 is a failure, and `SE_ERR_NOASSOC` means no application | `explorer.exe` with `raw_arg("/select,\"<path>\"")` (Rust's quoting of a spaced argument would wrap `/select,` too) |

`open` returns once the handler is launched. `xdg-open` usually does too, but not in its generic
mode (no recognised desktop: Xvfb, many tiling window managers). There it runs the handler in the
foreground and may fall back to a text browser, so its exit status is the handler's and it may
never return. Hence null stdio and a bounded launch window: the request reaches the OS at spawn,
which is what SC-003 measures, and a handler still running when the window ends counts as launched. The call runs through `Task::perform` over `tokio::task::spawn_blocking`, never on
the render thread. Its result comes back as `SessionMsg::LinkOpenFinished`, and a failure becomes
`notify_error("Couldn't open <address>: <reason>")` (`app.rs:248`). The user is also told when the
file does not exist (`std::fs::metadata` → `NotFound`) or the path is unreachable. Nothing is
fetched, previewed or validated over the network (FR-019). A web address is handed over as text.

**Rejected.**

- The `open` crate: on Windows it goes through `cmd /c start`, where `&` in a URL is a command
  separator.
- The `opener` crate: its Linux `reveal` pulls a D-Bus client library into the build, and it would
  be a new crate to vet.
- `webbrowser`: http(s) only.
- `explorer.exe <url>` for opening on Windows: no failure signal, since explorer's exit status is
  always 1.

---

## R12 — Sessions must not advertise hyperlinks (FR-006)

**Finding**: portable-pty 0.9 `CommandBuilder::new` starts from the daemon's environment and has
`env_remove`. The daemon inherits the client's environment (`micold-core/src/spawn.rs:73-86`), so a
client started from WezTerm or VS Code passes `TERM_PROGRAM` to every session. Both spawns apply
`spec.env` (include-script values) after `CommandBuilder::new` (`micold-daemon/src/supervisor.rs:181-184,
216-219`). The include script's values are computed by `diff_env` against a baseline shell's
environment, which drops values equal to the baseline (`micold-core/src/env_include.rs:63`). The
baseline and attempt shells start from the daemon's environment too (`env_include.rs:295-377`).
Claude Code 2.1.270 decides in this order: attacher capabilities (internal), `FORCE_HYPERLINK`,
the `TERM_PROGRAM` list, `TERMINAL_EMULATOR=JetBrains-JediTerm`, then `WT_SESSION`. The Rust
`supports-hyperlinks` crate additionally reads `VTE_VERSION`, `KONSOLE_VERSION`, `DOMTERM` and
`COLORTERM=xfce4-terminal`.

**Decision**:

1. `micold_core::env_include::is_inherited_terminal_identity(key, value, keys_case_insensitive) -> bool`
   is pure and tested. It is true for `FORCE_HYPERLINK`, `TERM_PROGRAM`, `TERM_PROGRAM_VERSION`,
   `VTE_VERSION`, `WT_SESSION`, `WT_PROFILE_ID`, `TERMINAL_EMULATOR`, `KONSOLE_VERSION` and
   `DOMTERM`. It is also true for `COLORTERM` **only** when its value is not `truecolor` or `24bit`,
   so colour detection is unaffected.
2. The **daemon strips them at spawn**. A shared `strip_inherited_terminal_identity(&mut
   CommandBuilder)` runs **before** `spec.env` is applied, in both `spawn_ai_cli` and `spawn_shell`.
   An include-script value then overrides the removal (FR-006's opt-in). It is the same code on the
   host and in the container, so every session gives the same answer.
3. The **baseline and attempt shells strip them too** (`Command::env_remove` at each of the three
   `Command` construction sites: the Unix `bash` builder, which `baseline_env` reuses from
   `attempt_env`, and the two Windows `powershell.exe` builders). Without this, a daemon that inherited
   `FORCE_HYPERLINK=1` would drop the user's `FORCE_HYPERLINK=1` from the diff as "unchanged", and
   step 2 would remove it. The opt-in would silently fail for exactly the users who had it already.

**Rejected.** Stripping in the client before it spawns the daemon: a daemon started at login, or by
a previous client, keeps the old environment, and so would a sandbox. Setting `TERM_PROGRAM=micold`:
that is advertising in all but name, and the user declined it.

---

## R13 — Declared hyperlinks on Windows (spec Assumptions, edge case "Platforms whose terminal layer drops declared hyperlinks")

**Finding**: portable-pty uses the inbox ConPTY, without `PSEUDOCONSOLE_PASSTHROUGH` and without a
bundled `conpty.dll`. Whether inbox ConPTY re-emits OSC 8 depends on the Windows build, and this
machine cannot run Windows.

**Decision**: verify it on CI instead of assuming. The new daemon integration test
`osc8_passthrough.rs` spawns the test binary itself through the real PTY supervisor, as a child
whose only job, gated by an env flag, is to print one OSC 8 link. It asserts that the resulting grid
cell carries the URI. The test runs on all three CI platforms. If the Windows runner shows ConPTY
drops the sequence, the Windows arm becomes `#[cfg_attr(windows, ignore = "…")]`, citing the run.
The finding is recorded here and in the user guide, and the spec's degradation edge case applies:
detected plain-text links still work there. That is not a requirement change, because the spec
already states the fallback.

**Rejected.** Assuming passthrough from documentation: the behaviour differs across Windows builds.
Bundling `conpty.dll`/`OpenConsole.exe` for `PSEUDOCONSOLE_PASSTHROUGH`: a binary to ship, sign and
update, in feature 012's PTY layer, for a gap the spec's fallback already covers. winpty: deprecated,
and it drops more escape sequences than ConPTY.

---

## R14 — The right-click menu (FR-020, FR-021, US4)

**Finding**: a right press emits `TerminalContextMenuOpened { x, y }` (`terminal_pane.rs:998`). The
menu is built in `features/session.rs:217` and rendered in `ui/terminal.rs:376-398`. Copying goes
through `Outcome::ClipboardWrite` (`features/mod.rs:42`). `tests/gates/context_menu_anchor.rs` and
`tests/context_menu_anchor_call_sites.rs` guard the anchor.

**Decision**: `TerminalContextMenuOpened` gains `link: Option<ResolvedLink>`, resolved from the
grid at the press (FR-017), and the session state keeps it while the menu is open. Two items, "Open
Link" and "Copy Link Address", appear first, and only when `link` is `Some`. There is no divider,
because `MenuItem` has no separator variant and `menu_panel_size` sizes the panel by item count,
which the anchor gates rely on.
"Open Link" dispatches the same `LinkActivated` path as the gesture. "Copy Link Address" emits
`Outcome::ClipboardWrite(link.address)`: the declared URI, or the detected text after punctuation
trimming, unwrapped. Under mouse reporting the right press already needs Shift to reach the
terminal, so the menu inherits that.

**Rejected.** A "Link" submenu: two actions do not need one, and submenus are a new menu component.
Adding a `MenuItem` separator: a component change with its own showcase and sizing gates, for
grouping two items. Resolving the link when an item is chosen rather than at the press: FR-017
requires the link under the pointer when the menu opened.

---

## R15 — Test layers

Principle I's glue exception covers `main.rs`, `src/ui/` and `src/showcase/` only. The pane is in
`src/ui/material/`, but its decisions (gesture, hint placement, hover resolution) are pure
functions with tests in the pane's own `#[cfg(test)]` module, as `press_routing` and `grid_at`
already are (`terminal_pane.rs:1244, 2141`).

| Layer | What |
|---|---|
| `micold-core` unit (`mise run test-core`) | detect, logical line, declared runs, address classify, resolve, percent-decode, runnable (3 platforms), pathmap reverse (Windows pairs on Linux), identity predicate |
| `micold-core/tests/link_corpus.rs` + `tests/fixtures/link_corpus.txt` | SC-002 over ≥ 50 real lines |
| `micold-daemon/tests/` | identity stripped at spawn, include value survives (`session_identity_env.rs`); OSC 8 through the real PTY (`osc8_passthrough.rs`) |
| client pane tests (`terminal_pane.rs` test module) | `link_gesture` table, SC-004 scripted 100 selections, hint rect, hover resolution follows output, pointer interaction |
| client `tests/` | reducers: activation → outcome, confirm/decline/pending-gone, menu items present/absent, copy address; shell: `shell/links.rs` with a recording `LinkOpener` fake, including failure → notification |
| `shell/links.rs` `#[cfg(test)] mod acceptance` | outer loop A1–A22: a laid-out headless `TerminalPane`, pane events in, messages through `crate::update_inner`, a recording `LinkOpener` and clipboard (integration of composed modules, not end-to-end) |
| `tests/no_concrete_implementations.rs` | `SystemLinkOpener` is named in client `src/` only at its definition and in `Capabilities::real()` |
| `showcase/samples.rs` test module (new) | showcase terminal sample shows a detected and a declared link |
| quickstart §B (visual-pass skill) | underline and hint legibility in both themes; real browser, mail client, document, folder and reveal on Linux; macOS and Windows arms by CI build plus `cargo check --target aarch64-apple-darwin` |

**Rejected.** Driving the gesture through an iced runtime or a rendered window: iced has no
event-injection harness in this workspace. The acceptance module instead calls the pane's pure
functions over `PaneState` directly and feeds their messages to `update_inner`. Snapshot-testing the underline and hint pixels: the geometry is asserted
through `link_hint_rect` and the spans, and legibility needs eyes (quickstart §B).

---

## Summary of decisions

| # | Decision |
|---|---|
| R1 | No protocol change; hyperlinks and wrap flags already reach the client |
| R2 | Links computed only for the pointer's logical line, cached per session, context, cell and a hash of the consulted rows |
| R3 | Hand-written scanner in `micold_core::link::detect`; no regex/linkify |
| R4 | Logical line joins soft-wrapped rows only, capped at 64 rows each way |
| R5 | Declared link = maximal same-URI run; declared wins over detected |
| R6 | Pure `link_gesture` state machine; activation on motionless release; `Modifiers::command()` |
| R7 | Pane-drawn link hint with pure placement; middle-elided when wider than the pane |
| R8 | Pure `classify` + `resolve` with hostnames from `gethostname`; no `url` crate, no DNS |
| R9 | Pure platform-parameterised `runnable::action_for`; facts gathered in one blocking task |
| R10 | Share only the locations the running container mounts (inspected on attach); `pathmap::reverse`, secret excluded, token path denied; hostname = container id prefix; confirm dialog |
| R11 | `LinkOpener` capability; xdg-open / dbus-send, open / open -R, ShellExecuteW / explorer /select |
| R12 | Strip inherited identity at spawn and in include-script shells, before include values apply |
| R13 | OSC 8 through ConPTY verified by a CI integration test on all three OSes |
| R14 | Context menu captures the link at open; two items only over a link |
| R15 | Core unit + corpus, daemon integration, pane pure-function tests, client reducers/shell, quickstart §B |

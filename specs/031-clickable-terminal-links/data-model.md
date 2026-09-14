# Data Model: Clickable Links in the Terminal

**Feature**: 031-clickable-terminal-links | **Plan**: [plan.md](./plan.md) |
**Research**: [research.md](./research.md)

Nothing here is persisted (spec Assumptions: "nothing about links is saved") and nothing crosses the
wire (research R1). Every value is derived from the grid under the pointer, from the app's placement
and sandbox state, or from the filesystem at the moment of opening.

---

## 1. Core values (`micold_core::link`, render-free)

### `LinkRows` — the view core reads rows through (R4)

A trait, not a struct, so the client lends `GridCache` lines without copying. Rows are addressed
relative to the viewport's top line (`i64`, negative in scrollback):

| Method | Returns | Meaning |
|---|---|---|
| `text(row)` | `Option<&str>` | One `char` per cell, wide-char spacer cells included (as `CachedLine.text`); `None` when the line is not cached |
| `wrapped(row)` | `bool` | This row soft-wraps into the next |
| `hyperlink(row, col)` | `Option<&str>` | Declared URI at a cell (`CachedExtra.hyperlink`) |
| `spacer(row, col)` | `bool` | The cell holds no char of its own: its style run carries `WIDE_CHAR_SPACER` or `LEADING_WIDE_CHAR_SPACER`. The wire's spacer char is a space, so `text` alone cannot tell |

### `Link` — a followable span under the pointer

| Field | Type | Meaning |
|---|---|---|
| `address` | `String` | What opens: the declared URI, or the detected text after trimming (FR-005, FR-021) |
| `origin` | `LinkOrigin { Detected, Declared }` | How it was found (spec Key Entities) |
| `cells` | `Vec<CellSpan { row: i64, cols: Range<u16> }>` | One span per row the link covers, relative to the viewport top; the pane underlines the spans inside the viewport (FR-007) |

Identity: two `Link`s are the same link when `address`, `origin` and `cells` are equal. That is what
lets US2 scenario 4's two same-address runs be two links.

### `Address` — the classified address type (FR-011, R8)

```text
Address = Web(String)                    // http, https — verbatim
        | Mail(String)                   // mailto — verbatim
        | File { host: String, path: String }   // path percent-decoded
        | NotFollowable
```

### `LinkContext` — what resolution may consult, all known without I/O (R8, R10)

| Field | Type | Meaning |
|---|---|---|
| `host_names` | `Vec<String>` | This machine's hostname and its first DNS label |
| `windows_host` | `bool` | The client runs on Windows (a parameter, not a `cfg`) |
| `sandbox` | `Option<SandboxLinkContext>` | `Some` iff `Sandbox::locations()` is `Some` (§2) |

`SandboxLinkContext { host_names: Vec<String>, locations: Vec<SharedLocation>, denied: Vec<String> }`. `host_names`
holds the container id's 12-character prefix (`id.get(..12).unwrap_or(id)`) and the full id. `locations` is most-specific first
and excludes the secret mount and every location the running container does not mount. `denied`
holds the host token path, which `reverse` never returns even through the state mount (R10).

`SharedLocation { container: String, host: String }` is the spec's *Shared location* entity.

### `ResolvedLink` — what the hint displays and what an activation carries (FR-008, SC-006)

| Field | Type | Meaning |
|---|---|---|
| `link` | `Link` | As above |
| `display` | `String` | Exactly what the hint shows: the address, or for a sandboxed file the host path, or `"<path> — not reachable from this machine"` |
| `target` | `Target` | See below |
| `needs_confirmation` | `bool` | `true` iff the session is sandboxed and `target` is `HostPath` (FR-018a) |

```text
Target = Url(String)            // Web or Mail: handed to the OS as-is
       | HostPath(PathBuf-as-String)   // a file or folder on this machine, not yet checked
       | Unreachable(Reason)    // a sandboxed file path outside every shared location

Reason = NotShared              // the only reason today; a variant, so the notification text is chosen by match
```

`Target` and `Reason` derive `Clone, Debug, PartialEq, Eq`.

`resolve` returns `Option<ResolvedLink>`: `None` for anything not followable (FR-011, FR-012). The pane
neither marks nor activates a `None` (US3 scenario 6).

**Invariant (SC-006)**: for `Url(u)`, `display == u`. For `HostPath(p)`, `display == p`. The opener
receives exactly `u` or `p`. `Unreachable` arises only from a sandboxed `file` link. It is marked and
its hint says it is not reachable (FR-008), and activating it notifies (FR-015).

### `FileFacts` and `FileAction` — runnable classification (FR-013, R9)

`FileFacts { kind: Kind{File, Dir}, any_exec_bit: bool, is_bundle: bool }` together with
`HostPlatform { Linux, MacOs, Windows }` and `pathext: Vec<String>` feed a pure function whose output
is `FileAction { Open, Reveal }`.

---

## 2. Client state

### Pane widget state (`ui/material/terminal_pane.rs` `PaneState`, per pane, not in `App`)

| Field | Type | Meaning |
|---|---|---|
| `hover` | `Option<HoverCache { session: SessionId, context: LinkContext, cell: (u16, u16), grid_version: (u64, u64), rows_hash: u64, resolved: Option<ResolvedLink> }>` | The pointer's last resolved link. Reused while the session, context, cell and the hash of the consulted rows are unchanged; the hash is recomputed only when `grid_version` (generation, seq) moves (R2) |
| `link_press` | `Option<LinkPress { link: ResolvedLink, cell: (u16, u16) }>` | A modifier-press on a link awaiting release (R6) |

Being widget-local state is what scopes hover and activation to the pane under the pointer (FR-022).

### Session feature state (`features/session.rs`)

| Field | Type | Meaning |
|---|---|---|
| `menu_link` | `Option<ResolvedLink>` | The link captured when the context menu opened; cleared when it closes (FR-017, FR-020) |
| `pending_link_open` | `Option<PendingLinkOpen { session: SessionId, link: ResolvedLink }>` | A sandboxed file open awaiting confirmation. Its presence *is* the confirm surface being shown, mirroring `remove_target` |

### Where the context comes from

`ui::view` already receives `state: &app::State` and `sandbox: &features::sandbox::Sandbox`, so both
new values live there and its signature does not change:

| Owner | Field | Type | Meaning |
|---|---|---|---|
| `features::sandbox::Sandbox` | `locations` | `Option<SandboxLocations { shared: Vec<SharedLocation>, denied: Vec<String> }>` | Set by `started(started, locations)`. `locations()` returns it only while the state is `Running` or `Stale`; every other transition, and `for_placement` (which `shell/persist.rs::apply_placement` calls), drops it (R10) |
| `app::State` | `host_names` | `Vec<String>` | Read once at boot through `gethostname` in `main.rs`; empty by `Default` in tests |

`ui/terminal.rs::pane` builds the `LinkContext`: `host_names` from `state`, `windows_host` from
`cfg!(windows)`, and `sandbox = Some` **iff** `sandbox.locations()` is `Some`, which holds only while
the placement is `LocalSandbox` and the container is `Running` or `Stale`.

---

## 3. Messages and transitions

`TerminalPane` publishes `Message::Session(..)` directly, as it does `TerminalContextMenuOpened`
today. Reducers live in `features/session.rs`, and I/O lives in `shell/links.rs`.

**How an effect reaches the shell.** `app::State::update` sends a session reducer's outcomes to
`app::interpret`, whose `ClipboardWrite` arm does nothing, and whose `OpenLink` arm will do nothing
too. So the link messages never take that path. `main.rs::update_inner` matches `LinkActivated`,
`LinkOpenConfirmed`, `ContextMenuOpenLink` and `ContextMenuCopyLinkAddress` **before** the general
`Message::Session` arm, the way it already routes `RemoveConfirmed` to `shell::daemon_sync`, and
calls `shell::links::on_link_message(app, msg)`. That handler runs the reducer, then splits its
outcomes: `OpenLink` goes to `shell::links::perform`, `ClipboardWrite` to
`shell::clipboard::interpret`, and every other outcome through `app::drain`/`app::interpret`.

| # | Message | From state | To state / effect |
|---|---|---|---|
| T1 | *(pane-internal)* cursor moved to a cell, `ModifiersChanged`, or `window::Event::RedrawRequested` whose grid `(generation, seq)` differs from the cached one (`draw` gets `&Tree` and cannot update state, so `update` does it) | any | recompute `hover` if the session, the context, the cell, the consulted rows' hash or (under mouse reporting) Shift changed; redraw if `resolved` changed (FR-007, FR-008, FR-016, SC-005) |
| T2 | *(pane-internal)* left press, `command()`, routing = terminal, single click, `hover.resolved` is `Some` | `link_press = None` | `link_press = Some`; `last_click` updated through `Click::new`; no `TerminalSelectStart` (FR-004, G3b) |
| T3 | *(pane-internal)* cursor moved off the press cell | `link_press = Some` | `link_press = None`; emit `TerminalSelectStart` at the press cell (FR-004) |
| T4 | left release on the press cell | `link_press = Some` | re-resolve under the pointer (FR-017); if `Some`, emit `SessionMsg::LinkActivated(resolved)`; `link_press = None` |
| T5 | `LinkActivated(r)`, `r.target = Url(u)` | — | `Outcome::OpenLink(OpenRequest::Url(u))` |
| T6 | `LinkActivated(r)`, `r.target = HostPath(p)`, `!needs_confirmation` | — | `Outcome::OpenLink(OpenRequest::Path { path: p, address: r.link.address })` |
| T7 | `LinkActivated(r)`, `needs_confirmation` | `pending_link_open = None` | `pending_link_open = Some { active session, r }`; confirm surface shows `r.display` (FR-018a) |
| T8 | `LinkActivated(r)`, `r.target = Unreachable` | — | notification "Couldn't open `r.link.address`: the sandbox doesn't share that location with this machine" (FR-015) |
| T9 | `LinkOpenConfirmed` (the dialog's Open); `shell/links.rs` reads `App.sandbox` and calls `session::link_open_confirmed(state, sandbox_live)` | `pending_link_open = Some(p)` | if `p.session` is still in `state.workspace.sessions` and `sandbox_live`: T6 with `p.link`; else notify "the session has closed" / "the sandbox has stopped". Then `pending_link_open = None` |
| T10 | `LinkOpenDeclined` | `pending_link_open = Some` | `pending_link_open = None`; nothing opens |
| T11 | *(shell)* `Outcome::OpenLink(Path { path, address })` | — | one blocking task: `metadata(path)` → `NotFound` ⇒ `LinkOpenFinished { address, result: Err(NotFound) }`; else `action_for` ⇒ `opener.open` or `opener.reveal` (FR-010, FR-013). `Url(u)` calls `opener.open(u)` with `address = u` |
| T12 | `LinkOpenFinished { address, result: Err(e) }` | — | `notify_error("Couldn't open <address>: <e>")` (FR-015) |
| T13 | `LinkOpenFinished { result: Ok(()), .. }` | — | nothing |
| T14 | `TerminalContextMenuOpened { x, y, link }` | — | today's effect plus `menu_link = link` |
| T15 | `ContextMenuOpenLink` | `menu_link = Some(r)` | as `LinkActivated(r)`; menu closes (FR-020) |
| T16 | `ContextMenuCopyLinkAddress` | `menu_link = Some(r)` | `Outcome::ClipboardWrite(r.link.address)` (FR-021) |
| T17 | context menu closed | any | `menu_link = None` |
| T18 | `SandboxMsg::Started` with `(started, locations)` (`shell/sandbox.rs::update` → `Sandbox::started`) | — | `locations = Some(..)`. Created: every shared location. Attached or started: only those in `started.mounted` |
| T19 | the sandbox leaves `Running`/`Stale` (stopped, failed, lost, fallback), or the placement changes (`apply_placement` → `Sandbox::for_placement`) | — | `locations()` is `None`, so `LinkContext.sandbox` is `None` |

```text
OpenRequest       = Url(String) | Path { path: String, address: String }
LinkOpenFinished  { address: String, result: Result<(), OpenFailure> }   // OpenFailure as in contract link-opening §2
```

T1–T4 are tested in the pane's test module, T5–T10 and T14–T17 in `tests/features_session_links.rs`,
T11–T13 and the `sandbox_live` read in `shell/links.rs`'s test module, and T18–T19 in
`features/sandbox.rs`'s own tests (`Sandbox::locations` across every state transition and `for_placement`). The route itself is tested in `shell/links.rs`'s test module by
calling `crate::update_inner` on a `base_app()` with a recording `LinkOpener`: `LinkActivated` of a
URL reaches `open`, and `ContextMenuCopyLinkAddress` returns the clipboard write task rather than
dropping it.

Exactly one message is emitted per release (T4), so a single activation opens once.

---

## 4. Wire

No change. `PROTOCOL_VERSION` stays 12 and `SCHEMA_HASH` is unmoved (research R1). The daemon-side
change is to session **environment** at spawn (contract [session-terminal-identity.md](./contracts/session-terminal-identity.md)),
which is not a protocol type.

---

## 5. Validation rules

| Rule | Source |
|---|---|
| Only `http`, `https`, `mailto`, `file` classify to anything other than `NotFollowable` | FR-011 |
| A detected candidate needs an explicit scheme prefix not preceded by `[A-Za-z0-9]` | FR-001 |
| Trailing `.,;:!?'*` trimmed; closing bracket kept only if balanced inside; enclosing quote excluded | FR-005 |
| A web host must be `localhost`, dotted, a bracketed IPv6 literal, or carry a port | spec Edge Cases (malformed) |
| Rows join only across `wrapped = true`, at most 64 rows each way | FR-003, R4 |
| A declared link is a maximal run of equal URIs over the logical line; declared wins | FR-002, FR-007 |
| A `file` host must be empty, `localhost`, or in `host_names` / `sandbox.host_names` (ASCII case-insensitive) | FR-012 |
| Percent-decoding failure or non-UTF-8 ⇒ `NotFollowable` | Edge Cases (encoded paths) |
| A sandbox path is normalised lexically; climbing above `/` ⇒ `Unreachable` | FR-018, R10 |
| The secret mount is never a shared location, and the host token path is never a `reverse` result | R10 |
| A location the running container does not mount is not shared | R10 |
| A file judged runnable for the platform ⇒ `Reveal`, never `Open` | FR-013 |

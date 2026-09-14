# Contract: Link opening

**Feature**: 031-clickable-terminal-links | **Modules**: `crates/micold-client/src/shell/links.rs`
(new), `shell/link_opener.rs` (new), `shell/capabilities.rs`, `micold_core::link::runnable` |
**Research**: R6, R9–R11, R14

## 1. The gesture (pane → session feature)

| # | Input | Output |
|---|---|---|
| G1 | left press + release on one cell, `Modifiers::command()` held at press, pointer over a link, routing = terminal | exactly one `SessionMsg::LinkActivated(ResolvedLink)`, resolved at release |
| G2 | the same press, then the pointer leaves the cell | no activation; a selection starts at the press cell |
| G3 | double or triple click without the modifier | selection as today; no activation |
| G3b | double or triple click with the modifier, on a link | the first press + release is G1 and opens once. The press records `last_click` through `Click::new`, as a plain press does, so the second and third presses are double and triple: they select and never activate. The open is not delayed to wait for them (FR-004) |
| G4 | plain left press (no modifier) | today's focus and selection; no activation |
| G5 | mouse reporting on, no Shift, any modifier | the press is reported to the program as today; no activation |
| G6 | mouse reporting on, Shift + `command()`, press + release on a link | as G1 |
| G7 | middle click, wheel | as today |
| G8 | unfocused pane, G1 | focus as today, and G1 |
| G9 | the link under the pointer changed between press and release | the link at release opens; if there is none, nothing |

No activation writes to the PTY, changes the selection or scrolls (FR-014).

## 2. The capability

```text
pub trait LinkOpener: Send + Sync {
    fn open(&self, target: &str) -> Result<(), OpenFailure>;   // a URL, or a host path
    fn reveal(&self, path: &Path) -> Result<(), OpenFailure>;
}
pub enum OpenFailure { NoApplication, LaunchFailed(String), NotFound }   // NotFound: set by shell/links.rs, never by an opener
```

`Capabilities::real()` is the only place `SystemLinkOpener` is named
(`tests/no_concrete_implementations.rs`). Tests use a recording fake.

| Platform | `open` | `reveal` |
|---|---|---|
| Linux | `xdg-open <target>` with null stdio, waited for at most 2 s (**launch window**). Exit 3 inside the window → `NoApplication`, other non-zero inside it → `LaunchFailed`, zero → success; still running at the end of the window → success, and the child is left to run (xdg-open's generic mode runs the handler in the foreground) | `dbus-send … org.freedesktop.FileManager1.ShowItems array:string:file://<path> string:""`; on failure `open(<parent>)` |
| macOS | `open <target>` with null stdin/stdout, same launch window; non-zero inside it → `NoApplication` when stderr names no application, else `LaunchFailed` | `open -R <path>` |
| Windows | `ShellExecuteW(null, "open", target, null, null, SW_SHOWNORMAL)`; `SE_ERR_NOASSOC`/`SE_ERR_ASSOCINCOMPLETE` → `NoApplication`, other ≤ 32 → `LaunchFailed` | `explorer.exe` with `CommandExt::raw_arg(format!("/select,\"{path}\""))`, because Rust's own quoting would wrap the whole `/select,…` argument, which explorer does not parse (spawn success = success) |

Arguments are single argv elements, except that one explorer argument, whose path is quoted by hand and cannot contain `"` on Windows; no shell is involved. The call runs on a blocking task. The
launch window keeps SC-003 intact (the request reaches the OS at spawn, before any wait) and stops a
foreground handler from holding the task.

## 3. From activation to the operating system

| # | `ResolvedLink.target` | Session | Effect |
|---|---|---|---|
| O1 | `Url(u)` | any | `opener.open(u)` |
| O2 | `HostPath(p)` | not sandboxed | O4 |
| O3 | `HostPath(p)` | sandboxed | confirm surface (§4); on confirm, O4 |
| O4 | `HostPath(p)` | — | in one blocking task: `shell/links.rs` gathers `FileFacts` with `std::fs::metadata(p)` (follows symlinks). `NotFound` → failure "not found"; otherwise `runnable::action_for(platform, file_name(p), facts, pathext)` → `Open` → `opener.open(p)`, or `Reveal` → `opener.reveal(p)` |
| O5 | `Unreachable` | sandboxed | failure "not shared with this machine" |

### Runnable (`action_for`), FR-013 verbatim

| Platform | `Reveal` when |
|---|---|
| Linux | file with any of `0o111`; or extension in `desktop appimage jar deb rpm snap flatpak flatpakref run` |
| macOS | file with any of `0o111`; directory that is a bundle (extension in `app bundle framework plugin kext prefpane appex xpc`, or has `Contents/Info.plist`); or extension in `app command terminal tool pkg mpkg jar workflow action scpt applescript webloc fileloc inetloc url shortcut` |
| Windows | extension in `%PATHEXT%`; or in `exe com bat cmd ps1 psm1 vbs vbe js jse wsf wsh hta scr pif cpl msc msi msp reg lnk url jar appref-ms application appx msix chm inf scf settingcontent-ms library-ms search-ms` |
| all | otherwise `Open` (a document opens in its application, and a folder in the file manager) |

Extensions compare ASCII case-insensitively on the last extension. `AppImage` matches `appimage`.

## 4. Confirmation (FR-018a)

A floating surface, `SurfaceId::new("confirm_link_open")`, built like `confirm_session_remove`: a
`ConfirmLinkOpenDialog` implementing `FloatingSurface` and `Registered` in `features/session.rs`, its
view in `ui/confirm_link_open.rs`, an entry in `overlay/registry.rs`, and a row in each overlay list
test (`overlay_registry.rs`, `overlay_dispatch_ordering.rs`, `overlay_dismissal_delta.rs`,
`overlay_transition_identity.rs`), plus `popover_displacement.rs`, whose `DIALOGS` count goes from 9 to 10.

- Title: **Open a file from the sandbox?**
- Body: `The sandboxed session linked to <host path>. Files the sandbox wrote can contain scripts or
  macros.`
- Actions: **Open** (confirms), **Cancel** (declines; Escape and scrim-click decline too).

The Open button publishes `SessionMsg::LinkOpenConfirmed`. `main.rs` routes it to `shell/links.rs`,
which reads whether the sandbox is `Running` or `Stale` from `App.sandbox` and calls
`features::session::link_open_confirmed(state, sandbox_live)`. The reducer decides. On confirm, when the session is gone the notice is `Couldn't open <host path>: the session has
closed`, and when the sandbox is neither `Running` nor `Stale` it is `Couldn't open <host path>: the
sandbox has stopped`.

## 5. Notifications (FR-015)

Through `notify_error`, one per failed activation:

| Cause | Text |
|---|---|
| `NoApplication` | `Couldn't open <address>: no application is set up to open it` |
| `LaunchFailed(e)` | `Couldn't open <address>: <e>` |
| not found | `Couldn't open <address>: the file doesn't exist on this machine` |
| unreachable | `Couldn't open <address>: the sandbox doesn't share that location with this machine` |

`<address>` is always `link.address`, the text the program printed or declared. It is the same in
every row, and never carries `display`'s "not reachable" suffix twice.

## 6. The context menu (FR-020, FR-021)

| # | Menu opened | Items |
|---|---|---|
| M1 | over a link (resolved at the right press) | **Open Link**, **Copy Link Address**, then today's items (no divider: `MenuItem` has none, and adding one is a component change this feature does not need) |
| M2 | not over a link | today's items only |
| M3 | Open Link | as `LinkActivated(captured)` |
| M4 | Copy Link Address | `Outcome::ClipboardWrite(captured.link.address)`, performed by `shell::clipboard::interpret` through the route in data-model §3 (never `app::interpret`, which drops it) |

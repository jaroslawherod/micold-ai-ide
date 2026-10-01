# Contract: The sentences, and where each is said

One pure module writes every sentence: `micold_core::cli_reason`
(`crates/micold-core/src/cli_reason.rs`, new). Research: [R4–R8](../research.md).

```rust
pub const LABEL_ENABLED: &str = "Source a script before each session";
pub const LABEL_SCRIPT_PATH: &str = "Script path";
pub const LABEL_TIMEOUT: &str = "Timeout";

pub fn name_list(clis: &[AiCli]) -> Option<String>;        // moved from features/settings.rs
pub fn explain(missing: &[AiCli], env: SpawnEnv, place: Place<'_>, dir: AttemptDir<'_>)
    -> Option<Explanation>;                                // None when `missing` is empty
pub fn start_refusal(cli: AiCli, env: SpawnEnv, place: Place<'_>, dir: AttemptDir<'_>,
    launch: LaunchMode) -> String;
```

## W1 — Placeholders

| Placeholder | Value |
|---|---|
| `{names}` | `name_list(missing)`: display names, "A", "A and B", "A, B and C" (FR-003, FR-004) |
| `{them}` | "it" for one CLI, "them" for several |
| `{lead}` | host: `A session would not find {names}:` · image: `A session in {image} would not find {names}:` |
| `{path}` | host: `the login PATH` · image: `the image's PATH` |
| `{dir}` | `AttemptDir::Home`: `your home directory` · `Dir(p)`: `p` as displayed |
| `{install}` | host: `install {them} on the login PATH` · image: `use an image that puts {them} on its PATH` |

## W2 — `explain`: reason and action per state

| `SpawnEnv` | `reason` | `action` |
|---|---|---|
| `IncludeOff` | `{lead} sessions get only {path}, because "Source a script before each session" is off.` | `Turn it on if your startup file puts {them} on the PATH, or {install}.` |
| `NoScriptPath` | `{lead} no script is sourced, because "Script path" is empty.` | `Set "Script path" if a startup file puts {them} on the PATH, or {install}.` |
| `ScriptNotFound` | `{lead} the startup script was not found for {dir}, so its PATH additions are not applied.` | `Correct "Script path".` |
| `ScriptFailed` | `{lead} the startup script exited with an error for {dir}, so its PATH additions are not applied.` | `Fix the script named in "Script path".` |
| `ScriptTimedOut` | `{lead} the startup script timed out for {dir}, so its PATH additions are not applied.` | `Fix the script named in "Script path", or raise "Timeout".` |
| `Applied`, host | `{names} was not found on the PATH sessions get for {dir}: the login PATH plus what the startup script adds.` ("were" for several) | `Install {them}, or make the script add its directory.` ("their directories" for several) |
| `Applied`, image | `{names} isn't in {image}.` ("aren't" for several) | `Sessions run in that image, so it has to provide any AI CLI you want to use.` |

Rules the table obeys, each asserted by a test over all rows × both places:

- **W2a (FR-002, SC-003)**: where `!env.script_applied()`, neither string contains
  `isn't installed`, `not installed`, `isn't in`, or `aren't in`, and the action is never only an
  instruction to install.
- **W2b (FR-003)**: every label in a sentence is one of the three constants, in double quotes. The
  Settings page renders its checkbox and field labels from the same constants.
- **W2c (FR-004a)**: `{dir}` appears in exactly the rows `ScriptNotFound`, `ScriptFailed`,
  `ScriptTimedOut` and `Applied` on the host. It appears in no other row.
- **W2d (FR-005)**: the `Applied`/image row is today's sentence byte for byte
  (`tests/missing_cli_is_reported_where_it_is_chosen.rs` keeps its image assertions).
- **W2e (FR-016)**: no sentence names a startup file of a platform. "the startup script" refers to
  the stored script path.
- **W2f (FR-007)**: no sentence contains script output or says "see below".

## W3 — `start_refusal`: the sentence said when a start is refused

`{reason}` and `{action}` are `explain(&[cli], ..)`'s.

| State and place | `LaunchMode::Fresh` | `LaunchMode::Resume` |
|---|---|---|
| any, except `Applied`/image | `{reason} {action} Or start this session on another AI CLI.` | `{reason} {action} Then restart this session: its conversation can only continue in {name}.` |
| `Applied`, image | `{name} isn't in {image}, where sessions run. Choose an image that provides it, or start this session on another AI CLI.` | `{name} isn't in {image}, where sessions run, and this conversation can only continue in it. Choose an image that provides it, then restart this session.` |

- **W3a (FR-009)**: the `Resume` form never offers another CLI.
- **W3b (Story 2 scenario 4)**: the `Applied`/image row is the text `missing_cli_reason` returns
  today, byte for byte.
- **W3c (Story 2 scenario 2)**: in the three failed-attempt states no form contains "install".

## W4 — Surfaces

| # | Surface | Written by | Sentence | Subject |
|---|---|---|---|---|
| U1 | Note under **Default AI CLI** (FR-006, FR-007) | client `features::settings::missing_cli_notice(availability)` | `{reason} {action}` | `Home`, place from `source` |
| U2 | Note under *Image reference* (FR-005) | the same function (`ui/settings/daemon.rs` calls it today) | the same string as U1 | the same |
| U3 | Missing-default message as the list opens (FR-008) | client `features::session::start_menu_toggled` | `start_refusal(cli, .., Fresh)` | the answer in use for the row: `asked_for` |
| U4 | Start or restart failure: pane text and banner (FR-009) | service, the launch gate in `state.rs` (replaces `missing_cli_reason`) | `start_refusal(cli, .., launch)` | `Dir(plan.cwd)`, place from `MICOLD_IMAGE_REFERENCE` |
| U5 | Reply to an AI session's `create_session` (FR-009a) | service `mcp/tools.rs` | `{reason} {action}` | `Dir(cwd)` |
| U6 | Note in a row's CLI list (FR-010) | client `features::session::State::start_menu_note(dir)`, drawn by `MenuOverlay::note` | `{reason} {action}` | the answer in use for the row: `asked_for` |

## W5 — When a surface says nothing (FR-011)

| Condition | U1, U2 | U3 | U6 |
|---|---|---|---|
| No answer in use | nothing | not reached: with no answer the press goes to the session service, and U4 answers | nothing |
| Answer in use, nothing missing | nothing | nothing (the default is available) | nothing |
| Answer in use, `env` is `None` | nothing | `{name} would not be found by a session here. Start this session on another AI CLI.` (R8) | nothing |
| Row's answer offers fewer than two CLIs | — | said, as above | nothing (FR-010, D5, D6) |

U4 and U5 are said only when the service has resolved the directory, so they always have a state.

## W6 — Standing and event surfaces (FR-012, FR-013)

- U1, U2 and U6 are computed from state on every draw, so they follow a newer answer with no
  further code.
- U3 is a notification posted once by `start_menu_toggled`. U4 is stored in `start_failures` and
  shipped in `WireLifecycle::Failed`. U5 is one reply. None is recomputed afterwards.

## W7 — The menu note (U6)

`material::MenuOverlay::note(text: impl Into<String>) -> Self` (`ui/material/menu.rs`):

- drawn under the items, after a `material::Divider`;
- text wraps at the panel's width less the item padding at both sides; `TypeRole::Label`, muted;
- not a button: no `on_press`, no ripple, no state layer;
- `material::menu_panel_size_with_note(items, note) -> (u16, u16)` is the size the start list's
  anchor clamping uses. `menu_anatomy` holds it to the laid-out panel, as it holds
  `menu_panel_size` today;
- shown in the component showcase (`crates/micold-client/src/showcase/sections/floating.rs`, the `MenuOverlay` entry).

A menu without a note is laid out exactly as before.

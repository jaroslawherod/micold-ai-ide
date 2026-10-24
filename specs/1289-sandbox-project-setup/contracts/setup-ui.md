# Contract: setup in the client UI

Feature 1289. These are shared components only (Principle VIII). The words are binding. The
layout follows the components' own anatomy.

## U1. Settings › Project › Setup command (FR-009)

- The rail gains **Project** after **Jira** (`SettingsSection::Project`, NEW).
- With a project open, the section shows:
  - the heading *<project display name>*;
  - a `TextArea` labelled **Setup command**, in monospace, 4 rows tall and growing to 10;
  - the help line *Runs in each new worktree of this project, on the daemon it runs on. Leave
    empty for none.*
- With no project open, it shows *Open a project to set its setup command.* and no field.
- **Save** stores `SetupCommand::parse(text)`. A blank value removes the project's entry. There is
  no validation error: any non-blank text is a command.
- The value follows the open project. Switching projects while the page is open reloads the field,
  as the Jira base URL field does (`SettingsDraft::jira.follow`).

## U2. Row cue (FR-015, SC-003)

A `Tag` sits beneath the name of the worktree row and of the Default row. It goes after the
type and issue tags, and in the same tag line.

| State | Tag text | Tooltip |
|---|---|---|
| none | (no tag) | — |
| Running | *Setting up…* | *Setup running since <local time>* |
| Succeeded | *Set up* | *Setup finished at <local time>* |
| Failed | *Setup failed* | *Setup failed at <local time>: <reason>* |
| Stopped | *Setup stopped* | *Setup stopped at <local time>* |
| Interrupted | *Setup interrupted* | *Setup interrupted: the daemon stopped* |

Tints come from `tokens::Roles`: neutral for Running, Succeeded and Stopped, the error role for
Failed, and the warning role for Interrupted. The words carry the state without the colour
(Principle VIII theming, accessibility). The tag never moves the row's name or state icon
(geometry gate G1).

## U3. Menu entries (FR-016, FR-022, FR-023, SC-006)

The entries are built in `location_menu_items`, so worktree rows, the Default row and Overview
cards all get them (feature 972). They are inserted directly above **Delete** on a worktree, and
appended on the Default row, in this order:

| Entry | Icon | Offered when |
|---|---|---|
| **Setup output** | existing terminal-like icon (reuse; no new glyph) | a run record exists |
| **Run setup** | existing play/refresh icon (reuse) | the project has a setup command, and no run is Running for this location |
| **Stop setup** | existing stop icon (reuse) | a run is Running for this location |

**Run setup** on a failed row is two actions: open the menu, choose it (SC-006). A daemon that
answers `Busy` shows its reason as an error notification.

## U4. Output view (FR-016, FR-021)

The view is a `ScrollingDialog` with these parts:

- **Title**: *Setup — <location display name>*.
- **Header lines**: the command in monospace, clamped to 3 lines with the full text in its
  tooltip, and the state, worded as in U2.
- **Body**: a `VirtualRows` list of monospace lines. When `dropped > 0`, the first row reads
  *<n> earlier lines not kept*. With no output yet, the body reads *No output yet.*
- **Follow**: the view stays at the bottom while new lines arrive and the user has not scrolled
  up. Scrolling up stops the follow. Reaching the bottom again resumes it.
- **Actions**: **Copy output** copies the kept lines, joined with `\n`. **Stop setup** is shown
  while Running. **Close** is the last action.

Opening the view sends `SetupOutputWatch`, and closing it sends `SetupOutputUnwatch`. A
reconnect while the view is open sends the watch again from the last line held (FR-018).

## U5. Failure notification (FR-017, SC-004)

Each client sends one error notification per failed run: *Setup failed in <project> /
<location>: <reason>.* `<reason>` is the reason line without its full stop, for example *Exited
with status 2*. A run that ended `Stopped` or `Interrupted` sends none.

## U6. Delete with setup running (FR-024)

The delete confirmation for a worktree whose setup is running adds the line *Setup is running here.
It will be stopped first.* The daemon stops the run, then deletes.

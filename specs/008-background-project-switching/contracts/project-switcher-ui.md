# UI Contract: Top-Bar Project Switcher

The switcher is a shared, reusable **builder** primitive (Principle VIII) placed in the top app bar next to the overflow-menu button. This contract defines its placement, structure, states, interactions, and the messages it emits. It complements — does not replace — the shell body "Known projects" list and the folder-browser modal (2026-07-17 clarification).

## Placement (FR-004)

- Composed in `toolbar::view` as `Toolbar::new(app_name, roles).action(project_switcher_trigger).action(menu_trigger).into()`.
- `.action()` order places the switcher trigger **immediately left of** the existing `MenuTrigger` (the three-dots/menu button), so the two sit adjacent at the right of the bar.
- `toolbar::view` signature extends from `(scheme: ColorScheme)` to also receive the data the switcher renders (see Data inputs); the menu trigger is unchanged.

## Component API (builder, Principle VIII)

```
ProjectSwitcher::new(active_label, roles)          // required inputs only
    .projects(rows: Vec<ProjectRow>)               // chainable, self-consuming
    .on_select(|path| Message)                      // row activation
    .on_add(Message)                                // "Add project…" row
    .into()                                          // impl From<ProjectSwitcher> for Element
```

- Terminates in `.into()` (an `impl From<ProjectSwitcher> for Element<'_, Message>`), exactly like iced's built-in widgets and the existing `MenuTrigger`.
- Theming supplied through `roles`/`ColorScheme` in the constructor (no ambient theme lookup).
- The floating panel reuses the `menu_overlay` machinery via `ui::mod::view`; opening it MUST NOT reflow the top bar.

## Data inputs (rendered purely from core `State`)

`ProjectRow` per known project:

| Field | Source | Used for |
|-------|--------|----------|
| `display_name` | `Project.display_name` | row label |
| `path` | `Project.path` | select payload |
| `is_active` | `path == workspace.active` | active marker (FR-006) |
| `running_count` | `workspace.running_session_count(path)` | running-background indicator (FR-007, R6) |
| `available` | `Project.availability` | unavailable badge + disabled select (FR-008) |

Plus a trailing **"Add project…"** row (FR-009).

## Rendered states

- **Trigger**: always visible; shows a switcher affordance (label/icon of the active project). With no active project, shows a neutral "Select project" affordance.
- **Panel open**: lists all known projects. Each row: name; active row visibly marked; rows with `running_count > 0` show the count badge (e.g. "2 running"); unavailable rows show an "unavailable/missing" badge and are not selectable. Last row: "Add project…".
- **Empty catalog**: panel shows only the "Add project…" row (the shell body still owns the full first-run empty state — complement, not replace).

## Interactions → Messages

| User action | Message emitted | Effect |
|-------------|-----------------|--------|
| Open the switcher trigger | `Message::ProjectSwitcherToggled` (new) | Toggles the switcher panel; closes the overflow menu if open (mutually exclusive overlays). |
| Select an **available**, non-active project | `Message::KnownProjectReopened(path)` (reused) | Runs `State::switch_active(path)`; sessions of both projects keep running (FR-001/002); incoming foreground restored (FR-003). Panel closes. |
| Select the already-active project, or dismiss (Esc / outside click) | none / close only | Active project unchanged (Story 2, scenario 3). |
| Select an **unavailable** project | none (row disabled) | No switch; unavailable indication remains (FR-008). |
| Select "Add project…" | `Message::ProjectSelectorOpened` (reused) | Opens the existing folder-browser modal (FR-009). |

## Non-goals / boundaries

- The switcher does not add, rename, or remove projects (only switch + a shortcut into the existing add flow).
- Visual form details (exact iconography, ordering of rows, animation) are implementation choices deferred to tasks; ordering was noted as a deferred, low-impact item in `/speckit-clarify`.
- No new persisted state (see data-model.md).

# Icons

Micold AI IDE uses a single set of [Material Symbols](https://fonts.google.com/icons)
(Outlined) icons across the whole application, so the same concept always looks the same
wherever it appears. Icons sit next to their text labels on actions, and stand in for small
status markers.

## Where icons appear

| Icon | Meaning | Where you see it |
|------|---------|------------------|
| App mark (the application's icon) | Names the application on hover; not a button | Title bar, left end |
| Menu (three dots) | Open the overflow menu | Title bar, right end |
| Info | Open the About dialog | Overflow menu |
| Help (question mark) | Open Session service diagnostics | Overflow menu |
| Settings (gear) | Open Settings; drawn selected while Settings is open | Title bar, before the overflow menu, and the overflow menu |
| Light mode (sun) | The [Appearance](settings.md#appearance) section of Settings | Settings rail |
| Open folder | Open or choose a project | Empty state, "Open another project", the known-projects **Open** button, the project switcher, and "Open this folder" in the selector |
| Drop-down arrow | Opens a list to choose from | The title bar's project segment (it opens the project switcher), select fields |
| Pencil | Rename a known project | Known-projects list, project right-click menu |
| Commit | This folder is a git repository | "git" badge in the known-projects list and the project selector |
| Check circle | This is the active project | Known-projects list, project switcher |
| Error | This project's folder is currently unavailable | Known-projects list, project switcher, sidebar |
| Up arrow | Go up one folder | Project selector, folder tree |
| Hide sidebar | Collapse the sidebar | Sidebar chip bar |
| Show sidebar | Expand the sidebar | Collapsed sidebar |
| Add session | Start a new session | Sidebar worktree rows |
| Add worktree | Create a new worktree | Sidebar chip bar |
| Refresh | Re-read the project's worktree list from git | Sidebar chip bar's more (⋯) menu |
| Delete | Delete a worktree | Sidebar worktree rows |
| Filter | Toggle a filter panel; the sidebar no longer has one, so the app window does not show it | Component showcase only |
| More (⋯) | Open the menu holding refresh, attach existing and run in parallel | Sidebar chip bar |
| Search | Search a long list by typing; on the title bar's jump-to field, open the quick switcher | Branch search in the New worktree form, the title bar's jump-to field |
| Project root | The repository's own working directory | Sidebar **Default** entry |
| Copy | Copy a worktree's name to the clipboard | Worktree right-click menu (**Copy name**) |
| Raised hand | A session needs permission or an answer from you | **Needs permission** row in Settings and its notification |
| Error | A session stopped because of an error | **Session error** row in Settings and its notification |
| Task done (check in a circle) | A session finished a long task | **Long task finished** row in Settings and its notification |
| Chat bubble | A session finished a turn | **Turn finished** row in Settings and its notification |
| Triangle (play) | **Running**: a session in the worktree is working, starting or restarting | Sidebar worktree rows and the **Default** entry, after the toggle |
| Octagon with a mark | **Blocked**: a session cannot go on without you (it failed, or waits to be resumed) | Sidebar worktree rows and the **Default** entry, after the toggle |
| Two bars (pause) | **Idle / waiting**: nothing in progress, or waiting for your next prompt | Sidebar worktree rows and the **Default** entry, after the toggle |
| Dashboard (grid) | Open or close the worktree Overview | First row of the sidebar, **Overview** (opens only) |
| Exclamation mark | **Needs action**: a session needs permission or an agent request is waiting | Overview cards |
| Eye | **Needs attention**: a session failed or can be resumed, or the pull request has failing checks, requested changes or a conflict | Overview cards |
| Check circle | **Done**: the pull request is merged and the worktree can be removed | Overview cards |
| Light bulb | **Feature** worktree | Overview cards |
| Bug | **Bugfix** worktree | Overview cards |
| Wrench | **Chore** worktree (any other conventional type) | Overview cards |
| Folder | **Other**: a worktree with no conventional type | Overview cards |
| Link | **Included** worktree | Overview cards |
| Crossed-out folder | **Orphan**: a missing or invalid worktree | Overview cards |
| Paperclip | **Add attachment**: choose files to attach to the message | Structured session composer, before **Send** |
| Arrow into a door | **Open**: go to the card's location, as clicking the card does | First entry of an Overview card's right-click menu |

Every icon follows the active theme: it is tinted to match the text around it and is legible
in both the light and the dark theme. When you switch your system between light and dark, the
icons re-color together with the rest of the window.

## Appearance and licensing

The icons come from Google's Material Symbols, distributed under the Apache License 2.0 — the
same license as this project. The bundled font ships with full glyph coverage (every icon the
upstream font defines), so adding a new icon never requires touching the font file itself — see
[`assets/fonts/PROVENANCE.md`][provenance] in the repository. The font is embedded in the
application and needs no internet connection.

[provenance]: https://github.com/jaroslawherod/micold-ai-ide/blob/{{MICOLD_TAG}}/assets/fonts/PROVENANCE.md

## For contributors: adding a new icon

Icons are defined once, in the render-free core, as a closed `Icon` enum. Referencing an icon
that does not exist is a **compile error** — a missing or misspelled icon can never slip
through to a blank box at runtime. To add one:

1. Find the glyph's codepoint in the upstream
   [`.codepoints`](https://github.com/google/material-design-icons) manifest.
2. Add an `Icon` variant and its codepoint in `crates/micold-material/src/icons.rs`, and add a
   row to the mapping table in `assets/fonts/PROVENANCE.md`.
3. Extend the mapping assertion in `crates/micold-client/tests/icons.rs`. The font-integrity test
   (`crates/micold-client/tests/icons_font.rs`) then verifies the glyph is present in the bundled font — which it
   already is, since the font carries full upstream coverage.

See [Appearance & Theming](appearance-theming.md) for how themes and colors work.

---

**Alignment**: 2026-07-20 — Spec/code alignment audit (spec 004 FR-013). The table advertised a
"Help (question mark)" app-bar icon that no longer exists — the top app bar renders `Icon::Menu`
— and listed 9 of the 21 icons the application defines. All rendered icons are now documented.

**Alignment**: 2026-10-25 (#1128). `Icon::Help` draws the overflow menu's
**Session service diagnostics** item and is in the table. `Icon::DarkMode` and `Icon::AutoMode` are defined with no
rendered call site since the overflow menu's theme toggle was removed, and are left out of it.

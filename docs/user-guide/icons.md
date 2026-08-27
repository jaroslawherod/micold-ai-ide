# Icons

Micold AI IDE uses a single set of [Material Symbols](https://fonts.google.com/icons)
(Outlined) icons across the whole application, so the same concept always looks the same
wherever it appears. Icons sit next to their text labels on actions, and stand in for small
status markers.

## Where icons appear

| Icon | Meaning | Where you see it |
|------|---------|------------------|
| Menu (three lines) | Open the overflow menu | Top app bar |
| Info | Open the About dialog | Overflow menu |
| Settings (gear) | Open the Settings dialog | Overflow menu |
| Light mode (sun) | The theme is following light mode | Theme toggle in the overflow menu |
| Dark mode (moon) | The theme is following dark mode | Theme toggle in the overflow menu |
| Auto mode | The theme is following your operating system | Theme toggle in the overflow menu |
| Open folder | Open or choose a project | Empty state, "Open another project", the known-projects **Open** button, the project switcher, and "Open this folder" in the selector |
| Pencil | Rename a known project | Known-projects list, project right-click menu |
| Commit | This folder is a git repository | "git" badge in the known-projects list and the project selector |
| Check circle | This is the active project | Known-projects list, project switcher |
| Error | This project's folder is currently unavailable | Known-projects list, project switcher, sidebar |
| Up arrow | Go up one folder | Project selector, folder tree |
| Hide sidebar | Collapse the sidebar | Sidebar header |
| Show sidebar | Expand the sidebar | Collapsed sidebar |
| Add session | Start a new session | Sidebar worktree rows |
| Add worktree | Create a new worktree | Sidebar header |
| Delete | Delete a worktree | Sidebar worktree rows |
| Filter | Toggle the tag-filter panel | Sidebar toolbar |
| Search | Search a long list by typing | Branch search in the New worktree form |
| Project root | The repository's own working directory | Sidebar **Default** entry |
| Copy | Copy a worktree's name to the clipboard | Worktree right-click menu (**Copy name**) |

Every icon follows the active theme: it is tinted to match the text around it and is legible
in both the light and the dark theme. When you switch your system between light and dark, the
icons re-color together with the rest of the window.

## Appearance and licensing

The icons come from Google's Material Symbols, distributed under the Apache License 2.0 — the
same license as this project. The bundled font ships with full glyph coverage (every icon the
upstream font defines), so adding a new icon never requires touching the font file itself — see
[`assets/fonts/PROVENANCE.md`][provenance] in the repository. The font is embedded in the
application and needs no internet connection.

[provenance]: https://github.com/Cumulocity-IoT/micold-ai-ide/blob/{{MICOLD_TAG}}/assets/fonts/PROVENANCE.md

## For contributors: adding a new icon

Icons are defined once, in the render-free core, as a closed `Icon` enum. Referencing an icon
that does not exist is a **compile error** — a missing or misspelled icon can never slip
through to a blank box at runtime. To add one:

1. Find the glyph's codepoint in the upstream
   [`.codepoints`](https://github.com/google/material-design-icons) manifest.
2. Add an `Icon` variant and its codepoint in `src/icons.rs`, and add a row to the mapping
   table in `PROVENANCE.md`.
3. Extend the mapping assertion in `tests/icons.rs`. The font-integrity test
   (`tests/icons_font.rs`) then verifies the glyph is present in the bundled font — which it
   already is, since the font carries full upstream coverage.

See [Appearance & Theming](appearance-theming.md) for how themes and colors work.

---

**Alignment**: 2026-07-20 — Spec/code alignment audit (spec 004 FR-013). The table advertised a
"Help (question mark)" app-bar icon that no longer exists — the top app bar renders `Icon::Menu`
— and listed 9 of the 21 icons the application defines. All rendered icons are now documented.
`Icon::Help` remains defined in `src/icons.rs` with zero call sites and is deliberately omitted
here; it is tracked as orphaned code for removal.

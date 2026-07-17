# Icons

Micold AI IDE uses a single set of [Material Symbols](https://fonts.google.com/icons)
(Outlined) icons across the whole application, so the same concept always looks the same
wherever it appears. Icons sit next to their text labels on actions, and stand in for small
status markers.

## Where icons appear

| Icon | Meaning | Where you see it |
|------|---------|------------------|
| Help (question mark) | Open the Help menu | Top app bar |
| Info | Open the About dialog | Help menu |
| Open folder | Open or choose a project | Empty state, "Open another project", the known-projects **Open** button, and "Open this folder" in the selector |
| Pencil | Rename a known project | Known-projects list |
| Commit | This folder is a git repository | "git" badge in the known-projects list and the project selector |
| Check circle | This is the active project | Known-projects list |
| Error | This project's folder is currently unavailable | Known-projects list |
| Up arrow | Go up one folder | Project selector |
| Copy | Copy a worktree's name to the clipboard | Worktree right-click menu (**Copy name**) |

Every icon follows the active theme: it is tinted to match the text around it and is legible
in both the light and the dark theme. When you switch your system between light and dark, the
icons re-color together with the rest of the window.

## Appearance and licensing

The icons come from Google's Material Symbols, distributed under the Apache License 2.0 — the
same license as this project. The bundled font ships with full glyph coverage (every icon the
upstream font defines), so adding a new icon never requires touching the font file itself — see
[`assets/fonts/PROVENANCE.md`](../../assets/fonts/PROVENANCE.md). The font is embedded in the
application and needs no internet connection.

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

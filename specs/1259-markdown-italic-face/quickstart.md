# Quickstart: Markdown italic draws in a true italic face

Validation guide for feature 1259. Tests are listed in [plan.md](./plan.md#test-strategy-by-layer);
interfaces in [contracts/fonts.md](./contracts/fonts.md).

## Prerequisites

- A checkout of this branch, `mise trust` done once.
- For §B: a Linux desktop session; for B5, permission to install a user font in
  `~/.local/share/fonts`.

## §A — Automated

```sh
mise run gate
sha256sum assets/fonts/Roboto-*Italic.ttf
ls -l assets/fonts/Roboto-*.ttf
git diff --stat origin/main -- crates/micold-client/tests/fixtures/layout_snapshot.txt
```

Expected:

- The gate is green, including `tests/roboto_font.rs` (six faces), the `roboto.rs` unit tests, the
  `link_text.rs` span-font tests and `markdown_subset.rs` (bold italic span).
- The three hashes equal those in `assets/fonts/PROVENANCE.md`; each italic file is about 160 KB
  (SC-004: under 0.5 MB together).
- `layout_snapshot.txt` is unchanged, or changes only in `session-transcript-rich`, in the
  paragraph holding `*leaves*` (research R7). Any other state changing fails FR-004.

## §B — Recorded pass on Linux (visual-pass)

`mise run showcase` and `mise run run`; capture with `mise run screenshot`.

| # | Do | Expect | Covers |
|---|---|---|---|
| B1 | Showcase, `MarkdownView` section: the formatted paragraph. | `italic` slanted with Roboto's italic letterforms; `bold italic` slanted and as heavy as `bold`; the other words as before. | US1.1, US2.1, FR-001, FR-003, FR-009 |
| B2 | Showcase, `LinkText` section. | `italic` slanted; links, bold, strike and code unchanged. | FR-001, FR-004 |
| B3 | In a structured session, a reply with `this will *not* touch`, `_underscore_`, `***both***`, ``*see `cfg` here*`` and `*[a link](https://example.com)*`. | `not` and `underscore` slanted alike; `both` slanted and bold; `cfg` upright Fira Mono on its code fill with the words around it slanted; the link slanted, underlined, in the link colour. | US1.1–US1.2, US2.1, Edge: inline code, Clarifications (link) |
| B4 | A reply with `# A *slanted* heading` and a table whose header cell has `*x*`. | Slanted, at the heading's and header's weight. | US2.2 |
| B5 | Copy any Roboto Italic (e.g. `Roboto-Italic.ttf` from a Roboto release) into `~/.local/share/fonts`, `fc-cache -f`, relaunch, repeat B1. | Identical to B1. | US3.2, FR-006 |
| B6 | A reply with `*漢字 and 🙂*`. | No missing-glyph boxes; the Latin word slanted. | Edge: coverage |
| B7 | Two sessions open, each showing B3's reply; then the Overview. | Emphasis the same in both; Overview card text unchanged. | Edge: several sessions, FR-004 |

## §C — macOS and Windows

CI runs the gate (including the layout snapshot) on all three runners (SC-003). By hand, when a
macOS or Windows machine is at hand: repeat B1 and B3 in a release build.

## Results

(filled in by the implementation units)

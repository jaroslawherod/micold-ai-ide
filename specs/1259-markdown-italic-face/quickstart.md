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

### §A (T023, close unit, 2026-10-25, at `e0536872` on `main`)

- **Gate**: green before each milestone push (`scripts/gate.sh`, e.g. at `468b4608` for M1 and
  `fd518b5d` after its rebase); CI's `build + test` passed on ubuntu, macOS and Windows for #1377
  and #1389.
- **Hashes**: `sha256sum assets/fonts/Roboto-*Italic.ttf` prints
  - `2f700a535d646e50887c1623339d31c4bda6b808d2c30db7e7721c8264a26f97` `Roboto-Italic.ttf`
  - `709d3b3b226aa58d6a1ca6ab55906644197a47c68b5856aadab1b811eaa4ee64` `Roboto-MediumItalic.ttf`
  - `a446d79e013cbcf40aea0bf7294d13e68e10a727d1b35412878fe1d0c2ed0713` `Roboto-SemiBoldItalic.ttf`

  All three equal the table in `assets/fonts/PROVENANCE.md`.
- **Sizes (SC-004)**: 163,212 + 163,504 + 164,068 = 490,784 bytes for the three italics, under
  0.5 MB.
- **Layout snapshot (FR-004)**: no feature 1259 commit (`git log --grep=1259 56515707..e0536872`)
  touches `crates/micold-client/tests/fixtures/layout_snapshot.txt`: the emphasis face changes no
  laid-out bound the snapshot records. Its changes since the design PR come from features 1125 and
  1186.

### §B

Recorded in [visual-pass/results.md](./visual-pass/results.md): B1–B7 PASS (B1, B3 and B6 after
the D10/D11 re-run; B5 by the M2 unit).

### §C (T024)

- **Proven by CI on macOS and Windows** (`build + test (macos-latest)` and
  `build + test (windows-latest)` green on #1377, run 40147222770, and #1389, run 40157020830):
  the layout snapshot, `tests/roboto_font.rs` (the six faces parse and report the right weight and style; resolution is
  tested in `roboto.rs`), the `roboto.rs` resolution
  and pin tests (`pin_drops_host_italics_and_off_weight_faces_only` builds its own font database from
  temp-dir host copies, so it runs the same on every host; `the_renderer_draws_each_weight_and_bold_in_the_bundled_faces`, which reads the host's fonts, skips where a
  host Roboto is installed, which neither runner image has), the `link_text.rs` span-font tests and `markdown_subset.rs`.
  #1377 also passed `package + smoke (windows-11-arm)`.
- **Needs a host, not run**: the look itself (B1 and B3 in a release build) on macOS and Windows.
  No macOS or Windows machine was at hand; the faces are bundled and loaded by the same code on
  every platform, so the Linux pass stands for the letterforms, and only a host's own font
  fallback (CJK, emoji) can differ.

# `site/` — the publication tooling

Everything under this directory is **code**, not documentation.

`.gitattributes` marks `docs/**` as `micold-docs` and treats every other path as code, so a change
here takes the full three-platform pipeline while a typo in a guide page still skips it. That split
is deliberate and load-bearing: `site/media.toml`, the scene scripts and the checks are *build
inputs*, and a build input that could skip the build is exactly what feature 023's declaration
exists to prevent.

**Nothing under `site/` may be added to the `micold-docs` set.**

## What is here

| Path | What it does |
|---|---|
| `book.toml` | mdBook configuration — reads the staged tree, uses `theme/` |
| `media.toml` | the declared capture set: id, scene, scheme, alt text, caption |
| `stage.sh` | `docs/` + fonts + substitutions + media directives → a staging tree |
| `build.sh` | emit theme → capture → stage → render → check |
| `theme/index.hbs` | the page template: app-bar header, table of contents, version, source link |
| `theme/css/site.css` | documentation furniture, written only against `--micold-*` variables |
| `theme/css/tokens.css` | **generated** by `micold-tokens-css`; never edited by hand, never committed |
| `capture/` | the private display, the demonstration project, the stub provider, the scenes |
| `checks/` | the five checks — three before merge, three before deploy (`links.sh` runs in both) |

## What it needs installed

The same list `.github/workflows/pages.yml` installs, and the reason publication runs on Linux
only:

    xvfb  mesa-vulkan-drivers  xdotool  imagemagick  ffmpeg
    mdbook  lychee  node (20)

`mdbook` and `lychee` are Rust and install with `cargo install mdbook lychee --locked`; the rest
come from the distribution's packages. `node` is needed only for `checks/page-checks.mjs`, which
needs a real browser engine to check WCAG 2.2 AA on a rendered page.

## Running it

    mise run site-build          # full: emit, capture, stage, render, check
    site/build.sh --no-media     # skip capture — fast local iteration on prose and theme
    mise run site-check          # the checks alone, against an existing build

## Decisions recorded here

- **`MaterialSymbolsOutlined.ttf` is not subsetted.** The three font files total ~1.2 MB, are
  cached across pages, and sit outside FR-015c's per-page still-image budget, which is about
  images. Subsetting is a later optimisation, not a requirement (research §4).
- **Clips are video, not GIF.** A 15-second GIF of the application window cannot meet the 3 MB
  ceiling at a legible size, has no play control, and fetches its animation with its first frame.
  See research §7.
- **Frames are captured step by step, not recorded.** `ffmpeg -f x11grab` samples on a wall clock
  and so cannot satisfy FR-011d. The cost is that transitions are not shown.

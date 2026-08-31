# Implementation Plan: Published documentation site

**Branch**: `docs/prepare-github-pages-for-iceflow` | **Date**: 2026-08-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/028-docs-site-github-pages/spec.md`

## Summary

Publish `docs/` as a browsable site on GitHub Pages, dressed in the application's own design
system, with screenshots and short clips captured from the released build during publication.

The approach in one line: **stage, derive, capture, check, deploy.** A publication copies `docs/`
into a staging tree (the prose is never edited in place), derives the theme's CSS custom properties
from `micold-core`'s design tokens with a small Rust emitter, builds the released application and
drives it on a private Xvfb display against a synthetic demonstration project to produce every
image the manifest declares, renders the site with mdBook, runs the link / page-set / media-budget /
WCAG checks, and only then deploys. Any check failing stops the deploy, so the previously published
site stands.

Two properties do most of the work and are worth naming up front:

- **Freshness is mechanical, not editorial.** No media file is committed. Every image is produced
  during the publication that ships it, from the build of the version being named (FR-011). The
  price is that publication builds the application; that is accepted in the spec.
- **The theme is derived, not transcribed.** Colour, type scale, shape, elevation and motion are
  emitted from `crates/micold-core/src/tokens/` (FR-030), so a token changed for the application's
  own reasons reaches the site at the next publication with no second edit — and a token that would
  fail contrast on the site fails a test at merge time and the publication after it (FR-033).

No application code changes. The client, the daemon and the core's runtime behaviour are untouched;
the core gains one render-free module (token → CSS) and one thin binary that prints its output.

## Technical Context

**Language/Version**: Rust (workspace stable toolchain, `rust-toolchain.toml`) for the token → CSS
emitter and its tests; POSIX shell for the staging, capture and check scripts; Node 20 for the two
checks that need a real browser engine. The published site is static HTML/CSS/JS.

**Primary Dependencies**: mdBook (site generation + built-in client-side search, Rust, no runtime
third-party fetch); lychee (link checking, Rust, single binary); Xvfb + Mesa lavapipe + xdotool +
ImageMagick (`import`, `convert`) for capture — the route already verified by the repository's
manual visual pass; ffmpeg for encoding clips from captured frames; `@axe-core/playwright` for
WCAG 2.2 AA; GitHub Actions `actions/configure-pages`, `actions/upload-pages-artifact`,
`actions/deploy-pages` (pinned to commit SHAs, as `release.yml` already requires).

**Storage**: None at runtime — the site is static and stores nothing about a reader (no analytics,
no comments; Assumptions). Build inputs are `docs/`, `site/`, `assets/fonts/` and
`crates/micold-core/src/tokens/`; every intermediate (staging tree, captured frames, encoded clips)
is ephemeral and lives only inside the publication run.

**Testing**: `cargo test -p micold-core` covers the token → CSS emitter and the site-contrast gate
(FR-033) at merge time. `scripts/tests/*.test.sh` covers the shell checks, following the existing
`classify-change.test.sh` / `documentation-set.test.sh` precedent. The site's own checks run twice:
against the Markdown sources in CI's `docs` job before merge (FR-021, FR-022, FR-023) and against
the built HTML in the publication workflow before deploy (FR-005, FR-015c, FR-027a, FR-023a). The
judgement half of SC-001 and SC-006 is recorded once per release in this feature's
[quickstart](./quickstart.md) Part B.

**Target Platform**: GitHub Pages over HTTPS at `https://cumulocity-iot.github.io/micold-ai-ide/`.
The publication host is `ubuntu-latest`. Readers are on current desktop and mobile browsers.

**Project Type**: A static documentation site plus the build tooling that produces it, inside an
existing Rust workspace. It adds no application feature and no application state.

**Performance Goals**: A documentation page readable in under 3 seconds on typical broadband with
its still images (SC-007); the site describing the newest release within 60 minutes of that release
being published, unattended (SC-005). Clips are not fetched until started, so they sit outside the
page-load measurement by construction (FR-028).

**Constraints**: Still images ≤ 1 MB per page and each clip ≤ 3 MB, checked (FR-015c); zero
third-party requests from any published page (SC-015); WCAG 2.2 Level AA on every page in both
schemes, checked (FR-027a); capture deterministic and credential-free (FR-011c, FR-011d); the
documentation-only CI skip of feature 023 must survive intact (FR-020).

**Scale/Scope**: 13 documentation pages today (7 user-guide, 5 development, 1 daemon) plus two new
pages the site needs and the repository does not yet have — a home page and an installation page.
Roughly 8 still captures (4 views × 2 schemes where appearance is the subject) and 3–4 clips.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: The only new Rust is a pure function — design tokens in,
      a stylesheet out — and it lands after its failing test, in `micold-core`, where the suite
      already runs without a display. The binary that prints its output holds no decision logic.
      Each shell check ships with a `scripts/tests/*.test.sh` counterpart written first, against
      fixture trees, exactly as `classify-change.sh` and `check-assertions-frozen.sh` already do.
      No part of this feature relies on the Principle I GUI-glue exception.
- [x] **II. Multi-Session Support**: No new application state, per-session or otherwise. Capture
      drives the real application, and each capture run gets its own `XDG_RUNTIME_DIR` /
      `XDG_DATA_HOME`, so it can neither read nor disturb any other session's state.
- [x] **III. Worktree Integration**: Unchanged. The demonstration project capture drives is a real
      git repository built by the capture script; the application creates and removes its worktrees
      in it exactly as it does anywhere else. No new session location is introduced.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: The application keeps working with no network
      and is not touched by this feature. The site is a published *view* of documentation, not
      application state — it holds nothing about a reader, sets no cookie, runs no analytics, and
      fetches nothing from a third party (FR-031, SC-015), so reading it discloses nothing beyond
      the request for the page itself.
- [x] **V. Rust + iced Stack**: The application stays Rust + iced, untouched. The publication
      toolchain is Rust-first (mdBook, lychee, the emitter) with one Node-based step; that step is
      build tooling, not application code, and is justified in Complexity Tracking below.
- [x] **VI. Cross-Platform Parity**: No user-facing application behaviour changes, so there is
      nothing to hold at parity. The site itself is served identically to every reader. Publication
      runs on Linux only, which is a property of the publishing host, not of the product — and the
      merge pipeline that does cover all three platforms is left exactly as it is (FR-020).
- [x] **VII. Documentation First-Class**: This feature *is* the gate, taken further. The two new
      pages (home, installation) land in `docs/` as documentation, not as site-only templates
      (FR-001). The CI `docs` job gains link, page-set and media-reference checks, so a broken
      documentation link fails before merge rather than after publication (FR-021).
- [x] **VIII. Reusable UI Component Foundation**: No application UI is added. The site deliberately
      does *not* recreate application components (FR-029b); it inherits the token set the shared
      components already render from, which is the same "one source, reused" discipline applied
      across the repository boundary rather than a second implementation of it.

**Post-design re-check (after Phase 1)**: unchanged — all eight PASS. The design added no
application code, no session state and no UI component, so II, III, V and VIII are decided the same
way after the contracts as before them. Three points were sharpened by the design rather than
softened:

- **I** — the new Rust surface turned out smaller than expected: one pure function and one contrast
  gate, both in `micold-core`, both test-first. Every other new thing is a shell script with a
  fixture-driven test beside it ([contracts/site-checks.md](./contracts/site-checks.md)).
- **VI** — `ci.yml` gains steps inside the existing `docs` job and no new job, so
  `ci_gate_covers_every_job.rs` and the default branch's required `ci complete` check are both
  untouched, and the three-platform matrix keeps exactly the scope it has today.
- **IV** — the site turned out to need no reader state at all: no cookie, no analytics, no comment
  system, and a search index that is a static file rather than a service.


## Project Structure

### Documentation (this feature)

```text
specs/028-docs-site-github-pages/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── media-manifest.md
│   ├── theme-variables.md
│   ├── publication-workflow.md
│   └── site-checks.md
├── checklists/
│   └── requirements.md
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
site/                                 # publication tooling — code, NOT declared documentation
├── book.toml                         # mdBook configuration (staged src, custom theme)
├── media.toml                        # the declared capture set: id, scene, scheme, alt text
├── stage.sh                          # docs/ + media + substitutions -> a staging src tree
├── build.sh                          # stage -> emit theme -> capture -> mdbook build -> check
├── theme/
│   ├── index.hbs                     # app-bar header, table of contents, version, source link
│   ├── css/site.css                  # documentation furniture, written against the variables
│   └── css/tokens.css                # GENERATED by micold-tokens-css; never edited by hand
├── capture/
│   ├── display.sh                    # private Xvfb + lavapipe, start/stop, own XDG dirs
│   ├── demo-project.sh               # the synthetic project every scene drives
│   ├── stub-cli.sh                   # the fake `claude`/`copilot` that replays a transcript
│   ├── transcript/                   # the canned session transcripts it replays
│   └── scenes/                       # one script per declared capture, still or clip
└── checks/
    ├── page-set.sh                   # docs/**.md <-> docs/SUMMARY.md, both directions
    ├── media-references.sh           # media directives <-> site/media.toml, both directions
    ├── links.sh                      # lychee over sources (pre-merge) and over HTML (pre-deploy)
    ├── media-budget.sh               # per-page still total and per-clip size
    └── page-checks.mjs               # WCAG 2.2 AA (axe-core) + the SC-001/SC-006 structural proxies

crates/micold-core/
├── src/tokens/css.rs                 # NEW: token set -> CSS custom properties (pure, tested)
├── src/bin/micold-tokens-css.rs      # NEW: thin binary, prints what css.rs returns
└── tests/site_theme_contrast.rs      # NEW: every derived pair meets WCAG AA (FR-033, at merge)

docs/
├── SUMMARY.md                        # NEW: the declared page set and reading order (FR-023)
├── README.md                         # becomes the site home page (FR-003)
└── install.md                        # NEW: what the release actually ships (FR-004, FR-004a)

scripts/tests/                        # fixture-driven counterparts for each new shell check
.github/workflows/
├── pages.yml                         # NEW: reusable (workflow_call) + workflow_dispatch
├── release.yml                       # calls pages.yml after the release is published
└── ci.yml                            # the `docs` job gains the three pre-merge checks
```

**Structure Decision**: The site's tooling lives in a new top-level `site/` directory, and the
prose stays in `docs/` untouched. That split is not cosmetic — it is what keeps feature 023's
documentation-only CI skip intact. `.gitattributes` marks `docs/**` as `micold-docs` and treats
every other path as code, so a directory named `site/` is code by default: editing the capture
manifest or the theme takes the full three-platform pipeline, while fixing a typo in a guide page
still skips it. Nothing in `site/` is added to the `micold-docs` declaration.

Two consequences follow, both deliberate:

- **The publication reads documentation, and that is fine.** Feature 023's precondition — asserted
  by `crates/micold-core/tests/documentation_is_not_read.rs` — is that nothing *the merge pipeline
  skips* reads a documentation path. Publication is not part of that pipeline; it is triggered by a
  release, builds its own copy of the application, and gates nothing about a merge. The checks that
  *do* run before merge live in CI's existing `docs` job, which already runs on documentation-only
  changes by design, and they are shell scripts rather than Rust tests — which is also what keeps
  them outside that gate's scan of `crates/**/*.rs`.
- **Adding a guide page touches two files.** The page itself in `docs/`, and its entry in
  `docs/SUMMARY.md` — also documentation, so the change stays documentation-only. `page-set.sh`
  compares the two directions and fails the `docs` job if a page exists without an entry, which is
  what SC-008's "or is told at review time that they must" means in practice.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Publication builds the whole application (~15–25 min, full GUI toolchain, on a documentation deploy) | FR-011 requires every image to come from the build of the version being published. Building it is the only way that is a mechanical fact rather than an author's promise. | Committing screenshots and reviewing their freshness was rejected in the spec: it degrades to a checkbox, and stale media is worse than none because the reader trusts it. |
| Node 20 + Playwright + axe-core in the publication toolchain, alongside an otherwise Rust-first stack | FR-027a demands WCAG 2.2 AA checked on every page on every publication, and FR-023a demands the site's own search be exercised. Both need a real browser engine executing the page's CSS and JavaScript; axe-core is the reference implementation of the rule set. | A Rust HTML linter cannot compute rendered contrast, focus visibility or keyboard reachability — it sees markup, not a rendered page — so it would check a proxy for the requirement rather than the requirement. Contrast *of the derived tokens* is separately caught earlier by a Rust test (FR-033), which is where a Rust check is genuinely stronger. |
| mdBook as a fourth-party generator, plus a staged copy of `docs/` | The site needs navigation, client-side search and a fully custom theme, from Markdown already in the repository, with nothing fetched from a third party at page load. mdBook is Rust, does all four, and ships its search index as a static file. The staging copy is what lets version and release-asset substitution happen without editing the prose in place (FR-001, Assumptions: source of truth). | Rendering the Markdown ourselves means writing a generator, a search index and a theme engine to get the same result. A generator that requires the prose to move (front matter, a `content/` layout) was rejected because it forks `docs/` in all but name. |
| Publication runs on Linux only | Capture needs an X display, a software rasteriser and a window-driving tool; that combination is verified here on Linux and nowhere else. The site it produces is identical for every reader. | Capturing on three platforms would triple the publication cost to produce three near-identical images of a UI that is deliberately identical across platforms (Principle VI), and would make FR-011d's determinism a three-way problem instead of a one-way one. |

# Phase 0 — Research: Published documentation site

Every decision the plan rests on, with what was rejected. Nothing here is left as
NEEDS CLARIFICATION; where a decision carries a cost, the cost is stated rather than hidden.

---

## 1. Site generator

**Decision**: mdBook, configured to build from a staged copy of `docs/`, with a fully custom theme
and its built-in (elasticlunr) client-side search.

**Rationale**: Four requirements decide this together — Markdown already in the repository (FR-001),
full-text search (FR-026), a theme that can be *replaced* rather than tinted (FR-029), and zero
third-party requests at page load (SC-015, FR-031). mdBook satisfies all four out of the box: it is
a single Rust binary, it rewrites inter-page `.md` links to `.html` so FR-005 is largely satisfied
by construction, and its search index ships as a static file with no service behind it. Its theme is
a directory of Handlebars templates and CSS that a book may override wholesale, which is what
FR-029a's "theme, not reimplementation" asks for.

**Alternatives considered**:

- *Hugo / Zola* — both capable and both fast, but both want the prose to move into their own
  content layout with front matter. That forks `docs/` in all but name and breaks the spec's
  "source of truth: `docs/`, unchanged".
- *Docusaurus / VitePress* — the richest themes, and the ones most likely to fetch a font or an
  analytics script by default. A Node toolchain for the whole site, in a Rust workspace, to obtain
  features (versioned docs, i18n, blog) explicitly out of scope.
- *Rendering the Markdown ourselves* (pulldown-cmark + templates) — attractive for about an hour.
  It means writing a search index, a navigation model and a theme engine to arrive at mdBook.
- *GitHub Pages' built-in Jekyll* — no search, and its theme system fights a bespoke design system
  rather than hosting one.

**Cost recorded**: mdBook insists on `SUMMARY.md` at the root of its source directory and on all
content sitting beneath it. Hence the staging step (§2). One further wrinkle: mdBook treats a
directory's `README.md` as that directory's index, which is why `docs/README.md` becomes the site's
home page rather than a new file competing with it.

---

## 2. Staging, and where the two new pages live

**Decision**: `site/stage.sh` copies `docs/` into a throwaway `src/` tree, then performs three
substitutions before mdBook runs: the version identifier (FR-006), the release's actual asset list
on the installation page (FR-004a), and the expansion of media directives into figures (§5). The
prose in `docs/` is never edited by the build. The home page is `docs/README.md`, extended to state
what the application is and to route to installation and the user guide (FR-003); the installation
page is a new `docs/install.md` (FR-004).

**Rationale**: The spec's Assumptions require that where the site needs content the repository does
not have, the content is added *to the repository*, not to a site-only template. Both new pages are
genuine documentation and read correctly on GitHub as well as on the site. Substitution has to
happen somewhere, and doing it in a staging copy is the only option that leaves the committed prose
free of build placeholders' side effects while keeping FR-001 literally true.

**Alternatives considered**: an mdBook preprocessor (a subprocess mdBook pipes JSON through) would
do the same substitutions inside the build. Rejected for this round: it is a second executable to
build and test for a transformation a shell step already performs legibly, and its input is the
whole book as JSON rather than files on disk, which makes the media-reference check harder to run
independently. Worth revisiting if the substitutions grow.

**Cost recorded**: the staging tree is a second copy of the prose for the duration of the build.
It is deleted with the run and never published as a source.

---

## 3. Deriving the theme from the application's tokens

**Decision**: a new pure module `crates/micold-core/src/tokens/css.rs` turns the token set —
`Roles` (colour), the type scale, shape, elevation and motion — into a sheet of CSS custom
properties, one block for the light scheme and one for the dark. A five-line binary
`crates/micold-core/src/bin/micold-tokens-css.rs` prints it; `site/build.sh` writes it to
`site/theme/css/tokens.css`, which is generated on every build and never committed as truth.
`site/theme/css/site.css` — the documentation furniture — is written entirely against those
variables and contains no literal colour, radius, duration or type size.

**Rationale**: FR-030 requires derivation rather than restatement, and the tokens already live in
the render-free core precisely so they can be read without a display. This makes "the site looks
like the application" a property of the build: a token changed for the application's own reason
reaches the site at the next publication, with no second edit anywhere. It also makes FR-014's
sibling requirement, FR-033, checkable — see §11.

**Alternatives considered**:

- *A hand-written stylesheet copying the palette* — exactly what FR-030 forbids, and the failure
  mode is silent: the two drift apart between releases and nobody notices until a screenshot looks
  wrong on its own page.
- *Emitting from `micold-client`* — the tokens are in `micold-core`; reaching through the GUI crate
  to read them would drag iced into a tool that needs no renderer.
- *A build script writing the CSS into the crate* — makes the generated file look committed and
  invites someone to edit it.

**Enforcement of SC-014**: `site.css` is grepped for literal colours, `px`/`rem` type sizes,
`border-radius` values and transition durations. A literal is a failure; the fix is to add the
missing variable to the emitter. That check is cheap and it is what stops the derivation from being
80% true.

---

## 4. Fonts, icons, and the monospace exception

**Decision**: `assets/fonts/Roboto-Regular.ttf`, `Roboto-Medium.ttf` and
`MaterialSymbolsOutlined.ttf` are copied into the site and served from it with `@font-face`, along
with `LICENSE`, `LICENSE-Roboto-OFL.txt` and `PROVENANCE.md`, surfaced on the site's licence page
(FR-031, FR-008). Code blocks and terminal output use `ui-monospace, SFMono-Regular, Menlo,
Consolas, "Liberation Mono", monospace` — the reader's own font, nothing shipped (FR-031a).

**Rationale**: these are the exact files the application renders with, so the site and a screenshot
of the application inside it are set in the same faces. Serving them locally is what makes SC-015
("0 requests to a third-party host") hold; Google Fonts would break it in one line. The monospace
stack mirrors the application's own choice for its terminal, which ships no monospaced face either.

**Cost recorded**: the three font files total ~1.2 MB. They are cached across pages and are outside
the per-page still-image budget of FR-015c, which is about images. Subsetting Material Symbols to
the glyphs the site actually uses is a later optimisation, not a requirement.

---

## 5. Referencing media from the prose

**Decision**: a guide page refers to a capture by an HTML comment directive —
`<!-- media: worktree-sidebar-dark -->` — which `stage.sh` expands into the figure markup. The
manifest `site/media.toml` holds each capture's id, its scene script, its scheme, its alt text and
its caption. `site/checks/media-references.sh` compares the two directions: a directive naming an
id the manifest does not define fails, and a manifest entry no page references fails.

**Rationale**: this solves three problems at once. It satisfies FR-022 — the reference and the
thing that produces it are checked against each other before merge, not discovered at publication.
It keeps the prose readable on GitHub, where the media does not exist: a comment renders as nothing,
whereas a committed `![](media/...)` link would render as a broken image on every page. And it puts
the alt text (FR-014) beside the capture definition, so a new capture cannot be added without one.

**Alternatives considered**: ordinary Markdown image links with the files generated into place.
Rejected because of the broken-image problem above and because nothing then forces alt text to
exist. A front-matter block per page was rejected because `docs/` has no front matter and adding it
is the "prose must move" objection from §1.

**Cost recorded**: `site/media.toml` is code, not documentation, so editing a caption or an alt
text takes the full three-platform pipeline. That is the correct trade: the file is read by the
capture driver, so it is a build input, and a build input that could skip the build is exactly what
feature 023's declaration exists to prevent.

---

## 6. Capturing stills

**Decision**: the route the repository already verified for its manual visual passes — a private
`Xvfb` display, Mesa's lavapipe software Vulkan rasteriser, `xdotool` to size and drive the window,
ImageMagick's `import` to capture. Each run gets its own short `XDG_RUNTIME_DIR` and `XDG_DATA_HOME`,
and the binaries are copied out of the target directory before launch.

**Rationale**: the spec names this route as a dependency, and it is verified rather than assumed —
including the parts that are counter-intuitive and cost time when rediscovered: `WGPU_BACKEND=gl`
fails on Xvfb (no usable GLX) and Vulkan-via-lavapipe is required; `env -u WAYLAND_DISPLAY` is
required or winit ignores `DISPLAY` entirely; there is no window manager, so `xdotool windowfocus`
must precede any key event; and the client and daemon must come from one build or the client
refuses the daemon over a protocol schema hash while printing matching version numbers.

**Explicitly not used**: `mise run screenshot`. It captures the whole logged-in desktop through
Mutter's ScreenCast API, cannot target a window, and is documented as not being an automated gate.
It is the wrong instrument twice over — it would put a developer's desktop into published media,
which FR-013 forbids outright.

**Cost recorded**: on a CI runner, none of the above is a hypothetical — the runner needs `xvfb`,
`mesa-vulkan-drivers`, `xdotool`, `imagemagick`, `ffmpeg` and the application's own X11/Wayland
development libraries installed before capture.

---

## 7. Clips: what "animated GIF" becomes, and why

**Decision**: not a GIF. Each clip is a sequence of **deterministically captured still frames**,
encoded to a muted, looping video (H.264 MP4 with a VP9/WebM sibling) and shown behind a poster
image — its own first frame — with a visible play control. Nothing is fetched until the reader
presses play.

**This is a deviation from the words in the original request** ("gifs from running application") and
is flagged rather than buried. Three requirements make a literal GIF unusable:

1. **FR-015c** caps a clip at 3 MB. A 15-second GIF of a 1600×1400 application window is tens of
   megabytes; getting one under 3 MB means dropping to a few frames per second at a quarter size,
   which is illegible for a UI. The same content as H.264 is comfortably under the cap at full size.
2. **FR-015a** requires a still first frame with a play control, and a clip that does not start on
   its own. A GIF has no play control and no pause — it animates as soon as it is fetched.
3. **FR-028** requires that no moving content is fetched until the reader starts it. A GIF is one
   file; requesting the poster requests the animation.

The result is what a reader would call an animated clip. It is what the spec has said since its
first clarification session ("animated clips", never "GIF"); this note records that the word in the
original request was answered rather than dropped.

**Why captured frames rather than a screen recording**: FR-011d requires two publications of the
same version to produce the same frames. `ffmpeg -f x11grab` samples on a wall clock, so it cannot.
Driving the application step by step and capturing after each step is deterministic by construction,
and encoding a known frame list at a fixed rate keeps it so.

**Cost recorded, and it is a real one**: a step-captured clip shows the *states* of an interaction,
not the transitions between them. The application's dialog fades and sidebar slides will not appear
in published media. This is the same limitation the repository's visual-pass route already records
("a screenshot pipeline cannot reliably catch a chosen frame of a 150 ms transition"), and the honest
framing is that the site shows what the application does, not how it eases. If showing the easing
ever matters more than determinism, FR-011d is the requirement to revisit — deliberately, not by
letting a recorder drift.

---

## 8. What capture drives: the demonstration project and the stub CLI

**Decision**: `site/capture/demo-project.sh` builds a synthetic git repository from scratch inside
the run's own temporary directory — fabricated file names, fabricated branches, a fabricated project
name — and the client's known-projects list is seeded to point at it, since the client binary takes
no arguments. `site/capture/stub-cli.sh` is installed on the session's `PATH` under the provider's
name (`claude`, `copilot`) and replays a canned transcript from `site/capture/transcript/`.

**Rationale**: FR-011b requires independence from the capture machine's state and FR-013 forbids any
personal content in published media; building the subject makes both true by construction rather
than by scrubbing afterwards. FR-011c requires session media to be produced with no AI CLI installed
and no credential; a stub on `PATH` means no network, no secret, and the same bytes every run — while
the terminal doing the rendering is the application's own emulator, so the media shows the real
component, not a mock-up of it.

**Determinism checklist** (FR-011d), each item a thing that would otherwise vary between runs:

- fixed window geometry, fixed display size, fixed scheme per capture;
- the demonstration repository built with fixed commit metadata (fixed author, fixed timestamps) so
  git output is stable;
- no clock, no network and no host path visible in any captured view;
- the stub replays its transcript at step boundaries, not on a timer;
- captured PNGs written without embedded timestamps, and clips encoded with ffmpeg's bit-exact flags.

**How it is verified**: the quickstart's Part A runs capture twice and compares frame hashes. Two
runs producing different hashes is a bug in a scene, not an acceptable variance.

---

## 9. What triggers a publication — and the trap in the obvious answer

**Decision**: `.github/workflows/pages.yml` declares `on: workflow_call` and `on: workflow_dispatch`.
`release.yml` gains a final job that *calls* it after `publish` succeeds. It does **not** listen for
`on: release: [published]`.

**Rationale**: this is the one place where the obvious design silently does not work. `release.yml`'s
`publish` job flips the draft release to published with `gh release edit --draft=false`, using the
workflow's own `GITHUB_TOKEN` — and GitHub does not start a new workflow run from an event raised by
`GITHUB_TOKEN`. A `pages.yml` listening for `release: published` would therefore never fire on a real
release, and would appear to work in every manual test, which is the worst possible failure shape for
a release-time job. Calling the reusable workflow from the same run avoids the event entirely, keeps
the ordering explicit (`needs: [release-please, publish]`), and makes a publication failure show up
as a failed job in the release run (FR-018).

**Alternatives considered**:

- *A personal access token or a GitHub App token on the `publish` step* — works, and adds a secret
  to rotate plus a token with `contents: write` outside the run's least-privilege model. Rejected;
  `release.yml` deliberately elevates one permission per job.
- *`gh workflow run` from the publish job* (`workflow_dispatch` is one of the exceptions to the
  rule above) — works too, but splits one release into two runs, so the release can report success
  while the site build has not started, and the two have to be correlated by hand.

**FR-017's manual republish** is the `workflow_dispatch` entry point on the same file, so a
republish runs the identical job definition as a release publication — which is what FR-020a means
by "differ only where the documentation was corrected".

---

## 10. Republish semantics, overlap, and the first publication

**Decision**: `pages.yml` takes two inputs. `release_tag` names the version to build the application
from and to label every page with; it is required. `docs_ref` names the ref the prose comes from and
defaults to `release_tag`. A release publication passes only `release_tag`, so both come from the
tag. A manual republish defaults `docs_ref` to the default branch, which is how a documentation
correction merged after a release reaches the site without cutting one.

**The tension, named**: a republish with `docs_ref: main` can pull prose describing something the
released build does not have. The spec accepts this — its Out of Scope section says documentation
merged to `main` is unpublished "until a maintainer triggers the republish of FR-017" — and the
mitigation is that `docs_ref` is an explicit input a maintainer chooses, not a default that drifts.
A maintainer who wants the tag's prose passes the tag.

**Overlapping publications** (FR-019): `concurrency: { group: pages, cancel-in-progress: true }`.
A newer release cancels an in-flight older build, so the site ends up describing the newer release
rather than whichever build finished last. GitHub Pages' own deployment is likewise last-write-wins
within one environment.

**Failure** (FR-018): every check runs before the deploy step. A failed check fails the job, the
deploy never runs, and the previously published site is untouched — GitHub Pages serves the last
successful deployment. There is no partial-publish state to recover from.

**The first publication** (edge case): Pages must be enabled on the repository with source "GitHub
Actions" before the first run, which is a repository setting outside this change and is recorded as
a prerequisite in the quickstart. Until the first successful publication the address serves GitHub's
own 404, which satisfies "unpublished" — no empty shell is deployed, because nothing is deployed.

---

## 11. The checks, and where each one runs

Two rounds, deliberately. Everything that can be judged from the Markdown sources runs **before
merge** in CI's existing `docs` job; everything that needs a rendered page runs **before deploy** in
`pages.yml`.

| Check | Tool | Runs | Requirement |
|---|---|---|---|
| Page set: every `docs/**/*.md` has a `SUMMARY.md` entry, and vice versa | `page-set.sh` | pre-merge | FR-023, SC-008 |
| Media references resolve against the manifest, both directions | `media-references.sh` | pre-merge | FR-022, FR-011a |
| Internal links between documentation sources resolve | lychee | pre-merge | FR-021 |
| Internal links across the built site resolve | lychee | pre-deploy | FR-005, SC-003 |
| Derived token pairs meet WCAG AA contrast | `cargo test -p micold-core` | pre-merge **and** in the publication's own build | FR-033 |
| `site.css` contains no literal colour/size/radius/duration | `page-set.sh`'s sibling grep | pre-merge | SC-014 |
| WCAG 2.2 AA on every rendered page, both schemes | axe-core via Playwright | pre-deploy | FR-027a, SC-013 |
| Home page names the product, shows it, links install within the first screen | `page-checks.mjs` | pre-deploy | FR-023a, SC-001 |
| Every page reachable from every page in ≤2 steps | `page-checks.mjs` | pre-deploy | FR-023a, SC-006 |
| A search for each guide topic returns that topic's page first | `page-checks.mjs`, driving the site's own search box | pre-deploy | FR-023a, SC-006 |
| Per-page still total ≤ 1 MB; each clip ≤ 3 MB | `media-budget.sh` | pre-deploy | FR-015c, SC-012 |
| Every declared capture was produced | `build.sh` | pre-deploy | FR-011a, SC-004 |
| No `<img>`, `<link>`, `<script>` or `url()` points off-origin | `page-checks.mjs` | pre-deploy | SC-015, FR-031 |

**Why the pre-merge checks are shell and not Rust**: `crates/micold-core/tests/documentation_is_not_read.rs`
scans Rust sources under `crates/` and fails on any string literal resolving to a path marked
`micold-docs`. A Rust test that opened `docs/SUMMARY.md` would trip it — correctly, since the
documentation-only skip rests on no *test* reading documentation. The `docs` job is the right home
for these instead: it is the one job feature 023 deliberately keeps running on documentation-only
changes, so a check placed there is never skipped by the exemption it must survive.

**Why the new checks extend the existing `docs` job rather than adding a new one**:
`crates/micold-core/tests/ci_gate_covers_every_job.rs` asserts that every job in `ci.yml` appears in
`ci-complete`'s `needs:`, and the default branch's ruleset requires the `ci complete` check by name
from outside the repository. Extending `docs` adds steps and changes neither.

**Tool choices**: lychee is a single Rust binary that checks Markdown and HTML, understands
fragments, and can be restricted to internal links so a publication never fails because someone
else's website is down. axe-core is the reference implementation of the WCAG rule set and reports
per-rule, which makes a violation actionable rather than a score.

---

## 12. Contrast: caught twice, on purpose

**Decision**: `crates/micold-core/tests/site_theme_contrast.rs` asserts that every foreground /
background pair the emitter produces meets WCAG 2.2 AA (4.5:1 for body text, 3:1 for large text and
UI boundaries) in both schemes. It runs in the ordinary suite, so a token change that would break the
site fails at merge — in the change that made it — rather than at the next release.

**Rationale**: FR-033 requires the *publication* to fail on such a conflict, and it will, because
axe-core sees it on the rendered page. But finding it at publication means finding it after the
release is already out. The Rust test computes the same ratios from the same numbers and reports the
offending token by name, which is where the fix belongs. FR-033's insistence that the site must not
quietly substitute a colour is what makes both checks correct: neither has a repair path, only a
report.

**Alternatives considered**: only checking at publication (too late, as above); only checking in
Rust (misses anything composited — a state layer over a surface, a shadow under text — which is
exactly what a rendered check is for). Both, cheaply, is the answer.

---

## 13. Installation page and the release's actual assets

**Decision**: `docs/install.md` carries the prose — what the release contains, how to install a
`.deb`, and build-from-source instructions for macOS and Windows with a plain statement that no
packaged build exists for them yet. The concrete download list is substituted at staging time from
the release's actual assets, queried from the release the publication is building.

**Rationale**: FR-004a forbids naming a file the release does not contain. Hand-maintained download
links go stale at the first release whose asset naming changes; querying the release makes the page
correct by construction. The prose stays in the repository (FR-001); only the list is generated.

**Cost recorded**: on GitHub, `docs/install.md` shows the placeholder rather than a link list. It is
one line of "the current release's downloads are listed on the site", which is honest — the
repository's own release page is one click away there anyway.

---

## Open items deliberately left to `/speckit-tasks`

- The exact scene list: which four views become stills and which three-to-four interactions become
  clips. It is a content decision, it is bounded by `site/media.toml`, and it does not change any
  structure above.
- Whether Material Symbols is subsetted for the site (§4) — an optimisation, measurable after the
  first build.
- The precise `book.toml` and `index.hbs` contents, which follow from
  [contracts/theme-variables.md](./contracts/theme-variables.md).

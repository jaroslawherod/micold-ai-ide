# Quickstart — Published documentation site

How to run this feature end to end, what each check proves, and the half no check can prove.

**Part A** is automated and runs in CI and in the publication. **Part B** is the judgement a reader
makes; it is recorded once per release, against the published site (FR-023a).

---

## Prerequisites

**Once per repository, outside this change**: GitHub Pages enabled with source **GitHub Actions**.
Until that is done, no publication can deploy and the address serves GitHub's own 404.

**On a Linux machine, to run a publication locally**:

```sh
sudo apt-get install -y xvfb mesa-vulkan-drivers xdotool imagemagick ffmpeg \
  libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev
cargo install mdbook lychee --locked
npm i -g @axe-core/playwright playwright && npx playwright install --with-deps chromium
```

macOS and Windows can run everything in Part A except capture (§A5–A7): the display, the software
rasteriser and the window driver are Linux-verified here and nowhere else. That is a property of
the publishing host, not of the product — see plan.md's Complexity Tracking.

---

## Part A — automated

### A1. The pre-merge checks, as CI runs them

```sh
site/checks/page-set.sh
site/checks/media-references.sh
site/checks/links.sh --sources
```

**Expected**: all three exit 0 and print what they checked.

**Prove they can fail** — each is only worth its exit code if you have watched it fail:

```sh
# Add a page with no SUMMARY.md entry
echo '# Scratch' > docs/user-guide/scratch.md && site/checks/page-set.sh   # expect: non-zero
rm docs/user-guide/scratch.md

# Reference a capture that is not declared
printf '\n<!-- media: no-such-capture -->\n' >> docs/user-guide/settings.md
site/checks/media-references.sh                                            # expect: non-zero
git checkout docs/user-guide/settings.md
```

Their fixture-driven counterparts live in `scripts/tests/` and run in the same job.

### A2. The theme is derived, not transcribed

```sh
cargo test -p micold-core tokens::css
cargo test -p micold-core --test site_theme_contrast
cargo run -p micold-core --bin micold-tokens-css | head -20
```

**Expected**: the emitted sheet defines every `--micold-*` variable in both the light and dark
blocks, and the contrast test passes over every pair the site uses.

**Prove the coupling is real** (FR-030 in one command): change a colour in
`crates/micold-core/src/tokens/palette.rs`, re-run the emitter, and confirm the emitted value
follows with no edit anywhere in `site/`. Revert.

**Prove SC-014 is enforced**: add `color: #ff0000;` to `site/theme/css/site.css` and run
`site/checks/page-set.sh` — expect non-zero. Revert.

### A3. Build the site without media

```sh
site/build.sh --no-capture --release-tag micold-ai-ide-v0.0.0-local
```

**Expected**: a built site under the staging output directory; every page present; the header is the
application's app bar; panels are separated by shade and shadow, not outlines.

### A4. Link and budget checks over the built site

```sh
site/checks/links.sh --built <output>
site/checks/media-budget.sh <output>
```

**Expected**: 0 broken internal links; every page under 1 MB of stills; every clip under 3 MB.

### A5. Capture, once

```sh
site/capture/display.sh start
site/build.sh --release-tag micold-ai-ide-v0.0.0-local
site/capture/display.sh stop
```

**Expected**: every entry in `site/media.toml` has a produced file. A declared capture that failed
fails the build — it is never published as a gap or filled from an earlier run (FR-011a).

**If the window never appears**, three causes account for nearly every occurrence, in order:
`WGPU_BACKEND=gl` instead of Vulkan-via-lavapipe; `WAYLAND_DISPLAY` still set, so winit ignores
`DISPLAY`; or a client and daemon from different builds, which the daemon logs as `refusing client:
contract or build mismatch` **while printing matching version numbers on both sides**.

### A6. Capture is deterministic (FR-011d)

```sh
site/build.sh --capture-only --out /tmp/cap-a
site/build.sh --capture-only --out /tmp/cap-b
diff <(sha256sum /tmp/cap-a/* | awk '{print $1}') <(sha256sum /tmp/cap-b/* | awk '{print $1}')
```

**Expected**: no difference. A difference is a bug in a scene — a timer instead of a settle, a clock
or a host path in frame, unfixed commit metadata in the demonstration project — not an acceptable
variance.

### A7. Capture is credential-free and contains nothing personal (FR-011c, FR-013)

**Expected, and checkable by inspection of the run**: no `claude` or `copilot` binary on the
capture `PATH` other than `site/capture/stub-cli.sh`; no credential file read; no network request;
no host path, real project name or window other than the application's own in any frame. The subject
is a repository the run built, so this holds by construction rather than by scrubbing.

### A8. Accessibility, the structural proxies, and third-party requests

```sh
node site/checks/page-checks.mjs <output>
```

**Expected**: 0 WCAG 2.2 AA violations on every page in both schemes; the home page names the
product, shows it and links installation within the first screen; every page reachable from every
page in ≤2 steps; each guide topic's own page first for its search; 0 off-origin requests.

### A9. A publication, end to end

Push a branch and run the workflow by hand:

```sh
gh workflow run pages.yml -f release_tag=<newest tag>
gh run watch
```

**Expected**: the site describes that release; a failure at any check means no deploy and the
previous site still serves.

**Prove FR-018**: introduce a deliberate broken link, dispatch, and confirm the run fails at the
link check and the live site is unchanged.

---

## Part B — the manual pass

Run **once per release**, against the published site, from a browser. Record the date, the version,
and what was and was not covered. These are the two criteria that rest on a reader's judgement; the
structural facts beneath them are already asserted in A8, and this is not a re-run of that.

### B1. A stranger can tell what this is (SC-001, FR-023a)

Open the site's root URL on a standard laptop viewport, cold. Without scrolling: can you say what
the application does, and can you get to the installation instructions?

**Record**: yes/no for each, and the seconds it took. Under 60 seconds is the criterion.

### B2. A reader can find a topic (SC-006, FR-023a)

Pick three topics at random from the user guide — say, revealing hidden agent worktrees, the
scrollback limit, running the service in a container. Find each from wherever you are standing,
by navigation or search.

**Record**: seconds for each. Under 30 seconds is the criterion.

### B3. One design language (FR-029, User Story 1 scenario 5)

Look at a page with a screenshot on it. Does the screenshot sit inside a page of the same product,
or does it read as a picture of one product inside another? Check the header against the
application's app bar, and check that panels are separated by shade and shadow rather than outlines.

**Record**: which page, which scheme, and the judgement.

### B4. Motion (FR-015a, FR-030b)

Load a page with clips and do nothing. **Expected**: nothing moves, and each poster tells you what
its section is about. Then start one: it loops, it is silent, it is under 15 seconds. Then set the
system to reduced motion and reload: the site's own transitions are gone, and the clips still do
not start on their own.

### B5. On a phone (FR-025)

Open two guide pages on a real phone. **Expected**: no horizontal scrolling, images fitted, text
readable without zooming.

### What this pass cannot answer

The clips are step-captured for determinism (research §7), so they show the application's *states*,
not its transitions. Do not read them as evidence about the application's easing or frame pacing —
the application's motion is covered by its own quickstart passes, not by this one.

---

description: "Task list for the published documentation site"
---

# Tasks: Published documentation site

**Input**: Design documents from `/specs/028-docs-site-github-pages/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY. The new Rust — one pure emitter and one contrast gate — lands after its failing test.
Each shell check ships with a `scripts/tests/<name>.test.sh` counterpart written first, driving the
script over fixture trees that contain the failure it exists to catch
([contracts/site-checks.md](./contracts/site-checks.md)).

**Documentation**: Per Constitution Principle VII, every user-facing story ships its documentation
in the same change. Here the documentation *is* the deliverable for US1–US3; US4 is developer-facing
and ships `docs/development/docs-site.md`.

**Cross-platform**: Per Constitution Principle VI, no application behaviour changes, so there is
nothing new to hold at parity. What must be preserved is the existing three-platform matrix and
feature 023's documentation-only skip — neither may change shape (FR-020). The new pre-merge checks
are **steps inside CI's existing `docs` job**, never a new job: `ci_gate_covers_every_job.rs` and the
out-of-repo ruleset's required `ci complete` check both depend on the job set staying as it is.

**Organization**: Tasks are grouped by user story. Priority order is US1 (P1), US2 (P1), US4 (P1),
US3 (P2).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

Repository root of the existing Rust workspace. Publication tooling in `site/` (code — deliberately
**not** in `.gitattributes`' `micold-docs` set), prose in `docs/` (documentation), the token → CSS
emitter in `crates/micold-core/`, shell-check tests in `scripts/tests/`, workflows in
`.github/workflows/`. See plan.md § Project Structure.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: The empty shape of the publication tooling, and the declaration that it is code.

- [X] T001 Create the publication tooling tree — `site/theme/css/`, `site/capture/scenes/`, `site/capture/transcript/`, `site/checks/` — and `site/README.md` stating that everything under `site/` is a build input and MUST NOT be added to `.gitattributes`' `micold-docs` set (plan.md § Structure Decision)
- [X] T002 [P] Write `site/book.toml`: `[book]` title/authors/language/`src = "build/src"`; `[output.html]` with `theme = "theme"`, `default-theme`, `preferred-dark-theme`, `git-repository-url`, `edit-url-template` (FR-007), `no-section-label = true`; `[output.html.search] enable = true` (FR-026); `[output.html.fold] enable = true`
- [X] T003 [P] Add `site-build` and `site-check` tasks to `mise.toml` wrapping `site/build.sh` and the `site/checks/` entry points, so the site is driven the way everything else in this repo is (CLAUDE.md)
- [X] T004 [P] Add the generated publication artifacts to `.gitignore`: `site/theme/css/tokens.css`, `site/build/`, `site/book/` — no media and no generated CSS is ever committed (FR-011, Out of Scope)
- [X] T005 [P] Create `site/media.toml` carrying the header comment from [contracts/media-manifest.md](./contracts/media-manifest.md) and no entries yet
- [X] T006 Record the publication toolchain in `site/README.md` — `xvfb`, `mesa-vulkan-drivers`, `xdotool`, `imagemagick`, `ffmpeg`, `mdbook`, `lychee`, Node 20 — as the single list `pages.yml` step 2 installs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The spine every story stands on — the derived theme, the staging pipeline, the mdBook
render, and the capture harness. Nothing story-specific is built here.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests first (MANDATORY — Constitution Principle I) ⚠️

- [X] T007 [P] Write the failing unit tests for the emitter in `crates/micold-core/src/tokens/css.rs` (`#[cfg(test)] mod tests`): every colour role, type-scale field, shape, elevation, motion and state token is emitted; every name appears in **both** the `:root` and the `:root[data-scheme="dark"]` block; names are `--micold-<group>-<token>` kebab-cased from the token identifier; units are emitted by the emitter (`#rrggbb`, `px`, `ms`, complete `cubic-bezier(...)`, complete `box-shadow`) per [contracts/theme-variables.md](./contracts/theme-variables.md)
- [X] T008 [P] Write the failing contrast gate `crates/micold-core/tests/site_theme_contrast.rs`: every foreground/background pair the site uses meets WCAG 2.2 AA (4.5:1 body, 3:1 large text and UI boundaries) in both schemes, naming the offending token on failure (FR-033)
- [X] T009 [P] Write the failing `scripts/tests/site-stage.test.sh`: over a fixture `docs/` tree, `site/stage.sh` copies prose without editing it in place, substitutes the version, expands a `<!-- media: id -->` directive into figure markup, and fails on a directive naming an id `site/media.toml` does not declare
- [X] T010 [P] Write the failing `scripts/tests/capture-harness.test.sh`: `site/capture/display.sh` starts and stops a private `Xvfb` with its own `XDG_RUNTIME_DIR`/`XDG_DATA_HOME`; `site/capture/demo-project.sh` produces a git repository with fixed commit metadata and no host path in it; `site/capture/stub-cli.sh` replays its transcript byte-identically twice (FR-011b, FR-011c, FR-011d)

### Implementation

- [X] T011 Implement `crates/micold-core/src/tokens/css.rs` — a pure function from the token set to the stylesheet text — and declare `pub mod css;` in `crates/micold-core/src/tokens/mod.rs` (T007 green)
- [X] T012 Implement `crates/micold-core/src/bin/micold-tokens-css.rs` — prints what `css.rs` returns, no decision logic — writing `site/theme/css/tokens.css` when redirected (FR-030)
- [X] T013 [P] Write `docs/SUMMARY.md` — the declared page set and reading order (FR-023): home (`README.md`), `install.md`, the 7 user-guide pages, `daemon.md`, then the 6 development pages as a secondary section (FR-002, Assumptions § Audience)
- [X] T014 Implement `site/stage.sh` — copy `docs/` into `site/build/src/`, copy `assets/fonts/` into `site/build/src/fonts/`, substitute the version, expand **still** media directives into `<figure><img …>` markup reading `alt`/`caption` from `site/media.toml` (clips arrive in US3) — per [contracts/media-manifest.md](./contracts/media-manifest.md) (T009 green)
- [X] T015 [P] Write `site/theme/index.hbs` — the mdBook page template carrying the app-bar header slot, the table of contents, the version line and the source link (filled in US1/US2)
- [X] T016 Write `site/theme/css/site.css` — documentation furniture written **only** against `--micold-*` variables, with `@font-face` for the three local font files and the system monospace stack `ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace` for code and terminal output (FR-031, FR-031a); no literal colour, size, radius, shadow or duration anywhere in the file (SC-014)
- [X] T017 Implement `site/build.sh` — emit tokens.css → capture → stage → `mdbook build` → run the pre-deploy checks, with a `--no-media` flag that skips capture for local iteration (quickstart A3)
- [X] T018 [P] Implement `site/capture/display.sh` — private `Xvfb`, `WGPU_BACKEND=vulkan` with `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`, `env -u WAYLAND_DISPLAY`, a private short `XDG_RUNTIME_DIR`, start/stop (research §6) (T010 green)
- [X] T019 [P] Implement `site/capture/demo-project.sh` — a synthetic git repository with fabricated names, branches and fixed commit metadata, plus the seeded `projects.json` the client needs to open it without a CLI (FR-011b, FR-013) (T010 green)
- [X] T020 [P] Implement `site/capture/stub-cli.sh` and `site/capture/transcript/claude-session.txt` — a program on `PATH` named as the provider that replays a canned transcript at step boundaries, never on a timer (FR-011c, FR-011d) (T010 green)
- [X] T021 Write `site/capture/scenes/lib.sh` — the shared scene helpers: copy both binaries out of the target directory in one build, launch client+daemon, fix window geometry with `xdotool windowsize`/`windowmove`, `windowfocus` before any key, force the scheme, capture with `import -window root`
- [X] T022 Implement `site/capture/capture.sh` — read `site/media.toml`, run each entry's scene at its scheme, write `site/build/src/media/<id>.png`, and fail if any declared entry produced no file (FR-011a, SC-004)
- [X] T023 Run `mise run test-core` and `site/build.sh --no-media`, confirming the theme is derived and the site renders with no media (quickstart A2, A3)

**Checkpoint**: Foundation ready — the site builds, dressed in the application's tokens, from the
repository's existing prose. User story work can begin.

---

## Phase 3: User Story 1 - A prospective user sees what the application is (Priority: P1) 🎯 MVP

**Goal**: A front door. A visitor opening the root URL sees the name, one paragraph, a screenshot of
the application, and one-click routes to installation and to the user guide — all within the first
screen, on a laptop or a phone, in the application's own design language.

**Independent Test**: Open the built site's root page with no prior knowledge of the project.
Confirm that within one screen the visitor can name what the application does, see it, and reach both
the installation instructions and the user guide — and that the installation page names only files
the published release actually contains.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [X] T024 [P] [US1] Write the failing `scripts/tests/page-checks.test.sh` with a fixture site whose home page hides the install link below the fold, and confirm `site/checks/page-checks.mjs` fails it (FR-023a, SC-001)
- [X] T025 [P] [US1] Extend `scripts/tests/page-checks.test.sh` with two more fixtures — a page fetching a stylesheet from another host, and a page with a missing `alt` and a failing contrast pair — and confirm the off-origin and axe-core assertions fail them (FR-027a, FR-031, SC-013, SC-015)

### Implementation for User Story 1

- [X] T026 [US1] Rewrite `docs/README.md` as the site's home page (FR-003): the product name, one paragraph on what it is, a media directive for the main window, and explicit links to `install.md` and to the user guide — while keeping it readable as the documentation index it already is on GitHub
- [X] T027 [P] [US1] Write `docs/install.md` (FR-004): the `.deb` install steps for Linux amd64 and arm64 linked to the newest release's downloads, and build-from-source instructions for macOS and Windows stating plainly that no packaged build exists for them yet
- [X] T028 [US1] Write `site/capture/scenes/main-window.sh` — the application at rest with the demonstration project open, captured at a fixed geometry in the scheme it is given
- [X] T029 [US1] Declare `main-window-light` and `main-window-dark` in `site/media.toml` with alt text and captions (FR-009, FR-012, FR-014)
- [X] T030 [US1] Add the `<!-- media: main-window-light -->` directive to `docs/README.md` and the dark counterpart to `docs/user-guide/appearance-theming.md`
- [X] T031 [US1] Add release-asset substitution to `site/stage.sh`: the install page's download links are resolved against the assets of `release_tag`, and staging **fails** if the page names a file the release does not contain (FR-004a)
- [X] T032 [US1] Implement the app-bar header in `site/theme/index.hbs` and `site/theme/css/site.css` — the application's top-app-bar surface role, elevation and title type, with panels separated by shade and shadow rather than outlines (FR-029, FR-029a); no application component without a documentation counterpart is recreated (FR-029b)
- [X] T033 [US1] Add the phone layout to `site/theme/css/site.css`: no horizontal scrolling at 360 px, images fitted to the viewport, the table of contents collapsing to a control (FR-025)
- [X] T034 [US1] Generate the licences page in `site/stage.sh` from `/LICENSE`, `assets/fonts/LICENSE`, `assets/fonts/LICENSE-Roboto-OFL.txt` and `assets/fonts/PROVENANCE.md`, and add its entry to `docs/SUMMARY.md` (FR-008, FR-031)
- [X] T035 [US1] Implement `site/checks/page-checks.mjs` with three of its five assertions — axe-core WCAG 2.2 AA over every page in both schemes, the home page's first-viewport facts, and the off-origin scan over `<img>`/`<link>`/`<script>`/`<source>`/CSS `url()` (T024, T025 green)
- [ ] T036 [US1] Link the site from the repository: add it to the root `README.md`, and set the repository's website field with `gh repo edit --homepage https://cumulocity-iot.github.io/micold-ai-ide/` (FR-008a)
- [X] T037 [US1] Run `site/build.sh` end to end locally and walk quickstart A5 and A7 — the home page's capture is produced, and the media contains no personal path, no real project name and no window but the application's own (FR-013, SC-010)

**Checkpoint**: The site has a front door with a real screenshot of the application on it, in the
application's own design language, and an installation page that promises only what the release
ships. This is the MVP.

---

## Phase 4: User Story 2 - A user reads the user guide for the version they installed (Priority: P1)

**Goal**: Every documentation page published, each stating the version it describes and linking its
source file, every internal link resolving, every page reachable from every other, and search that
finds the topic's own page first.

**Independent Test**: Publish from a release, then compare every user-guide page on the site against
the same file in the repository at that release's tag, confirm the version identifier is visible on
each page, and follow every internal link on the built site without reaching a missing page.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T038 [P] [US2] Write the failing `scripts/tests/links.test.sh`: over fixture trees, `site/checks/links.sh --sources` fails a link between documentation pages that does not resolve and a fragment naming no heading, `--built <dir>` fails the same in rendered HTML, and neither fetches an external URL (FR-021, FR-005)
- [X] T039 [P] [US2] Extend `scripts/tests/page-checks.test.sh` with a fixture site whose navigation buries a page three steps deep and whose search returns the wrong page first, and confirm the two new assertions fail it (FR-023a, SC-006)

### Implementation for User Story 2

- [X] T040 [US2] Implement `site/checks/links.sh` with both modes — `--sources` over `docs/**/*.md` for the pre-merge run and `--built <dir>` over the rendered HTML for the pre-deploy run — running lychee restricted to internal links and fragments (T038 green)
- [X] T041 [US2] Substitute the published version in `site/stage.sh` and render it in `site/theme/index.hbs`, so the identifier is visible on **every** page, not only the home page (FR-006, and the deep-arrival edge case)
- [X] T042 [US2] Wire the per-page source link through `book.toml`'s `edit-url-template` and `site/theme/index.hbs`, resolving against the published tag so the link lands on the file as published (FR-007)
- [X] T043 [US2] Make the navigation reachable by keyboard alone with a visible focus indicator in `site/theme/index.hbs` and `site/theme/css/site.css`, using the outline treatment the application reserves for focus (FR-024)
- [X] T044 [US2] Add the remaining two assertions to `site/checks/page-checks.mjs` — navigation depth (every page reachable from every other in ≤2 steps) and search (a query for each guide topic returns that topic's own page first, driven through the site's own search box) (T039 green, FR-023a, FR-026)
- [X] T045 [P] [US2] Write `site/capture/scenes/worktree-sidebar.sh` — a project open with the worktree sidebar listing three worktrees, the second selected
- [X] T046 [P] [US2] Write `site/capture/scenes/session-terminal.sh` — a session running in a worktree with the stub provider's coloured output in the application's own terminal (FR-011c)
- [X] T047 [P] [US2] Write `site/capture/scenes/settings-view.sh` — the settings view showing appearance, scrollback and session-service placement
- [X] T048 [US2] Declare the six remaining stills in `site/media.toml` — `worktree-sidebar-{light,dark}`, `session-terminal-{light,dark}`, `settings-view-{light,dark}` — each with alt text and a caption (FR-009, FR-012, FR-014)
- [X] T049 [US2] Add the directives to their pages: `docs/user-guide/worktrees-and-sessions.md`, `docs/user-guide/settings.md`, and both schemes of each appearance-relevant view to `docs/user-guide/appearance-theming.md` (FR-012)
- [X] T050 [US2] Run `site/build.sh` and then `site/checks/links.sh --built site/book` and `site/checks/page-checks.mjs site/book`, confirming 0 broken internal links and that navigation and search assertions pass (quickstart A4, A8; SC-003, SC-006)

**Checkpoint**: The whole documentation set is browsable, versioned, searchable and cross-linked,
with screenshots on the pages whose subject they are. US1 and US2 are both independently usable.

---

## Phase 5: User Story 4 - The site republishes itself when a release goes out (Priority: P1)

**Goal**: A release publishes the site with no human step; a failure is a failed check that leaves
the previous site standing; a maintainer can republish deliberately; and a pull request that breaks a
link, references an undeclared image or omits a page from the declared set fails **before** merge.

**Independent Test**: Trigger `pages.yml` by hand against an existing tag and confirm the live site
changes with no other action; then break a documentation link in a branch and confirm CI's `docs` job
fails on it while the three-platform matrix stays skipped.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

- [X] T051 [P] [US4] Write the failing `scripts/tests/page-set.test.sh`: over fixture trees, `site/checks/page-set.sh` fails a Markdown file with no `SUMMARY.md` entry, an entry naming a missing file, and a literal colour / size / `border-radius` / `box-shadow` / duration in `site.css` (FR-023, SC-002, SC-008, SC-014)
- [X] T052 [P] [US4] Write the failing `scripts/tests/media-references.test.sh`: `site/checks/media-references.sh` fails a directive naming an undeclared id, a manifest entry no page references, a `scene` naming a script that does not exist, and an empty `alt` (FR-022, FR-011a, FR-014)
- [X] T053 [P] [US4] Write the failing `scripts/tests/media-budget.test.sh`: `site/checks/media-budget.sh` fails a page whose stills total more than 1 MB and a clip file over 3 MB, reports the page and its assets, and never downscales (FR-015c, SC-012)

### Implementation for User Story 4

- [X] T054 [US4] Implement `site/checks/page-set.sh` (T051 green)
- [X] T055 [US4] Implement `site/checks/media-references.sh` (T052 green)
- [X] T056 [US4] Implement `site/checks/media-budget.sh` (T053 green)
- [X] T057 [US4] Add the completeness assertion to `site/build.sh`: every `[media.*]` entry has a produced file, and a missing capture is never published as a gap nor filled from a previous run (FR-011a, SC-004)
- [X] T058 [US4] Write `.github/workflows/pages.yml` as a reusable workflow — `workflow_call` + `workflow_dispatch` with `release_tag`/`docs_ref`, `concurrency: {group: pages, cancel-in-progress: true}` (FR-019), `permissions: {contents: read, pages: write, id-token: write}`, `environment: github-pages`, and the 13 ordered steps of [contracts/publication-workflow.md](./contracts/publication-workflow.md) with every check before the deploy (FR-018); third-party actions pinned to full commit SHAs
- [X] T059 [US4] Add the `pages` job to `.github/workflows/release.yml` — `needs: [release-please, publish]`, gated on `release_created`, calling `./.github/workflows/pages.yml` with the published tag. It MUST NOT be an `on: release: [published]` trigger: `GITHUB_TOKEN` raises that event and GitHub will not start a run from it (FR-016, research §9)
- [X] T060 [US4] Add the three pre-merge checks as **steps inside the existing `docs` job** of `.github/workflows/ci.yml` — `page-set.sh`, `media-references.sh`, `links.sh --sources` — adding no job, so `ci-complete`'s `needs:` and the required `ci complete` check name are untouched (FR-021, FR-022, FR-023, FR-020)
- [X] T061 [US4] Assert the documentation-only skip survives: `git check-attr micold-docs site/media.toml site/checks/page-set.sh` reports the paths as code, `scripts/tests/documentation-set.test.sh` still passes, and `scripts/classify-change.sh` on a docs-only diff still classifies it as documentation (FR-020, quickstart A1)
- [X] T062 [US4] Write `docs/development/docs-site.md` — how a publication works, what each check catches, how to trigger a republish, and why the trigger is a reusable workflow rather than a release event — and add its `docs/SUMMARY.md` entry (Principle VII)
- [ ] T063 [US4] Enable GitHub Pages on the repository with source **GitHub Actions** (a repository setting, outside this change; until the first successful publication the address serves GitHub's own 404, which is the "unpublished" edge case)
- [ ] T064 [US4] Run `pages.yml` via `workflow_dispatch` against an existing tag and confirm the site deploys, then break a check deliberately and confirm the run fails with the previous site still reachable (quickstart A9; FR-017, FR-018, FR-020a)

**Checkpoint**: The site publishes itself on release, republishes on demand, and a documentation
mistake is caught at review rather than at publication.

---

## Phase 6: User Story 3 - The site shows the application in motion (Priority: P2)

**Goal**: Short, click-to-play clips of the interactions that are hard to describe and obvious to
watch — nothing moving until the reader asks, nothing fetched until they press play.

**Independent Test**: Open the guide pages and confirm each headline workflow carries a screenshot or
a clip a reader can follow without the surrounding prose; load a page and confirm nothing has moved
and no video bytes were requested until a play control was pressed.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T065 [P] [US3] Write the failing `scripts/tests/clip-encode.test.sh`: over a fixture frame sequence, `site/capture/encode.sh` produces an `.mp4`, a `.webm` and a poster from the first frame, carries no audio track, runs under 15 seconds, and encodes bit-identically on two runs (FR-015, FR-011d)
- [ ] T066 [P] [US3] Extend `scripts/tests/page-checks.test.sh` with a fixture page carrying an `autoplay` video and one with `preload="auto"`, and confirm the no-autoplay assertion fails both (FR-015a, FR-028, SC-011)

### Implementation for User Story 3

- [ ] T067 [US3] Implement `site/capture/encode.sh` — a known frame list at a fixed rate to muted H.264 MP4 and VP9 WebM plus the poster, with ffmpeg's bit-exact flags (T065 green, research §7)
- [ ] T068 [P] [US3] Write `site/capture/scenes/create-worktree.sh` — creating a worktree and a session starting in it, captured step by step
- [ ] T069 [P] [US3] Write `site/capture/scenes/switch-session.sh` — switching between a session's terminal and a plain shell instance scoped to its worktree
- [ ] T070 [P] [US3] Write `site/capture/scenes/theme-follow.sh` — cycling the overflow menu's theme toggle Auto → Light → Dark
- [ ] T071 [P] [US3] Write `site/capture/scenes/open-project.sh` — the empty state to a project open in the main area
- [ ] T072 [US3] Declare the four clips in `site/media.toml` with `kind = "clip"`, alt text and captions, and extend `site/capture/capture.sh` to route clip entries through `encode.sh` (FR-010)
- [ ] T073 [US3] Extend `site/stage.sh` to expand a clip directive into the `<video controls loop muted playsinline preload="none" poster=…>` figure markup of [contracts/media-manifest.md](./contracts/media-manifest.md) — no `autoplay`, `preload="none"` (FR-015a, FR-015b, FR-028)
- [ ] T074 [US3] Add the fifth assertion to `site/checks/page-checks.mjs`: no element on any page moves without the reader starting it, and no `<video>` carries `autoplay` or a preload that fetches (T066 green, SC-011)
- [ ] T075 [US3] Add the clip directives to their pages — `docs/user-guide/worktrees-and-sessions.md`, `docs/user-guide/project-selection.md`, `docs/user-guide/appearance-theming.md` — each beside the prose it illustrates
- [ ] T076 [US3] Write `site/capture/verify-determinism.sh` and run quickstart A6: capture twice, compare frame and clip hashes, and treat any difference as a bug in a scene (FR-011d)

**Checkpoint**: All four stories are independently functional.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T077 [P] Remove every site transition under `@media (prefers-reduced-motion: reduce)` in `site/theme/css/site.css`, while keeping the motion tokens in force for everyone else (FR-030a, FR-030b)
- [ ] T078 [P] Record the decision not to subset `MaterialSymbolsOutlined.ttf` in `site/README.md` and `docs/development/docs-site.md`, with the ~1.2 MB font total it costs and the note that fonts are cached across pages and sit outside the per-page still budget (research §4)
- [ ] T079 Run `cargo fmt --check`, `cargo clippy` and `mise run test` — CI stops at `cargo fmt --check` before any other job, so the local gate is not the CI gate
- [ ] T080 Confirm the merge gate is unchanged: `cargo test -p micold-core ci_gate_covers_every_job` and `cargo test -p micold-core documentation_is_not_read` both pass with the new checks in place (Principle VI, FR-020)
- [ ] T081 Run the whole of [quickstart.md](./quickstart.md) Part A on a clean checkout, including the prove-they-fail commands for every check
- [ ] T082 Run [quickstart.md](./quickstart.md) Part B against the published site and record the judgement halves of SC-001 and SC-006 in the pass record (FR-023a)
- [ ] T083 Cross-cutting documentation review: `docs/SUMMARY.md` matches the page set, `docs/README.md` still reads as the repository's documentation index on GitHub, and `docs/development/ci-pipeline.md` mentions the new `docs`-job steps

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies — start immediately
- **Foundational (Phase 2)**: depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: depends on Foundational only
- **US2 (Phase 4)**: depends on Foundational; shares `page-checks.mjs` with US1, so T044 follows T035
- **US4 (Phase 5)**: depends on Foundational; its pre-merge wiring (T060) expects `links.sh` from US2 (T040) and its pre-deploy step list expects `page-checks.mjs` from US1/US2
- **US3 (Phase 6)**: depends on Foundational; extends `stage.sh` (T014), `capture.sh` (T022) and `page-checks.mjs` (T035, T044)
- **Polish (Phase 7)**: depends on all desired stories

### User Story Dependencies

- **US1 (P1)**: independent after Foundational. Delivers the MVP on its own.
- **US2 (P1)**: independent after Foundational. Does not need US1's home page to be testable — its own pages carry the version, the source link and the navigation.
- **US4 (P1)**: independent after Foundational, but only *useful* once there is something worth publishing; its pre-merge checks are testable the moment they exist.
- **US3 (P2)**: independent after Foundational. Adds clips to pages that already work without them.

### Within Each Story

- Tests are written and MUST fail before their implementation (Principle I)
- The emitter before the theme; the theme before the pages that wear it
- Scenes before the manifest entries that name them; manifest entries before the directives that reference them (`media-references.sh` checks both directions)
- Documentation ships with its story, not after it (Principle VII)

### Parallel Opportunities

- Setup: T002, T003, T004, T005 are four different files
- Foundational tests: T007, T008, T009, T010 are four different files and can be written together
- Foundational capture harness: T018, T019, T020 are independent scripts once T010 exists
- US2 scenes: T045, T046, T047 are three different scene scripts
- US3 scenes: T068, T069, T070, T071 are four different scene scripts
- Across stories: once Phase 2 is done, US1, US2 and US4's check scripts can proceed in parallel by different people; the two shared files (`page-checks.mjs`, `stage.sh`) are the only serialization points

---

## Parallel Example: Foundational

```bash
# The four failing tests, written together:
Task: "Unit tests for the emitter in crates/micold-core/src/tokens/css.rs"
Task: "Contrast gate in crates/micold-core/tests/site_theme_contrast.rs"
Task: "scripts/tests/site-stage.test.sh"
Task: "scripts/tests/capture-harness.test.sh"

# The capture harness, once its test exists:
Task: "site/capture/display.sh"
Task: "site/capture/demo-project.sh"
Task: "site/capture/stub-cli.sh + transcript"
```

---

## Implementation Strategy

### MVP First (US1 only)

1. Phase 1: Setup
2. Phase 2: Foundational (CRITICAL — blocks everything)
3. Phase 3: US1
4. **STOP and VALIDATE**: build the site locally and open its root page. A visitor can name the
   product, see it, and reach installation and the guide within one screen.
5. The site is worth publishing at this point even with only the home page's screenshot.

### Incremental Delivery

1. Setup + Foundational → the site renders from existing prose in the application's design language
2. + US1 → a front door with a real screenshot and an honest installation page (MVP)
3. + US2 → the whole documentation set, versioned, searchable, cross-linked, illustrated
4. + US4 → it publishes itself, and documentation mistakes are caught before merge
5. + US3 → the interactions that are hard to describe become watchable

US4 before US3 is deliberate: an unpublished site is a stale site, and clips are an enhancement to a
site that is already useful.

### Parallel Team Strategy

After Foundational: one person on US1 (home, install, theme treatments), one on US2 (navigation,
links, guide captures), one on US4 (checks and workflows). US3 joins once `stage.sh` and
`capture.sh` have settled.

---

## Notes

- `[P]` = different files, no dependencies on incomplete tasks
- The page set is 16 pages: `docs/README.md` (home), the new `docs/install.md`, 7 user-guide pages,
  `docs/daemon.md`, 6 development pages, and the generated licences page. plan.md's "13 documentation
  pages" undercounts `docs/development/` by one (`screenshots.md`); `docs/SUMMARY.md` and
  `page-set.sh` are the authority, and they check each other.
- Three open items plan.md deferred to this phase are settled here: the scene list (4 views × 2
  schemes = 8 stills in T028/T045–T047, and 4 clips in T068–T071), Material Symbols subsetting (no —
  T078), and the precise `book.toml`/`index.hbs` contents (T002, T015).
- Nothing under `site/` may be added to `.gitattributes`' `micold-docs` set. A build input that could
  skip the build is exactly what feature 023's declaration exists to prevent.
- No media file and no generated stylesheet is ever committed (T004).
- Verify tests fail before implementing. Commit after each task or logical group.
- `docs/install.md`'s `docs/SUMMARY.md` entry was written in T027, not T013, and `docs/SUMMARY.md`
  also carries a `[Licences](licences.md)` entry (T034) for a page that exists only in the staging
  tree. `page-set.sh` (T054) has to know that generated page, or it will call it a missing file.
- T036's second half is not done and cannot be done from here: `gh repo edit --homepage` would set
  the field on `jaroslawherod/micold-ai-ide`, which is what `origin` points at, while every URL in
  this feature names `Cumulocity-IoT/micold-ai-ide` — the repository the workspace `Cargo.toml`
  declares and the one plan.md targets. That repository is not reachable with the credentials here.
  Whoever administers it runs:
  `gh repo edit Cumulocity-IoT/micold-ai-ide --homepage https://cumulocity-iot.github.io/micold-ai-ide/`
  If the site is instead published from the fork, the address changes to
  `https://jaroslawherod.github.io/micold-ai-ide/` and has to be replaced in `site/book.toml`,
  `site/stage.sh`, `docs/install.md`, `docs/README.md`, `README.md` and the publication workflow.
- T037's walk of quickstart A5/A7 failed on the first run and cost three fixes to the capture
  harness, all of them in the frame rather than in the check: `scene_shot` grabbed the whole root
  window, so every still carried a black margin down two sides (it now crops to the window's own
  rectangle, read back from the server); the demonstration project lived under the run's private
  work directory, so the application's header published `/tmp/micold-cap-<uid>-<hash-of-a-worktree>/…`
  — a host path, and a different one on every machine (it is now the fixed, neutral
  `/tmp/micold-demo/aurora-fleet`, with `MICOLD_CAPTURE_PROJECT` to move it and an `flock` so two
  runs on one machine cannot rebuild it under each other); and the project had no worktrees at all,
  so the window showed "No worktrees yet" where `media.toml`'s alt text promised a worktree sidebar
  (`demo-project.sh` now creates three, named the way `naming.rs` derives them).


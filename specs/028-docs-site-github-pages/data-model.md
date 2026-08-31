# Phase 1 — Data model: Published documentation site

This feature stores nothing at runtime. Its "data" is the set of build-time entities the publication
reads, produces and checks. Each one below names where it lives, what it holds, what it relates to,
and the rules that must hold — with the requirement each rule comes from.

---

## Documentation page

One in-repo Markdown document and its published counterpart.

| Field | Source | Notes |
|---|---|---|
| `source_path` | the file's path under `docs/` | e.g. `docs/user-guide/settings.md` |
| `title` | the entry in `docs/SUMMARY.md` | the navigation label, not the H1 |
| `section` | position in `SUMMARY.md` | Home, User guide, Session service, Development, Licences |
| `order` | position in `SUMMARY.md` | reading order within its section |
| `published_url` | derived by mdBook | `source_path` with `.md` → `.html` |
| `version` | the publication's `release_tag` | rendered on every page (FR-006) |
| `edit_url` | derived from `source_path` + `docs_ref` | the "edit this page" link (FR-007) |

**Relationships**: references zero or more *Media assets* (by directive); linked to by zero or more
other pages; belongs to exactly one *Site navigation* entry.

**Rules**

- Every file matching `docs/**/*.md` has exactly one `SUMMARY.md` entry, and every entry names an
  existing file. Both directions, checked pre-merge. — FR-023, SC-002, SC-008
- Every internal link from the page resolves to another published page. — FR-005, FR-021, SC-003
- A link out of the published set (into `specs/`, into source) points at the repository, never at a
  site path that does not exist. — Edge case, FR-005
- Two pages are special because the site requires content the repository did not previously have:
  `docs/README.md` is the home page (FR-003) and `docs/install.md` is the installation page
  (FR-004). Both are ordinary documentation; neither is a site-only template. — FR-001

---

## Media asset

A still capture, or a clip together with its poster frame. Declared in `site/media.toml`, produced
during publication, never committed.

| Field | Where | Notes |
|---|---|---|
| `id` | manifest key | the name a page's directive uses, e.g. `worktree-sidebar-dark` |
| `kind` | manifest | `still` or `clip` |
| `scene` | manifest | the script under `site/capture/scenes/` that produces it |
| `scheme` | manifest | `light` or `dark` |
| `alt` | manifest | alternative text, required for every asset (FR-014) |
| `caption` | manifest | optional visible caption |
| `poster` | derived | a clip's first frame; a still has none |
| `bytes` | measured at build | checked against the budget |
| `version` | the publication | the build the frames came from |

**Relationships**: referenced by one or more *Documentation pages*; produced by exactly one scene
script; belongs to exactly one *Publication*.

**Rules**

- Every directive in the prose names a declared id, and every declared id is referenced by at least
  one page. Both directions, checked pre-merge. — FR-022, FR-011a
- Every asset carries non-empty alt text. — FR-014, SC-009
- Produced during the publication that ships it, from the build of the version being published. No
  asset survives from a previous publication. — FR-011, SC-004
- A declared asset that failed to be produced fails the publication; the page is never published
  without it or with an older copy. — FR-011a
- Reproducible: two publications of the same version produce identical frames. — FR-011d
- A clip is under 15 seconds, silent, loops, and does not start until the reader starts it; its
  poster is legible alone. — FR-015, FR-015a, FR-015b
- Still images total ≤ 1 MB per page; each clip ≤ 3 MB. Over budget fails the publication rather
  than being downscaled. — FR-015c, SC-012
- Contains nothing from a real desktop or account. Guaranteed by construction: the subject is a
  repository the capture script builds. — FR-013, FR-011b, SC-010
- Media of an AI session is produced with no CLI installed and no credential; the transcript is
  canned and the terminal is the application's own. — FR-011c
- Where a guide page's subject is appearance, the asset exists in both schemes. — FR-012

**State**: `declared` → `captured` → `encoded` (clips only) → `within budget` → `published`. Any
transition failing stops the publication; there is no degraded path.

---

## Capture scene

The script that drives the application to produce one or more assets.

| Field | Where | Notes |
|---|---|---|
| `script` | `site/capture/scenes/<name>.sh` | drives the real application |
| `produces` | the manifest entries naming it | one still, or an ordered frame list for a clip |
| `steps` | in the script | each step ends with a capture, never a timer |

**Relationships**: produces *Media assets*; runs against the *Demonstration project*; a session
scene depends on the *Stub provider CLI*.

**Rules**

- A step's capture happens after the driven action settles, not after a wall-clock delay. — FR-011d
- Runs on a private display with its own runtime and data directories, from binaries copied out of
  the shared target directory before launch. — FR-011b
- The client and daemon it launches come from one build. — practical: a mismatched pair is refused
  at handshake while reporting matching version numbers

---

## Demonstration project

The synthetic git repository every scene drives, built from scratch inside the run.

| Field | Notes |
|---|---|
| project name | fabricated; never a real path or user name |
| files, branches, worktrees | fabricated, fixed |
| commit metadata | fixed author and fixed timestamps, so git output is stable |
| registration | seeded into the run's own known-projects list, since the client takes no arguments |

**Rules**: created by the run, discarded with it, dependent on nothing on the host. — FR-011b, FR-013

---

## Stub provider CLI

A program named as the provider (`claude`, `copilot`), on the session's `PATH`, replaying a canned
transcript into the application's real terminal.

**Rules**: no network, no credential, no AI CLI installed; the same bytes every run; the terminal
emulator, its colour handling and its scrollback are the application's own. — FR-011c, FR-011d

---

## Design token set

The application's colour roles, type scale, shape, elevation and motion values, plus the font and
icon files it ships. One source, two consumers.

| Field | Source |
|---|---|
| colour roles (light and dark) | `crates/micold-core/src/tokens/palette.rs`, `mod.rs` |
| type scale | `tokens/typography.rs` |
| shape | `tokens/shape.rs` |
| elevation | `tokens/elevation.rs` |
| motion (durations, easing) | `tokens/motion.rs` |
| fonts and icons | `assets/fonts/` |

**Relationships**: consumed by the application's renderer, and by `tokens::css` which emits the
site's *Theme variables*.

**Rules**

- The site's presentation values are derived from this set, never restated. Changing a token changes
  the site at the next publication with no second edit. — FR-030, SC-014
- Motion values drive the site's own transitions, which are removed entirely under a reduced-motion
  preference. — FR-030a, FR-030b
- A derived value that would fail contrast fails a test at merge and the publication after it; no
  substitution. — FR-033
- Fonts and icons are served from the site with their licences. Nothing is fetched from a third
  party. — FR-031, FR-008, SC-015
- Code and terminal text use the reader's monospace stack; none is shipped. — FR-031a

---

## Theme variables

The generated stylesheet: CSS custom properties, one block per scheme. See
[contracts/theme-variables.md](./contracts/theme-variables.md) for the naming contract.

**Rules**: `site/theme/css/tokens.css` is generated on every build and is never authored by hand;
`site/theme/css/site.css` references only variables and contains no literal colour, type size,
radius or duration. — FR-030, SC-014

---

## Site navigation

The ordered structure a reader moves through: `docs/SUMMARY.md`.

**Rules**

- Declares the required page set in one place. — FR-023
- Every documentation page is reachable from every other without returning home, by keyboard, with a
  visible focus indicator. — FR-024, FR-027a
- Every page is reachable from every page in at most two steps; asserted on the rendered site. —
  FR-023a, SC-006
- The home page routes to installation and to the user guide within the first screen. — FR-003,
  FR-023a, SC-001

---

## Publication

One act of building the site and putting it live.

| Field | Source |
|---|---|
| `release_tag` | workflow input; the version built, captured and named |
| `docs_ref` | workflow input; defaults to `release_tag` |
| `trigger` | `workflow_call` from the release run, or `workflow_dispatch` |
| `started_at` | the run |
| `outcome` | success, or the first check that failed |

**Rules**

- Automatic on a published release, with no manual step. — FR-016, SC-005
- Deliberately triggerable without cutting a release. — FR-017
- A failure leaves the previous site intact and reachable, and surfaces as a failed check. — FR-018
- When two overlap, the newer release's site wins. — FR-019
- Does not depend on the merge pipeline's three-platform matrix having run. — FR-020
- Reproduces the site from the released version's source and build, so a republish differs only
  where documentation was corrected. — FR-020a
- Every check passes before the deploy step runs; there is no partial publish.

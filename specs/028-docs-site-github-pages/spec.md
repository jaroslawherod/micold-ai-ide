# Feature Specification: Published documentation site

**Feature Branch**: `docs/prepare-github-pages-for-iceflow`

**Created**: 2026-08-27

**Status**: Draft

**Input**: User description: "GitHub Pages documentation site for the project, published automatically on each release, built from the in-repo docs (primarily the user guide), and including screenshots and animated GIFs captured from the running application."

## Overview

Today the only way to read this project's documentation is to clone the repository — or to browse
`docs/` on GitHub, where the rendering is plain, there is no navigation between pages, no search,
and not a single picture of the application. Someone deciding whether to install Micold AI IDE has
a README and a `.deb`; someone who has installed it has a folder of Markdown files they have to
find.

The site is also the first thing a prospective user sees of the product's design, so it is dressed
in the product's own design system rather than a generic documentation theme — the application's
colours, type scale and shapes, generated from the same tokens the application renders from.

This feature publishes those same in-repo documents as a browsable web site with a front door, a
table of contents, working links, and — the part the repository cannot give them — screenshots and
short animated clips of the application actually running.

The documents stay where they are. The site is a *view* of `docs/`, not a second copy: Principle VII
requires documentation to live in-repo and ship with the code that it describes, and a site that
holds prose of its own would immediately begin to drift from the release it claims to describe.

## Clarifications

### Session 2026-08-27

- Q: The capture machine has no `claude`/`copilot` binary and no credentials — what should the
  session terminal show in the site's media? → A: A stub program on `PATH`, named as the provider,
  replays a canned session transcript into the real terminal — no network, no credentials,
  byte-identical every run.
- Q: The release ships only Debian packages today — what should the installation page say to a
  macOS or Windows visitor? → A: Document what the release actually ships (the `.deb` for Linux)
  and give build-from-source instructions for macOS and Windows, stating plainly that no packaged
  build exists for them yet.
- Q: Should animated clips start on their own when a page loads? → A: No. Each clip shows a still
  first frame with a visible play control; the reader starts it and it loops until they stop it.
- Q: What weight ceiling must a page's media stay under, enforced at build time? → A: Still images
  ≤ 1 MB total per page; each clip ≤ 3 MB. A publication whose media exceeds either ceiling fails.
- Q: Does the site have to meet a named accessibility standard, checked at build time? → A: WCAG 2.2
  Level AA, checked automatically on every published page at build time (contrast, alt text, heading
  order, keyboard reachability, focus visibility); a violation fails the publication.
- Q: How far does the resemblance to the application go — its tokens on documentation furniture, or
  its components rebuilt as web pages? → A: A documentation site wearing the application's design
  system: its tokens applied to normal documentation furniture, plus its app-bar header treatment
  and its surface/shade/shadow treatment for panels. Components with no documentation counterpart
  are not recreated.
- Q: What typeface should code blocks and terminal output use, when the application ships no
  monospaced font and the site may fetch nothing from a third party? → A: The reader's own
  monospaced font via the standard system stack — the same choice the application's terminal makes.
  Nothing shipped, nothing fetched.
- Q: Should the site's own transitions use the application's motion timing? → A: Yes — the site's
  transitions use the application's motion tokens (durations and easing), derived the same way its
  colours are, and are removed entirely for a reader whose system asks for reduced motion.
- Q: How are the two human-judgement success criteria verified? → A: Both halves — structural
  proxies checked automatically on every publication (home page names the product, shows it and
  links install within the first screen; every page reachable in ≤2 steps; a search for each guide
  topic returns that page first), plus the human judgement recorded once per release in the
  quickstart's Part B pass.
- Q: Where is the published site linked from? → A: The README and the repository's own website
  field. No application code is touched; linking the guide from the app's Help/About dialog is a
  separate change.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A prospective user sees what the application is (Priority: P1)

Someone arrives from a link, a search result, or the repository's README. They want to know, within
a few seconds, what this application looks like and whether it does what they need — before they
decide to download a package and install it.

They land on the site's home page: the name, one paragraph on what it is, a screenshot of the
application at rest, and a clear route to "install" and to the user guide.

**Why this priority**: This is the whole reason to publish anything. A documentation site nobody
can find their way into is the folder of Markdown files with extra steps. Everything else in this
spec is refinement of this journey.

**Independent Test**: Open the published site's root URL with no prior knowledge of the project.
Confirm that within one screen the visitor can name what the application does, see it, and reach
both the installation instructions and the user guide.

**Acceptance Scenarios**:

1. **Given** the site has been published, **When** a visitor opens its root URL, **Then** they see
   the project name, a one-paragraph description, and at least one screenshot of the running
   application without scrolling on a standard laptop viewport.
2. **Given** a visitor is on the home page, **When** they look for how to install the application,
   **Then** a link to the installation instructions is reachable in one click, and that page reaches
   the newest release's downloads.
3. **Given** a visitor on a platform the release ships no package for, **When** they open the
   installation page, **Then** they are told so plainly and given build-from-source instructions,
   rather than being offered a download that does not exist.
4. **Given** a visitor opens the site on a phone, **When** the page loads, **Then** text is
   readable and images fit the viewport without horizontal scrolling.
5. **Given** a visitor is looking at a screenshot of the application embedded in a page, **When**
   they take in the page around it, **Then** the two share one design language — the same colours,
   the same type, the same shapes — rather than a screenshot of one product sitting inside another.

---

### User Story 2 - A user reads the user guide for the version they installed (Priority: P1)

Someone has installed the application and wants to know how a feature works — how to reveal hidden
agent worktrees, how to run the session service in a container, what the environment-include script
does. They need the answer to match the build sitting on their machine.

**Why this priority**: The user guide is the substance of the site and the reason the feature was
asked for. Documentation that describes a different version than the reader is running is worse
than no documentation, because the reader trusts it.

**Independent Test**: Publish from a release, then compare every user-guide page on the site against
the same file in the repository at that release's tag, and confirm the site states which version it
describes.

**Acceptance Scenarios**:

1. **Given** the site is published, **When** a user opens any user-guide page, **Then** its prose
   is identical in substance to the corresponding file in `docs/user-guide/` at the published
   version.
2. **Given** a user is on any page of the site, **When** they look for which version they are
   reading about, **Then** the version identifier is visible on the page.
3. **Given** a user is reading one guide page, **When** they want another topic, **Then** every
   other documentation page is reachable from a persistent navigation element without returning to
   the home page.
4. **Given** a documentation page links to another documentation page, **When** the user follows
   that link on the site, **Then** it resolves to the corresponding published page and not to a
   missing page or to a raw file on GitHub.

---

### User Story 3 - The site shows the application in motion (Priority: P2)

Several things this application does are hard to describe and obvious to watch: creating a worktree
and having a session start in it, switching between sessions, a terminal rendering colored output,
the light/dark theme following the system preference. The user guide currently describes these in
prose alone.

**Why this priority**: This is the site's main advantage over reading `docs/` on GitHub, and the
user asked for it explicitly. It is P2 rather than P1 because the site is useful the moment the
prose is browsable, and moving pictures are an enhancement to that.

**Independent Test**: Open the site's guide pages and confirm each headline workflow carries either
a screenshot or a short animation that a reader can follow without the surrounding prose.

**Acceptance Scenarios**:

1. **Given** a guide page describes a multi-step interaction, **When** the reader reaches that
   section, **Then** a still frame of that interaction is shown with a control that plays it inline.
2. **Given** an animated clip is on the page, **When** the reader starts it, **Then** it is under
   15 seconds, loops until stopped, and needs no audio to be understood.
3. **Given** a reader who never starts any clip, **When** they read the page, **Then** nothing on it
   has moved, and each clip's still frame has told them what its section is about.
4. **Given** a reader is on a metered or slow connection, **When** a page containing animations
   loads, **Then** the page's text is readable before the animations have loaded at all.
5. **Given** any screenshot or clip on the site, **When** it is compared against the published
   version of the application, **Then** it depicts that version's interface and not an earlier one —
   because it was produced from that version's build during that version's publication.

---

### User Story 4 - The site republishes itself when a release goes out (Priority: P1)

A maintainer merges the release pull request. The release is created, packages are attached, and the
release is published. No further human step should be needed for the site to describe the version
that was just released.

**Why this priority**: An unpublished site is a stale site. Every documentation site that depends on
someone remembering to update it eventually describes a version nobody is running. This journey is
what makes User Story 2's guarantee hold over time rather than on the day it shipped.

**Independent Test**: Cut a release (or replay one) and confirm the live site changes to the new
version with no manual intervention, and that a failure to publish is visible rather than silent.

**Acceptance Scenarios**:

1. **Given** a release is published, **When** the release automation completes, **Then** the live
   site describes that release's version without any manual step.
2. **Given** the site build fails, **When** the release completes, **Then** the failure is reported
   as a failed check and the previously published site remains intact and reachable.
3. **Given** a documentation error is found after a release, **When** a maintainer needs the site
   corrected before the next release, **Then** a republish can be triggered deliberately without
   cutting a release, and it rebuilds the site — media included — from the released version.
4. **Given** a pull request changes documentation, **When** its checks run, **Then** a broken
   internal link or a missing referenced image fails the check before merge rather than after
   publication.

---

### Edge Cases

- **A guide page links to a file that is not published** (for example a link from a user-guide page
  into `specs/`, or into a source file). The site must either publish a resolvable target or send
  the reader to the repository — never to a dead page.
- **A release is published while a previous site build is still running.** The site must end up
  describing the newer release, not whichever build finished last.
- **A screenshot's subject was removed or renamed by the release being published.** The site must
  not silently show a control that no longer exists.
- **The first publication.** Before any release exists that carries this feature, the site's address
  must either be unpublished or state plainly that it is not yet available — it must not serve an
  empty shell.
- **Documentation-only changes.** The project deliberately skips its three-platform build pipeline
  when a change touches only declared documentation paths. Publication must remain possible for such
  a change, and adding this feature must not force every documentation typo through the full merge
  pipeline.
- **A reader arrives from a search engine on a deep page** rather than the home page, with no
  context about which version they are reading or how to reach the rest of the guide.
- **A design token changes and the site inherits it.** A colour adjusted in the application for a
  reason of its own arrives on the site at the next publication. If the new value fails contrast on
  the site, that must stop the publication and be fixed in the token (FR-033) — not worked around in
  a site stylesheet, which would put the two back out of step.
- **A capture comes out over budget.** A view that legitimately needs a large image, or an
  interaction that cannot be shown in 15 seconds under 3 MB, must fail the publication loudly enough
  to be fixed — by recapturing smaller, by splitting the interaction, or by raising the ceiling
  deliberately — rather than being published over budget or silently downscaled to illegibility.

## Requirements *(mandatory)*

### Functional Requirements

**Content**

- **FR-001**: The site MUST be generated from the documentation already in the repository. No page's
  prose may exist only on the site.
- **FR-002**: The site MUST publish every page of the user guide, and MUST additionally publish
  the developer documentation and the session-service document as a secondary section, so that every
  document in the repository's documentation set has a published counterpart.
- **FR-003**: The site MUST provide a home page that states what the application is, shows it, and
  links to installation instructions and to the user guide.
- **FR-004**: The site MUST provide installation instructions that describe what the published
  release actually contains. For each platform the release ships a package for, the page gives the
  install steps for that package and links to the newest release's downloads. For each supported
  platform the release does *not* yet ship a package for, the page gives build-from-source
  instructions and states plainly that no packaged build exists yet.
- **FR-004a**: The installation page MUST NOT name or link a downloadable file that the published
  release does not contain.
- **FR-005**: Every internal link between documentation pages MUST resolve to a published page on
  the site.
- **FR-006**: Each page MUST state the version of the application it describes.
- **FR-007**: Each page MUST link to the corresponding source file in the repository, so a reader
  who spots an error can propose a correction.
- **FR-008**: The site MUST carry the project's license and a link to the repository.
- **FR-008a**: The repository MUST point back at the site: the README links it, and the repository's
  own website field names it. A reader who finds the project before the site MUST be one click from
  the site.

**Media**

- **FR-009**: The site MUST include screenshots of the running application: at minimum the main
  window with a project open, the worktree sidebar, a session terminal, and the settings view.
- **FR-010**: The site MUST include animated clips of at least three multi-step interactions from
  the user guide.
- **FR-011**: Every screenshot and clip MUST be produced from the build of the version the site
  describes, during that version's publication. No published image may originate from a stored copy
  captured against an earlier version.
- **FR-011a**: The set of images the site requires MUST be declared, and a publication in which any
  declared image failed to be produced MUST fail rather than publish the page without it or with an
  older copy.
- **FR-011b**: Capture MUST drive the application against a demonstration project created for the
  purpose, with no dependence on any state of the machine performing the capture.
- **FR-011c**: Media showing an AI session MUST be produced without any AI CLI installed and without
  any credential. A stub program on the session's search path, named as the provider the session
  claims to run, replays a canned transcript into the real terminal. The terminal itself is the
  application's own — the emulator, its colour and styling, its scrollback — so the media shows the
  real component rendering scripted content, never a mock-up of it.
- **FR-011d**: Capture MUST be deterministic: two publications of the same version MUST produce the
  same frames. No captured image may depend on the network, on a clock, or on anything the capture
  environment did not itself create.
- **FR-012**: Screenshots MUST be shown in both the light and dark themes wherever the guide's
  subject is appearance.
- **FR-013**: Media MUST NOT contain anything from a real person's desktop or account — no personal
  file paths, real project names, credentials, or windows other than the application's own.
- **FR-014**: Every image MUST carry alternative text describing what it shows, so the guide remains
  usable to a reader who cannot see it.
- **FR-015**: Animated clips MUST be under 15 seconds and MUST carry no audio.
- **FR-015a**: Clips MUST NOT play on their own. Each is presented as a still first frame with a
  visible control that starts it; once started it loops until the reader stops it. Nothing on a page
  moves until the reader asks it to.
- **FR-015b**: A clip's still first frame MUST be legible on its own, so a reader who never starts
  it still sees what the section is about.
- **FR-015c**: A page's still images MUST total no more than 1 MB, and each clip MUST be no larger
  than 3 MB. A publication in which any page or clip exceeds its ceiling MUST fail rather than
  publish over budget — the ceiling is checked, not aspired to.

**Publication**

- **FR-016**: The site MUST be published automatically when a release is published, with no manual
  step.
- **FR-017**: A maintainer MUST be able to trigger a republish deliberately, without cutting a
  release.
- **FR-018**: A failed publication MUST leave the previously published site intact and reachable,
  and MUST surface as a failed check rather than silently.
- **FR-019**: When two publications overlap, the site MUST end up describing the newer release.
- **FR-020**: Publication MUST NOT depend on the project's three-platform test matrix having run
  for the change being published. A documentation-only change that skips that matrix MUST still be
  publishable. (Publication does build the application itself, for capture — see FR-011 — but that
  build is publication's own, not a precondition inherited from the merge pipeline.)
- **FR-020a**: A deliberate republish (FR-017) MUST reproduce the site from the released version's
  source and the released version's build, so that a republish and the original publication differ
  only where the documentation was corrected.

**Verification**

- **FR-021**: A pull request that breaks an internal documentation link MUST fail its checks before
  merge.
- **FR-022**: A pull request that references a screenshot or clip which the declared capture set
  does not produce MUST fail its checks before merge — the reference and the thing that produces it
  are checked against each other, not left to be discovered at publication.
- **FR-023**: The set of pages the site is required to publish MUST be declared in one place and
  checked, so a page added to the user guide cannot be silently absent from the site.
- **FR-023a**: The criteria that rest on a reader's judgement (SC-001, SC-006) MUST be verified in
  two halves. The structural facts they depend on MUST be asserted automatically on every
  publication: the home page names the product, shows it, and links the installation instructions
  within the first screen; every documentation page is reachable from every other in at most two
  steps; and a search for each guide topic returns that topic's own page first. The judgement those
  facts stand for MUST be recorded once per release in this feature's manual pass, against the
  published site.

**Reading experience**

- **FR-024**: The site MUST provide navigation to every documentation page from every documentation
  page, reachable by keyboard alone with a visible focus indicator.
- **FR-025**: The site MUST be readable on a phone: no horizontal scrolling, images fitted to the
  viewport.
- **FR-026**: The site MUST offer full-text search across its pages.
- **FR-027**: The site MUST honour the reader's light/dark preference, and MUST meet the contrast
  requirement below in both.
- **FR-027a**: Every published page MUST conform to WCAG 2.2 Level AA. Conformance MUST be checked
  automatically for every page on every publication — colour contrast, alternative text, heading
  order, keyboard reachability and focus visibility at minimum — and a violation MUST fail the
  publication rather than be recorded for later.
- **FR-028**: Page text MUST be readable before media has finished loading, and a page MUST NOT
  fetch a clip's moving content until the reader starts it.

**Appearance**

- **FR-029**: The site MUST present as the same product as the application: the same colour roles,
  the same type scale, the same corner shapes and the same way of separating surfaces by shade and
  shadow rather than by outlines. A reader who moves between a screenshot and the page around it
  MUST NOT see two different design languages.
- **FR-029a**: The resemblance is a **theme, not a reimplementation**. The site keeps documentation
  furniture — a table of contents, prose, code blocks, tables — and dresses it in the application's
  design system. Two treatments carry the resemblance beyond colour and type: the header is the
  application's top app bar, and panels are separated by shade and shadow at the application's
  elevation levels rather than by outlines.
- **FR-029b**: Application components with no documentation counterpart MUST NOT be recreated on the
  site. The site has no overflow menu, no chips, no worktree list — the reader sees those in
  screenshots, not in the page furniture around them.
- **FR-030**: The site's colour, type-scale, shape, elevation and motion values MUST be **derived
  from the application's own design tokens**, not restated by hand in a stylesheet. Changing a token
  in the application MUST change the site at the next publication, with no second edit anywhere.
- **FR-030a**: The site's own transitions — hover, focus, an expanding navigation section, switching
  theme — MUST use the application's motion durations and easing, so the site moves the way the
  application moves.
- **FR-030b**: A reader whose system asks for reduced motion MUST get none of those transitions.
  This is separate from FR-015a: clips never start on their own for anyone, and site transitions are
  removed for readers who have asked for that.
- **FR-031**: The site MUST use the typeface and icon set the application ships — the same Roboto
  and the same Material Symbols files in this repository — and MUST serve them from the site itself.
  No font, icon or stylesheet may be fetched from a third party at page load. The licences those
  files ship under MUST be carried on the site alongside the project's own (FR-008).
- **FR-031a**: Code blocks and terminal output MUST be set in the reader's own monospaced font,
  through the system stack — the same choice the application makes for its terminal, which ships no
  monospaced face either. The site MUST NOT ship or fetch one.
- **FR-032**: The site's light and dark presentations MUST be the application's light and dark
  schemes, so that a screenshot taken in either scheme sits on a page in the same scheme.
- **FR-033**: Where a value derived under FR-030 would fail the conformance requirement of FR-027a,
  the publication MUST fail. The site MUST NOT quietly substitute a different colour, and MUST NOT
  publish a page that fails conformance because a token was inherited faithfully — the conflict is
  reported so it can be resolved in the token, deliberately, for both the application and the site.

### Key Entities

- **Documentation page**: one in-repo Markdown document (a user-guide topic, a developer topic, or
  the session-service document) and its published counterpart. Attributes: source path, title,
  position in the navigation, the version it was published from.
- **Media asset**: a screenshot, or an animated clip together with its still first frame.
  Attributes: what interaction or view it depicts, which guide page(s) reference it, its theme
  (light/dark), the application version it depicts, its alternative text, and its size against the
  ceiling of FR-015c.
- **Publication**: one act of building and putting the site live. Attributes: the version published,
  when, its outcome, and whether it was triggered by a release or by hand.
- **Site navigation**: the ordered structure a reader moves through — sections, page order, and the
  home page's entry points.
- **Design token set**: the application's colour roles, type scale, shape, elevation and motion
  values, and the font and icon files it ships. One source, two consumers — the application renders from it and
  the site's theme is generated from it (FR-030). The tokens cross over; the components do not
  (FR-029b).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A visitor who has never seen the project can, from the site's root URL, correctly
  state what the application does and reach the installation instructions in under 60 seconds.
  Verified in two halves (FR-023a): the structural facts this rests on are asserted on every
  publication, and the judgement itself is recorded once per release in the manual pass.
- **SC-002**: 100% of the repository's documentation pages — the user guide, the developer
  documentation, and the session-service document — are published and reachable from the site's
  navigation.
- **SC-003**: 0 broken internal links across the published site, verified on every publication.
- **SC-004**: 100% of the screenshots and clips on the site were produced during that site's own
  publication, from the build of the version it names. No image is carried over from a previous
  publication.
- **SC-005**: The site describes the newest release within 60 minutes of that release being
  published, with no human action. (The allowance accommodates building the application and driving
  it to capture the media; the requirement is that no human is in the loop, not that it is instant.)
- **SC-006**: A reader can find the guide page covering a given topic in under 30 seconds using the
  site's navigation or search. Verified the same way as SC-001 (FR-023a): every page is reachable in
  at most two steps and each guide topic's own page is the first search result — both asserted
  automatically — with the judgement recorded in the manual pass.
- **SC-007**: Any documentation page loads and is readable in under 3 seconds on a typical broadband
  connection, its still images included. Clips are not fetched until started, so they are outside
  this measurement by construction.
- **SC-008**: A contributor who adds a user-guide page and merges it sees it on the site at the next
  publication without editing any site-specific list by hand, or is told at review time that they
  must.
- **SC-009**: 100% of images carry alternative text.
- **SC-010**: 0 personal or non-application content appears in any published image.
- **SC-011**: 0 elements on any published page move without the reader having started them.
- **SC-012**: 0 published pages exceed 1 MB of still images, and 0 published clips exceed 3 MB —
  measured at publication, not sampled afterwards.
- **SC-013**: 0 WCAG 2.2 Level AA violations across the published site, in both the light and the
  dark presentation, verified on every publication.
- **SC-014**: 0 colour, type-scale, shape, elevation or motion values in the published site's
  presentation that were not derived from the application's design tokens — checked, so the two
  cannot drift apart between releases.
- **SC-015**: 0 requests to a third-party host from any published page.

## Assumptions

- **Hosting**: GitHub Pages on the project's own repository, served over HTTPS at the default
  `github.io` address. No custom domain is assumed; adding one later must not invalidate anything
  in this spec.
- **Version model (decided)**: the site describes **the newest release only**. There is one site at
  one address, no version switcher, and no preview of the unreleased default branch — a reader is
  always reading about a version they can install. Documentation merged after a release appears at
  the next one, or sooner via the deliberate republish of FR-017.
- **Audience**: end users of the application first, contributors second. The user guide and the
  installation route are the site's spine; developer documentation is a secondary section, not the
  front door.
- **Source of truth**: `docs/` in this repository, unchanged. Publishing adds a rendering and a
  navigation structure; it does not fork the prose. Where a page is missing something the site
  needs (a home page, installation instructions), that content is added *to the repository* as a
  documentation page, not to a site-only template.
- **Release cadence**: releases are cut by the existing release automation from the default branch,
  frequently enough that "published at release" is not a long wait. FR-017's manual republish is the
  escape hatch when it is.
- **Media capture (decided)**: screenshots and clips are **captured during publication**, from the
  build of the version being published, on a private headless display — never captured by hand and
  committed. The repository already has a verified route for driving the application headlessly and
  capturing what it draws (the manual visual pass); this feature builds on that route rather than
  inventing a second one. Capture drives a synthetic demonstration project created for the purpose,
  never a real checkout, which is what makes FR-013 hold by construction.
- **Publication builds the application**: because capture runs against the released build, publishing
  the site requires building the application on the publishing machine. This is accepted; it is the
  price of FR-011 being a mechanical guarantee rather than an author's promise.
- **Localisation**: English only. Translated documentation is out of scope.
- **Shared design tokens**: the application's design tokens live in its render-free core, not in
  its GUI layer, so a value can be read out of them without a display. This feature assumes the
  site's theme is generated from that same source (FR-030) rather than transcribed — which is what
  makes FR-029 a property of the build instead of a promise made once and slowly broken. Publication
  already builds the application (above), so this costs nothing extra.
- **Accessibility (decided)**: WCAG 2.2 Level AA is the bar, and it is a build-time check rather
  than a review habit (FR-027a). The four rules stated elsewhere in this spec — alternative text,
  no motion the reader did not start, a phone-readable layout, and the reader's colour preference —
  are consequences of that bar, not a substitute for it.
- **Analytics**: none. The project is local-first and collects nothing; the site follows suit.
- **Comments/feedback**: readers propose corrections through the repository (FR-007). The site hosts
  no comment system.

## Out of Scope

- A version archive: keeping the sites of older releases browsable alongside the newest one. The
  site publishes the newest release and only the newest release. Readers on an older version read
  that version's documentation in the repository at its tag.
- A preview of the unreleased default branch. Documentation merged to `main` is unpublished until the
  next release, or until a maintainer triggers the republish of FR-017.
- Screenshots or clips committed to the repository as files. Media exists only as an output of
  publication.
- API reference documentation generated from source.
- A blog, changelog narrative, or release-notes microsite. The changelog already ships inside the
  application.
- Marketing pages beyond the home page described in FR-003.
- Translations.
- A custom domain.
- A separate marketing design language for the site. The site looks like the application (FR-029);
  it does not get a brand of its own.
- Linking the site from inside the application (the Help / About dialog). It is worth doing and it
  is a change to the client crate, with its own tests and its own three-platform run; it does not
  ride along with a documentation deliverable.
- Adding macOS or Windows packaging to the release automation. The site describes what the release
  contains (FR-004); it does not change what the release contains. When packaged builds for those
  platforms land, the installation page is updated in that change, not this one.

## Dependencies

- The existing release automation, which creates and publishes releases from the default branch.
- The existing continuous-integration pipeline, into which FR-021 through FR-023 add checks.
- The repository's declared documentation path set, which governs which changes may skip the build
  pipeline — publication must sit correctly with respect to it (FR-020).
- The existing headless capture route used for manual visual passes (see Assumptions, *Media
  capture*).


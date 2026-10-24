# Feature Specification: Bundled monospace typeface

**Feature Branch**: `claude/project-thread-c0bgc0`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Issue #1260 — Bundle a monospace font so code blocks and diffs look
the same on every host. Follow-up from #940 (structured session view). Problem: no monospace face is
bundled. `Font::MONOSPACE` falls back to whatever the host has (Liberation Mono, DejaVu Sans Mono,
Courier New…), so code blocks, diffs and other monospace text render with different glyphs and
widths on each OS. This already broke CI once: in #1234 the layout snapshot text widths differed by
about 1 px between machines. The fix pinned a committed Fira Mono
(`crates/micold-client/tests/fixtures/FiraMono-Regular.ttf`, OFL) in the test support code only.
Users still see the host fallback. Proposal: bundle a monospace face with the app (Fira Mono is
already vetted under OFL; see `FONT-PROVENANCE.md`) and point `Font::MONOSPACE` at it. The test
fixture can then load the same bundled font. Acceptance: code blocks and diffs use the bundled face
on Linux, macOS and Windows, and the layout tests measure that same face."

## Clarifications

### Session 2026-10-24

- Q: Should the terminal pane also switch to the bundled monospace face, or keep the host's
  monospaced font as the user guide documents as a deliberate exception? → A: **The terminal
  adopts the bundled face too** (option A). The user guide's "deliberate exception" comes from
  feature 018, whose FR-012 exempts the terminal from *Roboto and the type scale* only — it requires
  the terminal to stay monospaced with its own grid metrics, not to use the host's font. The terminal
  draws its cells with `Font::MONOSPACE` (`ui/terminal.rs::cell_font`), the same family the issue
  proposes to point at the bundled face, and its grid hard-codes a 0.6 em cell width
  (`CellMetrics::new`), which Fira Mono's 0.600 em advance matches exactly while host faces only
  approximate it. Keeping the host font would need a separate font path for the terminal and leave
  it the one fixed-width surface that differs per OS, against Principle VI. _(agent-resolved:
  specs/018-material-design-fidelity/spec.md#FR-012; crates/micold-client/src/ui/terminal.rs#cell_font,
  #CellMetrics::new; .specify/memory/constitution.md#VI)_

## Context

The application already ships its own interface typeface (Roboto) and icon font, so the interface
looks identical on every platform. Code-like text — fenced code blocks and inline code in the
session transcript, diff and inline-diff lines, tool-result output and similar fixed-width text —
is the exception: it asks for "the monospace family" and gets whichever fixed-width face the host
operating system resolves (Liberation Mono, DejaVu Sans Mono, Noto Sans Mono, Menlo, Consolas,
Courier New…). The glyph shapes and character widths therefore differ per machine, and the layout
tests had to pin a test-only copy of Fira Mono to stay reproducible — which means the tests now
measure a face that users never see.

This feature makes the shipped application carry one monospace typeface, draws all code-like text
with it on every platform, and makes the layout tests measure that same shipped copy instead of a
test-only duplicate.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Code looks the same on every machine (Priority: P1)

A developer reads an agent session transcript containing a code block and a diff. Whether they run
the application on Linux, macOS or Windows — and regardless of which fixed-width fonts happen to be
installed on that machine — the code and diff lines are drawn in the same typeface, with the same
glyph shapes and the same character widths.

**Why this priority**: This is the user-visible defect the issue reports. Without it, code and diff
columns line up differently per host and screenshots, reviews and bug reports are not comparable
across platforms (Principle VI, Cross-Platform Parity).

**Independent Test**: Launch the application on a host whose installed fixed-width fonts differ
from another host's (or on which no fixed-width font of the bundled face's name is installed), open
a session with a code block, inline code and a diff, and confirm that the code-like text is drawn
in the bundled face and that a fixed reference string of code occupies the same width on each host.

**Acceptance Scenarios**:

1. **Given** the application is running on Linux, macOS or Windows, **When** a session transcript
   shows a fenced code block, **Then** its lines are drawn in the bundled monospace face.
2. **Given** a diff (full diff view or inline diff) is on screen, **When** its added, removed and
   context lines are drawn, **Then** they use the bundled monospace face.
3. **Given** a transcript line contains inline code, **When** it is drawn, **Then** the inline code
   span uses the bundled monospace face while the surrounding prose keeps the interface typeface.
4. **Given** a host with no fixed-width fonts other than the operating system's defaults, **When**
   code-like text is drawn, **Then** it still uses the bundled face rather than a host substitute.
5. **Given** a host that has its own installed copy of a font with the same family name as the
   bundled face (possibly a different version), **When** code-like text is drawn, **Then** it uses
   the copy the application ships, not the host's copy.

---

### User Story 2 - Layout tests measure the face users see (Priority: P2)

A contributor runs the layout snapshot and apparatus tests on any machine or CI runner. The
monospace text in the recorded layouts is measured with exactly the face the application ships,
so a passing test means the shipped layout is what was recorded, and there is only one copy of the
monospace face in the repository.

**Why this priority**: It closes the gap #1234 opened: today the tests pin a test-only copy, so
they are reproducible but do not describe what users see. It depends on Story 1 (the face must be
shipped before the tests can load the shipped copy).

**Independent Test**: Run the layout tests on two hosts with different installed fixed-width fonts
and confirm identical recorded geometry for monospace text; confirm that the face the tests load is
byte-identical to the face the application ships and that no separate test-only copy remains.

**Acceptance Scenarios**:

1. **Given** the layout tests run, **When** they measure monospace text, **Then** the face they
   measure with is the shipped face, byte for byte.
2. **Given** the repository after this feature, **When** a contributor searches it for monospace
   font files, **Then** there is exactly one copy of each bundled monospace face, in the
   application's shipped font assets.
3. **Given** a host that has its own installed font with the bundled face's family name, **When**
   the layout tests run, **Then** they still measure the shipped copy, and a guard test fails with
   a message naming the face if anything else is substituted.

---

### User Story 3 - Licensing and provenance stay correct (Priority: P3)

A packager or auditor inspects a release and the repository. The bundled monospace face's licence
text travels with it in every release and in the repository, and its source, version and checksum
are recorded alongside the other shipped fonts.

**Why this priority**: Shipping a font under the SIL Open Font License carries obligations (licence
text travels with the font; Reserved Font Name rules). It is required for release but delivers no
visible behaviour on its own.

**Independent Test**: Inspect the shipped font assets and the release bundles for each platform and
confirm the licence text and the provenance record for the monospace face are present and match the
committed font's checksum.

**Acceptance Scenarios**:

1. **Given** the shipped font assets, **When** an auditor reads the provenance record, **Then** it
   lists the monospace face with its upstream source, version, checksum and licence file, alongside
   the faces already listed.
2. **Given** a release build for Linux, macOS or Windows, **When** it is inspected, **Then** the
   monospace face's licence text is included in the same way the other bundled faces' licences are.

---

### Edge Cases

- **Characters outside the bundled face's coverage** (e.g. CJK identifiers, emoji in a comment, box
  drawing beyond the face's repertoire): they still render through the system's fallback rather
  than as empty boxes, exactly as interface text outside Roboto's coverage does today; the
  characters the face does cover keep the bundled face.
- **Bold or italic code-like text**: bold uses the bundled Bold weight (FR-011); a weight or style
  the bundled set does not contain (italic, or weights other than Regular and Bold) resolves to the
  upright bundled face of the nearest bundled weight, not to a different host family.
- **A host font with the same family name** as the bundled face (a different build or version) must
  not displace the bundled copy, either in the running application or in the layout tests.
- **Font registration failure** (the bundled bytes cannot be parsed, which would be a build defect):
  the application still starts and draws code-like text through the host fallback, and the test
  suite fails, so the defect cannot ship unnoticed.
- **Multiple sessions and windows open at once** (Principle II): every session view, diff and
  transcript in every open window uses the same bundled face; there is no per-session font state.
- **Cross-platform** (Principle VI): the same face, at the same advance width per character, on
  Linux, macOS and Windows; platform differences in rasterisation (hinting, subpixel rendering) are
  acceptable, differences in glyph shapes or advance widths are not.
- **The terminal pane**: it uses the bundled face (FR-007); bold cells use the bundled Bold weight,
  italic cells the upright bundled face, and characters outside the face's
  coverage (box drawing beyond its repertoire, CJK, emoji) the system fallback (FR-004). Its grid
  keeps its own cell metrics.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The application MUST ship one monospace typeface, in a Regular and a Bold weight, as
  part of its own font assets on Linux, macOS and Windows, and the face MUST be available to all
  text from the first frame after launch, alongside the existing interface and icon fonts.
- **FR-002**: All code-like text the application draws in the monospace family — fenced code
  blocks, inline code spans, full and inline diff lines, tool-result and other fixed-width
  transcript content — MUST be drawn with the shipped monospace face on every platform, measurable
  as: the advance width of a fixed reference string of code equals the shipped face's own advance
  width for that string, on every platform.
- **FR-003**: The shipped monospace face MUST take precedence over any host-installed font,
  including a host font with the same family name; code-like text MUST NOT change appearance when
  the host's set of installed fixed-width fonts changes.
- **FR-004**: Characters the shipped monospace face does not cover MUST still render via the
  system fallback rather than as missing-glyph boxes.
- **FR-005**: The layout tests MUST measure monospace text with the shipped face itself (the same
  bytes the application ships), and the separate test-only copy of the monospace face MUST be
  removed, leaving one copy in the repository.
- **FR-006**: The test suite MUST fail, naming the face, when monospace text in the layout tests is
  measured with anything other than the shipped face.
- **FR-011**: Bold code-like text (e.g. strong emphasis inside inline code) MUST be drawn in the
  shipped Bold weight of the monospace face, not in a host face.
- **FR-007**: The terminal pane MUST draw its cells in the shipped monospace face (Bold weight for
  bold cells) on every platform, like other code-like text, while keeping its own character grid and
  cell metrics and staying outside the interface typeface and type scale (feature 018, FR-012).
- **FR-008**: The monospace face's licence text MUST ship with the font in the repository's font
  assets and in every platform's release bundle (alongside the other bundled faces' licence texts), and the font's upstream source, version and
  checksum MUST be recorded in the shipped fonts' provenance record.
- **FR-009**: Interface (non-code) text MUST be unaffected: it keeps the existing interface
  typeface, sizes and weights.
- **FR-010**: The user guide's description of the application's typefaces MUST be updated to state
  which text uses the bundled monospace face, including the terminal (FR-007): the terminal's
  exception from the interface typeface stays, but it no longer uses the host's monospaced font.

### Key Entities

- **Bundled monospace face**: the fixed-width typeface the application ships — its file(s), family
  name, the weights/styles included, licence, upstream source, version and checksum.
- **Code-like text**: every piece of text the application draws in the monospace family (code
  blocks, inline code, diff lines, tool output, and the terminal's cells per FR-007).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On Linux, macOS and Windows, a fixed reference string of code is drawn at the same
  advance width (to within 0.01 px at the same font size) on every platform, regardless of which
  fixed-width fonts each host has installed.
- **SC-002**: The layout snapshot records identical geometry for every monospace text node on every
  platform the layout tests run on (all three for this feature's own merge), with no
  platform-specific tolerance or per-platform fixture.
- **SC-003**: Exactly one copy of each bundled monospace font file exists in the repository, and
  the face the layout tests measure is byte-identical to the shipped one.
- **SC-004**: 100% of release bundles (Linux, macOS, Windows) include the monospace face's licence
  text.
- **SC-005**: Zero interface (non-code) text nodes change size or position in the layout snapshot
  as a result of this feature.

## Out of Scope

- A user setting to choose the code or terminal font, or its size.
- Syntax highlighting or other changes to how code is coloured or laid out.
- Changing the interface typeface (Roboto) or the icon font.
- Ligatures or other font-feature controls for code text.
- Bundling additional scripts (e.g. a CJK monospace face); uncovered characters keep the system
  fallback (FR-004).

## Assumptions

- The bundled face is **Fira Mono**, already vetted under the SIL Open Font License 1.1 and already
  committed (Regular) as a test fixture with its provenance recorded; reusing it avoids new
  licence review. The exact upstream version and checksum recorded for the fixture carry over.
- The bundled set is Regular and Bold: code-like text already requests bold (strong emphasis in
  inline code; terminal bold cells, FR-007). Italic is not bundled (Fira Mono has no
  italic); italic requests resolve to the upright bundled face of the requested weight.
- Fira Mono's character advance (0.600 em) is close to the host faces it replaces (0.600–0.602 em),
  so code lines users see shift by at most about a pixel. The recorded layout snapshot already
  measures Fira Mono (#1234) and is expected not to change; if it does, it is regenerated once and
  reviewed as part of this feature.
- Release bundles carry no font licence file today (the fonts are compiled into the binary; plan
  research R5). The monospace face's licence ships in a font licence directory of each bundle,
  together with the other bundled faces' licence files, which have the same obligation.
- Depends on the font-registration mechanism introduced for Roboto (feature 018) and the layout
  apparatus pinning added in #1234 / feature 940. Issue #1276 (review UI docs/sync, redesign epic
  #1277) depends on this feature.

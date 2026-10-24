# Feature Specification: Markdown italic draws in a true italic face

**Feature Branch**: `claude/project-thread-wxdcto`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Markdown italic renders upright: no italic font face bundled. Follow-up
from #940 (structured session view). The rich markdown view maps `*emphasis*` to an italic style.
The app only bundles Roboto Regular and Medium, though, so emphasis is drawn upright and looks like
plain text. Options: bundle Roboto Italic (and Medium Italic, for bold+italic) and register it with
the font loader, or accept upright emphasis and remove the italic mapping so the intent is explicit.
Where: the font bundling and loading code in the client, and the material `MarkdownView` component.
See the 'Follow-ups not done' section of the ledger in
`specs/940-transcript-rich-rendering/autopilot.md`." (issue #1259)

## Context

Agent replies in the structured session view (feature 940) are drawn as rich markdown. The markdown
model already marks emphasised runs (`*text*`, `_text_`) as italic, and the view asks the text
renderer for an italic slant. The application ships its interface typeface, Roboto, only as upright
faces (weights 400, 500 and 600). With no italic face to match, the renderer draws emphasis in the
upright face, so an emphasised word is indistinguishable from the words around it. The 940 visual
pass recorded this (M4) and left the choice open: bundle italic faces, or drop the italic mapping.

This feature takes the first option: emphasis must look like emphasis. Dropping the mapping would
silently discard meaning the author of the text put there (for example "do *not* delete"), which
plain upright text cannot carry.

## Clarifications

### Session 2026-10-24

- Q: Which weight does a bold-italic run request, and does an italic 600 face match it? → A: The
  markdown view requests bold (700) for strong runs; the nearest shipped interface weight is 600, so
  an italic 600 face is the one bold-italic resolves to (FR-002, FR-003; Assumptions). _(agent-resolved:
  crates/micold-client/src/ui/material/link_text.rs#painted; assets/fonts)_
- Q: Does a link inside emphasis keep both its link look and the slant? → A: Yes; the view applies
  the slant and the link underline/colour independently per run, so an emphasised link is drawn
  italic, underlined and in the link colour; this feature changes only which face the slant
  resolves to. _(agent-resolved: crates/micold-client/src/ui/material/link_text.rs#painted)_
- Q: Is SC-004's "about 0.5 MB" bound consistent with the added files? → A: Yes; each shipped
  static upright Roboto instance is about 158 KB, so three italic instances cut the same way add
  about 0.47 MB. _(agent-resolved: assets/fonts (Roboto-Regular/Medium/SemiBold.ttf sizes))_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Emphasis in an agent reply reads as italic (Priority: P1)

A user reads an agent's reply in the structured session view. The reply says "this will *not*
touch your working tree". The word "not" is drawn slanted, in the same typeface family and weight
as the surrounding text, so the user sees at a glance that the agent stressed it.

**Why this priority**: This is the whole defect. Without it, emphasis carries no visual signal and
stressed words in agent output are easy to miss.

**Independent Test**: Render a markdown paragraph containing an emphasised word in the rich view,
and compare the emphasised glyphs with the same word set without emphasis: the glyph shapes differ
(the italic face is used), while the plain words are unchanged.

**Acceptance Scenarios**:

1. **Given** a reply containing `*not*` in a paragraph, **When** the session view draws it,
   **Then** "not" is drawn with an italic face of the interface typeface at the paragraph's weight.
2. **Given** a reply containing `_emphasis_` (underscore form), **When** it is drawn, **Then** it
   looks the same as the asterisk form.
3. **Given** a paragraph with no emphasis, **When** it is drawn, **Then** it looks exactly as it did
   before this feature (no upright text changes face, size or width).

---

### User Story 2 - Bold italic stays bold (Priority: P2)

A reply uses `***important***` or `**bold _and italic_**`. The run is drawn both slanted and at the
same heavier weight that plain bold text is drawn at, so combining the two never loses the bold.

**Why this priority**: Combined emphasis is less common than plain italic, but if the italic face
existed only at the regular weight, adding italic to bold text would make it look lighter, which is
a new defect introduced by the fix.

**Independent Test**: Render bold, italic and bold-italic runs side by side; the bold-italic run
has the stroke weight of the bold run and the slant of the italic run.

**Acceptance Scenarios**:

1. **Given** a run that is both bold and italic, **When** it is drawn, **Then** it uses an italic
   face whose weight equals the weight plain bold runs are drawn at.
2. **Given** an emphasised word inside a heading or a bold table header cell, **When** it is drawn,
   **Then** it is slanted and keeps the heading's or header's weight.

---

### User Story 3 - The same look on every platform and machine (Priority: P3)

A user on Linux, macOS or Windows, with or without Roboto installed on the computer, sees the same
italic shapes and widths, because the italic faces ship with the application.

**Why this priority**: Cross-platform parity (Principle VI) and predictable layout; the defect is
fixed only if it is fixed everywhere, not just where a host font happens to exist.

**Independent Test**: In the shared renderer used by the tests, resolve the interface typeface at
each shipped weight with the italic slant and check that each resolves to the bundled italic bytes,
including when a host copy of the same family is present.

**Acceptance Scenarios**:

1. **Given** a machine with no Roboto installed, **When** emphasis is drawn, **Then** it uses the
   bundled italic face.
2. **Given** a machine that has its own Roboto Italic installed, **When** emphasis is drawn,
   **Then** it uses the bundled italic face, not the host copy.

### Edge Cases

- **Empty or whitespace-only emphasis** (`* *`, `**`): markdown does not treat it as emphasis; it is
  drawn as literal text exactly as today.
- **Inline code inside emphasis** (`*see `cfg`*`): the code run stays in the monospace face, upright
  (the bundled monospace face has no italic, and code must stay exact); the words around it are
  italic.
- **Characters outside the interface typeface's coverage** (CJK, emoji, arrows) inside emphasis:
  they fall back to a host font as upright text does today (feature 018, FR-013); no missing-glyph
  boxes appear, and whether the fallback is slanted is up to the host font.
- **A bundled italic face fails to parse or register**: the build-time asset tests fail, so a broken
  file cannot ship; there is no runtime error path, because the faces are embedded in the binary.
- **Several sessions open at once** (Principle II): fonts are registered once per process, before
  any window opens; every session view, the overview and the showcase draw emphasis the same way,
  and opening or closing sessions does not change which face is used.
- **Cross-platform** (Principle VI): the faces are embedded, so Linux, macOS and Windows draw the
  same glyphs; a host-installed Roboto of any style must not take precedence over the bundled one.
- **Terminal cells with the italic attribute**: out of scope; terminals draw in the monospace face,
  whose italic handling feature 1260 already settled.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: An emphasised run in the rich markdown view MUST be drawn with an italic face of the
  interface typeface (Roboto), not the upright face.
- **FR-002**: For every weight at which the application ships an upright interface face (400, 500,
  600), it MUST ship an italic face of the same weight, so that any role or bold run that gains
  emphasis keeps its weight.
- **FR-003**: A run that is both bold and italic MUST be drawn at the same weight as a bold run that
  is not italic.
- **FR-004**: Text that is not italic MUST resolve to the same face as before this feature;
  adding the italic faces MUST NOT change any upright text's glyphs or widths.
- **FR-005**: The italic faces MUST be embedded in the application, and emphasis MUST draw in them
  wherever markdown is drawn: in the application, in the component showcase, and in test renders.
  Where and how the faces are registered is left to the plan.
- **FR-006**: Emphasis MUST resolve to the bundled italic faces even when the host has a font of
  the same family installed.
- **FR-007**: Each new face MUST have a provenance entry in `assets/fonts/PROVENANCE.md` naming its
  upstream source, the upstream file's checksum, how it was produced, its own checksum, its weight,
  and the licence file that covers it; the shipped licence MUST cover the new files.
- **FR-008**: Automated tests MUST assert, for each italic face: it parses; it reports the expected
  weight and an italic style; it belongs to the same family the interface pins; and it is a distinct
  file from every upright face.
- **FR-009**: The component showcase's markdown sample MUST show a bold-italic run beside the italic
  run it already has, so the visual review covers both. Whether the run is added to the shared
  markdown fixture or to a showcase-only sample is left to the plan.
- **FR-010**: The italic mapping in the markdown view MUST be kept (the issue's second option, drop
  the mapping, is rejected; see Context).

### Key Entities

- **Interface face**: one shipped font file of the interface typeface, identified by its family,
  weight and style (upright or italic). The set of shipped faces is closed: upright and italic at
  400, 500 and 600.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a rendered sample, 100% of emphasised words differ in glyph shape from the same
  words set upright, and 0% of upright words change.
- **SC-002**: Bold-italic text has the same stroke weight as bold text in a side-by-side render
  (same face weight value).
- **SC-003**: The same italic glyphs are drawn on Linux, macOS and Windows, and with or without the
  typeface installed on the host (checked by resolving to the bundled bytes).
- **SC-004**: The application's size grows by no more than the three added font files (about
  0.5 MB in total).

## Assumptions

- The italic faces come from the same upstream as the upright ones: Google Fonts' Roboto italic
  variable font (`ofl/roboto/Roboto-Italic[wdth,wght].ttf`), cut to static instances at width 100
  and weights 400, 500 and 600 the same way the upright faces were cut. Roboto is under the SIL
  OFL 1.1, which the shipped `LICENSE-Roboto-OFL.txt` already covers.
- Markdown bold resolves today to the 600 face (the nearest shipped weight to bold); bundling
  italic at 600 matches it. If bold's resolved weight changes later, FR-002's rule keeps parity.
- The monospace face stays upright inside emphasis (out of scope; Fira Mono ships no italic).
- No user setting toggles italics; emphasis is always drawn italic.

## Out of Scope

- An italic monospace face for code or terminal cells.
- Changing which weight markdown bold resolves to.
- Synthesised (slanted-upright) italics for host fallback fonts.
- Other follow-ups in the 940 ledger (per-link focus ring, render budget with highlight spans).

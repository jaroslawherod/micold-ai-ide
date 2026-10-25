# Research: Markdown italic draws in a true italic face

Feature 1259. Each entry: decision, rationale, alternatives rejected.

## R1 — Which faces, from where

**Decision**: three static instances of Google Fonts' Roboto italic variable font,
`ofl/roboto/Roboto-Italic[wdth,wght].ttf`, at `wdth=100` and `wght` 400, 500 and 600, cut with the
same `fontTools.varLib.instancer` call (`updateFontNames=True, optimize=True`) that cut the upright
faces. Files: `Roboto-Italic.ttf` (400), `Roboto-MediumItalic.ttf` (500),
`Roboto-SemiBoldItalic.ttf` (600), Google's own static-file names. The upstream file is fetched once
at implementation from `raw.githubusercontent.com/google/fonts/main/ofl/roboto/` (reachable from the
sandbox, ledger D3) and pinned by sha256 in `PROVENANCE.md`; the fonttools version is recorded too.

**Rationale**: same upstream and same cut as the upright faces, so metrics (ascender, descender,
units per em) match and an italic run sits on the same baseline at the same line height. One italic
twin per upright weight is what FR-002 asks; markdown bold asks for 700 and resolves to 600 (spec
Clarifications), so the 600 italic is the one bold-italic needs.

**Alternatives rejected**:

- *Italic 400 and 500 only* (the issue's "Italic and Medium Italic"): bold-italic would resolve to
  500 italic, lighter than plain bold at 600 (US2, FR-003).
- *Ship the italic variable font*: one 500 KB+ file instead of three of ~160 KB; and a variable
  face risks every weight rendering at its default instance (the reason the upright faces are
  static, `PROVENANCE.md`).
- *Roboto Classic's static italics* (googlefonts/roboto-classic releases): a different build from
  the upright faces' upstream; metrics and glyph shapes could differ by a hair from the upright
  faces beside them.
- *Synthesised slant* (skewing the upright glyphs): a skewed upright is not the typeface's
  italic, whose letterforms are drawn separately; the spec puts synthesised italics out of scope.

## R2 — How emphasis reaches the bundled bytes, even with a host Roboto

**Decision**: a new `micold-material/src/roboto.rs`, the twin of `mono.rs`: `pin_roboto_italics(db)` removes
every face of family `Roboto` whose source is not `fontdb::Source::Binary` and that is either not
upright (a host italic or oblique copy) or upright at a weight the application does not ship (not
400, 500 or 600: a host Bold, Black, Light…), and `install_roboto(fonts)` loads all six
bundled Roboto faces with `FontSystem::load_font` and then calls the pin. It runs once per process,
before any text is shaped, at the four places that register fonts today (contract C-3).

**Rationale**: `fontdb::Database::query` breaks a tie between equal candidates in favour of the face
found first, and host faces are loaded first (the same reason feature 1260 pins Fira Mono, its
research R2). Removing the host italic copies leaves only the bundled italics to match an italic
query, whatever the tie-break rule, so FR-006 holds without depending on cosmic-text internals.

The off-weight upright faces go too because otherwise FR-003 fails on any host with a full Roboto
(e.g. Linux with `fonts-roboto`): markdown bold asks for 700 (`link_text::painted`), upright bold
then matches the host's exact Bold 700, while bold-italic can only reach the bundled 600 italic, so
bold-italic is drawn lighter than bold (plan review 1, F1). A host Black 900 is worse: `fontdb`
resolves a missing 700 to the nearest heavier weight first, so it would beat the bundled 600 too.
Removing them changes upright resolution only where the application already drifts from the
faces it ships: every upright query the client makes is 400, 500, 600 (`text.rs` roles,
`overview_card.rs`) or 700 (markdown bold, `Weight::Bold`), and only the 700 one could reach an
off-weight host face. That is the case FR-004 is narrowed for (spec FR-004, ledger D8); the
upright 400/500/600 faces, where a host Roboto can still win the tie (`layout_apparatus.rs`), stay.

**Alternatives rejected**:

- *Keep every host upright face and narrow FR-003/SC-002 to "where upright bold resolves to the
  bundled 600"* (plan review 1, F1 option b): leaves US2's defect, bold-italic lighter than bold,
  on the hosts most likely to have Roboto installed, and makes a requirement hold only on some
  hosts, which FR-006 and SC-003 reject for the italic faces themselves.
- *Remove every host `Roboto` face, upright too* (exactly `pin_monospace`'s rule): it would also
  change which face upright text resolves to on a host with Roboto installed, which FR-004 forbids.
  Upright host precedence is feature 018's concern, guarded by `tests/layout_apparatus.rs`; left as
  it is.
- *Only add `.font(ROBOTO_*_ITALIC_BYTES)` lines beside the existing ones*: a host Roboto Italic
  would win the tie (FR-006, US3.2).
- *Rename the bundled family* (e.g. `Micold Roboto`) so no host face can share it: changes the
  family every upright role resolves by, touching FR-004 and every font test; the OFL Reserved Font
  Name rules also make renaming a modified Roboto a licence question.

## R3 — One registration list, four callers

**Decision**: `install_roboto` replaces the three `.font(ROBOTO_*_BYTES)` calls in
`shell/startup.rs` and `showcase/main.rs`, the three `load_font` calls in
`micold-material/src/test_support.rs::renderer`, and the three Roboto `load_font` calls in
`tests/support/layout.rs::renderer`. The faces are listed once, in `ROBOTO_FACES`.

**Rationale**: FR-005 requires emphasis to draw in the bundled italics in the application, the
showcase and test renders. Four hand-kept lists drift: the showcase once lacked Medium (its
comment), and the test renderer once lacked the icon font (`test_support.rs` comment). A fourth copy
of a now-six-entry list is what `mise run duplication` budgets. Feature 1260 set the pattern with
`install_monospace` at the same four places.

**Alternatives rejected**: *add three more `.font` lines at each site*: twelve new lines in four
lists that must stay equal, and it does not pin (R2).

## R4 — Inline code inside emphasis stays upright

**Decision**: `link_text::painted` sets the italic slant only on runs that are not code.

**Rationale**: the spec's edge case requires code inside emphasis to stay upright. Today a code run
inside `*…*` asks for `Font::MONOSPACE` with `Style::Italic`; `fontdb`'s query falls back to upright
Fira Mono (asserted in `mono.rs`), but the request still says italic, so the result depends on the
matcher. Not asking is explicit and unit-testable on the span's font.

**Alternatives rejected**: *leave it to the matcher*: correct today only through a fallback, and a
host italic monospace could be preferred by a different matching path.

## R5 — Where the bold-italic sample goes (FR-009)

**Decision**: add `***bold italic***` to the formatted paragraph of the shared fixture
`crates/micold-core/tests/fixtures/stream-json/markdown-subset.md` (line 11), which the showcase's
`MarkdownView` section already draws (`showcase/sections/surfaces.rs`, `MARKDOWN_SUBSET`); extend
`markdown_subset.rs::emphasis_strikethrough_and_inline_code_are_styled_spans` with
`style("bold italic") == Style { bold: true, italic: true, .. }`.

**Rationale**: one sample serves the showcase and gains a core assertion of nested emphasis that
does not exist today. Core tests find spans by their text (`span_with`), so the new run disturbs no
other assertion.

**Alternatives rejected**: *showcase-only sample* (e.g. extend the `LINKS` paragraph): no core
coverage, and the `MarkdownView` section is the one the visual pass reviews for markdown.

## R6 — Tests that pin the faces (FR-008)

**Decision**: extend `crates/micold-client/tests/roboto_font.rs` (which already parses the upright
faces and asserts family, weight, static-ness and coverage) to the three italic faces: parses;
`weight()` 400/500/600; `is_italic()`; typographic family `Roboto`; not variable; covers the same
interface characters; bytes differ from every upright face. Assert the upright faces report
`!is_italic()`. `roboto.rs` unit tests test the pin and the resolution, red first.

**Rationale**: the same guard the upright faces have, in the file a reader looks in. The resolution
tests use `test_support::resolves_to_bundled`, the helper `mono.rs` uses.

**Alternatives rejected**: *a new `tests/roboto_italic_font.rs`*: a second file with the same
parsing and coverage helpers, which `mise run duplication` budgets, and the upright `!is_italic()`
check would have no obvious home. *Face assertions inside `roboto.rs`'s unit tests*: those test the
pin and the resolution on a database; the bytes' own properties belong with the upright faces'
guard, which runs as an integration test without the crate's private test support.

## R7 — Geometry impact

**Decision**: expect `tests/fixtures/layout_snapshot.txt` to change in `session-transcript-rich`
only, if at all: its recorded `markdown-turn` reply contains `*leaves*`, whose width changes from
upright to italic glyphs. Re-record with `UPDATE_LAYOUT_SNAPSHOT=1` and check the diff touches
only nodes of that paragraph (or its block's height through a changed wrap). Any other changed
state is an FR-004 failure.

**Rationale**: SC-001's "0% of upright words change" is checked across every covered state by the
snapshot's diff, on all three CI runners.

**Alternatives rejected**: *exclude italic runs from the snapshot*: hides the one geometry change
the feature is meant to make. *A new snapshot state for emphasis*: the existing
`session-transcript-rich` state already draws an emphasised word; a second state adds upkeep and
no coverage.

## R8 — Size

**Decision**: embed three files of ~158 KB each (the upright instances' size, cut the same way;
spec Clarifications), about 0.47 MB in the client binary, within SC-004. Nothing is packaged
separately: faces are `include_bytes!`, and the Roboto licence file every bundle already ships
covers them, so no packaging manifest changes.

**Rationale**: the same embedding as every other bundled face; no runtime path to resolve.

**Alternatives rejected**: *subset the italic faces to the interface's character set*: smaller,
but the upright faces are not subset, and an italic missing glyphs its upright twin has would fall
back to a host font mid-word. *Ship them beside the binary and load at runtime*: a new packaging
path per platform for 0.47 MB.

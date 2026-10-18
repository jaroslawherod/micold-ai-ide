# TDD cycle log — 890

## M1 — summary and required marker (T001–T004)

The tests were written first against builder stubs (`summary`, `summary_maybe`, `required` stored
and ignored, so the tests compile and fail on their assertions), then the header was built.

**Red** — `cargo test -p micold-client --lib accordion_section`: 8 passed, 4 failed.

| Behaviour | Red evidence |
|---|---|
| U1 | `a closed section paints its summary: [Painted { content: "Network", …` |
| U3 | `the summary is painted: [Painted { content: "Network", …` |
| U4 | `the closed title carries the marker: [Painted { content: "Network", …` |
| U5 | pin run: `left: 40.0 right: 0.0` — today's header measured with the stubs (which draw nothing new), then pinned as `TODAYS_HEADER_HEIGHT = 40.0` |
| U2, U6 | passed on stubs (negative and behaviour-preserving cases). Deliberate mutant for U2: dropping `.filter(|_| !s.open)` from the summary match fails `an_open_section_paints_no_summary` (11 passed, 1 failed); restored. U6 guards the existing toggle and focus code by construction. |
| U7 | `cargo test -p micold-client --test showcase_completeness accordion_section`: `the AccordionSection entry poses \`closed, with a summary\`: ["closed", "open"]` |

**Green** — header built per T003 (`row![column![title, Ellipsized summary].width(Fill), chevron]`
while closed with a summary; title + `MARKER` while closed and required; today's row otherwise):
`accordion_section` 12 passed, 0 failed. Showcase poses added (T004): `showcase_completeness` 41
passed, 0 failed.

**Refactor** — `Mounted::drawn` split into `paint` + `drawn`, so the new `Mounted::painted_text`
reads the same draw; the focus-stop counter takes a section (`stops`). Deviation from research R4:
the header row drops the `Space::Fill` when a summary is shown and lets the title column fill
instead — with both filling, the summary would be cut at half the header.

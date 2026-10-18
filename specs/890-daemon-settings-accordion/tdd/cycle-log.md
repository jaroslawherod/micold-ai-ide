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

### M1 review A fixes

- **U8** (added): an empty summary is drawn as none. Red with the fix reverted
  (`cargo test -p micold-client --lib accordion_section`): `an_empty_summary_is_drawn_as_none`
  FAILED, `left: ["Network", "", "\u{e313}"] right: ["Network", "\u{e313}"]`. Green: 13 passed.
- U1 strengthened: both lines must fit inside the fixed 40dp header (the button's
  `density::BUTTON_BASE`); passes. U3 finds the summary by its text rather than by size.

### M1 review B notes

- U5 and U6 are characterisation tests: U5 pins today's 40dp header (measured on stubs that draw
  nothing new) and U6 pins the existing toggle and focus behaviour with a summary and the marker
  present. Neither had a red against a real change; they guard FR-013's "as today".
- T003's `Space::Fill` wording is superseded by research R4's amended layout (see Refactor above);
  the task text stays as written.

## M2 — every daemon kind's form in sections (T006–T013)

Tests first (T006–T009), then `Section`, `State::section_open`, the container sections and the
daemon form.

**Red** — the new tests did not compile or failed against the old `Section` (`Limits`, Container
first): `daemon_sections` (`Section::of`, `title`, `holding_error`, `holding`), `daemons_feature`
(Connection-only default, `RuntimeChosen` reset, error opens its section, required-empty keeps
Connection open, `SectionToggled` leaves draft and registry equal), `daemons_section_geometry`
(titles in FR-002 order, only Connection's body, no group heading painted).

**T009 red**: not taken before T013. Review B measured it afterwards on a throwaway `origin/main`
worktree with the gate copied in: `the Kubernetes add form ends at 1851.0005, past 800`; green on the
branch (`layout_snapshot` 70 passed).

**Green** — `cargo test -p micold-client --test features_daemon_sections --test daemons_feature --test
daemons_section_geometry --test layout_snapshot`: all passed.

**Refactor** — `Section`, `title` and `holds` moved out of `features/daemons.rs` into
`features/daemon_sections.rs`, one table answering `of`, `holding_error` and `holding`.

### M2 review A fixes

- **R1** (MAJOR): a click on a section an error holds open is not recorded. Red:
  `a_click_on_a_section_an_error_holds_open_is_not_recorded` — `left: true right: false` (the stale
  flip closed Resources once the error cleared). Green after `State::held_open` and the guard in
  `SectionToggled`.
- **R2** (MAJOR): `allowed` maps to Credentials and is shown under the credential checkboxes. Red:
  `the_allowed_credentials_refusal_opens_credentials` failed. Green.
- **R3** (MINOR): `choosing_the_same_kind_again_keeps_the_sections_the_user_opened`: red, then green
  with `RuntimeChosen` clearing the flips only when the kind changes.
- Scoped gate had failed clippy (`Some(x).filter(|_| false)` in `daemon_sections.rs`): now `false.then_some(..)`.
- Full gate found two more things the scoped gate missed: `a_field_note_shares_its_fields_column` opens
  the container form and needs the Container section opened first (Connection is the default now), and
  `feature_registration_cost` wants `tests/features_<module>.rs` for the new feature module, so
  `tests/daemon_sections.rs` became `tests/features_daemon_sections.rs`.

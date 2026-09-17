---
feature: 030-settings-rail-slide
verdict: PASS_WITH_GAPS # decisive reason: M2 landed as one squashed commit, so its 27 behaviors are LIKELY not PROVEN; no HIGH smells, all criteria covered, mutation done by hand (no tool in the profile)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 4db4cb69
behaviors: 27 # A1-A11 (11) + U1-U25 excluding gaps (16): U1,U2,U3,U4,U5,U6,U7,U8,U9,U10,U11,U12,U13,U14,U15,U16,U17,U18,U19,U20,U21,U22,U23,U24,U25 = 25, +A1-A11 = 11 -> table below is authoritative
proven: 3 # U1, U23, U24 (M1, separate commits, cycle log red per commit)
likely: 24 # A1-A11, U2-U22, U25 (M2, squashed commit, cycle log red per cycle)
test_after: 0
no_test: 0
high_smells: 0
criteria_total: 18 # US1 scenarios 1-7, US2 scenarios 1-4, SC-001..SC-007
criteria_covered: 18
mutation_score: not applicable # no tool in tdd-profile.md; deliberate mutants only
mutants_sampled: 21 # ~19 recorded in cycle-log for "example (mutant)" behaviors + 2 independent ones run in this audit
mutants_survived_uncaught: 0
suite: 3248 passed, 0 failed, 6 ignored (cargo test --workspace --no-fail-fast)
---

# TDD Verification: The settings rail slides when it collapses and expands

**Verdict: PASS_WITH_GAPS.** The decisive reason: M2 (T005-T044, 24 of the 27 behaviors) landed
as one squashed commit (`3997a17f`), so git history cannot corroborate that each test preceded its
implementation — only the self-reported cycle log can, which caps every M2 behavior at `LIKELY`
rather than `PROVEN`. Nothing else keeps this out of `PASS`: no `HIGH` smells were found across the
four changed test/source files, every acceptance criterion traces to a real-entry-point test, no
existing test was weakened, the full workspace suite is green (3248 passed, 0 failed), and every
mutant sampled — both the ones the implementation loop recorded and two independent ones run for
this audit — was caught.

## Test-first evidence

M1 (`85987e5b`, `7c734de5`, `ff9d3a5a`, `c570037d`) is four separate commits, each pairing one test
with its own fix and each with a red command and failure output recorded in `tdd/cycle-log.md`
("Cycle 1" through the M1 round-3 corrections). That satisfies `PROVEN`'s two-part test: the cycle
log has the red, and history shows test and source changing together, one behavior per commit.

M2 (`3997a17f`) is a single squashed commit containing all of T005-T044: every test file, every
source change, the fixture regeneration and the docs, in one diff. `tdd/cycle-log.md` records 12
cycles with red commands and failure output for the M2 behaviors (including several "red only"
cycles — 7 and 8 — later made green in cycle 9, and deliberate-mutant reds for every behavior the
test list marks `example (mutant)`), but the git history has no per-cycle commits to check that
order against. Per the rubric, a squashed history that cannot corroborate order caps these at
`LIKELY`, not `PROVEN`, regardless of how detailed the self-reported log is.

| Behavior(s) | Class | Evidence |
| --- | --- | --- |
| U1 | PROVEN | Cycle 1 red (`navigation_drawer.rs:497` panic) recorded; commit `85987e5b` adds the test and the `.easing(EMPHASIZED)` fix together |
| U23 | PROVEN | Cycle 2 red recorded; commit `7c734de5`; re-cut and re-verified in M1 review rounds 2 and 3, each with its own red/mutant evidence, still one commit per behavior change |
| U24 | PROVEN | Cycle 3 red (`navigation_drawer.rs:611`) recorded; commit `c570037d` |
| A1, A2, A5 | LIKELY | Cycle 7 red (`settings_rail_motion.rs:311`, `358` for A5's mutant); green in cycle 9; all in `3997a17f` |
| A3 | LIKELY (approval-kind, not red/green) | No "red" exists for a snapshot test; T027 regenerated `layout_snapshot.txt` and the cycle-10 entry records quickstart §A.3's rectangle-set comparison against `origin/main` returning "unchanged" — self-reported, not independently re-run in this audit beyond confirming the fixture test itself is green today |
| A4, U15, U16 | LIKELY | Cycle 7 red + cycle 9 mutant reds (flag held, events withheld, redraw requested unconditionally), all self-reported in one commit |
| A6 | LIKELY | Cycle 8 red (`section_list.rs:1023`); green in cycle 9 |
| A7, U17, U18 | LIKELY | Cycle 10 red and mutant reds (U17's chip-cut mutant, U18's "survived as first written, then strengthened" note — itself good evidence the loop was actually exercising the test, not just recording success) |
| A8, A9, U19, U20 | LIKELY | Cycle 11 red and mutant reds; A8/A9's mutant is recorded as **surviving on first write** and the test was then strengthened — a genuine sign of a working (not rubber-stamped) loop |
| A10, U21 | LIKELY | Cycle 11 red |
| U22 | LIKELY | Cycle 11 red and two mutant reds (dropped forwarding, forwarded forever) |
| A11 | LIKELY | Passed on first write per cycle 11 (research R4 predicted this); its own mutant (`x_labelled` cached) is recorded as failing, i.e. caught |
| U25 | LIKELY | Found by the M2 visual pass, not the original test-list plan; cycle 12 records a red (`settings_rail_motion.rs:1346`) taken against `origin/main`'s behavior before the clip fix, so it is still test-first for the fix it drives, just discovered later than the rest |
| U2-U6 | LIKELY | Cycle 4 red (5 separate panics, one per behavior) |
| U7-U9 | LIKELY | Cycle 5 red (3 panics) |
| U10-U13 | LIKELY | Cycle 6 red (4 panics) |
| U14 | LIKELY | Cycle 7 red + cycle 9 mutant red + a corrected float-tolerance in the test itself (documented as a correction, not a weakening: the exact-edge assertion stayed, only a sub-pixel rounding tolerance was added) |

### What the diff did to tests that already existed

Checked `git show 3997a17f -- crates/micold-client/tests/gates/rail_icons_align.rs` and
`crates/micold-client/tests/support/covered_states.rs`: the gate gained an additive assertion (every
glyph's centre must also lie inside the rail, `rail_icons_align.rs:111-126`) with no existing
assertion removed or loosened; `covered_states.rs` was not touched at all (the `settings.rail` anchor
path did not move, as the plan expected). No weakened or skipped existing test was found anywhere in
the feature's range.

### `tasks.md` vs. test-list checkboxes

Every task in `tasks.md` (T001-T047) is ticked `[X]`, and every behavior in `tdd/test-list.md` is
marked `DONE`. No task is ticked with an undone behavior behind it, and no `DONE` behavior has an
unticked task. Consistent.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | MED | `rows_keep_their_height_and_icons_their_line`'s per-frame icon-step assertion is an upper bound (`(x - before).abs() <= bound`), not a check against the value `offset()` actually predicts, so a regression that makes an icon move *too slowly* (a stalled or under-scaled icon) would still satisfy "no more than the bound" and "does not move away from target" | `crates/micold-client/tests/settings_rail_motion.rs:689-694` |
| 2 | MED | `the_state_flips_on_the_press` asserts three distinct FR-010 sub-claims (flip-on-press, mid-slide section change, mid-slide Save/Cancel closing) in one test; each has its own failure message, but a CI summary line would not distinguish which sub-claim broke | `crates/micold-client/tests/settings_rail_motion.rs:479-540` |
| 3 | MED | The rail and section region are located by hardcoded tree-index paths (`"0/0/0/1/0/0"`, etc.) that ~15 tests in this file depend on; an unrelated structural change anywhere above the rail in `ui::view` silently repoints these at the wrong node, and the failure reads as "nothing at that path" rather than naming what moved. This is an existing repository convention (`layout.rs`/`rail_icons_align.rs` already do it), not novel to this feature, but this file leans on it far more heavily than any single existing gate | `crates/micold-client/tests/settings_rail_motion.rs:36-46` |
| 4 | LOW | One row of `a_sliding_drawer_is_never_narrower_than_its_rail`'s table (`(false, 0.125, 300.0, 37.5)`) exercises only the unclamped `full.width * progress` multiplication, restating the same arithmetic the production code performs there; the other four rows exercise the two clamps with independently justified literals, so this is a minor, isolated instance rather than a pattern | `crates/micold-client/src/ui/material/navigation_drawer.rs:545` |
| 5 | LOW | A3 (the layout-snapshot / rest-state invariant) is graded as an approval test with no red/green cycle of its own; its "proof" is a self-reported quickstart §A.3 set-comparison in the cycle log rather than something this audit re-derived independently | `specs/030-settings-rail-slide/tdd/cycle-log.md`, cycle 10, T027 |

Findings 1 and 2 were fixed in the close PR, after this audit's snapshot: T048 added the per-frame
on-the-line check (the lagging mutant `fraction.powf(1.2)` now fails it), and T049 split U15 into
three tests (cycle-log cycle 13). Findings 3–5 need no action (T050–T052).

No `HIGH` smells were found in any of the four changed files (`settings_rail_motion.rs`,
`gates/rail_icons_align.rs`, `section_list.rs`'s test module, `navigation_drawer.rs`'s test module):
no assertion-free, tautological, doubled-subject, over-mocked, vacuous, self-approving, or
conditional-logic tests, and no empty/skipped test. Magic numbers throughout are either explained
inline or deliberately restated with a comment defending the restatement (e.g.
`section_list.rs:1332-1335`, so a slip in `ROW_WIDTH` fails the test rather than agreeing with
itself). This pass was delegated to a fresh-context subagent per the rubric's instructions (full
rubric, profile conventions, exemplars and helpers supplied, hard rules 6/7 included verbatim); every
citation above was independently re-opened and confirmed against the file before inclusion.

Suite-level properties: no real `Instant::now()`-based timing decisions, no `thread::sleep`, no
randomness in any of the four files — every `Instant::now()` use is a fixed origin that only
frame-relative deltas are added to, consistent with the profile's documented-safe pattern. No test
order dependence found. `settings_rail_motion.rs` is structurally the slowest file (some tests pump
up to 120 frames through a real headless renderer, several for both starting states), but it ran the
full workspace suite in about the profile's usual ~5-6 minute range with nothing resembling a
sleep-based wait.

## Mutation results (no tool in the profile; deliberate mutants only)

Nineteen mutants are recorded and restored inside `tdd/cycle-log.md` itself, one per
`example (mutant)` behavior (A5, A9, A11, U14, U15 ×2, U16, U17, U18, U20, U22 ×2, plus the three M1
mutants for U23/U24). Two are notable because they **survived on first write** and forced the test to
be strengthened before they were caught — A8/A9's "operate reaching parked forms" mutant (count
doubled, so the naive assertion held) and U18's "no-icon row laid out at 272" mutant (held trivially
until the test also checked the row ends inside the rail) — this is the strongest evidence in the
whole feature that the loop was actually testing strength, not just recording green.

Independently, for this audit, two further deliberate mutants were applied, run, and restored
(verified back to green with the targeted unit tests before moving on):

| Mutant | File:line | Behavior | Result |
| --- | --- | --- | --- |
| `form()`'s boundary comparisons changed from `<=`/`>=` to `<`/`>` at the two thresholds | `crates/micold-client/src/ui/material/section_list.rs:112,114` | U4, U5 | Caught: both `a_row_draws_its_icons_only_form_at_the_collapsed_threshold_and_no_later` and `a_badged_row_draws_its_labelled_form_only_from_the_expanded_threshold` failed with exact left/right mismatches |
| `NavigationDrawer::layout`'s width floor (`.max(rail_floor)`) removed | `crates/micold-client/src/ui/material/navigation_drawer.rs:191-194` | U23 | Caught: `a_sliding_drawer_is_never_narrower_than_its_rail` failed (`open true at progress 0: … left: 6.0 right: 32.0`) |

Both mutants were restored from a scratchpad copy taken before mutating (not `git checkout`, since
the working tree carries uncommitted, non-behavioral doc-comment edits in `section_list.rs` that
`git checkout` would have discarded); `git diff --stat` after each restore matched the pre-mutation
diff exactly, and the targeted tests were re-run green before moving to the next mutant.

No survivor was found, sampled or recorded, inside any behavior marked `DONE`.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1-7 (A1-A7) | `settings_rail_motion.rs` tests named in `tdd/test-list.md`; A6 in `section_list.rs`'s test module | Yes for A1-A5, A7 (mounted through the real settings view via `view_of`); A6 is a component-level test of real (non-doubled) `NavigationDrawer`/`SectionList` widgets rather than the settings view itself, because Settings has no drawer to compare against — a deliberate, documented limitation, not a doubled boundary |
| US2.1-4 (A8-A11) | `settings_rail_motion.rs` | Yes, through the real settings view |
| SC-001 | A1, A2 | Yes |
| SC-002 | A6 | Component-level (see A6 note above) |
| SC-003 | A8, A9 | Yes |
| SC-004 | A4 | Yes |
| SC-005 | A3 (approval) | Yes, but self-reported comparison (see Finding 5) |
| SC-006 | A5 | Yes |
| SC-007 | A7 | Yes |
| FR-001 – FR-011, FR-013 – FR-015 | A- and U-series per `tdd/test-list.md`'s traces column | Yes, all resolve to an existing, passing test |
| FR-012 | None (documentation requirement) | Not applicable to automated testing; `docs/user-guide/settings.md` was updated (T029) and the wording is scoped to manual review ("checked in the milestone's Review A" per `tdd/test-list.md`'s Out of scope section), not a behavior a test could assert |

Every `traces` value in `tdd/test-list.md` was checked against the actual test files: every named
test exists and is in the green suite run for this audit. No test traces to nothing (all 27 map to a
requirement or success criterion), and no criterion is silently uncovered.

## What was not audited

- Mutation testing was not run with a tool: the profile records `mutation: null`, so this audit used
  deliberate mutants only — two independently, plus the nineteen the implementation loop already
  recorded and restored. This is not exhaustive; it samples the highest-risk boundary logic in the
  two files most central to the feature (`section_list.rs`'s `form()` thresholds, `navigation_drawer.rs`'s
  width floor), not every line either file touches.
- Coverage was not measured: the profile records `coverage: null`.
- A3's rest-state equivalence (SC-005) was verified by re-running `layout_snapshot`'s fixture test
  (green) but the quickstart §A.3 rectangle-set comparison against `origin/main` itself was not
  independently re-executed in this audit; it is taken on the cycle log's word.
- The acceptance/sandbox-real-runtime suite and the Windows-only cfg paths from feature 030's other
  (unrelated) work were not run: this feature makes no daemon, wire-protocol or Windows-specific
  change, and the profile's acceptance runner is unrelated to UI layout.
- `mise run gate`'s full CI-equivalent (fmt, clippy -D warnings, shell test suites) was not re-run in
  this audit; `cargo test --workspace` was, and it is green. Cycle-log's T042 records the gate as
  green at the M2 milestone, and nothing this audit found calls that into question.
- The visual pass (`specs/030-settings-rail-slide/visual-pass.md`, quickstart §B) was not re-run; it
  is outside this audit's remit (draw-time clip colour, tint, frame pacing — explicitly out of scope
  for `tdd/test-list.md`) and was already recorded as done at the M2 milestone.

# Contract: Layout Fixture Format

**Feature**: `specs/019-layout-snapshot-parity` | **Date**: 2026-07-28

The fixture is the feature's only artefact and its only interface. It is read by two audiences —
the assertion, which compares it byte-for-byte, and a human reviewing a pull request, who must be
able to say "this change moved these four things and nothing else" without building anything
(SC-005). Everything below serves one of those two.

Path: `crates/micold-client/tests/fixtures/layout_snapshot.txt`

---

## 1. File grammar

```text
file      := header blank state-block+
header    := "# layout snapshot v1" NL
             "# renderer: tiny-skia" NL
             "# font: " font-file NL
             "# window: " W "x" H NL
             "# scheme: light (dark asserted byte-identical, not recorded)" NL
             "# regenerate: UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot" NL
state-block := state-header NL anchor-block? record+ blank
state-header:= "## " name
anchor-block:= ("@ " anchor-name " -> " path NL)+
record    := layer SP indent path SP x SP y SP w SP h NL
layer     := "base" | "over"
path      := index ("/" index)*
```

- Encoding is UTF-8, line endings are `\n` on every platform. A fixture written with CRLF fails on
  the next run, which is correct: it is a difference.
- Exactly one trailing newline at end of file.

`window` and `scheme` are declared **once, for the whole file**, not per state. Every covered state
resolves at the same canonical window size (FR-008b), and the fixture records a single scheme
(FR-008a) — so repeating either on every section header would be noise that also invites the two to
disagree. A record is still never separated from the conditions that produced it; those conditions
are simply global.

## 2. Numbers

Every one of `W`, `H`, `x`, `y`, `w`, `h` is:

- rounded to **one decimal place** (FR-012);
- formatted with **exactly one** fractional digit, always present — `0.0`, not `0`;
- right-aligned in a fixed-width field so columns line up down the file;
- normalised so that `-0.0` is written `0.0`.

Formatting is explicit and fixed-precision. `{:?}` on an `f32` is forbidden — it is not a stable
text form and would make the fixture flap without any layout changing.

## 3. Ordering

Records appear in the tree's own depth-first order. **No sorting is applied.** Sorting would
conceal a structural reordering, which is a change the gate exists to report.

Within a state block, the `base` pass is emitted in full, then the `over` pass if the state has one
(R5). State blocks appear in registration order.

## 4. Worked example

```text
# layout snapshot v1
# renderer: tiny-skia
# font: Roboto-Regular.ttf
# window: 1280.0x800.0
# scheme: light (dark asserted byte-identical, not recorded)
# regenerate: UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot

## main-shell-sidebar-expanded
@ toolbar.title -> 0/0/0/0/0
@ sidebar.row.label -> 0/0/0/2/1/0/0
@ sidebar.row.close_button -> 0/0/0/2/1/0/1
base 0                   0.0    0.0 1280.0  800.0
base   0/0               0.0    0.0 1280.0  800.0
base     0/0/0           0.0    0.0 1280.0   35.2
base       0/0/0/0       8.0    4.0 1264.0   26.2
base         0/0/0/0/0   8.0    8.0   87.4   18.2
```

The indentation is two spaces per depth level on the path column only; the numeric columns stay
aligned regardless of depth. A depth change therefore shifts one column and is visible as such.

## 5. Assertion behaviour

- The check compares the generated text to the committed file **byte-for-byte** (FR-003).
- On difference, the failure message MUST name (FR-004):
  1. the covered state,
  2. the element — by anchor name where one covers the path, otherwise by path,
  3. the recorded geometry and the observed geometry, side by side.

  A message that says only "the layout changed" does not satisfy FR-004 and is a defect in this
  feature.
- On a state that can no longer be constructed, or an anchor whose path no longer resolves, the
  check fails naming it (FR-014). Coverage never narrows silently.
- The check MUST also resolve every covered state in the scheme the fixture does **not** record and
  require byte-identical geometry, failing and naming the state if it differs (FR-008a). This is an
  equality assertion, not a second fixture: nothing about the other scheme is committed.

## 6. Regeneration

```sh
UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot
```

- This is the only way the fixture is written (FR-013). A normal run never writes it, not even on
  failure, not even when the file is missing.
- The variable name deliberately mirrors `UPDATE_STYLE_SNAPSHOT` from feature 017, so the two gates
  read as one idea rather than two conventions.
- **`--test` is load-bearing** (BUG-001, FR-013a). It selects the *target*. Written as a bare
  `layout_snapshot` it is a **test-name filter**, and no test here carries that name — the run
  matches nothing, prints `0 passed; N filtered out`, and exits 0, so the caller is told it worked
  while the fixture is untouched. Every place this command appears — the failure message, the
  fixture's own header line (§2), the module doc, this contract, and
  `docs/development/layout-snapshot.md` — must carry the identical form, and
  `the_regenerate_hint_selects_this_target` asserts it rather than leaving it to review.
- **Judge a regeneration by the fixture, not the exit code.** The broken form's exit code was 0, and
  so is a correct run's; only one of them changes the file.

## 7. Stability guarantees this format does *not* make

Stated here because FR-015 requires the boundary to be explicit, and because a gate that is quietly
narrower than it looks is the exact failure this feature exists to correct.

| Not guaranteed | Consequence |
|----------------|-------------|
| Path stability across structural edits | Inserting a container near the root renumbers its descendants; one small change can produce a large, correct diff. Anchors are re-pointed by hand as part of that change. |
| Mid-animation geometry | Records are taken at rest (R6). Both endpoints of a transition are covered as resting states; the interpolation between them is not. |
| Scrolled geometry at other offsets | Records are taken at scroll offset zero (R7). A covered state overflows the sidebar's list, so that offset is a sampled point rather than an assumption — but only the top is recorded, and geometry that appears only part-way down a scroll is not. |
| Colour, border, radius, shadow | Owned by `style_snapshot` (feature 017). This fixture records geometry only. |
| Pixels | No rasterisation is compared. Two different-looking widgets occupying identical boxes are identical to this gate. |

**Production typography was on this list and is not any more.** It read: *"Until feature 018 ships
Roboto as the application font, the fixture measures a typeface users do not yet see."* 018 shipped,
and the fixture now measures the files the application draws with (`assets/fonts/`), all three faces.
The correction is recorded rather than deleted because a stale exclusion is the same defect as an
unclear one — it tells a reader the gate is narrower than it is, and the cost is that nobody looks
for the coverage they already have.

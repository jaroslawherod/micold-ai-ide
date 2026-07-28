# Quickstart: Validating the Layout Snapshot Parity Gate

**Feature**: `specs/019-layout-snapshot-parity` | **Date**: 2026-07-28

This feature's acceptance test is unusual in the opposite direction from feature 017's. 017 had to
prove *nothing changed* and could only half do it automatically. This one has to prove *a change is
caught* — and every part of that is machine-checkable. There is no manual walkthrough here, and
that is the point.

Every step below runs headlessly: no display, no GPU, no window manager (FR-001).

---

## Prerequisites

```sh
mise trust          # first time in a fresh worktree only
mise run test       # cargo test --workspace — establishes the suite is green
```

Nothing else. No compositor, no screenshot permission, no human at the machine — the three things
that blocked feature 017's T048/T049/T050.

---

## Part A — the gate holds

```sh
mise run test
```

| Gate | Test | Proves |
|------|------|--------|
| Fixture matches the application | `micold-client/tests/layout_snapshot.rs` | FR-003, SC-001 |
| Every covered state still constructs | same, coverage assertions | FR-014, SC-004 |
| The other scheme lays out identically | same, scheme-equality assertion | FR-008a |
| Reference font is the one we shipped | same, guard assertion | FR-006, SC-002 |
| Resolved styles still match | `micold-client` `style_snapshot` (feature 017) | unchanged by this feature (FR-019) |

**Expected**: all pass, and the total is **at or above** the count recorded at the start of this
feature. A drop means a test was lost, not that the suite got faster.

Confirm the check needs nothing from the environment:

```sh
# no DISPLAY, no WAYLAND_DISPLAY — must still pass
env -u DISPLAY -u WAYLAND_DISPLAY cargo test -p micold-client layout_snapshot
```

---

## Part B — the gate catches what it claims to (SC-001, FR-004)

### B1. A deliberate spacing change fails, and names what moved

Pick any covered component and increase one padding value by 8.

```sh
cargo test -p micold-client layout_snapshot
```

- [ ] The check **fails**.
- [ ] The message names the **covered state**.
- [ ] The message names the **element** — by anchor name if one covers it, otherwise by path.
- [ ] The message shows **recorded vs observed** geometry, side by side.
- [ ] A message reading only "the layout changed" is a **defect**, not a pass.

Revert the change.

- [ ] The check passes again with no fixture edit.

### B2. A structural edit that moves nothing passes (Acceptance Scenario 1.3)

Wrap a covered element in a container that adds no padding, no border and no alignment.

- [ ] The check **passes**. The record is of geometry, not of tree shape for its own sake.

> If this fails, the fixture is recording composition rather than layout, and the format contract
> has been implemented wrongly.

### B3. Removing a covered state fails rather than narrowing coverage (FR-014)

Make one registered covered state impossible to construct.

- [ ] The check **fails**, naming that state.
- [ ] It does **not** pass with one fewer state recorded.

Do the same for an anchor: re-point one anchor at a path that does not resolve.

- [ ] The check **fails**, naming the anchor.

### B4. The motivating defect is caught (FR-018, SC-003)

This is the one that justifies the feature. Reintroduce the defect feature 017 shipped: an
over-long sidebar label that overlaps its adjacent close button.

- [ ] The check **fails**.
- [ ] The failure names `sidebar.row.label` and/or `sidebar.row.close_button`.
- [ ] The geometry shown makes the overlap visible — the label's `x + width` exceeds the close
      button's `x`.

Revert.

- [ ] The check passes.

> Feature 017 found this defect by a person looking at a running application, after it shipped.
> If this step does not fail, the feature has not delivered its stated purpose regardless of what
> else passes.

---

## Part C — accepting an intended change (US2, SC-005)

Make a real, intentional layout change.

```sh
UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client layout_snapshot
git diff crates/micold-client/tests/fixtures/layout_snapshot.txt
```

- [ ] The fixture updates and the check passes.
- [ ] The diff is **limited to the affected elements** — an unrelated section did not churn.
- [ ] Each changed line identifies an element and its state, readable **without running the
      application**.
- [ ] Running the check *without* the variable never rewrites the file — not on failure, not when
      the file is missing (FR-013).

Check the negative case explicitly:

```sh
git checkout crates/micold-client/tests/fixtures/layout_snapshot.txt
cargo test -p micold-client layout_snapshot   # fails
git status --short                            # fixture must be UNMODIFIED
```

- [ ] The fixture is unmodified after a failing run.

---

## Part D — extending coverage (US3, FR-016)

Register one additional covered state.

- [ ] It took a change in **one place only**.
- [ ] The fixture gained that state's layout and nothing else changed.

---

## Part E — cross-platform determinism (FR-006, SC-002)

This is the requirement most likely to be quietly violated, because it passes locally by
construction.

- [ ] CI is green on Linux, macOS **and** Windows on the same commit, with the same committed
      fixture (FR-017, Principle VI).
- [ ] Two consecutive local runs produce identical output (FR-005).

> The gate measures text against a committed reference font precisely so this holds (research R2).
> The host still has its own fonts loaded — 391 faces were counted on the development machine — so
> the guard assertion exists to fail loudly if a same-named system font wins the family lookup.
> A *mass* geometry difference on one platform is that guard's failure mode; read it as a font
> problem, not as a layout regression.

---

## Part F — cost (SC-006)

```sh
cargo test -p micold-client layout_snapshot -- --nocapture
```

- [ ] Completes in **under 10 seconds** locally.
- [ ] Adds no more than **10%** to total `mise run test` runtime — measure the suite with and
      without it and record both numbers.

---

## Part G — the documented boundary (FR-015, SC-007)

Read the check's documentation alone, without opening the implementation. For each category below,
answer "would this be caught?" — and confirm the documentation answers it:

- [ ] a padding change
- [ ] a font-size change that widens a label
- [ ] a colour change *(no — `style_snapshot` owns that)*
- [ ] a widget swapped for a differently-drawn one of identical size *(no)*
- [ ] a change visible only mid-animation *(no — records are taken at rest)*
- [ ] a change visible only when scrolled *(no — offset zero)*
- [ ] a dropdown that opens in the wrong place *(yes — overlay pass, R5)*

- [ ] Every answer is stated in the documentation, not inferred by the reader.

---

## What this feature must not have done

- [ ] The application's appearance is unchanged (FR-019). `style_snapshot` still passes with **no
      fixture regeneration** — that is the mechanical proof.
- [ ] No layout defect found while building the gate was silently fixed. Any that surfaced is
      recorded as a finding and addressed separately (FR-019).
- [ ] Feature 017's parity claim is untouched; this feature does not reopen it (spec Assumptions).

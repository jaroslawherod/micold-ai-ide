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

- [x] The check **fails**.
- [x] The message names the **covered state** (`main-shell-sidebar-expanded`).
- [x] The message names the **element** — path `0/0/0/2/0/0/0/0`, and by anchor where one covers it.
- [x] The message shows **recorded vs observed** side by side: `4.0 43.2 252.0 26.2` against `8.0 43.2 244.0 26.2`.
- [x] Not a bare "the layout changed".

Revert the change.

- [x] The check passes again with no fixture edit.

### B2. A structural edit that moves nothing — **amended 2026-08-04, it cannot pass**

> **This step was written wrong and is kept with its correction.** It asked that wrapping a covered
> element in a no-op container leave the check green. **Run on 2026-08-04, it fails, and it must.**
>
> An element's identity in the fixture is its depth-first index path, because a `layout::Node`
> carries no name, type or id. A wrapper *is* a node: it has geometry of its own and gets recorded,
> and everything beneath it shifts one level down. Measured — the sidebar header wrapped in a bare
> `container`: the header kept its geometry exactly (`4.0 43.2 14.3 26.2`) and moved from
> `0/0/0/2/0/0/0/0/0` to `0/0/0/2/0/0/0/0/0/0`, with the wrapper taking the vacated path.
>
> So the original premise — "the record is of geometry, not of tree shape" — is not achievable
> alongside T007, which requires the emission order to be the tree's own precisely so a structural
> reordering cannot hide. A fixture that ignored inserted nodes would be blind to composition
> changes, which is a worse gate, not a better one.
>
> `docs/development/layout-snapshot.md` already had the correct account under **Path stability**:
> one correct structural edit can produce a large diff, and that is expected. The two documents
> contradicted each other and this one was wrong.

Wrap a covered element in a container that adds no padding, no border and no alignment.

- [x] The check **fails**, and the diff shows the wrapper as a new node with every descendant
      renumbered — **geometry unchanged**, paths shifted.
- [x] No element's recorded geometry changed. That, not a green run, is what "moves nothing" means
      here.

### B3. Removing a covered state fails rather than narrowing coverage (FR-014)

Make one registered covered state impossible to construct.

- [x] The check **fails**, naming that state.
- [x] It does **not** pass with one fewer state recorded.

Do the same for an anchor: re-point one anchor at a path that does not resolve.

- [x] The check **fails**, naming the anchor.

> Both are held by tests that run on every suite rather than by a one-off manual edit:
> `a_state_that_can_no_longer_be_constructed_fails_naming_it` and
> `an_anchor_whose_path_does_not_resolve_fails_naming_it`. Stronger than this step asked for — a
> manual check passes once, an automated one keeps passing.

### B4. The motivating defect is caught (FR-018, SC-003)

This is the one that justifies the feature. Reintroduce the defect feature 017 shipped: an
over-long sidebar label that overlaps its adjacent close button.

- [x] The check **fails** — 283.5px wanted in 187.6px, an overflow of 95.9px.
- [x] The failure names `sidebar.row.label`. (The adjacent control is `row_actions_cluster`'s **Delete**, not a close button, so the anchor is named `sidebar.row.delete_button` for what it is.)
- [x] The geometry shown makes the overlap visible: the label wants 283.5px in the 187.6px it was
      allowed, and the delete button begins 191.6px along the row.
- [x] The geometry fixture stays **green** throughout, which is what proves the split between the
      two checks is structural rather than incidental.

Revert.

- [x] The check passes.

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

- [x] The fixture updates and the check passes.
- [x] The diff is **limited to the affected elements**: 610 changed against 610 removed — nothing
      added or dropped, only moved — and the one state with no sidebar did not churn at all.
- [x] Each changed line identifies an element and its state without running the application.
- [x] Running the check *without* the variable never rewrites the file (FR-013).

Check the negative case explicitly:

```sh
git checkout crates/micold-client/tests/fixtures/layout_snapshot.txt
cargo test -p micold-client layout_snapshot   # fails
git status --short                            # fixture must be UNMODIFIED
```

- [x] The fixture is unmodified after a failing run. Checked by restoring it, re-running against
      still-modified source, and reading `git status --short`.

---

## Part D — extending coverage (US3, FR-016)

Register one additional covered state.

- [x] It took a change in **one place only** — *after* a defect was fixed. It did not at first: the
      new state rendered a sidebar, and the containment check demanded a second entry naming it.
      See T032; the exemption is now keyed by node path, which is what makes the claim true.
- [x] The fixture gained that state's layout and nothing else: 180 insertions, 0 deletions, one new
      header.

---

## Part E — cross-platform determinism (FR-006, SC-002)

This is the requirement most likely to be quietly violated, because it passes locally by
construction.

- [x] CI is green on Linux, macOS **and** Windows on the same commit, with the same committed
      fixture (FR-017, Principle VI). Commit `7065bba`, PR #62. The first attempt failed on Ubuntu
      while macOS and Windows passed — the icon font was never loaded, so icons measured against
      the host's fallback. See T035.
- [x] Two consecutive local runs produce identical output (FR-005), and every gate binary passes
      under `env -u DISPLAY -u WAYLAND_DISPLAY`.

> The gate measures text against a committed reference font precisely so this holds (research R2).
> The host still has its own fonts loaded — 391 faces were counted on the development machine — so
> the guard assertion exists to fail loudly if a same-named system font wins the family lookup.
> A *mass* geometry difference on one platform is that guard's failure mode; read it as a font
> problem, not as a layout regression.

---

## Part F — cost (SC-006, SC-006a)

Time the suite, not a single binary — the gates share test binaries, so which binary one lives in
is an implementation detail rather than something anyone waits on.

```sh
time mise run test
```

- [x] Completes in **under 60 seconds** locally. *(35.1s on 2026-07-29; 37.0s on 2026-08-04 with eleven states.)*

Then confirm the cost still scales with coverage and nothing else. Add one covered state to
`tests/support/covered_states.rs`, time `layout_snapshot` and `layout_text_overflow` with and
without it, and remove it again — the fixture will not match while it is there, which is expected
and does not affect the measurement, since the records are resolved before they are compared.

- [x] One additional covered state adds **no more than 3 seconds**. *(2.21s on 2026-07-29; the tenth 2.44s and the eleventh 2.09s on 2026-08-04. Measure **warm** — a first run after an edit rebuilds the test binaries and that compile lands inside the timing, which once read 6.23s for a state that costs 2.09s.)*

> Both numbers were budgets set before this work was measured, and both were amended once it was —
> see SC-006 in `spec.md` for what the original said and why it could not be met or, in the case of
> the 10% share, meaningfully held.

---

## Part G — the documented boundary (FR-015, SC-007)

Read the check's documentation alone, without opening the implementation. For each category below,
answer "would this be caught?" — and confirm the documentation answers it:

- [x] a padding change
- [x] a font-size change that widens a label
- [x] a colour change *(no — `style_snapshot` owns that)*
- [x] a widget swapped for a differently-drawn one of identical size *(no)*
- [x] a change visible only mid-animation *(no — records are taken at rest)*
- [x] a change visible only when scrolled *(no — offset zero)*
- [x] a dropdown that opens in the wrong place *(yes — overlay pass, R5)*

- [x] Every answer is stated in the documentation, not inferred by the reader — the
      "Would this be caught?" table in `docs/development/layout-snapshot.md` answers all seven
      directly, and gained a row distinguishing a clipped overhang from an unclipped one.

---

## What this feature must not have done

- [x] The application's appearance is unchanged (FR-019). `style_snapshot` still passes with **no
      fixture regeneration** — that is the mechanical proof.
- [x] No layout defect found while building the gate was silently fixed. Any that surfaced is
      recorded as a finding and addressed separately (FR-019).
- [x] Feature 017's parity claim is untouched; this feature does not reopen it (spec Assumptions).

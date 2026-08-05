# The layout snapshot

Three checks that pin *where things are*, so a layout regression fails the build instead of waiting
for someone to notice it on screen.

They exist because of a specific failure. Feature 017 pinned every colour the application resolves,
in both schemes, byte-for-byte — and pinned nothing about position or size. A long worktree name
drawn across its own close button shipped, was found by a person looking at the running application,
and could only be closed by eye, because the baseline needed to catch it had never been captured.

The boundary below is written out in full, including the parts these checks do **not** hold. A gate
that is quietly narrower than it looks is the exact failure this feature exists to correct, so
"would this be caught?" must be answerable from this page without reading the tests.

## Would this be caught?

| Change | Caught? | By what |
|---|---|---|
| A padding change | **Yes** | the fixture — every box moves or resizes |
| A font-size change that widens a label | **Yes** | the fixture, and the overflow gate if it now exceeds its box |
| A margin that pushes a control off its row | **Yes** | the fixture |
| A dropdown that opens in the wrong place | **Yes** | the fixture's overlay pass — one state opens one |
| Text painted past the box it was given | **Yes** | the text-overflow gate |
| A child laid out beyond a parent that will not clip it | **Yes** | the containment invariant |
| A child laid out beyond a parent that *does* clip it | **No** | nothing reads `draw`; see [the containment invariant](#3-the-containment-invariant--testsgatescontainmentrs) |
| A screen quietly dropped from coverage | **Yes** | coverage-narrowing check |
| A colour, border, radius or shadow change | **No** | `style_snapshot` owns these |
| A widget swapped for a differently-drawn one of identical size | **No** | nothing reads what is painted, only where |
| A change visible only mid-animation | **Almost never** — see [Animation](#animation) |
| A change visible only when scrolled | **No** | everything resolves at offset zero |
| A change in rasterised pixels — anti-aliasing, hinting, subpixel placement | **No** | no image is compared |
| A change to the typography a user actually sees | **Yes** — measured against the shipped faces |

If a change you care about is not in this table, it is probably not covered. Say so in review rather
than assuming; adding a row is cheap, and a wrong assumption here is what feature 017 paid for.

## The three checks

They are separate because they answer different questions, and the first cannot answer the other
two — that was measured, not assumed.

### 1. The geometry fixture — `tests/layout_snapshot.rs`

Resolves every element of every covered state and compares the result, byte for byte, against
`tests/fixtures/layout_snapshot.txt`. A failure names the element that moved and shows both
geometries.

This catches anything that **moves or resizes**. It is the broad one.

### 2. The text-overflow gate — `tests/layout_text_overflow.rs`

Draws each covered state and asks the renderer what it actually painted. For every piece of text,
the shaped paragraph's natural width must not exceed the bounds it was clipped to.

This exists because the fixture structurally cannot catch the defect that motivated the feature. The
sidebar label is `Length::Fill`, so its node is exactly the width its parent allots **whether the
text fits or not** — the fixture is byte-identical with and without the bug. What changes is what
gets painted, so this gate asks the renderer instead of the layout tree.

### 3. The containment invariant — `tests/gates/containment.rs`

Asserts that no layout node is laid out beyond the node that owns it, except where a wrapper is
known to reveal its child by clipping.

Every box it reads is already in the fixture, so why a second check? Because **a byte-compare
fixture can only catch changes, never pre-existing defects.** It records whatever it is shown as
correct; a defect older than the fixture is regenerated into the expected value and becomes the
baseline. Catching a defect that is already there needs an assertion about the numbers, not a
comparison against a file.

**Its limit is worth stating before its value.** A child laid outside its parent is not by itself
wrong: a widget may lay the child out at full size and clip it when drawing, and the layout tree
looks identical either way. Whether the overhang reaches the screen is decided in `draw`, which this
check does not read. So it catches an escape from a parent that does *not* clip, and is blind to the
distinction otherwise — which is why `CLIP_REVEALED` exists and why it does not expire.

That limit was found the hard way. The check was built against BUG-001, on the belief that
`Expand`'s oversized child *was* the defect. It was not: `Expand::layout` is unchanged by the fix,
the child is still laid out oversized on purpose, and the real cause was that `layout` never re-ran,
so the clip was handed stale bounds. Every escape this check reported survived the fix. It named the
right nodes for the wrong reason — and would have named them just as loudly with nothing wrong at
all. The mechanism BUG-001 broke is covered by `tests/animated_layout_relayouts.rs`, which asserts
on relayout requests rather than on boxes.

It runs inside the `layout_snapshot` test binary so it can reuse that binary's resolved records —
Cargo makes one binary per file directly under `tests/`, and a cache cannot cross processes.

## What is covered

Eleven states, listed in one place: `crates/micold-client/tests/support/covered_states.rs`. Feature
017's reduced parity set, plus the empty and error layouts no manual walkthrough reliably reached,
the Settings dialog, and the add-worktree dialog with its type dropdown open.

Every state resolves at one canonical window size, 1280×800, from fixed invented data. Nothing reads
your workspace, configuration or session store — a fixture recording the author's own machine would
be unreproducible anywhere else, including on the same machine tomorrow.

Both colour schemes are resolved. Only light is recorded; dark is asserted structurally identical
rather than duplicated, with two declared geometric exemptions that must each keep firing.

Both the base widget tree and widget-attached overlays are walked. The second pass matters: dialogs
and menus are composed in-tree and the base walk already sees them, but `material::Select` wraps a
`pick_list`, whose dropdown is laid out separately and is invisible to the base walk.

**That pass ran over every state and recorded nothing for most of this feature's life.** The only
widget reached through `Widget::overlay` is that dropdown, and no covered state opened one, so the
fixture held zero `over` records while the pass was documented as covering them — a check quietly
narrower than it looked, which is the failure this whole feature exists to correct, arrived at from
the opposite direction. `add-worktree-dialog-type-menu-open` now opens one, and
`the_overlay_pass_records_something_somewhere` fails if no state does.

Opening it is not a state you can set. `pick_list`'s open flag is private widget-tree state with no
accessor, so `StateUnderTest::pressing` *causes* it: a left press at the control's centre, the way a
person opens it. That press has to be preceded by settling the dialog's entrance transition — a
modal mounts at progress zero on purpose and swallows every event that is not a redraw until it has
appeared, so a press into a freshly built tree reaches nothing at all.

## What is not covered

### Appearance

Colour, border, radius, shadow, and every other resolved style value belong to `style_snapshot`.
These checks read geometry only. A widget replaced by a differently-drawn one of exactly the same
size passes all three without comment.

No pixels are compared. Anti-aliasing, hinting and subpixel placement are invisible here, by design
— a pixel comparison would fail on every platform for reasons that are not regressions.

### Animation

Records are taken at rest. A transition's intermediate frames are not recorded, and deliberately so:
a fixture holding a frame partway through a reveal would churn on any change to a duration or an
easing curve, which is motion's business rather than layout's.

**One exception, and it is narrow.** `revealing_states()` pins the sidebar's filter panel two frames
into its 90ms reveal, and the containment invariant — *only* that invariant — is asserted against it.
It is not recorded in the fixture and nothing checks its geometry. An invariant can be asked at a
moment no fixture would want to hold, and this one asks whether the reveal pushes any *neighbour*
out of the box that owns it — a defect visible only while the animation runs.

Pumping frames requires each redraw to carry a distinct `Instant`: a track ignores a repeat of the
frame it last advanced on, so N identical events advance it once. The pinned fraction is asserted
for exactly this reason, and it caught the apparatus reading 0.178 where it meant 0.356.

So: a change visible only mid-animation is not caught, unless it makes a node escape its parent
during that one pinned reveal.

### Scrolling

Everything resolves at scroll offset zero. A defect that appears only once a list is scrolled is not
covered by any of the three.

### Typography

**This was the largest gap and it is now closed** — feature 018 ships Roboto as the application
typeface, and these checks measure against the very files it ships (`assets/fonts/`), both Regular
and Medium, rather than a private copy.

It did not close by itself, and the way it nearly failed is worth keeping. When 018 landed, the gate
still pointed at its own `tests/fixtures/Roboto-Regular.ttf`. The two files were **different builds
of Roboto with different bytes** — a tenth of a pixel apart over the guard's reference string — so
the gate was measuring text against a face the application does not draw with: reproducible, stable,
and wrong. It also loaded only Regular, while `TypeRole` resolves weight >= 500 to Medium, so every
Medium label was measured with Regular metrics. Both are fixed; the duplicate file is deleted.

`tests/layout_apparatus.rs` still guards this: if a system font named Roboto ever wins the family
lookup, every measurement shifts at once. Read a mass geometry difference on one platform as a font
problem, not as a layout regression.

### Path stability

An element's identity is its depth-first index path (`0/2/1`), because a layout node carries no
name, type or id. Inserting a container near the root renumbers every descendant, so **one correct
structural edit can produce a large diff**. That is expected, not a defect. Named anchors are
re-pointed by hand as part of such a change.

### Anchors

An anchor is a name for a path, so a failure reads `sidebar.row.label` instead of
`0/0/0/2/0/0/0/2/0/0/2/0/1`. Declared per covered state in `tests/support/covered_states.rs`.

**Take the path from the resolved geometry, not from the source.** Reading the widget tree and
counting children is how anchors go wrong: `worktree_form.rs` pushes a `preview` element between
the fields and the action row, so by the source the actions are child 7 of the dialog column — but
`preview` returns a zero-height `Space` when there is nothing to preview and that produces no layout
node, so they are child 6. Other zero-sized elements in the same fixture *are* recorded, so there is
no rule to apply here; only the resolved tree is authoritative.

Two checks run on every suite. `an_anchor_whose_path_does_not_resolve_fails_naming_it` catches an
anchor pointing at nothing. That is not enough on its own — an anchor can resolve and still name the
wrong element, which is a bare path that lies — so each name is additionally held to a property only
its own element satisfies: `toolbar.title` must measure the width of `APP_NAME` at `type_scale::BODY`
and sit before `Toolbar`'s full-width zero-height spacer, and every `dialog.actions` must be the last
child of its column with two or more controls side by side on one line. Add an anchor, add its
property.

`dialog.actions` deliberately has a different path in each of the five dialog states: it is the last
child of a column whose length depends on which inputs the form shows. There is no single path that
means "the actions", which is why the check asserts the shape rather than the index.

## Exemptions currently in force

Each is required to keep firing. An exemption that has stopped being needed fails and must be struck
off, because a stale exemption widens a check silently — which is the failure mode these checks
exist to prevent. Note what that does and does not mean: a firing exemption proves the *shape* is
still there, not that it is still a defect. The first one below stayed green straight through the
fix for the bug it was written against.

- **`CLIP_REVEALED` — seven entries, one widget.** The sidebar's collapsed filter accordion, in
  every state that renders a sidebar. `material::Expand` reports a shrunken height to its parent
  while its child keeps full height, so at rest a 42px child sits inside a 0px parent — then reveals
  it top-down by clipping to its own bounds. The overhang is the mechanism, not a defect. Unlike the
  other two this one **does not expire**; what the assertion buys is that the list cannot grow
  quietly, since a new entry is either a new clip-revealing wrapper or a real escape.
- **Nodes parked entirely off-window are not checked for containment.**
  `material::NavigationDrawer` translates its inactive child by `-f32::MAX / 4` so the tree, node
  list and child list stay index-aligned without it occupying space. The exemption follows from the
  invariant's own premise — a node that is nowhere paints on nothing — but it does buy silence about
  content pushed off-screen *accidentally*, which is a real defect class and a different one.
- **`KNOWN_OVERFLOWS` — one chip, two paths.** The tag chip labelled `"Short"` inside the sidebar's
  collapsed filter panel wants 28.9px in the 19.2px the collapsed layout allows. Nothing paints
  there — the same collapsed `Expand` the entry above describes — and opening the panel makes it fit
  exactly. Proven rather than argued: `the_recorded_overflow_is_the_collapsed_filter_panel` reports
  one overflow closed and none open. Surfaced by feature 018's typography change, which widened the
  label past what the collapsed chip allows.
- **Two geometric differences between the schemes.** Structure is identical everywhere; two boxes
  differ in size because a string differs in length between schemes. Each carries its reason and
  each must keep firing.

## Working with the checks

### Accepting an intended change

```sh
UPDATE_LAYOUT_SNAPSHOT=1 mise exec -- cargo test -p micold-client layout_snapshot
```

Then read the diff. It is the reviewable artefact — a reviewer who has not run the application
should be able to see what moved and agree it was meant to.

The fixture is never written by a normal run. Regeneration is deliberate, and a missing fixture
fails loudly rather than being recreated.

### Adding a covered state

One entry in `tests/support/covered_states.rs`, and nothing else. That promise is enforced:
`tests/layout_coverage_registry.rs` scans for a second registration site and fails if it finds one.

Then regenerate the fixture. A new state costs roughly 2 seconds of suite time — the tenth measured
at 2.4s and the eleventh at 2.1s, each by timing the suite with and without it rather than deriving
it from a resolution count. Measure warm: a first run after any edit rebuilds the test binaries, and
that compile lands inside the timing. Doing it cold once put a state at 6.2s that is actually 2.1s.

One caveat the tenth state exposed. If the screen renders a sidebar, its collapsed filter accordion
is a clip-revealing wrapper, and the containment check used to need a second entry naming that
state. It is now keyed by node path alone, since being a clip-reveal is a property of the widget and
not of the screen — so registering really is one edit. Had that not been fixed, FR-016 would have
been false in exactly the way nobody notices: the claim held for the first nine states because they
were all added at once.

### Reading a failure

Failures name the covered state, the element — by anchor if it has one, otherwise by path — and both
geometries. If you need to find a node from an overflow or containment report, its path is a fixture
path: look it up directly in `layout_snapshot.txt`.

## Cost

The gates are about 32 seconds of a 37 second suite — `layout_snapshot` 17.0s (the fixture, the
containment invariant and the mid-reveal pin, sharing one process), `layout_text_overflow` 8.5s,
`layout_apparatus` 3.5s, and under 3s for the record-format, registry and regeneration checks.
Dominated by shaping real text across eleven screens in two schemes. Records are resolved once per
scheme and shared between checks; the naive form resolved the same views about 71 times.

Those binaries run in parallel with the rest of the suite, so their share of the total is smaller
than their sum — which is exactly why SC-006 budgets the suite rather than any one binary.

The budget is stated on the suite rather than on any one test binary — the checks share binaries, so
which binary a check lives in is an implementation detail. See SC-006 in
`specs/019-layout-snapshot-parity/spec.md`, which was amended once this had been measured, and
records why the original budget could not be held.

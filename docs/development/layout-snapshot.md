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
| A dropdown that opens in the wrong place | **Yes** | the fixture's overlay pass |
| Text painted past the box it was given | **Yes** | the text-overflow gate |
| A child laid out beyond its parent | **Yes** | the containment invariant |
| A screen quietly dropped from coverage | **Yes** | coverage-narrowing check |
| A colour, border, radius or shadow change | **No** | `style_snapshot` owns these |
| A widget swapped for a differently-drawn one of identical size | **No** | nothing reads what is painted, only where |
| A change visible only mid-animation | **Almost never** — see [Animation](#animation) |
| A change visible only when scrolled | **No** | everything resolves at offset zero |
| A change in rasterised pixels — anti-aliasing, hinting, subpixel placement | **No** | no image is compared |
| A change to the typography a user actually sees | **No** — see [Typography](#typography) |

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

Asserts that no layout node is laid out beyond the node that owns it.

Every box it reads is already in the fixture, so why a second check? Because **a byte-compare
fixture can only catch changes, never pre-existing defects.** It records whatever it is shown as
correct; a defect older than the fixture is regenerated into the expected value and becomes the
baseline. Catching a defect that is already there needs an assertion about the numbers, not a
comparison against a file.

It runs inside the `layout_snapshot` test binary so it can reuse that binary's resolved records —
Cargo makes one binary per file directly under `tests/`, and a cache cannot cross processes.

## What is covered

Nine states, listed in one place: `crates/micold-client/tests/support/covered_states.rs`. Feature
017's reduced parity set, plus the empty and error layouts no manual walkthrough reliably reached.

Every state resolves at one canonical window size, 1280×800, from fixed invented data. Nothing reads
your workspace, configuration or session store — a fixture recording the author's own machine would
be unreproducible anywhere else, including on the same machine tomorrow.

Both colour schemes are resolved. Only light is recorded; dark is asserted structurally identical
rather than duplicated, with two declared geometric exemptions that must each keep firing.

Both the base widget tree and widget-attached overlays are walked. The second pass matters: dialogs
and menus are composed in-tree and the base walk already sees them, but `material::Select` wraps a
`pick_list`, whose dropdown is laid out separately and is invisible to the base walk.

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
It is not recorded in the fixture and nothing checks its geometry. It exists to hold one defect
class: a wrapper that animates its own layout and paints outside the bounds it reports.

So: a change visible only mid-animation is not caught, unless it makes a child escape its parent
during that one pinned reveal.

### Scrolling

Everything resolves at scroll offset zero. A defect that appears only once a list is scrolled is not
covered by any of the three.

### Typography

**This is the largest gap, and it is temporary.** The checks measure against a committed Roboto
pinned as the snapshot's default font, not against the typeface the application actually requests.
Until feature 018 ships Roboto as the application typeface, text-derived geometry here is a
*consistent* measurement rather than a *faithful* one: it will catch a layout change, but the
absolute numbers are not the ones a user's machine produces.

The pinned font is also why these checks can claim identical results on Linux, macOS and Windows at
all. `tests/layout_apparatus.rs` guards it: if a system font named Roboto ever wins the family
lookup, every measurement shifts at once. Read a mass geometry difference on one platform as a font
problem, not as a layout regression.

### Path stability

An element's identity is its depth-first index path (`0/2/1`), because a layout node carries no
name, type or id. Inserting a container near the root renumbers every descendant, so **one correct
structural edit can produce a large diff**. That is expected, not a defect. Named anchors are
re-pointed by hand as part of such a change.

## Exemptions currently in force

Each is required to keep firing: if the underlying defect is fixed, the exemption fails and must be
struck off. A stale exemption widens a gate silently, which is the failure mode these checks exist
to prevent.

- **`KNOWN_ESCAPES` — seven entries, one defect.** The sidebar's collapsed filter accordion, in
  every state that renders a sidebar. `material::Expand` reports a shrunken height to its parent
  while its child keeps full height, so at rest a 42px child sits inside a 0px parent. Reported as
  BUG-001 against feature 017 and deliberately not fixed here: fixing it changes the sidebar's
  motion, and this feature is forbidden from changing the application's behaviour.
- **Nodes parked entirely off-window are not checked for containment.**
  `material::NavigationDrawer` translates its inactive child by `-f32::MAX / 4` so the tree, node
  list and child list stay index-aligned without it occupying space. The exemption follows from the
  invariant's own premise — a node that is nowhere paints on nothing — but it does buy silence about
  content pushed off-screen *accidentally*, which is a real defect class and a different one.
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

Then regenerate the fixture. A new state costs about 2.2 seconds of suite time.

### Reading a failure

Failures name the covered state, the element — by anchor if it has one, otherwise by path — and both
geometries. If you need to find a node from an overflow or containment report, its path is a fixture
path: look it up directly in `layout_snapshot.txt`.

## Cost

The gates are about 22 seconds of a 35 second suite, dominated by shaping real text across nine
screens in two schemes. Records are resolved once per scheme and shared between checks; the naive
form resolved the same views about 71 times.

The budget is stated on the suite rather than on any one test binary — the checks share binaries, so
which binary a check lives in is an implementation detail. See SC-006 in
`specs/019-layout-snapshot-parity/spec.md`, which was amended once this had been measured, and
records why the original budget could not be held.

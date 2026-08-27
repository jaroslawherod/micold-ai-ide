# Contract: the checks, their inputs and their exit semantics

Every check is a program that exits `0` or non-zero and prints what failed and where. None repairs
anything: a check with a repair path is a check that hides the thing it was written to find.

## Pre-merge — steps inside CI's existing `docs` job

The `docs` job is the one job feature 023 keeps running on documentation-only changes, which makes
it the only correct home for a check over documentation. These are shell scripts, not Rust tests:
`crates/micold-core/tests/documentation_is_not_read.rs` fails any Rust source under `crates/` whose
string literals resolve to a path marked `micold-docs`, and it must keep doing so.

### `site/checks/page-set.sh`

**Reads** `docs/**/*.md`, `docs/SUMMARY.md`, `site/theme/css/site.css`.

**Fails when** a Markdown file has no `SUMMARY.md` entry; an entry names a missing file; or
`site.css` contains a literal colour (`#rrggbb`, `rgb(`, `hsl(`), type size, `border-radius` value,
`box-shadow` value or transition duration instead of a `--micold-*` variable.

**Covers** FR-023, SC-002, SC-008, SC-014.

### `site/checks/media-references.sh`

**Reads** `docs/**/*.md`, `site/media.toml`, `site/capture/scenes/`.

**Fails when** a `<!-- media: id -->` directive names an id the manifest does not declare; a
declared id is referenced by no page; a declared `scene` names a script that does not exist; or an
entry has empty `alt`.

**Covers** FR-022, FR-011a, FR-014.

### `site/checks/links.sh --sources`

**Reads** `docs/**/*.md`. Runs lychee restricted to internal links and fragments.

**Fails when** a link between documentation pages does not resolve, or a fragment names no heading.
External links are not fetched — a publication must not fail because someone else's site is down.

**Covers** FR-021.

### `cargo test -p micold-core`

Carries `site_theme_contrast.rs` along with the emitter's own unit tests. Runs in the ordinary
suite, so a token change that would break the site's contrast fails in the change that made it.

**Covers** FR-033, FR-030.

## Pre-deploy — steps in `pages.yml`, all before the deploy step

### `site/checks/links.sh --built <dir>`

**Fails when** any internal link or fragment in the built HTML does not resolve. — FR-005, SC-003

### `site/checks/media-budget.sh <dir>`

**Reads** the built HTML and the files it references.

**Fails when** a page's still images total more than 1 MB, or a clip file exceeds 3 MB. Reports the
page, the assets and the total. Never downscales. — FR-015c, SC-012

**Also fails when** a file under `media/` is published that no page references. The per-page budgets
cannot see such a file — nothing links to it, so it is on no page's total — while the deploy still
carries it. The case that produced this rule is the frame directory a clip is encoded from: written
beside the encodes because `capture.sh` needs it there, copied into the book by the renderer, and
already contained in the `.webm` and the `.mp4`. `build.sh` drops those from the rendered copy after
the render; this asserts the result rather than trusting it. — FR-015c

### `site/checks/page-checks.mjs <dir>`

One headless browser, four assertions per run:

| Assertion | Fails when | Covers |
|---|---|---|
| WCAG 2.2 AA (axe-core), every page, both schemes | any violation — contrast, alt text, heading order, keyboard reachability, focus visibility | FR-027a, SC-013 |
| Home page, first viewport | the product name, an image of the application, or a link to the installation page is not within the first screen at a standard laptop viewport | FR-023a, SC-001 |
| Navigation depth | any documentation page is not reachable from any other in ≤2 steps | FR-023a, SC-006 |
| Search | a query for a guide topic does not return that topic's own page first, driving the site's own search box | FR-023a, SC-006 |
| Off-origin | any `<img>`, `<link>`, `<script>`, `<source>` or CSS `url()` resolves to another host | FR-031, SC-015 |

### `site/build.sh` — the completeness assertion

**Fails when** any entry in `site/media.toml` has no produced file. A missing capture is never
published as a gap and never filled from a previous run. — FR-011a, SC-004

## What no check covers, and where it goes instead

SC-001 and SC-006 are judgements a reader makes. The structural facts they rest on are asserted
above on every publication; the judgement itself is recorded once per release in
[quickstart.md](../quickstart.md) Part B, against the published site (FR-023a). Neither half
substitutes for the other, and the manual half is not a re-run of the automated one.

## Their own tests

Each shell check ships with `scripts/tests/<name>.test.sh`, written first, driving the script over
fixture trees that contain the failure it exists to catch — the precedent
`scripts/tests/classify-change.test.sh` and `scripts/tests/documentation-set.test.sh` already set.
A check that has never been observed failing is a check nobody knows works.

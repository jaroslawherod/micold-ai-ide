# Data model: Layout redesign — user guide, layout snapshot and visual pass

No application data changes (FR-020). The entities below are documentation and test-fixture
records.

## Owning section

| Field | Meaning |
|---|---|
| element | A region element: status bar, context header, tab strip; stage 2: Overview row, chip bar, sidebar groups, two-line rows, breadcrumb, jump-to field, Settings, overflow menu |
| page#anchor | The one user-guide section that describes it in full ([research R2](research.md#r2--owning-sections)) |
| first paragraph | Answers where the element is and what it is for (SC-007) |

Rules: exactly one per element (FR-004, FR-010); every other mention in the four pages links to
it (SC-002); its anchor resolves (FR-019).

## Removed element

| Field | Meaning |
|---|---|
| name | As a reader would search for it (`app bar`, `top bar`, `Overview button`, `bar under the terminal`, `filter button`, …) |
| region | Status bar / session area (stage 1) or sidebar / title bar (stage 2) |
| shipped replacement | The element now in its place, with its owning section |
| search patterns | The grep patterns that find it ([contracts/doc-edits.md](contracts/doc-edits.md) §Patterns) |

The list is read from what shipped (spec Terms): stage 2 adds the entries #1125 and #1127 actually
removed. A banner inside a region that still ships is not a removed element.

## Mention (one search hit)

| Field | Meaning |
|---|---|
| file:line | Under `docs/user-guide/` or `site/media.toml` (alt text, caption) |
| kind | *stale* (names a removed element as present), *ambiguous bar* (FR-003), *unlinked* (describes an element outside its owning section), *historical* (says it is gone: kept) |
| stage | 1 or 2, by the region it names |
| fix | Rewritten to the shipped element and a link to its owning section; nothing else on the page changes (FR-011) |

## Whole-window covered state

| Field | Value |
|---|---|
| name | `window-session-view`, `window-overview` |
| build | Composition in [research R5](research.md#r5--whole-window-covered-states-names-and-composition) |
| window | 1280×800 (canonical) |
| anchors | `shell.root`, `title_bar`, `shell.body`, `sidebar`, `session.area`, `status_bar`; paths from the resolved fixture |
| gates | fixture (`layout_snapshot`), text overflow, containment, `gates/window_regions_fit.rs` (R6); both schemes |

Invariants (SC-004): `title_bar.y = 0`, `title_bar.h = ` title bar height constant (44 after
#1127), `title_bar.y + title_bar.h = shell.body.y`, `shell.body.y + shell.body.h = status_bar.y`,
`status_bar.h = 28`, `status_bar.y + status_bar.h = 800`, `sidebar.h = session.area.h =
shell.body.h`, `sidebar.x + sidebar.w = session.area.x`.

## Visual-pass record

One file, `specs/1128-layout-redesign-docs/visual-pass.md`; contract
[contracts/visual-pass-record.md](contracts/visual-pass-record.md).

| Field | Meaning |
|---|---|
| commit | The `HEAD` the screenshots were drawn at |
| how it was run | Route (R8), host, renderer |
| screenshot | Board × theme, 1280×800 PNG under `visual-pass/` |
| verdict ×5 | Title bar, sidebar, session area, status bar, whole window: *matches* / *differs on purpose* (decision + spec) / *differs by defect* (region's issue + follow-up) |
| contrast | Per region and theme: the gate cited or the measured ratio (R9) |

20 verdicts in all (SC-006). Each *differs by defect* also adds a line to the ledger's *Follow-ups
not done* (D1).

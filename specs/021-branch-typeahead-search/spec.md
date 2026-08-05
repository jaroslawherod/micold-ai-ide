# Feature Specification: Branch Selector Type-Ahead Search

**Feature Branch**: `feat/make-branch-selector-a-type-a-head-search`

**Created**: 2026-08-04

**Status**: Draft

**Input**: User description: "the branch selector should allow type a head search. Should show branches that contain a given text or a close to it. For development we should create a type a head component found text should be highlighted in found items"

## Clarifications

### Session 2026-08-04

- Q: How should the branch results be revealed relative to the search field? → A: A floating list anchored beneath the field, shown while the field is focused and closed on pick or dismiss; the rest of the form never reflows.
- Q: Which characters get highlighted in an approximate (non-literal) match? → A: Per match kind — a subsequence match marks each corresponding character individually; a typo match marks the whole near-matching span as one run, absorbing the differing character.
- Q: (user directive) How should the search field and its result list be styled? → A: Follow Material Design 3 guidance, built from the existing shared Material component library and its tokens rather than bespoke styling.
- Q: At what search-text length should approximate matching start applying? → A: From 3 characters — a search text of 1 or 2 characters matches by literal substring only.
- Q: How should SC-002's typing responsiveness be made measurable? → A: Matching and ranking 500 branches completes within one 60 fps frame (≤16 ms) measured on the render-free logic, and typing drops no frames as measured by the frame-time probe already in the repository.
- Q: What happens when the matched text falls outside the visible part of a truncated branch name? → A: The row truncates with an ellipsis at whichever end is needed — leading, trailing, or both — so the highlighted run is always visible; rows stay one line tall.
- Q: Can a branch that is already checked out elsewhere still be picked from the results? → A: No. It is listed and visibly unavailable, with its reason readable in the row itself; it cannot be picked. This supersedes the previous behaviour, where such a branch could be selected and was refused only at the point of creating.
- Q: Where is the already-selected branch visible once the developer starts typing again? → A: The search field holds the search text only; the selection stays in the form's existing derived preview, and its row is marked as the current selection whenever it appears among the results.
- Q: How should SC-003 ("intended branch in the top five, 95% of the time") be made verifiable? → A: Against a pinned corpus of realistic branch names and a fixed set of query → intended-branch pairs, at least 95% of which must place the intended branch in the top five.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Narrow the branch list by typing (Priority: P1)

A developer creating a session from an existing branch opens the branch selector in a repository
that has dozens or hundreds of branches. Instead of scrolling a long list looking for the one they
want, they type a fragment of the branch name — `login`, `feat/rep`, `JIRA-412` — and the list
immediately shrinks to just the branches whose names contain that fragment, with the part that
matched visibly marked inside each row so the developer can see at a glance *why* each result is
there. They pick the branch they wanted and continue creating the session exactly as before.

**Why this priority**: This is the whole point of the feature and the only part required for it to
be useful. Substring search over a long branch list turns an unbounded scroll into a few keystrokes,
and it delivers value on its own even if approximate matching is never added.

**Independent Test**: Open the existing-branch picker in a repository with many branches, type a
fragment that several branch names share, and confirm that only those branches remain listed, that
the matched fragment is marked within each listed name, and that choosing one selects that branch
for the session being created.

**Acceptance Scenarios**:

1. **Given** the branch picker is open with the search field empty, **When** the developer has typed
   nothing, **Then** every branch that was previously offered is listed, in the order it was
   previously offered.
2. **Given** the repository has branches `feat/login`, `feat/logout`, and `chore/deps`, **When** the
   developer types `log`, **Then** only `feat/login` and `feat/logout` are listed and `chore/deps`
   is not.
3. **Given** the developer has typed `log`, **When** the results are shown, **Then** the `log` inside
   `feat/login` and `feat/logout` is visually distinguished from the rest of each name.
4. **Given** the repository has a branch `Feat/Login`, **When** the developer types `feat/l`, **Then**
   `Feat/Login` is listed — letter case does not affect whether a branch matches.
5. **Given** results are listed, **When** the developer picks one, **Then** that branch becomes the
   selected branch for the session being created and the picker reflects the choice, exactly as
   picking from the unfiltered list does today.
6. **Given** the developer has typed a fragment no branch contains, **When** the results are shown,
   **Then** the picker says plainly that no branch matches that search rather than showing an empty
   void, and the search text remains editable so the developer can correct it.
7. **Given** a branch is unavailable because it is already checked out elsewhere, **When** it matches
   the typed fragment, **Then** it is still listed, marked unavailable, and its reason is readable in
   the row — searching does not hide a branch or change why it cannot be used.
7a. **Given** an unavailable branch is listed, **When** the developer tries to pick it, **Then**
   nothing happens: it does not become the selected branch and the result list stays open.
8. **Given** the form's other fields are visible around the picker, **When** the number of matching
   branches changes from many to few or back, **Then** none of those fields moves — the result list
   floats over the form rather than displacing it.
9. **Given** a branch name too long to fit a result row, **When** the matched text sits near the end
   of the name, **Then** the row is truncated so that the matched text remains visible.

---

### User Story 2 - Forgiving matching for near misses (Priority: P2)

The developer half-remembers a branch name. They type `reportng` (a dropped letter), or `frep`
(initials of `feat/reporting`), and the branch they were after still shows up — ranked below any
branch that contains the typed text literally, so an exact hit is never buried under approximate
ones. The parts of the name the search text corresponds to are still marked, so the developer can
tell an approximate hit from a literal one.

**Why this priority**: Valuable but not essential — a developer who gets no results from a typo can
fix the typo. It builds directly on User Story 1's search field and result list and can ship
separately.

**Independent Test**: With a repository containing `feat/reporting`, type a search text one character
off (`reportng`) and a scattered abbreviation (`frep`), and confirm the branch appears in both cases
while a literal-substring match for a different search text still ranks above any approximate match.

**Acceptance Scenarios**:

1. **Given** the repository has `feat/reporting`, **When** the developer types `reportng`, **Then**
   `feat/reporting` is listed.
2. **Given** the repository has `feat/reporting`, **When** the developer types `frep`, **Then**
   `feat/reporting` is listed.
3. **Given** the repository has `feat/log` and `feat/dialog-cleanup`, **When** the developer types
   `log`, **Then** both are listed and `feat/log` — where the search text appears literally and
   earlier in the name — is listed above `feat/dialog-cleanup`.
4. **Given** an approximate match is listed, **When** the developer reads it, **Then** the characters
   of the name the search text corresponds to are marked, the same way a literal match's are.
5. **Given** the developer types a search text bearing no resemblance to any branch, **When** the
   results are shown, **Then** no branch is listed — approximate matching does not degrade into
   listing everything.
6. **Given** the developer has typed only one or two characters, **When** the results are shown,
   **Then** only branches containing those characters literally are listed; approximate matching does
   not apply until a third character is typed.

---

### User Story 3 - A reusable type-ahead the rest of the app can adopt (Priority: P3)

A developer building a future picker — a project switcher, a command palette, a file jump — needs the
same "type to narrow a list, see what matched" behavior. Rather than rebuilding it, they reach for the
shared type-ahead component the branch selector is built from, see it demonstrated with live examples
in the component gallery alongside the app's other shared components, and use it directly.

**Why this priority**: An investment in the codebase rather than in the user-facing feature. The
branch selector works whether or not the behavior is packaged reusably, but the project's
constitution requires shared UI primitives rather than per-feature widgets, so this is how the work
lands rather than an optional extra.

**Independent Test**: Open the component gallery, find the type-ahead entry, type into it, and confirm
it filters and marks matches over its sample data — without the branch selector or a repository being
involved at all.

**Acceptance Scenarios**:

1. **Given** the component gallery is open, **When** the developer looks for the type-ahead, **Then**
   it appears as its own catalogued entry alongside the other shared components.
2. **Given** the gallery's type-ahead example, **When** the developer types into it, **Then** it
   narrows its sample list and marks matched text, demonstrating the same behavior the branch selector
   shows.
3. **Given** the gallery is displayed in either light or dark appearance, **When** the type-ahead is
   shown, **Then** its field, its results, and its match marking are legible in both.
4. **Given** the branch selector's search, **When** its behavior is compared to the gallery entry's,
   **Then** they are the same component rather than two implementations that happen to look alike.

---

### Edge Cases

- **Search text matches nothing**: an explicit "no branches match" message, not a blank list; the typed
  text stays put so it can be edited rather than retyped.
- **Repository has no reusable branches at all**: the existing "this repository has no other branches"
  and "every branch is already checked out" messages still govern — the search field must not replace
  them with a "no matches" message that misstates why the list is empty.
- **Every match is unavailable**: matches are listed with their unavailability reasons, as they are
  without searching; the developer is not told "no matches" when branches matched but none can be used.
- **Search text longer than any branch name**: no matches, handled as above.
- **One- or two-character search text**: only literal matches are listed (FR-006a); a developer who
  types a third character may therefore see *more* results than before, not fewer, as approximate
  matching switches on. This is the intended behavior, not a defect.
- **Whitespace and case**: leading and trailing whitespace in the search text is ignored; matching is
  case-insensitive in both directions.
- **Very long branch names**: the row stays one line tall and truncates with an ellipsis placed at
  whichever end keeps the emphasised run in view — leading, trailing, or both. A branch is never
  listed with its emphasis hidden behind the truncation.
- **Punctuation-heavy names** (`feat/JIRA-412_retry-v2`): searching for a fragment spanning `/`, `-`,
  or `_` matches literally; those characters need no escaping by the developer.
- **A branch already selected, then narrowed away by the search**: the selection stands until the
  developer picks something else — narrowing the visible list does not silently clear a made choice.
- **Rapid typing**: results reflect the full typed text once typing stops; no stale result set from an
  earlier keystroke may remain displayed.
- **Hundreds of branches**: the list stays scrollable and responsive; searching must not become the
  slow path that scrolling was.

## Requirements *(mandatory)*

### Terminology

Two different things were both called "highlighting" in earlier drafts. They are now named apart, and
the two names are used consistently below:

- **Emphasis** — the treatment applied to the characters of a branch name that the search text
  matched. It is about the *text*.
- **Highlight** — the row the keyboard is currently on, moved with Up and Down. It is about the *row*.

A row may carry emphasis, the highlight, and the selection marker at once; all three must stay
individually legible.

### Functional Requirements

#### Searching the branch list

- **FR-001**: The existing-branch picker MUST offer a text field the developer can type into to narrow
  the branch list.
- **FR-001a**: Matching branches MUST be presented in a list anchored beneath the search field that
  floats above the rest of the form. Every other field in the form MUST keep its position regardless
  of how many branches match.
- **FR-001b**: The list MUST open when the search field takes focus — before anything is typed, so
  the branches on offer are visible from the outset — and MUST close when a branch is picked, when
  the developer dismisses it, or when the field loses focus.
- **FR-002**: While the search text is empty, the picker MUST offer exactly the branches, in exactly
  the order, it offers today.
- **FR-003**: A branch MUST be listed when its name contains the search text, ignoring letter case and
  ignoring leading and trailing whitespace in the search text.
- **FR-004**: A branch whose name neither contains the search text nor approximately matches it
  (FR-006) MUST NOT be listed.
- **FR-005**: Results MUST update as the developer types, without requiring a confirming keystroke, and
  MUST always correspond to the complete text currently in the field.

#### Approximate matching

- **FR-006**: A branch MUST also be listed when its name approximately matches the search text —
  specifically when the search characters occur in order but not adjacently within the name
  (abbreviation-style), or when the name contains a fragment differing from the search text by at most
  one inserted, deleted, or substituted character (typo-style).
- **FR-006a**: Approximate matching MUST apply only once the search text is at least 3 characters
  long. A search text of 1 or 2 characters MUST list exactly the branches matching it literally
  (FR-003), because single-edit tolerance over so short a text would match nearly every branch.
- **FR-007**: Results MUST be ordered so that literal-substring matches precede approximate matches;
  among literal matches, an earlier match position ranks higher; ties resolve to the picker's existing
  branch order, so results are stable and repeatable for the same search text.
- **FR-008**: Approximate matching MUST NOT cause a branch bearing no resemblance to the search text to
  be listed.

#### Showing what matched

- **FR-009**: Each listed branch MUST visually distinguish the characters of its name that the search
  text matched from the rest of the name.
- **FR-010**: Which characters carry emphasis MUST follow the kind of match:
  - a literal match emphasises the run of characters equal to the search text;
  - a subsequence match emphasises each corresponding character individually, leaving the characters
    between them unemphasised;
  - a typo match emphasises the whole near-matching span as one run, including the single differing
    character, so the emphasised text still reads as a word.
- **FR-011**: Emphasis MUST remain legible in both light and dark appearance, and MUST remain
  distinguishable from the keyboard highlight and from the selection marker when a row carries more
  than one of them.

#### Visual design

- **FR-011a**: The search field and its result list MUST follow Material Design 3 guidance for a
  text field with an attached menu of results: the field carries the design system's text-field
  treatment (label, leading search affordance, trailing clear affordance for FR-016), and the result
  list is a menu surface anchored to the field, with its own elevation, corner shape, and item
  height taken from the design system rather than chosen ad hoc.
- **FR-011b**: The field, the list, its rows, their states (hovered, focused, keyboard-highlighted,
  selected, unavailable), and the match emphasis MUST all draw their colour, type, shape,
  spacing, and elevation from the application's existing Material design tokens and shared
  components. No new one-off colour, type size, or spacing value may be introduced for this feature.
- **FR-011c**: Emphasis MUST be expressed as a Material emphasis treatment (a token-backed
  colour role and/or type weight) that keeps the unemphasised remainder of the branch name fully
  legible — it distinguishes without obscuring.
- **FR-011d**: A result row MUST occupy a single line. When a branch name is too long for the row,
  it MUST be truncated with an ellipsis placed at whichever end — leading, trailing, or both — keeps
  the emphasised run visible. A matching branch MUST NOT be listed with its emphasis hidden behind
  the truncation.

#### Preserving what the picker already does

- **FR-012**: Searching MUST NOT change which branches are eligible: unavailable branches MUST still be
  listed when they match, with their existing unavailability marking and explanation intact.
- **FR-012a**: An unavailable branch MUST NOT be pickable, and its reason MUST be readable in its own
  row. Attempting to pick it MUST do nothing — in particular it MUST NOT close the result list, and it
  MUST NOT become the selected branch. This supersedes the previous behaviour, in which such a branch
  could be selected and the refusal happened only on attempting to create.
- **FR-012b**: An unavailable row MUST be distinguishable from an available one by more than the
  absence of emphasis, so that "unavailable" and "did not match here" never look alike.
- **FR-013**: Picking an available branch from the search results MUST have exactly the effect that
  picking it from today's list has on the session being created.
- **FR-014**: A branch already selected MUST remain selected when the search text changes or is cleared,
  until the developer picks a different branch.
- **FR-014a**: The search field MUST hold only the search text — never the selected branch's name. The
  selected branch MUST stay visible in the form's existing derived preview regardless of what is typed.
- **FR-014b**: Whenever the selected branch appears among the results, its row MUST be marked as the
  current selection, so reopening or re-narrowing the list shows what is already chosen.
- **FR-015**: When the search text matches no branch, the picker MUST say so explicitly, and MUST keep
  this message distinct from the existing "repository has no other branches" and "every branch is
  already checked out" messages.
- **FR-016**: The developer MUST be able to clear the search text and return to the full list in a single
  action.

#### Keyboard operation

- **FR-017**: The developer MUST be able to move through the results and choose one from the keyboard
  without leaving the search field, and MUST be able to dismiss the results without choosing.
- **FR-017a**: Moving MUST stop at the ends of the list rather than wrapping around, MUST be able to
  land on an unavailable row so its reason can be read, and MUST NOT be able to choose one (FR-012a).
  Every other key MUST reach the search field, so typing never requires leaving it.

#### The reusable component

- **FR-018**: The type-ahead MUST be built as a shared, reusable UI component consumed by the branch
  selector, not as a widget private to the branch selector.
- **FR-019**: The component MUST accept an arbitrary list of items and MUST NOT contain any knowledge of
  branches, worktrees, or git.
- **FR-020**: The component MUST be catalogued in the component gallery with a live, typeable example,
  rendered correctly in both light and dark appearance.
- **FR-021**: The matching and ranking behavior in FR-003 through FR-008 (including FR-006a), the
  match positions that FR-009, FR-010, and FR-011d read, **and the keyboard rules in FR-017 and
  FR-017a** MUST all be determined by logic that is exercisable without rendering the interface. What
  remains in the rendering layer MUST be limited to translating input events into that logic's terms
  and applying its answers.
- **FR-021a**: The component MUST NOT be able to name a branch, a worktree, or a version-control
  concept, and this MUST be enforced the way the project's other component rules are — by a check
  that runs on every build rather than by review (FR-019).

#### Documentation

- **FR-022**: The user guide MUST describe branch search — that typing narrows the list, that near
  misses still surface, and what the emphasis means — shipped in the same change as the behavior.

### Key Entities

- **Branch candidate**: an existing branch offered by the picker — its name, its origin (local, or a
  named remote), and whether it is currently unavailable and why. Unchanged by this feature; it is what
  search filters, ranks, and emphasises.
- **Search text**: what the developer has typed. Empty means "no filtering".
- **Match**: the outcome of testing one candidate against the search text — whether it matched, how
  strongly (literal versus approximate, and where in the name), and which character positions of the name
  the search text corresponds to. The ranking in FR-007 and the emphasis in FR-009 and FR-010 are both
  read off this.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a repository with 200 branches, a developer who knows the branch name can select it by
  typing at most 8 characters, without scrolling.
- **SC-002**: Narrowing a 500-branch list keeps up with typing: producing the matched, ranked, and
  emphasis-annotated results for 500 branches takes at most 16 ms — one frame at 60 frames per
  second — and typing into the field drops no frames.
- **SC-003**: Against a pinned corpus of realistic branch names and a fixed set of query →
  intended-branch pairs, at least 95% of the pairs place the intended branch among the first five
  results. The corpus and the pairs are part of the deliverable, so the figure is measured rather
  than asserted, and a ranking change that degrades it fails.
- **SC-004**: A search text with a single typographical error — one wrong, missing, or extra character —
  still surfaces the intended branch.
- **SC-005**: In every listed result, a developer can identify which part of the name caused it to be
  listed without comparing it against what they typed.
- **SC-006**: Selecting an existing branch takes no more steps than it does today when the repository has
  few branches — search adds a shortcut, it does not add a required step.
- **SC-007**: The type-ahead exists once in the codebase: any future picker needing this behavior adopts
  the shared component without copying it.

## Assumptions

- **Scope is the existing-branch picker.** This feature changes the picker that appears when creating a
  session from an existing branch. Other lists in the application (project switcher, worktree sidebar,
  settings) are out of scope; they may adopt the shared component later.
- **Existing branches only.** The search field narrows a known list. It does not accept an arbitrary
  branch name — creating a new branch remains the other, already-existing mode of the form.
- **"Close to it" means abbreviation-style and single-typo matching** (FR-006). This is the common reading
  of near-miss search and covers both remembering only initials and mistyping. Broader fuzziness
  (multi-error tolerance, phonetic matching, synonym expansion) is deliberately excluded: it produces
  results a developer cannot explain, which conflicts with SC-005.
- **Matching is over the branch name only** — not the remote name, commit message, or author. A branch's
  remote is shown as it is today but is not searched.
- **No result cap.** All matching branches are listed; the list scrolls as it does today. Truncating
  results would let a correct branch silently vanish.
- **No persistence.** The search text lasts only as long as the open picker and is not remembered across
  openings of the form; branch relevance changes too quickly for a remembered query to help.
- **No new data source.** Search operates on the branch candidates the picker already obtains, including
  remote branches from the last fetch. Nothing is fetched or downloaded to support search.
- **The reusable component is a client-side UI primitive**, joining the existing shared component library
  and its gallery; the matching and ranking logic underneath it is render-free so it can be tested
  directly (FR-021).
- **Material Design 3 is the design language**, as it already is for the rest of the application. The
  search field and its result list are assembled from the existing shared Material components and
  token set (FR-011a–FR-011d); this feature introduces no new visual vocabulary, and any gap in the
  shared library is closed by extending it rather than by styling this picker specially.
- **Ellipsis placement follows the match, not a fixed end** (FR-011d). Material's guidance on
  truncating path-like strings permits an ellipsis wherever meaning is best preserved, and branch
  names are path-like; a fixed trailing ellipsis would hide exactly the text the developer searched for.
- **Constitutional constraints apply as usual**: matching, ranking, emphasis-position and
  keyboard-intent logic is
  developed test-first; the component is a shared primitive with a chainable builder API rather than a
  bespoke widget; both appearances are supported; behavior is identical on Linux, macOS, and Windows; and
  user-guide documentation ships in the same change.

# Research: Hide Agent Worktrees

Phase 0 for [plan.md](./plan.md). The Technical Context carried no `NEEDS CLARIFICATION` markers —
the stack, storage, and testing story are fixed by the constitution and the existing codebase — so
this phase resolves design questions instead: where classification lives, what the detection rule
actually is, and where the hiding is applied so it cannot be applied inconsistently.

## R1 — Where ownership is decided

**Decision**: A pure predicate in the render-free core, `src/worktree.rs`:

```rust
pub enum WorktreeOwner { User, Agent }
impl Worktree {
    pub fn owner(&self) -> WorktreeOwner;
    pub fn is_agent_owned(&self) -> bool;   // convenience over owner()
}
```

Ownership is **derived on demand** from `dir_name` + `branch`, not stored as a struct field and not
applied inside `reconcile()`.

**Rationale**: The spec's Key Entities call ownership "a derived property … with no extra stored
state or user input", and that is also the cheapest correct choice here. A stored field would have
to be set in `reconcile()`'s two construction sites plus every `Worktree { .. }` literal in the
existing test suite, and would then be a second source of truth that can disagree with the names it
was derived from. A method has neither problem, and `reconcile()` already guarantees the location
half of FR-005: it only emits worktrees whose parent is `worktrees_root`, for both git-registered
records and orphan on-disk directories. So the method only has to decide the naming half.

**Alternatives considered**:

- *Filter inside `reconcile()`* — drop agent worktrees at discovery so they never reach
  `State::worktrees`. Simplest possible hiding, and it was the obvious first design, but it makes
  FR-010's reveal control impossible: revealing would require a second discovery pass with
  different arguments. Rejected.
- *Filter in `main.rs::discover_worktrees()`* — same fatal problem as above, and it puts a product
  decision in the `gui`-only binary where `cargo test --no-default-features` cannot reach it
  (Principle I). Rejected.
- *Stored `owner: WorktreeOwner` field on `Worktree`* — marginally faster (computed once per
  discovery rather than once per render). Rejected: the cost is a handful of string comparisons
  over tens of worktrees, far below SC-004's perceptibility bar, and it is not worth the churn or
  the duplicate source of truth.

## R2 — The detection rule

**Decision**: A worktree is agent-owned when **either** identifier carries the reserved convention:

- its `dir_name` is `agent-` + a run of **≥ 16 ASCII hex digits and nothing else**, or
- its `branch` is `worktree-agent-` + the same.

Comparison is case-sensitive on the prefix and accepts either case for the hex digits
(`is_ascii_hexdigit`).

**Rationale**: The observed identifiers in this repository are 17 hex characters
(`agent-a885b42dc521fbda1`, `agent-abf6a58b16c3c9e6f`, `agent-ae474105b29fbeb68`, each paired with
`worktree-agent-<same>`), so a fixed length of 17 would work today but is brittle if the generator
changes width. A ≥ 16 floor keeps that tolerance while being long enough that FR-006's false
positives are not merely unlikely but essentially impossible: an ordinary word can only match if it
is 16+ characters drawn solely from `[0-9a-f]`. The user's own example `agent-foo` fails twice over
(too short, and `o` is not a hex digit), and `agent-face` fails on length alone — exactly the guard
US2 asks for.

Requiring the whole remainder to be hex (not just a hex prefix) is what makes it safe: a real
branch like `agent-deadbeef-cafe-refactor-the-parser` is long enough, but the `-refactor…` tail is
not hex, so it stays visible.

**Confirmed by clarification (2026-07-23)**: this was originally a plan-level choice over the
spec's unquantified "long … identifier". The clarify session promoted it into FR-005/FR-006
verbatim, so the ≥ 16 floor and the all-hex remainder are now spec-normative. The practical
consequence is that the 16-vs-15 boundary rows in the classification truth table are required
tests, not defensive extras.

**Alternatives considered**:

- *Prefix-only matching* (`starts_with("agent-")`) — what the raw feature request suggested. Hides
  `agent-foo`, directly violating FR-006. Rejected.
- *Exact length 17* — precise for today's generator, silently stops working if it widens.
  Rejected in favor of the ≥ 16 floor.
- *A regex crate* — a dependency for one pattern that is four `std` calls. Violates the
  "justify each addition" dependency constraint. Rejected.
- *Requiring both dir and branch to match* — stricter, but fails the detached-worktree edge case
  (no branch at all) and the orphan-directory case (branch is `None` by construction in
  `reconcile()`), both of which the spec requires to be hidden (FR-007). Rejected in favor of OR,
  which the spec's "Name/branch mismatch" edge case already sanctions.

## R3 — Where hiding is applied

**Decision**: One choke point on `State`:

```rust
pub fn visible_worktrees(&self) -> impl Iterator<Item = &Worktree>
```

which yields every worktree when `show_agent_worktrees` is on, and only user-owned ones when it is
off. `worktree_tree()` and `available_tag_filters()` are rebased onto it, and the sidebar's
empty-state hint asks it for a count.

**Rationale**: FR-003 and FR-004 are really one requirement — "everything downstream agrees" — and
the reliable way to satisfy them is to give downstream a single source rather than three
independent filters that can drift. Today `worktree_tree()` and `available_tag_filters()` both walk
`self.worktrees` directly, and `src/ui/sidebar.rs:102` reads `state.worktrees.is_empty()`; leaving
any of the three unrebased produces a visible bug (see R7). Routing all three through one accessor
makes "hidden here but counted there" unrepresentable without a new direct `self.worktrees` walk,
which is easy to spot in review.

`State::worktrees` itself keeps holding **every** discovered worktree. That preserves
`set_worktrees()`'s pruning semantics (expansion, hover, menu, delete-target and rename-override
state are pruned against the full name set, which is correct — a rename override for a worktree
that still exists must not be dropped just because it is hidden) and keeps the reveal control a
pure view concern.

**Alternatives considered**:

- *Filter at each render site in `src/ui/sidebar.rs`* — puts the rule in the untestable
  `gui`-only binary, three times over. Rejected on Principle I.
- *A separate `visible: Vec<Worktree>` field recomputed in `set_worktrees()`* — would go stale the
  moment the reveal control toggles without a re-discovery, requiring the toggle reducer to
  recompute it. A derived iterator cannot go stale. Rejected.

## R4 — The reveal control: state, placement, and the empty-panel trap

**Decision**: A transient `pub show_agent_worktrees: bool` on `State` (default `false`), toggled by
a new `Message::ShowAgentWorktreesToggled`, rendered as a chip in the existing filter accordion —
**above** `filter_bar()`'s early return, in its own always-present row.

**Rationale**: Transience is what delivers FR-010a for free: the field is not in the persisted
store, so every launch starts hidden with no migration and no settings key. It also matches
`sidebar_filters`, which is already documented as "Transient — not persisted" (`src/app.rs:652`).

**Amended by clarification (2026-07-23) — FR-010e**: the field is transient *and* project-scoped.
`restore_after_activation()` resets it to `false` on every project switch, exactly as it already
does for `default_expanded` (`src/app.rs:1378`) and its stated reason: view state switched on for
one project must not render in another. This is the one place the control deliberately diverges
from `sidebar_filters`, which does survive a switch. The deciding argument was the failure mode:
the filter accordion is collapsed by default, so a sticky toggle would show unexplained extra rows
in the incoming project with its cause hidden behind a closed panel. Remembering the choice per
project was rejected for the same reason — it makes "why are these here?" depend on history the
user cannot see.

The placement detail matters more than it looks. `filter_bar()` currently returns early with
"No tags to filter yet." when `available_tag_filters()` is empty — which is exactly the state of a
project whose only worktrees are agent-owned (they are hidden, so no tags are available, so the
panel would be empty). Rendering the reveal chip inside `filter_bar()` after that early return
would make the control vanish in the one case a user most needs it. So the accordion body becomes
`column![reveal_chip, filter_bar(..)]`, with the reveal chip unconditional. This is FR-010c stated
as a layout rule.

FR-010d falls out of keeping the two independent: the reveal reducer touches only
`show_agent_worktrees` and must not clear `sidebar_filters`, and because revealed worktrees flow
through the same `matches_filters()` call as everyone else in `filtered_worktree_tree()`, tag
filters apply to them without any extra code.

**Alternatives considered**:

- *A Settings entry* — discoverable only by users who go looking, and would need a persisted field,
  contradicting the spec's transience assumption. Rejected.
- *Persisting the toggle* — considered and explicitly rejected in the spec's Assumptions: the safe
  default (hidden) should be restored on every launch, and it saves a store-schema change.
- *A count badge ("3 hidden")* on the filter trigger — a nice affordance, but it is a new
  always-visible surface for something the spec wants out of the way, and nothing in the
  requirements asks for it. Deferred.

## R5 — Marking revealed worktrees

**Decision**: A new `Tag::Agent` variant in `src/naming.rs`, appended by `State::worktree_tags()`
when the worktree is agent-owned, rendered by the existing shared `Tag` chip primitive with a
neutral `on_surface_variant` accent. `Tag::Agent` deliberately has **no** corresponding `TagFilter`
variant.

**Rationale**: Rows already carry a chip strip (type, issue, and a `Status` chip for
missing/invalid), and `Status` is the exact precedent: a tag that exists to *label* a row and is
never offered as a filter. Following it means the badge costs one `match` arm in `tag_chip()`, no
new widget, and no new layout — Principle VIII satisfied by reuse rather than by a new component.

Keeping `Tag::Agent` out of `TagFilter` is what stops the badge from leaking into the filter chip
row: `available_tag_filters()` matches only `Tag::Type` and `Tag::Issue`, and its `Untyped`
detection now runs over `visible_worktrees()`, so a hidden agent worktree cannot conjure an
`Untyped` chip out of nowhere.

The neutral accent (rather than `error`, which `Status` uses, or a type color) reads as
"informational, not broken" — an agent worktree is not a fault condition.

**Label fixed by clarification (2026-07-23)**: the chip reads `agent`, not `assistant`. The spec's
prose deliberately says "assistant-owned" to stay vendor-neutral, but the user-visible word is
"agent" because that is what the `agent-` / `worktree-agent-` names on disk already say — a user
who goes looking in a terminal finds the same word they saw in the sidebar. See the Terminology
section of the reveal-control contract for the full string list.

**Alternatives considered**:

- *An icon on the row* — needs an `Icon` glyph addition and a font regeneration (see feature 009's
  `icon-font-coverage.md` contract for how much that costs). Disproportionate. Rejected.
- *Dimming the whole row* — conveys "disabled", which is wrong: per the spec's Assumptions a
  revealed row is fully actionable. Rejected.
- *A separate collapsible "Agent worktrees" group* — a genuinely nicer UI at a much larger scope
  (`SidebarEntry` gains a variant, `TreeView` gains nesting). Not required by any FR. Deferred.

## R6 — Promoting `ToggleChip`

**Decision**: Extract `src/ui/sidebar.rs::filter_chip()`'s look into a shared builder
`src/ui/material/toggle_chip.rs`:

```rust
ToggleChip::new(label, on_press, roles).active(bool).accent(fill, on).into()
```

`filter_chip()` is rewritten to delegate to it; the reveal chip is its second call site.

**Rationale**: The Component-reuse gate rejects "a duplicate or one-off widget instead of reusing
or extending a shared primitive". The reveal chip must look and behave exactly like a filter chip
(same pill, same active/inactive treatment, same accordion), so copying `filter_chip()`'s 30 lines
of button styling next to it is precisely the accretion that gate exists to prevent. Promoting is
also the pattern the constitution names — "the reusable primitive MUST be created in (or promoted
to) the shared library".

The builder shape is mandated by Principle VIII's builder-API rule: required inputs in `new()`
(label, message, roles), optional configuration chained (`.active()`, `.accent()`), terminating in
`impl From<ToggleChip> for Element`.

**Alternatives considered**:

- *Generalize `filter_chip()` in place to take `(label, active, message)`* — avoids a new file but
  leaves a shared-by-two-call-sites widget private to the sidebar, which is the letter of what the
  gate forbids. Rejected.
- *Reuse the `Tag` primitive with a press handler* — `Tag` is a non-interactive label pill with no
  pressed/inactive states; bolting interaction onto it would degrade both uses. Rejected.

## R7 — The empty-state and filter-chip consequences

**Decision**: Rebase both onto `visible_worktrees()`:

- `src/ui/sidebar.rs:102` — `state.worktrees.is_empty()` becomes a visible-count check.
- `State::available_tag_filters()` — iterates `visible_worktrees()` instead of `self.worktrees`.

**Rationale**: These are not cosmetic follow-ons; each is a defect that ships if missed, which is
why they are called out here rather than left to the implementer.

Without the first, a project whose only worktrees are agent-owned takes the `else if` branch and
shows *"No worktrees match the filter."* with a "Clear filters" button — even though no filter is
active and clearing it changes nothing. FR-003 and US1 scenario 2 require the honest
*"No worktrees yet. Add one to get started."* instead.

Without the second, hidden agent worktrees still contribute an `Untyped` chip (their machine names
carry no conventional type), so the filter panel offers a chip that matches nothing visible —
FR-003's "filter results, worktree counts" clause, failing in the most confusing possible way.

**Alternatives considered**: none worth recording — both are direct consequences of R3's choke
point being applied consistently.

## R8 — Sessions bound to a hidden worktree (FR-011)

**Decision**: Add nothing. Sessions are joined to worktrees inside `worktree_tree()`, so a session
whose worktree is not in the visible list simply renders nowhere — which is already exactly what
happens when a worktree is deleted externally. `set_worktrees()` prunes expansion, hover, menu,
delete-target, and rename state, but has never pruned sessions, and this feature does not change
that.

**Rationale**: This is the option the user chose (Q2: B) and it is also the one that adds no code.
Verified against the current implementation rather than assumed: `set_worktrees()`
(`src/app.rs:1279`) touches no session collection, and `sidebar_entries()` sources its worktree rows
from `filtered_worktree_tree()`, so hiding a worktree hides its sessions with it.

**Known, pre-existing wrinkle (accepted, not introduced)**: the main terminal area renders whatever
session is *selected*, independent of the sidebar tree (`src/ui/terminal.rs:842`). A selection
restored onto a session in a now-hidden worktree would keep rendering that terminal with no
corresponding sidebar row. This is identical to the existing behavior for a worktree deleted
outside the app, FR-011 explicitly asks for no dedicated handling, and the situation requires a
session to have been started in an agent worktree in the first place — which the app never offers
(FR-004). Recorded here so it is a known accepted state rather than a surprise in review.

## R9 — Discovery, git, and the filesystem stay untouched

**Decision**: `main.rs::discover_worktrees()`, `parse_worktrees()`, `classify()`, and `reconcile()`
are unchanged. No new git invocation, no new `fs` call.

**Rationale**: FR-008 and SC-005 require hiding to be presentation-only, and the cheapest way to
guarantee that is for the change to contain no code that could mutate anything. It also keeps
SC-004 trivially true: the added work per discovery is zero, and per render it is one prefix test
plus a bounded hex scan per worktree.

This is worth stating as an explicit decision because the tempting shortcut — "just skip agent
worktrees during discovery" — is both the R1/R3 alternative that was rejected and the one that
would make it easy to later "helpfully" prune them.

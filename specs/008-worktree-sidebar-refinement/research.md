# Phase 0 Research: Worktree Sidebar Refinement

All decisions resolve the two spec-level deferrals (tag palette, missing/invalid cue) plus
the technical unknowns surfaced while mapping the feature onto existing code. No open
`NEEDS CLARIFICATION` items remain.

---

## D1 — Friendly-name derivation & tag parsing live in `src/naming.rs`

**Decision**: Add two pure functions to `src/naming.rs` (the existing single source of truth
for the naming convention):
- `display_name(dir_name: &str) -> String` — strip the leading `ConventionalType` token and
  any Jira-style key, replace `-` with spaces, sentence-case the result (`feat-abc-123-login-page`
  → "Login page"). Empty remainder falls back to the raw `dir_name`.
- `parse_tags(dir_name: &str) -> Vec<Tag>` — return a `Type(ConventionalType)` tag when the
  first token matches a known type, plus an `Issue(String)` tag when a Jira-style key is
  present (regex `\b[A-Z][A-Z0-9]+-\d+\b`, upper-cased). Order: type first, then issue.

**Rationale**: `naming.rs` already parses/derives these names (`ConventionalType`, `derive()`,
`slugify()`), is pure and unit-tested, and has no I/O — the natural home. Deriving from
`dir_name` (not `branch`) matches what the sidebar already keys on and what `Worktree` stores.

**Alternatives considered**: (a) Parse from `branch` — rejected: `branch` is `Option`, and the
sidebar/`expanded`/session join all key on `dir_name`. (b) Store tags as persisted metadata —
rejected: they are a pure function of the name, so derive on the fly (no schema growth, no
staleness).

---

## D2 — Tag colors as per-type semantic role pairs in `src/tokens.rs`

**Decision**: Model each tag as a filled chip with its own `(fill, on_fill)` `Rgb` pair added
to the `Roles` struct, for each conventional type (feat, fix, chore, docs, refactor, test,
build, ci, perf, style) plus one `issue` style, defined in both `LIGHT` and `DARK` consts.
Extend the fixed pair array in `tests/tokens.rs` `pairs()` so every `(on_fill, fill)` pair is
automatically WCAG-AA checked in both schemes. The issue tag uses a distinct neutral/outline
style so it never collides visually with a type color.

**Rationale**: FR-005 requires a distinct, stable, well-defined color per type; FR-006/SC-007
require AA in both themes. The tokens layer already encodes semantic role pairs and the AA
test already enforces contrast for enumerated pairs — extending that array is the lowest-risk
way to make "tag text is legible" a compile-time-adjacent guarantee rather than a hope. Colors
are chosen from the Material palette style already used in `LIGHT`/`DARK`.

**Alternatives considered**: (a) Neutral chip background with only per-type *text* color —
rejected: harder to keep 10 text colors AA on one background, and less scannable than filled
chips. (b) Compute chip colors at runtime by hashing the type name — rejected: unstable,
untestable, and can silently fail AA. (c) Reuse existing roles (primary/error/…) for types —
rejected: too few distinct roles for 10 types and semantically misleading.

---

## D3 — Missing/invalid cue without the leading git icon

**Decision**: Replace the removed leading git-status icon (FR-010) with (1) rendering the
worktree name in the existing `error` role color and (2) a compact status tag ("missing" /
"invalid") in the tag row, plus the existing tooltip. Valid worktrees show no status tag.

**Rationale**: FR-011 requires the state stay visible through a lightweight cue that is not the
old icon. The `error` role is already AA-tested, and the tag row is the new, natural place for
worktree metadata — reusing it avoids inventing a second visual channel.

**Alternatives considered**: (a) Keep a smaller icon — rejected: FR-010 removes the icon to
reclaim space. (b) Only tint the name — rejected: color-only cues are weak for
color-vision-deficient users; the text status tag adds a non-color signal.

---

## D4 — Right-click context menu reuses `MenuOverlay`, extended with an anchor

**Decision**: Extend the shared `MenuOverlay` (`src/ui/material/menu.rs`) with a builder
`.anchor(...)` so it can open near the invoking worktree row instead of the hard-wired
top-right toolbar position. The row uses iced `mouse_area`'s right-press to emit
`Message::WorktreeMenuToggled(dir_name)`; the menu is rendered row-anchored via the existing
`stack![base, backdrop, panel]` pattern with the invisible full-window dismiss backdrop.
Menu items reuse `MenuItem { icon, label, message }` → Rename, Delete.

**Rationale**: Principle VIII forbids forking a bespoke context menu when a shared menu
primitive exists. `MenuOverlay` already implements the panel + outside-click-dismiss overlay;
it only lacks positioning flexibility, which is a clean additive builder enhancement. Anchoring
to the row (not the raw cursor pixel) avoids needing low-level cursor coordinates that iced's
`on_right_press` does not provide, and is predictable for users.

**Alternatives considered**: (a) New cursor-tracking context-menu widget — rejected: violates
the component-reuse gate and needs custom overlay geometry. (b) Reuse `Overlay` full-modal for
the menu — rejected: `Overlay` is for modal dialogs; a lightweight transient dropdown is better
modeled as `Option<String>` menu-open state (matching the `help_menu_open` bool pattern).

---

## D5 — Display-name override: additive field on `StoredProject`

**Decision**: Add `#[serde(default)] worktree_display_names: BTreeMap<String, String>` to
`StoredProject` (keyed by worktree `dir_name`), mirror it on the pure `Project`
(`worktree_names: BTreeMap<String, String>`), thread it through
`StoredCatalog::from_workspace`/`into_workspace`, and expose a `Workspace` mutation
(`set_worktree_name` / `clear_worktree_name`) analogous to `Workspace::rename`. Persist via the
existing `persist(&app.core)` call at the `src/main.rs` boundary after the reducer mutation.

**Rationale**: Worktrees are scoped to their project's repo and `dir_name` is unique within it,
so the override belongs on `StoredProject`. `#[serde(default)]` is the codebase's documented
convention for forward-compatible additions (no `SCHEMA_VERSION` bump). Reusing the rename +
`persist` flow keeps the pure/side-effect split intact and testable via
`JsonFileStore::at(temp)`.

**Alternatives considered**: (a) Separate top-level map in `StoredCatalog` — rejected: less
cohesive; would need its own keying across projects. (b) Persist the whole derived display
(name + tags) — rejected: tags are derived (D1), only the *override* is user data worth storing.
(c) New store file — rejected: unnecessary; one atomic `projects.json` is simpler and matches
local-first.

---

## D6 — Tag filtering: typed `TagFilter` set, OR-combined, transient

**Decision**: Model the active filter as `sidebar_filters: BTreeSet<TagFilter>` on `State`
(transient, not persisted), where `enum TagFilter { Type(ConventionalType), HasIssue, Untyped }`.
Add `Message::SidebarFilterToggled(TagFilter)` and `Message::SidebarFiltersCleared`. A worktree
is shown if the set is empty OR it matches ANY active filter (FR-025). `Untyped` matches
worktrees whose name yields no `Type` tag (FR-024, D2). The predicate is a pure function applied
in `State::worktree_tree()` / `build_items`. The filter UI is a compact row of toggle chips at
the top of the sidebar body with a one-tap clear (FR-026).

**Rationale**: A typed set makes "impossible filter" states unrepresentable and keeps the
predicate trivially unit-testable (Principle I/V). OR semantics were chosen in clarification.
Transient matches the nature of a quick view filter and avoids persistence surface; `sidebar_*`
fields already mix persisted (`sidebar_width`) and transient (`sidebar_dragging`) so this fits.

**Alternatives considered**: (a) Free-text search box — rejected: spec asks for tag-based
filtering, and typed chips are unambiguous and match the tags. (b) Persist filters — rejected:
not required; a stale filter hiding worktrees on next launch would surprise users.

---

## D7 — Sidebar 80% typography as explicit named constants

**Decision**: Introduce sidebar-scoped size constants (e.g. `sidebar` sizes = 80% of the
current `type_scale::BODY` 14 → 11 for the name and `type_scale::LABEL` 12 → 10 for tags/
sessions), defined once in `src/tokens.rs` and consumed only by the sidebar/`tree_view`
rendering. Do NOT mutate the app-wide `type_scale` constants (FR-012 scopes the reduction to the
sidebar).

**Rationale**: Named constants keep the 80% decision auditable and unit-checkable, and scoping
them to the sidebar honors "only within the sidebar" without touching the rest of the app.

**Alternatives considered**: (a) Multiply inline at each call site — rejected: scatters the
magic number, hard to verify. (b) Global 80% reduction — rejected: violates FR-012's sidebar-only
scope.

---

## D8 — Minimal padding by lowering existing spacing tokens in the sidebar

**Decision**: Reduce the sidebar's horizontal insets from `spacing::MD` (16px) toward
`spacing::XS` (4px): the outer content column horizontal padding and the per-depth row indent
in `tree_view.rs` use the minimal legible value (target ~4px outer, reduced indent step).
Vertical rhythm stays comfortable. Exact pixel values are finalized during implementation and
verified in `quickstart.md`.

**Rationale**: FR-009 asks for minimal left/right padding to reclaim width; the spacing scale
already provides `XS`. Using existing tokens keeps consistency and avoids new magic numbers.

**Alternatives considered**: (a) Zero padding — rejected: text would touch the panel edge and
the resize handle, hurting legibility. (b) New sub-`XS` token — deferred unless 4px proves too
wide in practice.

---

## D9 — Delete orchestration order (reuse `CleanupStep`)

**Decision**: On confirmed delete, the `src/main.rs` boundary: (1) kills the PTY children of the
worktree's sessions (iterate `app.terminals` for sessions whose `worktree_dir == dir_name`, call
`rt.kill()`, mirroring `stop_active_project_sessions`); (2) runs git removal in `CleanupStep`
order — `worktree_remove(force=true)` → `worktree_prune` → `branch_delete`; (3)
`fs::remove_dir_all(target)`; (4) drives the pure reducer to drop the session records + worktree
and clears `active_session` if it belonged to the deleted worktree; (5) re-runs
`discover_worktrees` → `Message::WorktreesLoaded`; (6) `persist(&app.core)`. Git steps are
idempotent and tolerate an already-missing worktree (FR-023, edge case: delete missing/invalid).

**Rationale**: The `CleanupStep` ordering already encodes the git constraint that a checked-out
branch cannot be deleted until the worktree registration is removed. `force=true` implements the
"branch removal is authoritative after explicit confirmation" assumption. Reusing the existing
kill + persist boundary keeps side effects out of the pure core (testable with `FakeGit` +
`FakeTerminalBackend`).

**Alternatives considered**: (a) Safe (non-force) branch delete — rejected: contradicts the
clarified decision to remove the branch even with unmerged work after confirmation. (b) Do git
work in the reducer — rejected: violates the pure/side-effect split and Principle I testability.

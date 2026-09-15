# Constitution amendment history

Sync Impact Reports moved out of `constitution.md`, so the skills that load the constitution on
every run read only the principles. Newest report first; prepend the next one here.

```text
SYNC IMPACT REPORT
==================
Version change: 1.6.0 → 1.6.1
Bump rationale: PATCH — the Documentation gate obliges exactly what it obliged before. What
  changes is that the gate now names the check that enforces it, the declaration that check
  reads, and the label that waives it. Nothing is newly required and nothing newly permitted,
  so this is a clarification rather than a material expansion.

  The amendment exists because the Documentation gate was the only one of its three siblings
  left standing on its wording alone. The TDD gate names `.gitattributes`/`micold-docs` and
  `crates/micold-core/tests/documentation_is_not_read.rs`; the Cross-platform gate defers to
  it by reference; the Documentation gate named nothing, so "user-facing changes MUST update
  the user guide" was a rule a reviewer had to remember. Feature 028 built the missing
  enforcer (`scripts/check-user-guide-updated.sh`, two steps in `ci.yml`'s `docs` job and 38
  cases of its own), and 1.6.0's own report closed by asking for exactly this: "Any future
  widening of the declaration SHOULD arrive with the same kind of check." The declaration was
  widened — `.gitattributes` gained `micold-user-facing` and `micold-user-guide` — and the
  check arrived with it. Recording it here is what keeps the gate's authority and the
  pipeline's behaviour the same document.

Modified in 1.6.1:
  - Development Workflow & Quality Gates, Documentation gate — gains an "enforcer" sub-bullet
    naming `scripts/check-user-guide-updated.sh`, the two `.gitattributes` attributes it reads,
    its fail-closed behaviour, and the `docs-not-needed` label.
  - Templates: ✅ no template carries the Documentation gate's mechanism, so none required a
    matching edit.

  Principle VII is deliberately untouched. It states the obligation; the gate states how the
  obligation is checked — the same division 1.6.0 drew between Principle VI and the TDD gate.

Prior report (1.5.0 → 1.6.0):
Version change: 1.5.0 → 1.6.0
Bump rationale: MINOR — the all-three-platform CI mandate gains one narrowly-scoped,
  explicitly-named exemption: a change whose every touched path is declared documentation MAY skip
  the suite. Consistent with 1.3.0 (Principle III's Default-session exception) and 1.5.0
  (Principle I's showcase-glue path), both of which treated a narrow, explicitly-named expansion of
  what is permitted as MINOR rather than PATCH.

  This amendment deliberately edits **three** statements, not one. The mandate appears in
  Principle VI's CI bullet, in the TDD gate, and in the Cross-platform gate; amending only the gate
  that prompted the work would have left the principle itself forbidding what the pipeline does. A
  gate narrowed in one place and left standing in two is not narrowed, it is contradicted — the
  same erosion 1.5.0's report objected to, approached from the other direction. Found by feature
  023's /speckit-analyze pass (finding D1): the plan originally named only the TDD gate.

Modified in 1.6.0:
  - Principle VI — the CI bullet is scoped to changes able to affect what is built or tested, and
    points at the Cross-platform gate for the exemption's definition.
  - Development Workflow & Quality Gates, TDD gate — scoped to changes able to affect what is
    built, linted, packaged, or tested, and carries the documentation-only exemption in full,
    including the declaration's location and the check that enforces its precondition.
  - Development Workflow & Quality Gates, Cross-platform gate — scoped by reference to the TDD
    gate's exemption.
  - Templates: ✅ `.specify/templates/plan-template.md` — the Principle VI Constitution-Check line
    ("CI covers all three") was left imprecise by this amendment and is updated in the same change
    to "CI covers all three for any change able to affect the build".

  Following 1.5.0's precedent, the exemption does not stand on its own wording:
  `crates/micold-core/tests/documentation_is_not_read.rs` asserts on every build that nothing under
  test reads a declared documentation path, so the precondition is checked rather than reviewed. It
  earned its place before it was written — the changelog is `include_str!`'d into the binary, so
  `CHANGELOG.md` is a build input and is explicitly carved out of the declaration. Any future
  widening of the declaration SHOULD arrive with the same kind of check.

Prior report (1.4.1 → 1.5.0):
Version change: 1.4.1 → 1.5.0
Bump rationale: MINOR — Principle I's GUI/process-spawn exception gains one further covered
  location: a development-only binary's own render glue (`src/showcase/`), introduced by feature
  020's component showcase. Its glue (`src/showcase/main.rs`, `src/showcase/gallery.rs`) is
  structurally unreachable from `tests/` for exactly the reason the exception already names, and
  its decision logic lives in a render-free reducer (`src/showcase/state.rs`) exactly as `app.rs`
  does. The rule the exception states is unchanged — thin glue with no decision logic, branching,
  or business rule of its own MAY be validated by a recorded `quickstart.md` procedure; anything
  with decision logic MUST still land in tested render-free logic first and MUST still follow
  Red-Green-Refactor there. Treated as MINOR by the same reasoning as 1.3.0 (Principle III's
  Default-session exception: one further sanctioned location, explicitly a material expansion) and
  1.2.0 (Principle VIII's builder-API sub-rule).

  Deliberately NOT a PATCH, though it was first drafted as one on the analogy of 1.4.1. That
  amendment corrected the exception's *description* of why `src/main.rs` and `src/ui/` are
  unreachable from `tests/`, and changed the set of exempt code not at all. This one adds a path:
  before it, code in `src/showcase/gallery.rs` was ordinary production code requiring a covering
  test; after it, that code may be validated by a manual procedure instead. The path list is
  constitutive rather than illustrative — it is what a reviewer checks a diff against — so
  extending it is an expansion, and a NON-NEGOTIABLE principle whose coverage can be widened by an
  edit filed as "wording" is a principle that erodes quietly.

Modified in 1.5.0:
  - Principle I — the GUI/process-spawn exception's covered locations now include a
    development-only binary's own render glue (`src/showcase/`) alongside `src/main.rs` and
    `src/ui/`, and the exception names `showcase/state.rs` beside `app.rs` as a render-free reducer
    it does NOT cover.
  - Templates: no edit required — `.specify/templates/plan-template.md`'s Principle I line asks
    for a TDD confirmation and names no paths, so it neither widens nor conflicts with this
    amendment.
  Follow-up (non-artifact): feature 020's `/speckit-analyze` pass raised this (finding C1). The
    plan's substantive claim was already sound — every state transition lives in the tested
    reducer — but the principle's text did not cover the paths it relied on. Because this widens a
    NON-NEGOTIABLE principle's exemption, it does not stand on the amendment alone:
    `crates/micold-client/tests/showcase_glue.rs` asserts that the two glue files hold no branch on
    showcase state, so the exception's precondition ("no decision logic of its own") is checked on
    every build rather than left to review. Any future extension of this path list SHOULD arrive
    with the same kind of check.

Prior report (1.4.0 → 1.4.1):
Version change: 1.4.0 → 1.4.1
Bump rationale: PATCH — a retrofit/convergence sweep (found via /speckit-converge on feature
  004-material-icons) confirmed Principle I's GUI/process-spawn wiring exception still described
  the codebase's *original* architecture (a single crate with a `gui` Cargo feature gating a
  render-free `lib` core vs. a `gui`-only binary). The workspace has since split into three crates
  (`micold-core`, `micold-client`, `micold-daemon`, introduced by feature 010-daemon-session-
  persistence); `micold-client` has no `gui` feature and unconditionally depends on `iced`, and it
  is where the render-free reducer (`app.rs`) and other pure modules now live alongside the actual
  rendering code. Tests still run headlessly without a display in practice — the exception's
  underlying rule (thin glue with no decision logic of its own MAY use quickstart.md validation)
  is unchanged — only the description of which crates/features provide the boundary was stale.
  Wording-only clarification, no principle added, removed, or materially expanded: PATCH.

Modified in 1.4.1:
  - Principle I — corrected the GUI/process-spawn wiring exception's description of the
    codebase's crate/feature split to match the current core/client/daemon workspace, replacing
    the stale single-crate "`gui`-feature binary" / "render-free `lib` core vs. `gui`-only binary"
    framing. The exception's actual rule is unchanged.
  - Templates: no edit required — none of the templates name the `gui` feature or crate layout
    directly.
  Follow-up (non-artifact): the same stale "`gui` feature" / "cargo test --no-default-features"
    language recurs across several early features' plan.md/tasks.md/CLAUDE.md (e.g. feature 004's
    FR-008, CLAUDE.md's description of `mise run test`). Not corrected here — out of scope for a
    single constitution patch; left for those artifacts' own convergence passes or a dedicated
    documentation sweep, per user decision during this retrofit session (not per-feature
    duplication).

Prior report (1.3.0 → 1.4.0):
Version change: 1.3.0 → 1.4.0
Bump rationale: MINOR bump — Principle I (Test-First Development) gains one narrowly-scoped,
  explicitly-named exception: thin GUI/process-spawn wiring in the `gui`-feature binary
  (`src/main.rs`, `src/ui/`) that only invokes already-unit-tested pure/core logic, with no
  decision logic or branching of its own, MAY be validated by a recorded `quickstart.md` manual
  procedure instead of an automated test — because this codebase's binary/library split (the
  render-free `lib` core vs. the `gui`-only binary) makes such glue structurally unreachable from
  `tests/`. This formalizes a practice already used (and already merged) in features 006 and 010,
  rather than introducing a new allowance. Treated as MINOR, consistent with how Principle III's
  1.3.0 Default-session exception and Principle VIII's 1.2.0 builder-API sub-rule were both
  treated as MINOR narrow expansions rather than MAJOR redefinitions.

Modified in 1.4.0:
  - Principle I — added the GUI/process-spawn wiring exception: this specific, narrow category of
    code MAY rely on `quickstart.md` validation instead of an automated test; all other production
    code is unaffected — the NON-NEGOTIABLE Red-Green-Refactor requirement is untouched for
    anything with decision logic, branching, or business rules of its own.
  - Templates: no edit required — `.specify/templates/plan-template.md`'s Constitution Check
    Principle I line already asks for the same TDD confirmation this exception narrows, not
    broadens.
  Follow-up (non-artifact): tracked by feature `specs/011-env-include-script/` — its
    `/speckit-analyze` pass surfaced that several of its tasks rely on this exact undocumented
    practice (mirroring features 006/010's own precedent), which prompted this amendment rather
    than leaving the tension unresolved.

Prior report (1.2.0 → 1.3.0):
Version change: 1.2.0 → 1.3.0
Bump rationale: MINOR bump — Principle III (Native Worktree Integration) gains one
  narrowly-scoped, explicitly-named exception: a session MAY now map to the project's
  own root directory (presented to users as "Default") instead of a git worktree. This
  is a material expansion of the principle's allowed session locations, not a removal
  or redefinition of its core commitment — worktree-bound sessions are entirely
  unaffected, the app still owns worktree lifecycle natively, and no other non-worktree
  location is sanctioned. Treated as MINOR, consistent with how Principle VIII's 1.2.0
  builder-API sub-rule (a comparable in-scope tightening/expansion) was treated as MINOR
  rather than MAJOR.

Modified in 1.3.0:
  - Principle III — added the "Default" project-root session exception: every session
    MUST map to either a git worktree or the project's root directory (the sanctioned
    "Default" location); no other non-worktree location is permitted. A Default session
    MUST NOT create/modify/remove a worktree and MUST NOT be presented as one.
  - Development Workflow & Quality Gates — Isolation & lifecycle gate now also requires
    integration-test coverage of the Default-session exception, not just worktree
    lifecycle.
  - Templates: ✅ .specify/templates/plan-template.md — Principle III Constitution-Check
    line updated to name the Default exception.
  - Templates: ✅ .specify/templates/spec-template.md — technology-agnostic; no principle
    conflict, no edit required.
  - Templates: ✅ .specify/templates/tasks-template.md — no task-category change required;
    the Default exception is covered by the existing Isolation & lifecycle gate's
    integration-test requirement.
  - Templates: ✅ .specify/templates/checklist-template.md — generic sample items; no
    principle references to reconcile, no edit required.
  Follow-up (non-artifact): tracked by feature `specs/010-root-dir-session/` (start a
    session in the project root without a worktree). This amendment was made to unblock
    that feature's `/speckit-plan` Constitution Check gate. User-guide docs
    (`docs/user-guide/worktrees-and-sessions.md`, `README.md`) still describe only
    worktree-bound sessions — intentionally left as-is here, since the Default-session
    behavior does not exist yet; updating them is that feature's own Documentation gate
    (Principle VII) deliverable, not part of this constitution amendment.

Prior report (1.1.0 → 1.2.0):
Version change: 1.1.0 → 1.2.0
Bump rationale: MINOR bump — Principle VIII (Reusable UI Component Foundation) was
  materially expanded with a builder-style component-API convention, and the
  Component-reuse review gate was tightened to enforce it. Additive expansion of an
  existing principle and its gate; no principle removed or redefined (MINOR, not MAJOR).

Modified in 1.2.0:
  - Principle VIII — added the builder-API rule: shared components expose a chainable
    builder terminating in `.into()` (iced widget idiom), not free/procedural functions
    with many positional parameters.
  - Development Workflow & Quality Gates — Component-reuse gate now rejects shared
    components added/edited as free-function/many-parameter signatures instead of the
    chainable builder-into-Element form (unless justified and recorded).
  - Templates: ✅ .specify/templates/plan-template.md — Principle VIII Constitution-Check
    line updated to name the builder-API convention.
  Follow-up (non-artifact): existing shared components in `src/ui/material/`
  (`icon_button`, `tree_view`, `toolbar`, `menu_trigger`, `menu_overlay`, `with_tooltip`,
  and feature 006's `terminal_pane`) SHOULD be migrated to the builder form; new
  components MUST follow it. Tracked as implementation tasks, not a deferred placeholder.

Prior report (1.0.0 → 1.1.0):
Version change: 1.0.0 → 1.1.0
Bump rationale: MINOR bump — a new core principle (VIII. Reusable UI Component
  Foundation) was added along with a matching Component-reuse review gate under
  Development Workflow & Quality Gates. No existing principle was removed or
  redefined, so this is additive (MINOR), not MAJOR.

Principles (final set — 8):
  - I.    Test-First Development (NON-NEGOTIABLE)
  - II.   Native Multi-Session Support
  - III.  Native Worktree Integration
  - IV.   Local-First Storage (NON-NEGOTIABLE)
  - V.    Rust + iced Stack
  - VI.   Cross-Platform Parity
  - VII.  Documentation as a First-Class Citizen
  - VIII. Reusable UI Component Foundation           (added in 1.1.0)

Added sections / expansions:
  - Core Principles — added Principle VIII (Reusable UI Component Foundation):
    UI built from a shared, reusable component library; features MUST reuse or
    extend shared primitives rather than fork bespoke one-off widgets. Kept at
    principle altitude — no specific component inventory hardcoded.
  - Development Workflow & Quality Gates — added Component-reuse gate: a change
    that introduces a duplicate/one-off widget instead of reusing or extending a
    shared primitive MUST be rejected in review unless explicitly justified and
    recorded.
  - (1.0.0) Technology, Storage & Licensing Constraints — Distribution clause
    (open source under an OSI-approved license; releases MUST ship Linux, macOS,
    and Windows builds).
  - (1.0.0) Governance — open-source contribution clause.

Removed sections: none

Templates requiring updates:
  - ✅ .specify/templates/plan-template.md — Constitution Check gets a new
       Principle VIII checkbox (component reuse).
  - ✅ .specify/templates/tasks-template.md — Test-First language already
       reconciled (tests MANDATORY); documentation surfaced as a mandatory
       per-story deliverable (Principle VII); no change required for VIII.
  - ✅ .specify/templates/spec-template.md — technology-agnostic; no principle
       conflict, no edit required.
  - ✅ .specify/templates/checklist-template.md — generic sample items; no
       principle references to reconcile, no edit required.

Follow-up TODOs:
  - Non-artifact: mise.toml declares `uv` only; add the Rust stable toolchain to
     satisfy Principle V + Technology Constraints. Tracked as an implementation
     task, not a deferred constitution placeholder.
  - Non-artifact: choose and add the OSI-approved LICENSE file required by the
     Distribution constraint and Governance section.
```

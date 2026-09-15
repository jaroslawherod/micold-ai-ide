<!-- Amendment history (Sync Impact Reports): constitution-history.md in this directory. -->

# Micold AI IDE Constitution

Micold AI IDE is a desktop, AI-assisted integrated development environment. This
constitution defines the non-negotiable principles and constraints that govern its
design, implementation, and evolution.

## Core Principles

### I. Test-First Development (NON-NEGOTIABLE)

Test-Driven Development is mandatory for all production code. The Red-Green-Refactor
cycle MUST be enforced strictly: a failing test is written and reviewed BEFORE the
implementation that satisfies it.

- Production code MUST NOT be merged without covering tests.
- A test MUST be observed failing (Red) before the corresponding implementation is
  written; implementation proceeds only until the test passes (Green); refactoring
  follows under a green suite.
- No feature is considered "done" until its tests exist, are meaningful, and pass.
- **Exception — GUI/process-spawn wiring.** Thin glue code in `micold-client`'s binaries and
  rendering layers (`src/main.rs`, `src/ui/`, and a development-only binary's own render glue such
  as `src/showcase/`) that only invokes already-unit-tested pure/core
  logic — with no decision logic, branching, or business rule of its own — MAY be validated by a
  recorded `quickstart.md` manual procedure instead of an automated test, because this codebase's
  crate split (the iced-free `micold-core` crate, plus `micold-client`'s own render-free reducer
  modules such as `app.rs` or `showcase/state.rs`, versus its `src/main.rs`/`src/ui/`/`src/showcase/`
  rendering glue) makes such glue structurally unreachable from `tests/`. This exception does NOT cover any code with decision
  logic, branching, or a business rule of its own — that MUST still land in tested pure/core logic
  first (`micold-core`, or a render-free module of the crate that needs it), and MUST still follow
  Red-Green-Refactor there.

Rationale: Tests written after the fact codify existing behavior rather than intended
behavior. Writing and reviewing the failing test first forces the specification of
behavior up front and guarantees every line of production code exists to satisfy a
verified expectation.

### II. Native Multi-Session Support

Sessions are first-class primitives of the application, not an afterthought bolted onto
a single-session core. The application MUST support multiple concurrent sessions that
are fully isolated from one another.

- Each session MUST be independently addressable.
- Each session MUST be persisted and restorable across application restarts.
- No session may leak state — filesystem, in-memory, or configuration — into another
  session.

Rationale: Developers routinely work across parallel lines of effort. Treating sessions
as core primitives with guaranteed isolation prevents cross-contamination of work and
makes concurrent, interruptible workflows reliable rather than accidental.

### III. Native Worktree Integration

Git worktrees are first-class primitives. The application MUST manage worktree
lifecycle natively. Every session MUST map to either a git worktree or the project's
own root directory — the single, sanctioned non-worktree location, presented to users
as "Default". No session may run in any other unmanaged or arbitrary directory.

- The application MUST create, switch between, and clean up worktrees on the user's
  behalf, without requiring the user to run manual git steps in a terminal.
- All file and version-control operations MUST be worktree-aware, operating against the
  worktree — or, for a Default session, the project root — bound to the active session.
- The project root MAY host session(s) under the "Default" label, alongside its
  worktrees. A Default session MUST NOT create, modify, or remove any git worktree, and
  MUST NOT be presented or styled as one.
- This exception is scoped narrowly to the project's own root: it exists solely to let
  a session run directly against the project's current checkout when branch isolation
  is unnecessary or undesired. It does not extend to any other non-worktree directory.

Rationale: Binding each isolated session to its own worktree is what makes true
concurrent, isolated development possible on a shared repository. Owning the worktree
lifecycle inside the application removes an entire class of user error and keeps session
isolation (Principle II) enforceable at the VCS layer for worktree-bound sessions. A
single, explicitly-named exception for the project root — rather than allowing sessions
against arbitrary non-worktree directories — accommodates work that is deliberately not
branch-isolated (quick commands, inspecting the current checkout) without opening the
door to unmanaged, ad hoc session locations.

### IV. Local-First Storage (NON-NEGOTIABLE)

All application and session state MUST live on the local filesystem. Core functionality
MUST NOT depend on any cloud service or network availability.

- The application MUST be fully functional offline.
- The user owns and controls all data. Nothing is transmitted off-device without the
  user's explicit, informed opt-in.

Rationale: A developer's code and working state are sensitive and must remain under the
developer's control. Local-first storage guarantees privacy, availability without
connectivity, and full ownership of data; any off-device transmission is an explicit,
auditable choice rather than a default.

### V. Rust + iced Stack

The application MUST be implemented in Rust and MUST use the iced framework for its GUI.
No alternative GUI framework may be introduced.

- The design MUST favor Rust's type system and ownership model to make invalid session
  and worktree states unrepresentable, rather than relying on runtime checks alone.

Rationale: A single, deliberately constrained stack keeps the codebase coherent and
leverages Rust's guarantees to enforce the other principles at compile time. Encoding
session/worktree invariants in the type system turns whole categories of isolation bugs
into build failures.

### VI. Cross-Platform Parity

The application MUST run on Linux, macOS, and Windows with feature parity. No platform is
a second-class target.

- Every user-facing feature MUST behave equivalently on all three platforms; a feature
  is not "done" until it works on Linux, macOS, and Windows.
- Platform-specific behavior MUST be isolated behind clear abstractions. Core logic MUST
  remain platform-agnostic and MUST NOT branch on the host operating system directly.
- CI MUST build and test the application on all three platforms, for every change able to
  affect what is built or tested. A change whose every touched path is declared documentation
  is exempt; the Cross-platform gate below carries the definition and the check that enforces
  it.

Rationale: Developers choose their own operating systems, and a tool that degrades on any
of them fragments the user base and the codebase. Confining platform differences to thin,
well-defined boundaries keeps the core testable once and portable everywhere.

### VII. Documentation as a First-Class Citizen

The user guide and documentation are deliverables, not afterthoughts. Documentation ships
with the code that it describes.

- Every user-facing feature MUST ship with corresponding user-guide documentation in the
  same change. A feature is not "done" until its documentation exists.
- Documentation MUST be kept in-repo and versioned alongside the code.
- Documentation MUST be verified in CI (for example: link checks, example correctness,
  and a successful docs build).

Rationale: Documentation written separately from the code drifts out of date and erodes
trust. Requiring docs in the same change, stored and versioned with the code and verified
in CI, keeps them accurate and makes the product usable by definition rather than by luck.

### VIII. Reusable UI Component Foundation

The user interface MUST be built from a shared, reusable component library rather than
per-feature bespoke widgets. Features MUST reuse or extend the shared UI primitives; they
MUST NOT fork one-off copies of a widget that a shared primitive already provides.

- When a needed UI element does not yet exist as a shared primitive, the reusable
  primitive MUST be created in (or promoted to) the shared library and consumed from
  there, rather than embedded privately in a single feature.
- Shared components MUST honor the same guarantees as the rest of the UI: light/dark
  theming (consistent with the iced-based app shell) and cross-platform parity
  (Principle VI).
- **Builder-style API (mandatory).** Shared components MUST expose an object-oriented,
  chainable builder API that mirrors iced's own widget idiom — NOT free/procedural
  functions that take many positional parameters. Specifically:
  - Each component is a public struct constructed with only its required inputs
    (for example `IconButton::new(icon, on_press)`); optional configuration is applied
    through chainable, `self`-consuming methods (for example `.tooltip(text)`,
    `.disabled(true)`, `.size(px)`, `.roles(r)`).
  - The chain terminates by converting into an `iced::Element` via
    `impl From<Component> for Element<'_, Message>`, so call sites end in `.into()`,
    exactly like iced's built-in `button` / `text_input` / `container` widgets.
  - Theming stays first-class: the active `Roles` / color scheme is supplied through the
    builder (a constructor argument or a `.roles(...)` / `.scheme(...)` method), preserving
    the light/dark theming guarantee above.
- The concrete catalog of components lives in the code and its documentation
  (Principle VII), NOT in this constitution. This principle governs the practice of
  reuse and the shape of component APIs, not a fixed inventory.

Rationale: A shared component foundation keeps the UI coherent, reduces duplicated and
divergent behavior, and makes theming and cross-platform fixes apply once rather than
feature-by-feature. Mandating reuse at the principle level prevents the slow accretion of
inconsistent one-off widgets that is expensive to reconcile later. Requiring the builder
API on top of that matches iced's own idiom (consistency and discoverability), keeps
optional parameters optional, and lets a component gain new configuration without breaking
every call site — which itself removes a common excuse to fork a bespoke widget.

## Technology, Storage & Licensing Constraints

- **Language**: Rust, stable toolchain, managed via `mise`.
- **GUI**: iced. No other GUI framework is permitted.
- **Persistence**: Local-only — plain files and/or an embedded store (e.g., SQLite or
  sled). No external database and no separate server process may be required for core
  functionality.
- **Distribution**: The project is open source under an OSI-approved license. Every
  release MUST provide builds for Linux, macOS, and Windows.
- **Dependencies**: Every dependency MUST be vetted for maintenance health and license
  compatibility before adoption. Prefer minimal, well-maintained crates; justify each
  addition against the principles above.

## Development Workflow & Quality Gates

- **TDD gate**: CI MUST run the full test suite on every change able to affect what is
  built, linted, packaged, or tested, on Linux, macOS, and Windows. Merges are blocked
  while the suite is red on any platform. This gate operationalizes Principle I.
  - **Exemption — documentation-only changes.** A change whose every touched path is
    declared documentation MAY skip the suite entirely. The declaration is a single list
    in the repository (`.gitattributes`, attribute `micold-docs`), and the exemption holds
    only while nothing under test reads those paths — a condition asserted on every build
    by `crates/micold-core/tests/documentation_is_not_read.rs`, not left to review. Any
    other path — source, manifest, lockfile, toolchain or tool configuration, build or
    helper script, workflow definition, or any file compiled into the binary — is NOT
    documentation, even when only its comments change.
- **Cross-platform gate**: CI MUST build and test the application on all three supported
  platforms, under the same scope and the same documentation-only exemption as the TDD
  gate above. This gate operationalizes Principle VI.
- **Documentation gate**: User-facing changes MUST update the user guide/docs in the same
  pull request, and the docs build MUST pass in CI. This gate operationalizes
  Principle VII.
  - **The enforcer.** The gate is mechanical, not a reviewer's recollection:
    `scripts/check-user-guide-updated.sh` blocks a pull request whose title declares a feature
    and whose diff touches a declared user-facing path without touching the user guide. Both
    sides of that comparison are declarations in the same single list the TDD gate's exemption
    uses (`.gitattributes`, attributes `micold-user-facing` and `micold-user-guide`), so the
    check hard-codes no paths and widening either side is an edit to the declaration rather
    than to the script. Every path the check cannot decide — a missing title, an unresolvable
    ref, no merge base — blocks rather than passes. The escape hatch is the `docs-not-needed`
    label, which records the reviewer's judgement in the pull request instead of leaving it
    unstated.
- **Review gate**: Every change MUST be reviewed before merge. Added complexity MUST be
  justified against these principles; unjustified complexity is grounds for rejection.
- **Component-reuse gate**: A change that introduces a duplicate or one-off widget instead
  of reusing or extending a shared UI primitive MUST be rejected in review, unless the
  divergence is explicitly justified and recorded. Additionally, a change that adds or edits
  a shared component using a free-function / many-positional-parameter signature instead of
  the chainable builder-into-`Element` form (Principle VIII) MUST be rejected in review,
  unless explicitly justified and recorded. This gate operationalizes Principle VIII.
- **Isolation & lifecycle gate**: Session isolation (Principle II), worktree lifecycle,
  and the project-root ("Default") session exception (Principle III) MUST be covered by
  integration tests, not unit tests alone.

## Governance

This constitution supersedes all other development practices. Where any other document,
convention, or habit conflicts with it, this constitution prevails.

- **Amendments**: Changes to this constitution require documented rationale, review, and
  approval before taking effect, together with a semantic version bump.
- **Versioning**: This constitution is versioned using semantic versioning.
  - MAJOR: Backward-incompatible governance changes, or the removal or redefinition of a
    principle.
  - MINOR: A new principle or section is added, or existing guidance is materially
    expanded.
  - PATCH: Clarifications, wording, and non-semantic refinements.
- **Open-source contributions**: As an open-source project, all contributions MUST follow
  these principles. The project's license and contribution guidelines are governed here.
- **Compliance**: All pull requests and reviews MUST verify compliance with these
  principles. Complexity that violates a principle MUST be either removed or explicitly
  justified and recorded.

**Version**: 1.6.1 | **Ratified**: 2026-07-13 | **Last Amended**: 2026-09-13

<!--
SYNC IMPACT REPORT
==================
Version change: (template / unversioned) → 1.0.0
Bump rationale: Initial ratification of the project constitution. The committed
  baseline was the raw, unfilled template; this is the first concrete, released
  version. Per the user's explicit direction, the initial version is 1.0.0 with a
  ratification date of 2026-07-13. No prior released version existed, so no
  amendment bump applies.

Principles (final set — 7):
  - I.   Test-First Development (NON-NEGOTIABLE)
  - II.  Native Multi-Session Support
  - III. Native Worktree Integration
  - IV.  Local-First Storage (NON-NEGOTIABLE)
  - V.   Rust + iced Stack
  - VI.  Cross-Platform Parity                       (added)
  - VII. Documentation as a First-Class Citizen      (added)

Added sections / expansions:
  - Technology, Storage & Licensing Constraints — added Distribution clause
    (open source under an OSI-approved license; releases MUST ship Linux, macOS,
    and Windows builds).
  - Development Workflow & Quality Gates — added Documentation gate and
    Cross-platform gate; TDD gate now runs on all three platforms.
  - Governance — added open-source contribution clause.

Removed sections: none

Templates requiring updates:
  - ✅ .specify/templates/tasks-template.md — Test-First language already
       reconciled (tests MANDATORY); documentation now surfaced as a mandatory
       per-story deliverable (Principle VII) and cross-platform validation noted
       (Principle VI).
  - ✅ .specify/templates/plan-template.md — Constitution Check populated with
       concrete gates covering all seven principles.
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
-->

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

Git worktrees are first-class primitives. Every session MUST map to a git worktree, and
the application MUST manage worktree lifecycle natively.

- The application MUST create, switch between, and clean up worktrees on the user's
  behalf, without requiring the user to run manual git steps in a terminal.
- All file and version-control operations MUST be worktree-aware, operating against the
  worktree bound to the active session.

Rationale: Binding each isolated session to its own worktree is what makes true
concurrent, isolated development possible on a shared repository. Owning the worktree
lifecycle inside the application removes an entire class of user error and keeps session
isolation (Principle II) enforceable at the VCS layer.

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
- CI MUST build and test the application on all three platforms.

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

- **TDD gate**: CI MUST run the full test suite on every change, on Linux, macOS, and
  Windows. Merges are blocked while the suite is red on any platform. This gate
  operationalizes Principle I.
- **Cross-platform gate**: CI MUST build and test the application on all three supported
  platforms. This gate operationalizes Principle VI.
- **Documentation gate**: User-facing changes MUST update the user guide/docs in the same
  pull request, and the docs build MUST pass in CI. This gate operationalizes
  Principle VII.
- **Review gate**: Every change MUST be reviewed before merge. Added complexity MUST be
  justified against these principles; unjustified complexity is grounds for rejection.
- **Isolation & lifecycle gate**: Session isolation (Principle II) and worktree lifecycle
  (Principle III) MUST be covered by integration tests, not unit tests alone.

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

**Version**: 1.0.0 | **Ratified**: 2026-07-13 | **Last Amended**: 2026-07-13

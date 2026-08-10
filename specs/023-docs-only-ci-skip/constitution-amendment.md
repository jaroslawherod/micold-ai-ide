# Drafted amendment text (applied by T032 / T033)

**Feature**: `specs/023-docs-only-ci-skip` | **Drafted**: 2026-08-09 | **Status**: not yet applied

This is the exact replacement text for the constitution and the plan template. It is **parked
here rather than applied**, because governance must not describe a pipeline that does not exist
yet: applying it before the workflow lands would leave the constitution saying documentation-only
changes may skip the suite while CI still requires four per-job checks and skips nothing. That is a
worse contradiction than the one it fixes.

Apply it as T032 and T033, in Phase 7, after the pipeline is live.

## 1. `.specify/memory/constitution.md:321` — Principle VI, third bullet

Replace:

```markdown
- CI MUST build and test the application on all three platforms.
```

With:

```markdown
- CI MUST build and test the application on all three platforms, for every change able to
  affect what is built or tested. A change whose every touched path is declared documentation
  is exempt; the Cross-platform gate below carries the definition and the check that enforces
  it.
```

## 2. `.specify/memory/constitution.md:394-396` — TDD gate

Replace:

```markdown
- **TDD gate**: CI MUST run the full test suite on every change, on Linux, macOS, and
  Windows. Merges are blocked while the suite is red on any platform. This gate
  operationalizes Principle I.
```

With:

```markdown
- **TDD gate**: CI MUST run the full test suite on every change able to affect what is built,
  linted, packaged, or tested, on Linux, macOS, and Windows. Merges are blocked while the
  suite is red on any platform. This gate operationalizes Principle I.
  - **Exemption — documentation-only changes.** A change whose every touched path is declared
    documentation MAY skip the suite entirely. The declaration is a single list in the
    repository (`.gitattributes`, attribute `micold-docs`), and the exemption holds only while
    nothing under test reads those paths — a condition asserted on every build by
    `crates/micold-core/tests/documentation_is_not_read.rs`, not left to review. Any other
    path — source, manifest, lockfile, toolchain or tool configuration, build or helper
    script, workflow definition, or any file compiled into the binary — is NOT documentation,
    even when only its comments change.
```

## 3. `.specify/memory/constitution.md:397-398` — Cross-platform gate

Replace:

```markdown
- **Cross-platform gate**: CI MUST build and test the application on all three supported
  platforms. This gate operationalizes Principle VI.
```

With:

```markdown
- **Cross-platform gate**: CI MUST build and test the application on all three supported
  platforms, under the same scope and the same documentation-only exemption as the TDD gate
  above. This gate operationalizes Principle VI.
```

## 4. Sync Impact Report — prepend to the header comment block

```markdown
SYNC IMPACT REPORT
==================
Version change: 1.5.0 → 1.6.0
Bump rationale: MINOR — the all-three-platform CI mandate gains one narrowly-scoped,
  explicitly-named exemption: a change whose every touched path is declared documentation MAY
  skip the suite. Consistent with 1.3.0 (Principle III's Default-session exception) and 1.5.0
  (Principle I's showcase-glue path), both of which treated a narrow, explicitly-named
  expansion of what is permitted as MINOR rather than PATCH.

  This amendment deliberately edits **three** statements, not one. The mandate appears in
  Principle VI's CI bullet, in the TDD gate, and in the Cross-platform gate; amending only the
  gate that prompted the work would have left the principle itself forbidding what the
  pipeline does. A gate that can be narrowed in one place and left standing in two is not
  narrowed, it is contradicted — which is the same erosion 1.5.0's report objected to from the
  other direction. (Found by feature 023's /speckit-analyze pass, finding D1: the original
  plan named only the TDD gate.)

Modified in 1.6.0:
  - Principle VI — the CI bullet is scoped to changes able to affect what is built or tested,
    and points at the Cross-platform gate for the exemption's definition.
  - Development Workflow & Quality Gates, TDD gate — scoped to changes able to affect what is
    built, linted, packaged, or tested, and carries the documentation-only exemption in full,
    including the declaration's location and the check that enforces its precondition.
  - Development Workflow & Quality Gates, Cross-platform gate — scoped by reference to the
    TDD gate's exemption.
  - Templates: ⚠️ `.specify/templates/plan-template.md`'s Principle VI line ("CI covers all
    three") is now imprecise and is updated in the same change to "CI covers all three for any
    change able to affect the build".

  Following 1.5.0's precedent, the exemption does not stand on its wording:
  `crates/micold-core/tests/documentation_is_not_read.rs` asserts on every build that nothing
  under test reads a declared documentation path, so the precondition is checked rather than
  reviewed. Any future widening of the declaration SHOULD arrive with the same kind of check.
```

## 5. `.specify/templates/plan-template.md:51` — Principle VI line

Replace:

```markdown
- [ ] **VI. Cross-Platform Parity**: Feature behaves equivalently on Linux, macOS, and Windows; platform-specific code sits behind clear abstractions; CI covers all three.
```

With:

```markdown
- [ ] **VI. Cross-Platform Parity**: Feature behaves equivalently on Linux, macOS, and Windows; platform-specific code sits behind clear abstractions; CI covers all three for any change able to affect the build.
```

## Note on the version bump

T032's scope grew when finding D1 landed: it now edits a **core principle**, not just two workflow
gates. MINOR still matches the repository's precedent — 1.5.0 edited Principle I itself and was
filed MINOR on the reasoning that a narrow, explicitly-named expansion is material but not a
redefinition. Confirm that reading deliberately when applying, rather than inheriting it from this
draft.

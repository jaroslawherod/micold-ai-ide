# Contract: The feature boundary

**Feature**: 028-feature-encapsulation | **Requirements**: FR-001 – FR-012

The interface this project exposes here is internal: it is the contract between a feature module,
the root, and the shell. It is stated as rules a guard can check, because
[research.md](../research.md) §R4 and 021's own record both show that a rule left to judgment
reaches one feature in ten.

Terms: **feature** = a module under `crates/micold-client/src/features/` other than `mod.rs`.
**Its view** = the files under `crates/micold-client/src/ui/` that draw it, listed in the guard.
**The root** = `app.rs`. **The shell** = `main.rs` and `crates/micold-client/src/shell/`.

---

## M1 — A feature declares its own vocabulary (FR-001)

A feature that has interactions declares `pub enum Msg` in its own module. Variants carry the
feature's own nouns and drop the feature's name as a prefix: `worktree_form::Msg::TicketChanged`,
not `AddWorktreeTicketChanged` — the type says which feature, so the variant does not have to.

## M2 — One entry point per feature (FR-002)

The root gains **one arm per feature**, never one per interaction. The entry point is one of the
three shapes in [data-model.md](../data-model.md) §1.1. A feature exposing both (A and B) splits by
*effect*, not by convenience: an arm belongs in B when it must return an `iced::Task`, and in A
otherwise. `worktree_form` is the reference implementation — 18 arms pure, 4 effectful.

## M3 — The root vocabulary retains only cross-cutting messages (FR-003)

`app::Message` holds a variant only when it is (a) a feature wrapper, (b) dispatched across more
than one feature by the overlay registry, or (c) produced by the environment — the iced runtime, a
subscription, the OS. The five that qualify today are enumerated in
[data-model.md](../data-model.md) §2 with the reason for each.

A message *produced* by the environment but consumed by exactly one feature belongs to that
feature, not to the root. `worktree_form::Msg::Created(Worktree)` is the settled precedent: the
daemon produces it and the form alone consumes it.

## M4 — A feature reducer does not write another feature's data (FR-004)

Unchanged from 021, restated because this feature widens its reach from one feature to ten. The
consequence travels as a `features::Outcome`; the root routes it in `app::interpret`. Held by
`tests/feature_write_isolation.rs`.

## M5 — Conversion is one feature at a time (FR-006)

Each feature's conversion is a single commit that leaves the workspace building, running and green.
No commit may depend on a later one. Order: [research.md](../research.md) §R9.

---

## S1 — A feature's state is named in that feature's module (FR-007, SC-007)

Every field the ownership map assigns to a feature is a field of that feature's own `State` struct,
declared in that feature's module. The root holds one field per feature. A maintainer reads one file
to know everything a feature remembers.

## S2 — A path with more than one reading feature stays shared, with its reason (FR-008)

A shared member stays a field of `app::State` and carries a doc comment naming the features that
read it and why it cannot be assigned to one. `workspace` is the only such member
([data-model.md](../data-model.md) §3.2).

## S3 — Moving state does not change its lifetime (FR-009)

No feature struct is ever assigned whole (`= <n>::State::default()`, or `..Default::default()` over
one). A group reset is a named operation on the feature module, so what survives a reset is written
down rather than implied by which fields happen to be in the struct.

## S4 — State moved into a component stays testable, or is re-covered (FR-010, FR-012)

Where FR-007 does move a path into a component, the component is a shared primitive with the
chainable builder API terminating in `.into()` (Principle VIII, held by
`tests/material_builder_api.rs`), and the behaviour the path governed is covered by a test that
opens no window. Today this contract binds nothing — the qualifying set is empty against
`tests/logical_state_ownership.rs` — and it is stated so that the first path that does qualify meets
it.

---

## B1 — No user-visible behaviour changes (FR-019, FR-021)

The pre-existing suite is this feature's behaviour specification. No assertion is removed. An
assertion whose *spelling* changes because a path moved is adjudicated in
`specs/028-feature-encapsulation/assertion-adjudications.md`, under a heading naming the task, with
the reason — the mechanism 021 built and this feature brings into scope
([research.md](../research.md) §R7).

## B2 — A decision forced by the restructuring is recorded and pinned (FR-020)

Where a behaviour turns out to be undefined today and the move forces a choice, the choice is
written down with its reasoning and a test is added for it. `ScrolledBeneathOverlay` — a root
variant with no producer — is the first known instance.

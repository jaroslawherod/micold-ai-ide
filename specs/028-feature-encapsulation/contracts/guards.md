# Contract: The three guards

**Feature**: 028-feature-encapsulation | **Requirements**: FR-013 – FR-018, SC-005

Story 3 is why this feature is not feature 021 again. Each guard below states its rule, what it
scans, how an exception is granted, and the violation that must be observed failing before the guard
is relied upon.

All three read source text, hold no window, and run on every supported platform (FR-018).

---

## G1 — No single-feature variant in the root vocabulary

**File**: `crates/micold-client/tests/root_vocabulary_is_cross_cutting.rs` (new)
**Requirement**: FR-013 | **Criterion**: SC-002

**Rule.** For each variant of `app::Message`, resolve the owner set of its arms in `State::update`
and in `main.rs::update_inner` — the `features::<n>::` calls they make, else the `shell::<n>::`
calls, else `overlay::registry::`. The guard **fails when that set is exactly one feature**, and
names the feature that should have declared the variant.

**Three verdicts, not two.** An owner set of size ≥ 2, or one containing only the registry, is
cross-cutting and passes. An **empty** owner set — a variant no arm resolves for, of which
`ScrolledBeneathOverlay` is one today — is reported in the guard's output and does not fail, because
a variant nobody produces is a different defect and forcing it into a feature would be the wrong fix
(contract B2 covers deciding about it).

**Exceptions.** `const ALLOWED: &[(&str, &str)]` — variant, written reason. Every entry names why
the variant is at the root despite resolving to one feature. An entry that stops being a violation
fails the guard, following `feature_write_isolation.rs`'s
`the_allowlist_names_only_live_violations` — an allowlist entry that outlives its reason is the same
failure as no guard at all, only quieter (spec Edge Cases).

**Non-vacuity probe (FR-017).** Add a variant to `app::Message` whose only arm calls
`features::help::about_opened`. Observe the guard fail naming `help`. Revert.

---

## G2 — No single-owner path in the root state

**File**: `crates/micold-client/tests/root_state_is_shared.rs` (new)
**Requirement**: FR-014 | **Criterion**: SC-003

**Rule.** Every public field of `app::State` is either a **feature struct** (its type resolves to
`crate::features::<n>::State`) or a **declared shared member** (listed in `SHARED`, with a written
reason naming the features that read it). A flat public field that is neither fails the guard,
which resolves its single writer through the same transitive `&mut State` scan
`feature_write_isolation.rs` already performs, and names that feature.

**Why this shape.** Stating the rule over field *types* rather than over a hand-derived reader set
is what Track 2A buys: after the migration, "no path with exactly one writing feature and no reader
outside it" is a property of the type, not of a `const` someone has to keep current. The 51-entry
`OWNERS` map shrinks to the shared members and stops being the thing a maintainer must consult
(SC-007).

**Exceptions.** `const SHARED: &[(&str, &str)]` — path, written reason. `workspace` is the only
entry planned.

**Non-vacuity probe (FR-017).** Add `pub scratch_pad: String` to `app::State`, written only from
`features/help.rs`. Observe the guard fail naming `help`. Revert.

---

## G3 — Every feature module has a reducer entry point

**File**: extends `crates/micold-client/tests/feature_registration_cost.rs`
**Requirement**: FR-015 | **Criterion**: SC-004

**Rule.** For each module under `src/features/` other than `mod.rs`: if it declares `pub enum Msg`,
it must expose shape A (`pub fn update(&mut State, Msg) -> Vec<Outcome>` in the same module) or
shape B (`pub fn update(&mut App, Msg) -> Task<Message>` in `src/shell/<n>.rs`). A module declaring
no `Msg` passes — that is FR-005's no-ceremony case, and it needs no allowlist entry.

**Why here rather than a new file.** `feature_registration_cost.rs` already enumerates feature
modules from the filesystem (not from a list, deliberately — its header records that a hardcoded
count went stale the day two modules were added) and already parses signatures taking the state
mutably, in both `&mut self` and `&mut State` spellings. Re-deriving that would be a second answer
to the same question.

**Exceptions.** None planned. If one is needed it takes the same `(&str, &str)` shape with a written
reason.

**Non-vacuity probe (FR-017).** Add `src/features/probe.rs` declaring `pub enum Msg { Tick }` and no
`update`. Observe the guard fail naming `probe`. Revert.

---

## Cross-platform enforcement (FR-018)

`.github/workflows/ci.yml`'s "component library + showcase gates, all platforms" step is the
authoritative list of client tests that run on macOS and Windows. It carries 11 tests today, and
none of feature 021's guards is among them — 021's T058 and T077 both recorded this and left it
open.

This feature adds to that step:

- the three guards above, and
- the four they extend: `feature_write_isolation`, `feature_registration_cost`,
  `root_is_routing_only`, `logical_state_ownership`.

All seven read source text from `CARGO_MANIFEST_DIR` and report findings by path, which is exactly
the shape where a `\` vs `/` difference goes unnoticed on a Linux-only run — the reason that step
exists.

## Assertion freeze (FR-021)

`scripts/check-assertions-frozen.sh` decides scope from the change and treats anything outside
feature 021 as report-only. Its `scope_reason()` gains feature 028 alongside 021, and
`specs/028-feature-encapsulation/assertion-adjudications.md` is created for the spelling changes
Track 2A forces. Without this, FR-021 has nothing enforcing it
([research.md](../research.md) §R7).

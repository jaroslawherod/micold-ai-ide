# Quickstart: Validating Feature Encapsulation

**Feature**: 028-feature-encapsulation | **Plan**: [plan.md](./plan.md)

This feature changes no pixel and no keystroke, so validation is **not** a click-through. It is
three things: the measurements that make the success criteria checkable (§A), the guards observed
failing before they are trusted (§B), and the one scenario a human still has to look at, because
"nothing changed" is only provable by looking (§C).

**Prerequisites**: `mise trust` once per fresh worktree. All commands run from the repository root.

---

## A. Measurements — reproducing the counts (SC-002, SC-003, SC-004)

Run these on `main` at `b43c11c` for the baseline and on the branch for the result. Each is a shell
one-liner over source text, so it needs no build.

### A.1 Root message vocabulary (SC-002)

```bash
awk '/^pub enum Message \{/{s=NR} /^\}/{if (s && !e) e=NR} END{print s+1, e-1}' \
  crates/micold-client/src/app.rs \
  | { read -r from to; awk -v f="$from" -v t="$to" 'NR>=f && NR<=t' \
      crates/micold-client/src/app.rs; } \
  | grep -oE '^    [A-Z][A-Za-z0-9_]*' | wc -l
```

Baseline **119** (the enum then spanned lines 43–504, which is why the range is derived rather
than written down). Target **15** (10 feature wrappers + 5 cross-cutting). The count is evidence;
the criterion is guard **G1** ([contracts/guards.md](./contracts/guards.md)).

Observed at T016: **15** — `Help`, `Project`, `Window`, `Settings`, `Worktree`, `Sidebar`,
`Session`, `WorktreeForm`, `Notifications`, `Connection`, then `ScrolledBeneathOverlay`,
`EscapePressed`, `OverlayTransitionFinished`, `WindowFocusChanged`, `NoOp`.

### A.2 Root application state (SC-003)

```bash
awk 'NR>=506 && NR<=720' crates/micold-client/src/app.rs \
  | grep -cE '^    pub [a-z_]+:'
```

Baseline **44 flat fields**. Target **9 feature structs + `workspace`**, the one declared shared
member (`connection` owns no state, so it gets no struct — T037). The
criterion is guard **G2**.

Observed at T048: **10** — nine of which are `crate::features::<n>::State` (count them with
`grep -cE '^    pub [a-z_]+: crate::features::'` over the same range), and `workspace`, which
`SHARED` declares. SC-007 reproduces beside it: each of the nine modules declares its own
`pub struct State`, and `features/connection.rs` says in its header why it declares none.

### A.3 Reproducing the attribution table

The per-variant owner table in [data-model.md](./data-model.md) §2 is derived, not transcribed. To
regenerate it, resolve each arm of `State::update` (`app.rs:866–1165`) and `update_inner`
(`main.rs:520–707`) to the `features::<n>::`, `shell::<n>::` and `overlay::registry::` calls it
makes; variants resolving to none are classified by their single emit site under
`crates/micold-client/src/ui/`. G1 performs the same resolution at test time, so after the guard
lands the table is checked rather than recomputed:

```bash
mise run test-core   # fast, no GUI
cargo test -p micold-client --test root_vocabulary_is_cross_cutting -- --nocapture
```

### A.4 Feature coverage (SC-004)

```bash
ls crates/micold-client/src/features/*.rs | grep -v mod.rs | wc -l   # 10 modules
grep -l 'pub enum Msg' crates/micold-client/src/features/*.rs | wc -l
```

Baseline **1 of 10**. Target **10 of 10**. Held by guard **G3**.

Observed at T026: **10 modules, 10 with a vocabulary**, and each has an entry point — nine are
shape A (`pub fn update(state: &mut State, …)` in the module), `connection` is shape B only
(`src/shell/connection.rs`), and `settings` has both. G3 reads the same two shapes, so the count
above is evidence and the guard is the criterion.

SC-002 reproduces alongside it: §A.1 reports **15**, and every one of the ten wrappers resolves to
its own feature under G1, so no root variant is produced and consumed by exactly one feature.

### A.5 The cost of one interaction (SC-001)

SC-001 is measured once rather than guarded (clarification of 2026-08-25). Make the change and
count:

```bash
git diff --name-only | wc -l
```

Observed at T051: **2**. A "Collapse all" interaction was added to the sidebar — a `Msg::AllCollapsed`
variant, a `pub fn all_collapsed` writing `state.sidebar.expanded`, its arm in `update`, and a button
in the view emitting it — and the two files it touched were `src/features/sidebar.rs` and
`src/ui/sidebar.rs`. Nothing in `app.rs`, `main.rs`, `features/mod.rs` or any other feature. All four
guards stayed green across the injection, which is the other half of the claim: the pattern needs no
root registration, so there is no third file to forget. Reverted with
`git checkout -- crates/micold-client`.

Before this feature the same interaction cost four files — the variant in `app::Message`, the arm in
`State::update`, the field in `app::State`, and the view — which is the coupling SC-001 names.

---

## B. Guards observed failing (SC-005, FR-017)

A guard nobody has seen fail is a guard nobody knows works — 021's own record has two probes that
did not compile, so nothing ran and both looked like passes. For each of the three, inject the
violation, observe the named failure, revert.

| Guard | Injection | Expected failure |
|---|---|---|
| **G1** | add a `Message` variant whose only arm calls `features::help::about_opened` | fails, naming `help` |
| **G2** | add `pub scratch_pad: String` to `app::State`, written only from `features/help.rs` | fails, naming `help` |
| **G3** | add `src/features/probe.rs` with `pub enum Msg { Tick }` and no `update` | fails, naming `probe` |

```bash
# after each injection
cargo test -p micold-client --test root_vocabulary_is_cross_cutting
cargo test -p micold-client --test root_state_is_shared
cargo test -p micold-client --test feature_registration_cost
git checkout -- crates/micold-client   # revert the injection
```

Record each observed failure message in `tasks.md` beside its task. An injection that fails to
compile has demonstrated nothing — check that the test actually ran.

---

## C. Behaviour preservation

### C.1 The whole suite, every step (SC-006, FR-021)

```bash
mise run test          # whole workspace, matches CI
```

Green after **every** commit, not only at the end (FR-006). No assertion removed — the freeze
reports it:

```bash
scripts/check-assertions-frozen.sh
```

Once `scope_reason()` recognises 028 this **blocks** rather than reports; a spelling change forced
by a moved path is adjudicated in `specs/028-feature-encapsulation/assertion-adjudications.md`.

### C.2 Cross-platform (FR-018)

The guards must appear in `.github/workflows/ci.yml`'s all-platforms step, not only in the Linux
full-workspace run:

```bash
grep -A 20 'component library + showcase gates' .github/workflows/ci.yml
```

Expect the three new guards and the four they extend among the `--test` entries. Confirm green on
all three matrix jobs before the last conversion merges.

### C.3 No extra frames while idle (SC-008, FR-011)

Already covered by an existing gate in the all-platforms list:

```bash
cargo test -p micold-client --test idle_requests_no_frames
```

### C.4 The one thing that needs eyes (FR-019, edge case: lifetime)

Everything above proves the *shape* changed correctly. What it cannot prove is that a draft, a
scroll position or a selection still survives what it survived before, because that is the class of
change invariant **S3** exists to prevent and a struct move is exactly how it would slip through.

Run the repository's `visual-pass` skill, or by hand:

```bash
mise run run
```

Then, for each, confirm the behaviour is **unchanged from `main`**:

1. **Add-worktree draft survives a refusal.** Open Add worktree, type a name that collides, submit,
   read the error — the form stays open with the text still in it.
2. **Sidebar expansion survives a re-discovery.** Expand two worktrees, create a third from the
   form, confirm the two stay expanded and the hover/menu state is not reset.
3. **Settings draft survives a cancel-and-reopen.** Change scrollback, cancel, reopen — the saved
   value is shown, not the abandoned draft.
4. **Terminal selection across a tab switch and back.** It is *cleared*, and the phrasing this
   line used to carry ("survives") was wrong: `shell::daemon_sync::view_and_start` opens with
   `app.selection = None;` deliberately, and so does the shell switch beside it — both identical
   on `main` since feature 021. Unchanged from `main` is the requirement, so confirm the clear,
   not a survival.
5. **Project switch resets what it resets today.** Switch projects and confirm the agent-worktree
   reveal and the default-expanded row behave as they do on `main` (`Outcome::ProjectEntered`).
6. **A dismissed popover loses its state; a dismissed dialog keeps what it keeps.** Open the sidebar
   filter panel, set a filter, dismiss with Escape, reopen — the filter is still applied.

Any difference here is a bug in this feature, not a decision to make now: FR-020 requires it be
recorded and pinned by a test before it is accepted.

Run and recorded on 2026-08-27 — see [visual-pass.md](./visual-pass.md), which carries the six
comparison images and the provenance of the binaries they were taken from. All six matched
`main`; no difference needed pinning.

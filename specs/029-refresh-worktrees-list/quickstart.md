# Quickstart: Refresh the Worktree List on Demand

**Feature**: 029-refresh-worktrees-list | **Plan**: [plan.md](./plan.md)

Validation splits the way Constitution Principle I splits the code. **Part A** is everything with a
decision in it — the wire arm, the reducer, the in-flight guard, the timeout, the predicate — and it
is automated. **Part B** is the thin iced wiring the GUI-glue exception covers (`src/ui/sidebar.rs`,
one routing arm in `src/main.rs`), plus the one thing no assertion can make: whether the button
looks like it belongs beside the two that were already there.

Run Part A first. Part B means nothing until it passes.

**Prerequisites**: `mise trust` once per fresh worktree. All commands run from the repository root.

---

## Part A — automated

### A.1 The whole gate

```bash
cargo fmt --all -- --check                     # CI stops here first; run it first too
mise run test                                  # whole workspace, matches CI
cargo clippy --workspace --all-targets
```

> `mise run test` and a bare `cargo clippy` share `target-shared/`, so the second may print
> `Blocking waiting for file lock on build directory` while another worktree builds. That is
> correct behaviour — wait it out (see CLAUDE.md).

### A.2 The gates that must contain new coverage

Each row is an obligation, not a suggestion: the row's test must fail against the tree without this
feature's change to the file beside it.

| Gate | Covers | Requirement |
|---|---|---|
| `micold-core/tests/protocol_roundtrip.rs` | `ClientMsg::WorktreeRefresh` survives a serialize/deserialize round trip | FR-003 |
| `micold-core/tests/protocol_auth.rs`, `micold-core/tests/schema_hash.rs` | `PROTOCOL_VERSION` reads 10 and `SCHEMA_HASH` moved exactly once | contracts §6 |
| `micold-daemon/tests/worktree_refresh.rs` **(new)** | the daemon re-discovers, broadcasts `CatalogChanged`, then acks — in that order; a worktree created behind the daemon's back appears | FR-002, FR-009, contracts §4 |
| `micold-client/tests/features_worktree.rs` | `RefreshRequested` → `refreshing = true`; `RefreshFinished` → `false`; a second `RefreshRequested` while refreshing is dropped; none of the three touches the listing | FR-006, FR-007, data-model T1–T6 |
| `micold-client/tests/app_state.rs` | `can_refresh_worktrees()` cases P1–P3 | FR-005, FR-006 |
| `micold-client/tests/icons.rs`, `micold-client/tests/icons_font.rs` | `Icon::Refresh` is in `ALL`, has a glyph, and its codepoint is really in the shipped font | research R8 |
| `micold-client/tests/layout_snapshot.rs` (the fixture's sidebar-header rows) | the third control was paid for by the title, not by its neighbours | FR-001 |
| `micold-client/tests/layout_text_overflow.rs` | "Worktrees" is not pushed past the sidebar's clip at the 180 px minimum | FR-010, research R10 |
| `micold-client/tests/layout_snapshot.rs` | the regenerated `tests/fixtures/layout_snapshot.txt` is the only layout that changed | FR-011 |
| `micold-client/tests/gates/refresh_busy_holds_the_header.rs` **(new)** | the busy form of the control moves nothing in the header, which is why it earns no second fixture block | FR-006 |
| `micold-client/tests/sidebar_state.rs` | a refreshed listing is reconciled by the shared `set_worktrees` path: an identical listing changes nothing, and a vanished worktree prunes only its own expansion | FR-004, FR-009 |
| `micold-client/tests/refresh_is_only_on_demand.rs` **(new)** | the refresh has exactly one starter in `src/`, so a timer or watcher cannot be added silently | FR-012 |

Two rows here were corrected during the T039 audit, and the corrections are worth recording because
both named a gate that reads a different subject:

- **`handshake.rs` was named for the version bump.** It reads `PROTOCOL_VERSION` and `SCHEMA_HASH`
  as symbols, so it passes at any value and cannot fail on a bump. The assertions that pin the
  number are `protocol_auth.rs` (`the_protocol_version_is_ten`) and `schema_hash.rs`.
- **`bar_controls_hold_their_size.rs` was named for FR-001.** It reads the `terminal.bottom_bar`
  anchor — the terminal's bottom bar, not the sidebar header — so no change to the sidebar could
  fail it. What actually answers the row is the regenerated fixture, and it answers it plainly:
  both existing header controls stay 22.0 × 26.2 with 14.0 × 18.2 glyphs, and the `Length::Fill`
  title goes 174.0 → 148.0. The slack came from the title, which is what a filling title is for.
  The case where the slack runs out is the 180 px one, and that is the `layout_text_overflow` row
  above rather than a size gate.

### A.3 The two that are cheap to forget

**The timeout.** This feature adds the codebase's first wire timeout (research R5). Its test must
prove the *bounded wait*, not the happy path: with no reply at all, the button returns to idle and
a notice is raised, and a reply that arrives afterwards is discarded without an error
(data-model T4).

```bash
cargo test -p micold-client --test features_worktree -- timed_out
```

**Single-flight is structural.** The reducer guard is the second line of defence; the first is that
`on_press` is attached only when `can_refresh_worktrees()` holds, so the message cannot exist
(research R7). Assert *both* — a test that only exercises the reducer would keep passing if the
view stopped guarding.

### A.4 Fast loop while iterating

```bash
mise run test-core                             # protocol + wire, no GUI, seconds not minutes
cargo test -p micold-client --test features_worktree --test app_state
```

---

## Part B — the manual pass

### B.0 Fixture

```bash
mkdir -p /tmp/wt-029 && cd /tmp/wt-029
git init -b main . && git commit --allow-empty -m "base"
```

Launch with `mise run run` and open `/tmp/wt-029` as a project.

### B.1 A worktree that appeared outside the app (US1, FR-002, FR-009)

1. Note the sidebar's worktree list.
2. In a terminal, *without touching the app*:
   ```bash
   git -C /tmp/wt-029 worktree add .claude/worktrees/outside -b feat/outside
   ```

   The path matters. `reconcile` (`micold-core/src/worktree.rs`) lists a worktree only when its
   parent directory *is* `<repo>/.claude/worktrees/`, plus any the user explicitly included. A
   worktree created at `/tmp/wt-029-outside` — which is what this step said until the 2026-09-03
   pass tried it — is one the app is designed never to list, so no number of refreshes can make it
   appear and the step proves nothing either way.
3. The sidebar still shows the old list. **This is the bug the feature fixes** — confirm it before
   pressing anything, or the next step proves nothing.
4. Press the refresh control in the sidebar header.

   **Expect**: `feat/outside` appears. A completion notice confirms the list was re-read.

### B.2 It is the same list, however it arrived (US3, FR-004)

1. Expand a worktree, select one, and open a row's context menu.
2. Press refresh.

   **Expect**: selection and expansion survive; the open menu behaves exactly as it does when a
   worktree is created from inside the app. Compare the two side by side — FR-004's claim is
   *indistinguishability*, so the check is a comparison, not an inspection.

### B.3 Nothing to refresh (FR-005)

Close the project (no project open).

**Expect**: no refresh control on screen — because there is no sidebar. `ui/mod.rs` renders the
navigation drawer only in the branch where `active_project().is_some()`, so the whole header, all
three controls included, is absent rather than greyed.

FR-005 asks for "unavailable (visibly non-actionable)", and absent satisfies it. But this step used
to say "the control is present but inert, with the two neighbours unchanged", which is a state the
shell cannot reach; the 2026-09-03 pass found it by looking for it. `can_refresh_worktrees()`'s
`workspace.active.is_some()` term is still doing work — it guards the *message* path in
`on_worktree_refresh_requested`, which a press is not the only way to enter — but it is not a term
any user can see the effect of.

The consequence for research R6 is larger than for this step: R6 accepted the inert-greyed
in-progress cue while noting it was "ambiguous with FR-005's 'no project' greying". There is no such
greying. With a project open the guard is false only while `refreshing`, so a dimmed refresh control
has exactly one meaning in the shipped UI.

### B.4 In progress (US2, FR-006, FR-007)

Refreshes are fast on a small repository. To see the busy state, either use a repository with many
worktrees or pause the daemon briefly (`kill -STOP`/`-CONT` on the `micold-daemon` pid — **never**
`pkill`, and never a process the user started for their own work).

**Expect**: while in flight the control is inert and its tooltip reads "Refreshing worktrees…";
pressing it again does nothing; it returns to idle when the reply arrives.

### B.5 Failure keeps the list (FR-008)

Stop the daemon while a project is open, then press refresh.

**Expect**: a notice saying the session service is unreachable; the previously shown worktrees are
**still on screen**, not cleared.

Then expect the list to change anyway, a second or two later, and do not read that as a refresh:
the client respawns the daemon and re-syncs, which is the reconnect path the notice itself names
("reconnecting will show the current state"). It is the project's initial sync, not a
`WorktreeRefresh`, so it is outside what FR-012 and `tests/refresh_is_only_on_demand.rs` constrain.
If you want the list to hold still, do not kill the daemon.

### B.6 The header still fits (FR-001, FR-010, research R10)

Drag the sidebar to its narrowest (180 px).

**Expect**: three controls and the "Worktrees" title coexist without the title being clipped
mid-word. If it is clipped, A.2's `layout_text_overflow` row already failed — fix it there, in the
order research R10 records (ellipsize the title first), not by moving the control.

### B.7 Nothing refreshes on its own (FR-012)

Leave the app open on `/tmp/wt-029` and create another worktree externally. Wait a minute without
touching the app.

**Expect**: the list does **not** change. On-demand means on demand.

---

## Cleanup

```bash
git -C /tmp/wt-029 worktree remove .claude/worktrees/outside --force
rm -rf /tmp/wt-029
```

# Quickstart: Validating Background Project Switching

Runnable validation scenarios that prove the feature works end-to-end. Behavior details are in [background-session-lifecycle.md](./contracts/background-session-lifecycle.md) (BS-*); UI details in [project-switcher-ui.md](./contracts/project-switcher-ui.md); state shape in [data-model.md](./data-model.md).

## Prerequisites

- Rust stable toolchain (via `mise`), as for the rest of the repo.
- Two local git repositories to open as Project A and Project B.
- `claude` CLI available on `PATH` (used by real sessions).

## Automated validation (preferred — headless)

Core logic (no GUI, no processes):

```bash
cargo test --no-default-features
```

Expected new/most-relevant tests to pass (see contracts for the assertions):

- `switch_active` keeps outgoing sessions `Running` and does not null their lifecycle (BS-1).
- `switch_active` restores the stored foreground session, else first running, else `None` (BS-3).
- `Workspace::find_session` / `find_session_mut` resolve a session in a **non-active** project (BS-6 lookup).
- background restart marks `restarted_while_inactive`, and switching in sets `notice` and clears the ids (BS-7).
- switching to an unavailable project returns `false` and leaves state unchanged (BS-10).
- `running_session_count(path)` matches the number of active sessions per project (FR-007).

GUI-gated behavior (retained terminals, background-crash restart, switcher rendering):

```bash
cargo test            # runs the gui-gated suite as well
```

Prefer these headless VT/logic tests over launching the GUI for verification.

## Manual end-to-end walkthrough

Build and run:

```bash
cargo run
```

1. **Start work in Project A** — open Project A (top-bar switcher → "Add project…", or the body list), create/select a worktree, and start a session. Confirm the terminal is live and producing output. *(FR-001 precondition.)*
2. **Switch to Project B while A runs** — open the switcher (control immediately left of the menu button), pick Project B (add it first if needed). Expected: B becomes active; **A's session is not killed** (BS-1). *(FR-004, FR-005.)*
3. **Confirm A keeps running in the background** — leave B active for a while. In the switcher, Project A shows a running-session count badge (FR-007); Project B (no sessions) shows none.
4. **Return to Project A** — select A in the switcher. Expected: the session that was in the foreground is shown again, **still running**, with the output it produced while you were away (BS-2, BS-3, SC-003). No restart occurred.
5. **Background crash + notify** — with A backgrounded, kill A's `claude` child process externally (e.g. `kill <pid>`). Expected: the poll loop auto-restarts it under the crash-loop guard (BS-6). On returning to A, a notice reports that a background session was restarted (BS-7, SC-007).
6. **Unavailable project** — move/rename Project B's folder on disk, then open the switcher. Expected: B shows an unavailable badge and cannot be selected; A's background sessions are unaffected (BS-10, FR-008).
7. **Complement check** — confirm the body "Known projects" list and the folder-browser modal still work as before (2026-07-17 clarification).

## Success signals (maps to spec Success Criteria)

- No session is stopped as a side effect of any switch above (SC-001, SC-006).
- Switching via the switcher is ≤ 2 interactions: open, select (SC-002).
- Output made while inactive is fully present on return within the scrollback cap (SC-003).
- Running projects are identifiable from the switcher without opening them (SC-004).
- The newly selected project displays within ~1 s (SC-005).
- The background restart/failure is surfaced on return, never silently (SC-007).

## Out of scope to validate here

- Survival of live processes across an app restart (explicitly out of scope — BS-11).
- Any cap/throttle on concurrent background projects (none exists — BS-5).

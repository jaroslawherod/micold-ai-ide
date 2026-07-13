# Quickstart & Validation: Application Shell with Help / About

How to build, test, and manually validate the shell end-to-end. This is a run/validation
guide — implementation details live in `tasks.md` and the code itself.

## Prerequisites

- Rust stable toolchain (managed via `mise`; see `mise.toml`). `mise install` to provision.
- No network access required — the app is fully offline (Principle IV).
- Applies identically on Linux, macOS, and Windows.

## Build & run

```bash
mise install            # provision the pinned Rust toolchain
cargo build             # compile the app + iced
cargo run               # launch the main window
```

Expected: a single main window opens with a toolbar containing a "Help" entry (and nothing
else).

## Automated tests (Principle I)

```bash
cargo test              # runs render-free core unit tests + integration tests
```

Expected: green suite covering —
- **metadata resolution** (`tests`/inline in `src/metadata.rs`): correct name/version, and
  empty license/description → `"unknown"` fallback (FR-016).
- **overlay transitions** (`tests/about_flow.rs`): `None → About` on `AboutOpened`;
  idempotent second `AboutOpened` stays `About` (FR-015); `About → None` on `AboutClosed`;
  `AboutClosed` while `None` is a no-op. See [data-model.md](./data-model.md) transitions.

These tests are written **failing first**, reviewed, then made to pass (Red-Green-Refactor).

## Manual validation walkthrough

Run `cargo run`, then verify each step. Full clause detail in
[contracts/ui-contract.md](./contracts/ui-contract.md).

| # | Action | Expected result | Contract |
|---|--------|-----------------|----------|
| 1 | Launch app | One window, top toolbar, only "Help" visible | C1, C2 |
| 2 | Select "Help" | An "About" action appears (only that) | C3 |
| 3 | Activate "About" | Modal About dialog opens; background non-interactive | C4 |
| 4 | Read dialog | Shows `Micold AI IDE`, version, license, one-line description | C5 |
| 5 | Confirm version | Matches `Cargo.toml` version (not hardcoded) | C5 / FR-007 |
| 6 | Activate "About" again (if reachable) | Still exactly one dialog | C4 / FR-015 |
| 7 | Click "Close" | Dialog closes; window unchanged; focus back in window | C6, C7 |
| 8 | Reopen, press `Esc` | Dialog closes; window unchanged | C6, C7 |
| 9 | Press `Esc` with no dialog | Nothing happens | C6 |

### Version-source spot check

```bash
grep '^version' Cargo.toml     # note the version
cargo run                      # open About → version in dialog must match
```

### Cross-platform parity (Principle VI / SC-005)

Steps 1–9 must produce identical results on Linux, macOS, and Windows. CI
(`.github/workflows/ci.yml`) runs `cargo build` + `cargo test` on all three; the manual
walkthrough is repeated per platform for any rendering/behavior differences before a feature
is considered "done".

## Documentation check (Principle VII)

- User guide page `docs/user-guide/help-about.md` exists and describes the Help → About flow.
- The docs check passes in CI.

## Definition of done for this feature

- [ ] `cargo test` green (unit + integration) on all three platforms
- [ ] Manual walkthrough steps 1–9 pass on Linux, macOS, and Windows
- [ ] `Cargo.toml` `license = "Apache-2.0"` + root `LICENSE` (Apache-2.0, owner-confirmed) present
- [ ] `docs/user-guide/help-about.md` shipped and docs check green

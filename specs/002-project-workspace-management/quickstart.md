# Quickstart & Validation: Project Selection and Workspace Management

How to build, test, and manually validate project selection & workspace management end-to-end.
This is a run/validation guide — implementation details live in `tasks.md` and the code itself.
Clause detail is in [contracts/ui-contract.md](./contracts/ui-contract.md) and
[contracts/storage-schema.md](./contracts/storage-schema.md).

## Prerequisites

- Rust stable toolchain (managed via `mise`; see `mise.toml`). Run `mise install` to provision.
- No network access required — the app is fully offline (Principle IV).
- Applies identically on Linux, macOS, and Windows (Principle VI).

## Build & run

```bash
mise install            # provision the pinned Rust toolchain
cargo build             # compile the app + iced (adds serde/serde_json/directories)
cargo run               # launch the main window
```

Expected on a machine that has never opened a project: the shell shows an **empty state**
inviting you to open a project (FR-016).

## Automated tests (Principle I)

```bash
cargo test --no-default-features --all-targets   # render-free core + integration, no GUI
cargo test                                        # (optional) also compiles GUI-gated code
```

Expected: a green suite covering —

- **Rename validation** (`tests/project.rs`): empty and whitespace-only names rejected; valid
  name accepted; default display name = folder name (FR-004, FR-020).
- **Workspace logic** (`tests/workspace.rs`): dedupe by path (no duplicate on re-open), single
  active-space replacement, last-active tracking, availability marking + reopen of unavailable is
  rejected (FR-012, FR-013, FR-010, FR-022, FR-023).
- **Selector navigation** (`tests/selector.rs`): enter/up navigation and git flags surfaced via a
  **fake** `FolderScanner`; unreadable dir → error status, no panic (FR-002, FR-006, edge case).
- **Store roundtrip** (`tests/store_roundtrip.rs`): save→load equality; missing/corrupt file →
  empty list; dangling `last_active` → none; atomic write — all against a `tempfile` dir, never
  the real user data directory (research R7/R8; storage-schema contract tests).

These tests are written **failing first**, reviewed, then made to pass (Red-Green-Refactor).

## Manual validation walkthrough

Run `cargo run`, then verify each step. Use two throwaway folders to prepare: one that is a git
repository (`git init` it beforehand) and one that is not.

| # | Action | Expected result | Contract |
|---|--------|-----------------|----------|
| 1 | Launch with no prior projects | Empty state invites opening a project | C1 / FR-016 |
| 2 | Open the project selector | In-app folder browser opens; lists folders only | C2, C3 |
| 3 | Browse to the two prepared folders | The git folder shows a **git icon**; the non-git folder does not | C3 / FR-006 |
| 4 | Navigate into a folder, then up | Enters/leaves folders; reaches roots at the top | C3 (research R5) |
| 5 | Choose the **non-git** folder | Project created (git not required); becomes active; shell shows its name (= folder name) | C3, C4 / FR-003, FR-004, FR-005 |
| 6 | Open the selector again; choose the **git** folder | New project active; replaces previous active; shell name updates | C4 / FR-013, FR-014 |
| 7 | Choose the **same** git folder again | No duplicate entry appears; existing entry activated | C4 / FR-012 |
| 8 | Rename a project to `My Project` | Display name updates everywhere; the folder on disk is unchanged | C6 / FR-017, FR-018 |
| 9 | Try to rename to `""` or `"   "` | Rejected; previous name kept; problem indicated | C6 / FR-020 |
| 10 | Quit and relaunch | Both projects reappear in the known list with stored names; last active is indicated | C5 / FR-008, FR-010, FR-019 |
| 11 | Reopen a known project from the list | Becomes active **without browsing the filesystem** | C5 / FR-011 |
| 12 | Delete/rename one project's folder on disk, relaunch | That project is marked **unavailable**; reopening it is blocked; app does not crash | C5 / FR-022, FR-023 |

### Persistence spot check

```bash
# Location resolved by `directories` (research R2). Example (Linux):
cat "${XDG_DATA_HOME:-$HOME/.local/share}"/*micold*/projects.json   # inspect the stored list
```

Expected: JSON matching [storage-schema.md](./contracts/storage-schema.md) — `schema_version`,
`last_active`, and a `projects` array with `path` / `display_name` / `is_git_repo`. Confirm a
renamed project's `display_name` here reflects the rename (FR-019) and that no folder on disk was
renamed (FR-018).

### Corruption resilience spot check

```bash
# With the app closed, overwrite the store with invalid JSON, then relaunch.
echo 'not json' > "$(printf '%s' "${XDG_DATA_HOME:-$HOME/.local/share}"/*micold*/projects.json)"
cargo run
```

Expected: the app launches to the empty state (or a recovered list) rather than crashing
(research R8; SC-009).

## Cross-platform parity (Principle VI / SC-010)

Steps 1–12 must produce identical results on Linux, macOS, and Windows (the only allowed
difference is the roots presentation — Windows drive letters vs `/`, step 4). CI
(`.github/workflows/ci.yml`) runs the logic-core tests + GUI build on all three platforms; repeat
the manual walkthrough per platform before the feature is considered "done".

## Documentation check (Principle VII)

- User guide page `docs/user-guide/project-selection.md` exists, is linked from
  `docs/README.md`, and describes opening/reopening/renaming projects and the unavailable state.
- The CI docs job asserts the page's presence.

## Definition of done for this feature

- [ ] `cargo test --no-default-features --all-targets` green (unit + integration) on all three platforms
- [ ] GUI build (`cargo build --features gui`) succeeds on all three platforms
- [ ] Manual walkthrough steps 1–12 pass on Linux, macOS, and Windows
- [ ] No filesystem mutation occurs at any step (rename affects only the stored display name)
- [ ] `docs/user-guide/project-selection.md` shipped, linked, and docs check green

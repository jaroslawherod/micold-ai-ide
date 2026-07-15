# Quickstart: Validating Material Design Layout & Theming

Runnable checks proving the feature works end-to-end. See `data-model.md` for types,
`contracts/` for the token values, resolution table, and settings schema.

## Prerequisites

- Rust stable toolchain (via `mise`), as provisioned by feature 001.
- Linux iced system deps already installed for GUI builds (see `.github/workflows/ci.yml`).

## 1. Logic core (no GUI) — the testable guarantees

```bash
cargo test --no-default-features --all-targets
```

Expected — all pass, including the new suites:

- `tests/theme.rs` — `resolve()` matches every row of the truth table in
  `contracts/theme-behavior.md` (Light/Dark overrides ignore the OS; FollowSystem tracks it;
  Unspecified → Light).
- `tests/tokens.rs` — for both `LIGHT` and `DARK`, every `on_*` role meets WCAG AA (≥ 4.5:1)
  contrast against its paired surface (SC-005).
- `tests/settings_roundtrip.rs` — a `Settings` round-trips through the store; a missing file and a
  corrupt file both yield `FollowSystem` with the right `LoadStatus` (FR-019); writes are atomic.
- All pre-existing 001/002 suites still pass unchanged (SC-006).

## 2. GUI build (per platform)

```bash
cargo build --features gui
```

Must compile on Linux, macOS, and Windows (CI enforces all three).

## 3. Manual walkthrough — layout (User Story 1, FR-001, FR-010…FR-016)

```bash
cargo run --features gui
```

Confirm:

- A Material top app bar shows the title and primary actions; the body is a structured region
  with card/surface containers and consistent spacing.
- Empty state (no project) is a Material surface with `display`/`body` typography and a **filled**
  primary "Open a project" button that still opens the selector.
- With an active project: the header surface shows name (`headline`) + path (`label`) and an
  action button; behavior identical to before.
- Known-projects list: each entry is a Material list item preserving the active marker, the "git"
  badge, the unavailable state, and Open/Rename (Open disabled when unavailable).
- Hover/press/focus/disabled states are visibly distinct on buttons (FR-014).
- Resize the window small: content reflows and stays usable without clipping (FR-016).
- About, project selector, and rename dialogs all share the design system (FR-013).

## 4. Manual walkthrough — system theming (User Story 2, FR-004…FR-006, SC-002/003)

1. Set the OS to **dark**, then `cargo run --features gui`. The app launches **dark** (no flash
   of light).
2. With the app running and preference on **Follow system**, switch the OS to **light**. Within
   ~1 second the app switches to light live — no restart (SC-003).
3. Switch the OS back to dark; the app follows again. Every screen is legible in both.

> Linux note: OS detection uses the XDG desktop portal; on a session without a portal/GTK/KDE
> settings, detection returns "unspecified" and the app shows light (FR-018) — expected.

## 5. Manual walkthrough — user override (User Story 3, FR-007…FR-009, SC-004)

1. With the OS in **light**, open the theme menu in the app bar and choose **Dark**. The app turns
   dark immediately and ignores the OS.
2. Quit and relaunch: the app is still **dark** (preference persisted to `settings.json`).
3. Open the theme menu and choose **Follow system**: the app returns to matching the OS and, on a
   subsequent OS theme change, updates live again.

## 6. Docs check (Principle VII)

```bash
test -f docs/user-guide/appearance-theming.md
```

The user-guide page documents the new look and how to choose/persist a theme, and is linked from
`docs/README.md`. CI asserts the file exists.

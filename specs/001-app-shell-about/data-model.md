# Phase 1 Data Model: Application Shell with Help / About

This feature has **no persistent data**. The "data model" is the in-memory application state
and the read-only application-identity value object. Types are described conceptually;
Rust field types are indicative, not prescriptive.

## Value object: `AppMetadata`

Read-only identity of the running application, shown in the About dialog. Populated once
from compile-time sources; never mutated at runtime.

| Field | Source | Rule |
|-------|--------|------|
| `name` | `const APP_NAME = "Micold AI IDE"` | Fixed literal; MUST equal "Micold AI IDE" (FR-006). |
| `version` | `env!("CARGO_PKG_VERSION")` | From `Cargo.toml`; never hardcoded (FR-007). Always non-empty for a Cargo build. |
| `license` | `env!("CARGO_PKG_LICENSE")` | From `Cargo.toml` `license`. If empty → display fallback (FR-008, FR-016). |
| `description` | `env!("CARGO_PKG_DESCRIPTION")` | From `Cargo.toml` `description`. One line. If empty → display fallback (FR-009, FR-016). |

**Validation / display rules**:
- A metadata string that is empty (`""`) is treated as "unavailable" and rendered as a
  clearly-labeled fallback token (`"unknown"`), never as a blank field (FR-016).
- `name` is a constant and is never subject to the fallback rule.
- Resolution is a pure function of the four source strings → unit-testable (Principle I).

## Application state: `State`

The root TEA state for the single main window.

| Field | Type (indicative) | Purpose |
|-------|-------------------|---------|
| `metadata` | `AppMetadata` | Cached identity for display. |
| `overlay` | `Overlay` | Which modal overlay (if any) is currently shown. |

### `Overlay` (state enum)

```
enum Overlay {
    None,      // no modal open; main window fully interactive
    About,     // About dialog shown as a modal overlay
}
```

- Modeling the overlay as an enum (not a `bool` per dialog) makes "About open twice"
  **unrepresentable** and leaves a single obvious extension point for future overlays —
  directly satisfies FR-015 at the type level (Principle V).

### State transitions

| From | Message | To | Notes |
|------|---------|----|-------|
| `Overlay::None` | `AboutOpened` | `Overlay::About` | Triggered by activating Help → About (FR-005). Focus moves into dialog (FR-014). |
| `Overlay::About` | `AboutOpened` | `Overlay::About` | Idempotent — no second instance (FR-015). |
| `Overlay::About` | `AboutClosed` | `Overlay::None` | Close button (FR-010) or Esc (FR-011). Focus returns to main window (FR-014). |
| `Overlay::None` | `AboutClosed` | `Overlay::None` | No-op — Esc with no dialog open has no effect (edge case). |

State is unchanged by any transition other than switching `overlay`; the rest of the window
(FR-012) is untouched.

## Message vocabulary (design-level)

| Message | Meaning |
|---------|---------|
| `HelpMenuToggled` | User selected the "Help" toolbar entry, revealing/collapsing its "About" action (FR-002, FR-004). |
| `AboutOpened` | User activated "About" (FR-005). |
| `AboutClosed` | User dismissed the About dialog via Close or Esc (FR-010, FR-011). |

> The Help menu's open/closed presentation is transient UI affordance state; if implemented
> as explicit state it lives alongside `overlay` but does not gate modality. Modality is
> governed solely by `Overlay`.

## Relationships & scope notes

- `State` owns one `AppMetadata` and one `Overlay`. No collections, no persistence, no I/O.
- **No session state** is introduced (Principle II is not applicable to this feature). When
  sessions arrive later, session-scoped state will compose into `State` without changing the
  `Overlay` pattern established here.
- **No filesystem or VCS state** (Principles III/IV): nothing is read or written at runtime.

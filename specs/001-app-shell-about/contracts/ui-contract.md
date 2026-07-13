# UI Contract: Application Shell with Help / About

This is a desktop application; its external contract is the **user-facing interaction
surface**, not a network API. This document is the authoritative description of what the
shell exposes and how it must behave. Each clause traces to a functional requirement (FR)
in [spec.md](../spec.md).

## C1. Main window

- On launch, exactly **one** main window opens (FR-001).
- The window has a **toolbar across the top** (FR-001).
- No window state persistence, no additional windows (out of scope per spec Assumptions).

## C2. Toolbar

| Element | Behavior | Traces to |
|---------|----------|-----------|
| "Help" entry | The **only** toolbar entry; always visible. | FR-002, FR-003 |
| (any other entry) | MUST NOT exist. | FR-003 |

## C3. Help menu

- Selecting "Help" reveals an **"About"** action (FR-004).
- "About" is the **only** action under Help (FR-003).

## C4. About activation

- Activating "About" opens the About dialog as a **modal overlay within the main window**
  (FR-005, FR-013).
- While the overlay is open, the toolbar and main content are **non-interactive**
  (backdrop blocks input) (FR-013).
- Activating "About" again while already open does **not** open a second dialog (FR-015).

## C5. About dialog contents

The dialog MUST display all four fields, all visible on first open without scrolling
(SC-002):

| Field | Value | Traces to |
|-------|-------|-----------|
| Application name | Exactly `Micold AI IDE` | FR-006 |
| Version | From build/package metadata (not hardcoded) | FR-007 |
| License | Project's OSI-approved license name | FR-008 |
| Description | One-line app description | FR-009 |

- Any field whose metadata source is empty displays a clearly-labeled fallback
  (e.g., `unknown`), never a blank (FR-016).

## C6. Dismissal

| Trigger | Result | Traces to |
|---------|--------|-----------|
| Click "Close" button | Dialog closes; return to main window | FR-010, FR-012 |
| Press `Esc` (dialog open) | Dialog closes; return to main window | FR-011, FR-012 |
| Press `Esc` (dialog closed) | No effect on the About feature | Edge case |
| Click backdrop | Dialog remains open (dismissal only via Close or Esc) | FR-013 |

- After dismissal, the main window and toolbar are in the **same state as before the dialog
  opened** (FR-012).

## C7. Focus

- On open: keyboard focus moves **into the dialog** (lands on the Close button) (FR-014).
- On close: keyboard focus returns to the **main window** (FR-014).

## C8. Cross-platform parity

- Every clause above behaves **identically on Linux, macOS, and Windows** (FR-017, SC-005).
- No clause may depend on OS-specific behavior; platform differences (if any) are confined
  behind abstractions and MUST NOT change the observable contract.

## Contract test checklist

These map directly to acceptance scenarios and are the behaviors the render-free core tests
(`update` transitions) and the manual `quickstart.md` walkthrough must cover:

- [ ] Launch → single window + toolbar with only "Help" (C1, C2)
- [ ] Help → reveals only "About" (C3)
- [ ] About → modal overlay opens; background non-interactive (C4)
- [ ] Second About activation → still one dialog (C4 / FR-015)
- [ ] Dialog shows name, version, license, description (C5)
- [ ] Empty license/description → fallback shown (C5 / FR-016)
- [ ] Close button → returns to unchanged window (C6, C7)
- [ ] Esc (open) → returns to unchanged window (C6, C7)
- [ ] Esc (closed) → no effect (C6)
- [ ] All of the above verified on Linux, macOS, Windows (C8)

# Contract: the showcase's launch surface

**Feature**: [020-component-showcase-gallery](../spec.md) | Covers FR-017, FR-018, FR-018a, FR-020,
SC-001, SC-008

The showcase's only external interface is the command that starts it. It takes nothing and touches
nothing, and that is the whole of the contract — the interesting claims are all negative.

---

## §1 The command

```
mise run showcase            # cargo run -p micold-client --bin micold-showcase
```

- **Arguments**: none. Accepts none, requires none, and has no configuration (spec, Out of Scope:
  "no editing, theming, or configuration capability beyond switching the colour scheme").
- **Environment**: reads none. Not `HOME`, not `XDG_*`, not the OS theme preference.
- **Standard input/output**: nothing is read from stdin; nothing is written but whatever iced and the
  renderer emit.
- **Exit**: closing the window exits 0.

`mise run run` (the application) is unchanged and stays the default binary — see
[research R1a](../research.md#r1a--default-run-is-not-optional).

## §2 What launching it must not do (FR-017, FR-020)

| Must not | Why it matters |
|---|---|
| Spawn or connect to the session daemon | US1's independent test inspects the process list and expects none (FR-020) |
| Read or write the project store, the settings file, or any state directory | FR-020 — the showcase must run with no saved application state and must not create any |
| Require or touch a git repository | FR-020 — US1 launches it on a machine with no repository present |
| Create a terminal session or a PTY | US1, acceptance scenario 2 |
| Start the application, or be started by it | FR-017 — one binary launching the other would make neither isolated |

Structurally: the showcase's `main` calls `iced::application` with the showcase's own
`update`/`view` and nothing else. It never names `micold_core::store`, `micold_core::settings`,
`micold_core::endpoint`, `micold_core::spawn`, `micold_core::git`, `micold_client::daemon`, or
`dark_light`. That absence is what makes the negative claims above true; it is worth reading the
import list of `src/showcase/main.rs` as the statement of this section.

## §3 What it must do (FR-002, FR-010)

- Register the Material Symbols font, so `Glyph` and every component that draws an icon render the
  real glyph rather than a fallback box.
- Hand the window `micold_client::ui::theme(scheme)` — the application's own theme function, the one
  part of the styling layer that reaches beyond the library.
- Resolve every colour through `micold_core::tokens::roles(scheme)`.

A component must resolve the same colours here as it does in the application, in the same scheme
(FR-010, SC-006). The showcase gets that by using the same two functions the application does, not
by copying their results.

## §4 Not installed (FR-018, FR-018a, SC-008)

The showcase reaches no end user through a normal installation:

- `crates/micold-client/Cargo.toml`'s `[package.metadata.deb] assets` list names
  `target/release/micold-ai-ide` and `target/release/micold-daemon`, and **must never** name the
  showcase. cargo-deb ships only the listed assets when `assets` is present.
- `packaging/micold-ai-ide.desktop` names one `Exec`, and **must never** name the showcase.
- No second `.desktop` entry is added, and no launcher entry is created.

Enforced by `crates/micold-client/tests/packaging_excludes_showcase.rs`, which reads both files as
text and fails when either names the showcase binary or its path — and which also asserts both files
exist and the `assets` list is present and non-empty, so a relocated manifest or an emptied list
fails rather than passing over nothing. See
[research R11](../research.md#r11--packaging-exclusion-as-a-gate-fr-018a-sc-008).

## §5 What it costs at rest (FR-023, SC-009)

With the window open, every replay and run control stopped, and any section on screen: zero frames
requested, no measurable CPU. The showcase holds no timer and no subscription that ticks; the only
code in the crate that may ask the runtime for a frame is `cdk::motion::Progress`, behind its
`animating()` guard, and the widened idle-frames gate holds that to exactly one call site across
both `src/ui/` and `src/showcase/`.

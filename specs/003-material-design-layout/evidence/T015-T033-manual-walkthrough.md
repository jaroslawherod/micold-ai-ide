# 003 T015 / T033 — the quickstart walkthrough, run for the first time

**Date**: 2026-08-21
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Build**: this branch's own `micold-ai-ide` + `micold-daemon`, built in one invocation and copied
out of the shared target directory **inside** the build lock (`~/vp83/bin`, 2026-08-20 21:03). The
newest commit touching `crates/` is `d28a0c6` (2026-08-19), so the pinned pair is this branch.
**Isolation**: `XDG_RUNTIME_DIR=/tmp/vp83`, scratch `XDG_DATA_HOME`/`XDG_CONFIG_HOME`. Everything
started here was stopped by PID afterwards.
**Fixture**: a catalog with two entries — `w2-git`, a real repo at a real (non-symlinked) path, and
"Gone Away", pointing at a path that does not exist, so the unavailable row renders too.

The quickstart's commands are stale in form but not in substance: §1's
`cargo test --no-default-features --all-targets` and §2's `cargo build --features gui` predate the
workspace split and the `mise` tasks. They were run as `mise run test-core` and as the pinned build
above.

## §1 Logic core — **PASS**

`mise run test-core`: **604 tests, 61 suites, 0 failures.** The four suites the section names all
pass, under their current names:

| Quickstart names | Now | Result |
|---|---|---|
| `tests/theme.rs` | `tests/theme.rs` | 9 passed |
| `tests/tokens.rs` | `tests/tokens.rs` (+ `tokens_contrast`, `tokens_anatomy`, `tokens_density`, `tokens_scales`, `tokens_move`) | 9 + 9 passed |
| `tests/settings_roundtrip.rs` | `tests/settings_roundtrip.rs` | 8 passed |
| OS detection | `tests/os_theme.rs` | 7 passed |

SC-006 ("all pre-existing tests continue to pass") holds for the whole core.

## §2 GUI build — **PASS on Linux only**

The binaries under test were built from this checkout, so Linux compiles. macOS and Windows are not
reachable from here; CI is what enforces them.

## §3 Layout walkthrough — **PASS except FR-016**

| Claim | Result |
|---|---|
| Material top app bar: title + primary actions | **PASS** — title, project chip, `⋮` overflow |
| Empty state is a Material surface, `display` + `body` type, **filled** primary button | **PASS** — "No project open" / "Open a folder to set it as your working space." / filled **Open a project** (`s3-empty-state.png`) |
| Active-project header: name (`headline`) + path (`label`) + action | **PASS** — "Active project: w2-git", the path in label type, "Open another project" |
| Known-projects list preserves marker, `git` badge, unavailable state, Open/Rename | **PASS** — Open is replaced by a disabled **Unavailable** on the unavailable row; Rename and Forget stay enabled |
| Hover / press / focus / disabled visibly distinct (FR-014) | **PASS, with one documented exception** — see below |
| Resize small: reflows, usable, no clipping (FR-016) | **FAIL** — [BUG-002](../bugs/BUG-002.md) |
| About, selector and rename dialogs share the design system (FR-013) | **PASS** |

### FR-014 — measured, not eyeballed

The filled **Open** button's container fill, sampled at the same pixel in four states
(`s3-button-states.png` stacks all four at 250%):

| State | Fill |
|---|---|
| normal | `srgb(207,188,255)` |
| hover | `srgb(195,175,244)` |
| pressed | `srgb(192,172,241)` |
| disabled | clearly muted, with muted label |

Hover→pressed is only ~3/255 because M3 specifies 0.08 vs 0.10 state-layer opacity
(`crates/micold-core/src/tokens/state.rs`); the difference is correct, not weak.

**Focus** is not reachable on buttons at all, and that is recorded design, not a defect: the same
file states focus is *"reachable only on text fields and the select control. Buttons, rows, menu
items and chips cannot hold focus in this rendering stack — accepted fidelity gap #2 (FR-043)."*
So the focus half of FR-014 was checked where it exists — the Settings dialog's text fields
(`s3-field-focus.png`, focused **Scrollback lines** above unfocused **Timeout**, identical geometry):

| | focused | unfocused |
|---|---|---|
| container fill | `srgb(71,69,73)` | `srgb(54,52,56)` |
| label | primary (purple) | on-surface-variant (grey) |
| active indicator | thick, primary | hairline, grey |
| caret | present | absent |

Four simultaneous signals — comfortably "visibly distinct".

### FR-016 — the failure

Narrowing the window makes each Known-projects row **lose content** rather than reflow: the project
name wraps and is vertically clipped (~870 px), then disappears entirely (~800 px); Forget loses its
label and then its icon, ending as an empty pill (~760 px); Rename disappears and the remaining pill
overflows its own card (~620 px). Ladder in `s3-reflow.png`, cause and two candidate fixes in
[BUG-002](../bugs/BUG-002.md).

## §4 System theming — **PARTIAL**

| Step | Result |
|---|---|
| 1 — launches matching the OS, no flash of light | **PASS** — the OS here is dark; every launch in this run came up dark, and no captured first frame was light |
| 2 — OS light→dark switch is followed live within ~1 s (SC-003) | **NOT RUN** — this would mean changing the user's own desktop theme, which is not mine to change. Covered by `tests/theme.rs` and `tests/os_theme.rs` at the logic level; the live-poll path is what BUG-001 patched, with FR-021's regression test |
| 3 — legible in both | **PASS** — both schemes were rendered in full during §5; text is legible on every surface in each (`s5-override-persists.png`) |

**The Linux note's fallback was tested instead, and holds.** The quickstart says a session with no
portal detects "unspecified" and shows light (FR-018). Relaunching the same binary with
`DBUS_SESSION_BUS_ADDRESS` unset — no portal reachable — and the preference on `follow_system`
renders **light**: background `srgb(253,248,253)` (`s4-no-portal-light.png`), against
`srgb(20,19,22)` for the identical build with the portal reachable. That also explains a puzzle
worth recording: under Xvfb the app comes up **dark**, not light, because the launch environment
inherits the real session bus and the actual XDG portal answers with the user's real preference.

## §5 User override — **PASS** (adapted to the shipped control)

The quickstart describes "the theme menu in the app bar". What ships is a single **cycling** item in
the `⋮` overflow, labelled with the current setting: `Theme: Auto` → `Theme: Light` → `Theme: Dark`
→ `Theme: Auto`, each with its own icon (`s5-theme-cycle.png`). The menu stays open across the
cycle, so the change is visible as it happens.

| Step | Result |
|---|---|
| 1 — choose the override opposite the OS; app changes immediately and ignores the OS | **PASS** — the OS is dark, so **Light** is the opposite. The whole window turned light on the press, with the menu still open, and `settings.json` gained `"theme": "light"` |
| 2 — quit and relaunch: still the override | **PASS** — the relaunched app is light while the portal still reports dark (`s5-override-persists.png`, red = relaunched with the override, blue = the same app back on Auto) |
| 3 — back to Follow system | **PASS** — two more presses walked Light → Dark → Auto; `settings.json` recorded `"dark"` then `"follow_system"`, and on Auto the app is dark again, matching the OS |

The stored values are the schema's (`follow_system`, `light`, `dark`), not the menu's labels.

## §6 Docs check — **PASS**

`docs/user-guide/appearance-theming.md` exists and is linked from `docs/README.md:13`.

## Harness artifacts (not app defects)

- **An empty-state launch showed a populated Known-projects list.** Pointing a *new* client at a
  *fresh* `XDG_DATA_HOME` while the daemon from the previous launch was still running produced "No
  project open" above two remembered projects: the client read no active project from its own
  (empty) store, while the daemon pushed its catalog. In real use the client spawns the daemon and
  they share the environment, so the two can never disagree. The frame is still valid evidence for
  the empty-state *surface*, which is what §3 asks about.
- **`xdotool click 1` is too fast for some controls** — press, dwell ~200 ms, release. Same family
  as the artifacts recorded in 014's evidence.

## What was not covered

- macOS and Windows: §2's other two platforms, and any of the walkthroughs.
- **A live OS theme change** (§4 step 2, SC-003) — see above.
- SC-005's contrast claim was not re-measured by eye; `tests/tokens_contrast.rs` measures it
  numerically for every `on_*` role in both schemes, which is stronger than a screenshot.

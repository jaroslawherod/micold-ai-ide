# 022 §B — the gallery half of the manual pass

**Date**: 2026-08-19
**Run by**: an agent, not a person at a display — Xvfb `:85` at 1600×1400, rendered by Mesa's
lavapipe (software Vulkan), driven with `xdotool`, captured with `import`. Per the repo's
`visual-pass` skill.
**Build**: this branch's own `micold-showcase`, copied out of the shared target directory **inside**
the build lock and launched from that private copy.
**Isolation**: its own `XDG_RUNTIME_DIR` (`/tmp/vp85`) and `XDG_DATA_HOME`; the display was
confirmed to hold no other window before the first capture, and everything started here was stopped
by PID afterwards.

**Scope**: the gallery. §B5's dialog placement and the whole of §B6 need the client and the daemon
and are **not** covered — see *What is still unrun*. T039's platform question is answered
separately in [T039-three-os-matrix.md](./T039-three-os-matrix.md).

## Why most of this is numbers rather than descriptions

The quickstart asks whether two lists look the same. "They look the same" is exactly the claim a
reader cannot check later, so wherever a property is a colour or a distance it is **sampled off the
frame** and quoted. A reader who doubts any line below can re-run the same crop.

## §B1 — the two lists, side by side (SC-001) — **PASS**, both schemes

`b1-lists-light.png` and `b1-lists-dark.png` stack the select's list (red border) over the
type-ahead's (blue), cropped at identical geometry from each panel's own top edge.

| # | Property | Select | Type-ahead |
|---|---|---|---|
| 1 | surface tone | `#F2ECF1` light, `#201F22` dark | identical |
| 2 | elevation / edge | left-edge profile `248 … 246 242 236 230` | **byte-identical at the same offsets** |
| 3 | corner radius | panel left edge at x=22 | x=22 |
| 4 | list padding | panel top → first row baseline 25dp; 12dp of vertical padding around the rows | 25dp; 12dp |
| 5 | row height | pitch 48dp | 48dp |
| 6 | row padding | first ink at x=63 | x=62 — the one-pixel difference is the glyph's own left bearing (`F` vs `f`), not a padding difference |
| 7 | hover | `#E1DBE0` over `#F2ECF1` | `#E1DBE0` over `#F2ECF1` |
| 8 | selection marker + reserved space | check glyph at x=41; unmarked rows keep the marked row's text column | x=41; same |

Selected-row fills, measured with the pointer parked off every row so no hover could leak in:

| State | Select | Type-ahead |
|---|---|---|
| selected, not highlighted | `#D4CAE4`¹ | `#E8DEF8` |
| selected **and** highlighted | `#D4CAE4` | `#D4CAE4` |
| dark, selected + highlighted | `#5A5368` | `#5A5368` |

¹ **The one difference the pass found, and it is not a defect.** On opening, the select highlights
its current choice while the type-ahead highlights nothing — so the same row reads `#D4CAE4` in one
and `#E8DEF8` in the other, which looks like property 7 differing. It is not: one press of `Down` in
the type-ahead takes its row to `#D4CAE4`, exactly the select's value. The *treatments* coincide;
what differs is where the highlight starts, and that difference is right — a select opens on the
choice it already holds, a search field opens with nothing chosen. Recorded because the frames
genuinely differ and the next reader will see it too.

## §B2 — the transition (SC-002, SC-003) — **PARTIAL**

**"Nothing outside the list moves, at any point" — PASS, and measured rather than watched.** Two
frames with the pointer in the same place, differing only in whether the list is open: **0**
differing pixels above the list. The first attempt at this reported 27,256 differing pixels and was
wrong — the pointer had been left hovering something in the compared region, so the diff was reading
a hover state layer, not the list. The controlled pair is the one quoted.

The list drawn at the bottom edge flips above its trigger; every row outside it is byte-identical to
the closed frame, which is the same claim from the other direction.

**Not covered**: whether the list *grows* while fading in and settles rather than snapping; whether
a reversal mid-flight resumes from where it is; whether a press where a row used to be during the
fade-out chooses nothing. A screenshot pipeline cannot reliably catch a chosen frame of a 150 ms
transition — the `visual-pass` skill says so and this pass did not beat it. **T011 and T028 stay
open on exactly this.**

## §B3 — the select's own feedback (SC-005) — **PASS**

Measured at the trigger's bottom edge, nothing on the page supplying the fact:

| | Rows of indicator | Colour |
|---|---|---|
| closed | 1 | `#E6E1E6` (the muted hairline) |
| open | 2 | `#6750A4` — Material 3 light `primary` |

The container's own fill also darkens from `225` to `205` while open, which is the state layer §7.7
asks for. This is accepted fidelity gap #3 closed rather than reworded.

## §B4 — keyboard only (SC-004) — **PASS on four keys, two clauses unrun**

`Down`, `Down`, `Up`, `Enter`, `Escape`, driven with the pointer parked off the list, sampled at
every step. The two controls are **byte-identical at every step** in the dark scheme:

| Step | Select rows | Type-ahead rows |
|---|---|---|
| opened | `5A5368` `201F22` `201F22` | `5A5368` `201F22` `201F22` `201F22` |
| Down | `4A4458` `343236` `201F22` | `4A4458` `343236` `201F22` `201F22` |
| Down | `4A4458` `201F22` `343236` | `4A4458` `201F22` `343236` `201F22` |
| Up | `4A4458` `343236` `201F22` | `4A4458` `343236` `201F22` `201F22` |
| Enter | list gone, trigger shows the taken row | list gone |
| Escape (after reopening) | list gone, nothing changed | list gone, nothing changed |

That is SC-004's "all five keys mean the same thing in both controls" as an equality rather than an
impression.

**Not covered**: reaching a picker by `Tab` and tabbing out of it — the gallery is a long scrolling
page and posing a tab order through it is not the same exercise the task describes. And "taking a
row with `Enter` must not also submit the dialog behind it" has no dialog here; it belongs to the
client half.

## §B5 — placement (SC-006) — **two of four**

| # | Placement | |
|---|---|---|
| 1 | inside the add-worktree dialog | **not run** — needs the client |
| 2 | at the bottom edge of the window | **PASS** — `b5-flips-above-at-the-bottom-edge.png`: the field sits at y≈1362 of a 1400-tall window and its list is drawn *above* it, complete and unclipped |
| 3 | at the right edge | **not posed** — every gallery control is full width, so there is no right-edge trigger to open |
| 4 | on a full-height page | **PASS** — this is the gallery, and both lists opened correctly at every scroll position used above |

## §B6 — the application still works (SC-009) — **not run**

Needs the client and the daemon: open the add-worktree dialog, pick a type, create a worktree, then
repeat choosing nothing and confirm the form still validates. Nothing in the gallery can answer it.

## §B7 — both schemes — **PASS for what was covered**

§B1 was measured in full in both. §B3, §B4 and §B5's flip were measured in dark, having been
established in light. The scheme toggle needed a deliberate press-and-release rather than a click;
a plain `xdotool click` left it on hover without activating.

## What is still unrun, and what it blocks

| Task | State |
|---|---|
| T011 — §B2 against the type-ahead, *including whether an interrupted transition resumes* | **open** — the interruption is beyond a screenshot pipeline |
| T025 — §B1, §B3–§B6 in both schemes | **open** — B1, B3, B4 and half of B5 are done here; B5's dialog and B6 need the client |
| T028 — §B2 against both pickers, *including interruption and press-during-exit* | **open** — same limit as T011 |
| T039 — the three-platform matrix | **done**, recorded separately |
| T040 — the whole quickstart end to end | **open** — B6 is the missing piece |

**Perceived smoothness** is also outside this pass by construction: lavapipe is a software
rasteriser, so frame pacing here says nothing about frame pacing on a real GPU.

# 022 §B2 — the one clause of the transition a machine *can* reach

**Date**: 2026-08-20
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, rendered by Mesa's
lavapipe (software Vulkan), driven with `xdotool`, captured with `import`. Per the repo's
`visual-pass` skill.
**Build**: this branch's own `micold-showcase`, copied out of the shared target directory **inside**
the build lock and launched from that private copy (`~/vp83/bin`).
**Isolation**: its own `XDG_RUNTIME_DIR` (`/tmp/vp83`) and `XDG_DATA_HOME`; everything started here
was stopped by PID afterwards.

## Why this clause and not the rest of §B2

§B2 has five clauses. Four of them are about what the transition *looks* like — it grows rather than
snaps, it leaves faster than it arrives, a reversal resumes from where it is, nothing else on the
page moves. A screenshot pipeline cannot catch a chosen frame of a 100 ms track, and lavapipe's
frame pacing says nothing about a real GPU's, so those stay where [B-gallery-pass.md](./B-gallery-pass.md)
left them.

One clause is different:

> Press where a row used to be while the list is fading out: **nothing is chosen.**

That is a *state* question, not an appearance question. The answer is a value, and a value can be
photographed after the fact. It is also the clause with a real defect behind it if it fails — a list
that is invisible but still live steals a press the user aimed at whatever is underneath.

It is deterministic by construction, which is what makes it testable at all: `cdk/picker.rs` sets
`leaving = !self.open` and hands that to the overlay, so inertness does not depend on how far the
fade has progressed. Any press after the close begins should be refused.

## Method — every trial carries its own control

The failure mode of a test like this is a coordinate that never hit a row at all, which looks
exactly like a refusal. So each trial begins by **legitimately picking a known row with the same
coordinates it will later press**. If the control pick lands and the trial press does not, the
refusal is the widget's and not the harness's.

| Step | Select | Type-ahead |
|---|---|---|
| control | open, press row 3 → value becomes `Always dark` | open, press row 4 → marker moves to `main` |
| trial | reopen, close, wait 50 ms, press row 1's position | reopen, `Escape`, wait 50 ms, press row 1's position |
| read | the trigger's value | reopen and read which row carries the marker |

The type-ahead needs the marker rather than the field because the showcase sample carries its
selection in `.selected(…)`, not in the search box — the box rests empty either way, so field text
would have reported "refused" no matter what happened.

## Results

**Select — PASS, 5/5.** `b2-select-press-during-exit.png`. Five consecutive trials, all with the
control pick landing first: the value stays `Always dark` every time. The press 50 ms into the exit
chooses nothing.

**Type-ahead — PASS, 8/9**, and the ninth is unexplained. `b2-typeahead-press-during-exit.png` is
four consecutive refusals at 50 ms; a delay sweep at 50/150/300/600 ms refused at every step.

`b2-typeahead-settled-control.png` is the control that makes those refusals mean something: the
*identical* press, sent 2 s after `Escape` when the list is entirely gone, also leaves the marker on
`main` — so a refusal is not simply "the click landed on empty page". Taken with the control picks
that do land, the coordinate is live when the list is live and inert when it is leaving.

The ninth trial is the honest part. An earlier run of the same 50 ms trial moved the marker to the
pressed row — the press *was* taken. It has not reproduced in nine attempts since, under both a
loaded and a quieter machine. Two things are worth writing down rather than explaining away:

- Only one trial in that early run was informative. Trials 2 and 3 reused a marker already sitting
  on the pressed row, so they could not have shown a change. The run's "3/3" was really 1/1.
- The two controls differ in where openness lives. The select owns its own (`catalogue.rs`: "the
  openness is the widget's"); the type-ahead's comes from the reducer via `.open(…)`, so `Escape`
  has to complete a message round trip before a view is built with `open = false`. Under a software
  rasteriser a slow frame widens the window in which the old view — still `open` — is the one
  holding the pointer. That is a plausible account of a one-off accept and it is *not* evidence of a
  defect on a GPU that redraws in 16 ms.

So: the clause holds on both controls as observed here, and the single contrary observation is
recorded rather than averaged away. It does not close §B2, which stays open on the four appearance
clauses above.

## What this run did **not** answer

- the grow-and-fade itself, the relative in/out durations, and whether a reversal resumes or snaps
- whether anything outside the list moves during the transition (FR-023 holds by construction —
  `scale` and `fade` transform drawing only — but that is an argument, not an observation)
- perceived smoothness at any frame rate a user would actually see

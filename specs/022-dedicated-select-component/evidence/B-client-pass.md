# 022 §B — the client half: §B5's dialog placement and §B6

**Date**: 2026-08-19
**Run by**: an agent, not a person at a display — Xvfb `:86` at 1600×1400, Mesa lavapipe, driven
with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Build**: `micold-ai-ide` **and** `micold-daemon` from one `cargo build` naming both binaries,
copied out of the shared target directory inside the build lock and launched from that private copy.
The pair was confirmed to connect before anything was measured — `attach: connected projects=1
sessions=0` in the client log, not `refusing client: contract or build mismatch`.
**Isolation**: `XDG_RUNTIME_DIR=/tmp/vp86`, `XDG_DATA_HOME` in the scratchpad, and a throwaway git
repository at `/tmp/vp86proj` created for this pass and deleted after it. The user's own daemon on
`/run/user/1000` was never contacted; everything started here was stopped by PID.

The companion to [B-gallery-pass.md](./B-gallery-pass.md), which covers §B1–§B4 and the placements
the gallery can pose. This is the half that needs a real project and a real daemon.

## How the project was opened

By **seeding the catalog** — `<XDG_DATA_HOME>/micold-ai-ide/projects.json` with one entry — rather
than by driving the native folder picker, which needs a portal Xvfb has not got. The client picked
it up on the next launch and the log went `projects=0` → `projects=1`.

Worth recording as a technique: it is the difference between this half being runnable headlessly and
not.

## §B5 placement 1 — inside the add-worktree dialog (SC-006) — **PASS**, both schemes

`b5-dialog-placement-dark.png`, `b5-dialog-placement-light.png`.

The quickstart calls this "the placement that defeated the first hand-rolled attempt at this in
feature 013", and it is the one a content-sized box makes hard: the card is sized by its contents,
so a list that had to fit *inside* it would be squeezed, and one that ignored the card would be
clipped by it.

Neither happens. The list is anchored to its trigger and drawn **over** the dialog, extending past
the card's bottom edge onto the page beneath, complete and unclipped. It scrolls — the eight
conventional types do not all fit, and the panel carries its own scrollbar rather than growing. The
card's own width is unchanged: the list matches the trigger, and the dialog behind it is the same
size open or closed.

## §B5 placement 3 — at the right edge — **not posed**

Neither surface has a trigger near the right edge: the gallery's controls are full width and the
dialog is centred. There is nothing to open there, so this is *unposable* rather than unrun. Said
plainly so it is not read as an omission that a longer session would have covered.

## §B6 — the application still works (SC-009) — **PASS**, both schemes

Dark: type `feat`, ticket `ABC-123`, name `login page` → created. Light: type `fix`, no ticket, name
`crash on open` → created. On disk afterwards:

```
.claude/worktrees/feat-abc-123_login-page   branch feat/abc-123_login-page
.claude/worktrees/fix-crash-on-open         branch fix/crash-on-open
```

and the sidebar shows **Login page** with `feat` + `ABC-123` chips, and **Crash on open** with a
`fix` chip (`b6-created-rows.png`). Same options, same result.

**Validation, with nothing chosen** (`b6-validates-with-nothing-chosen.png`): `Create` renders
disabled, and pressing it changes **0 pixels** — measured, not eyeballed, in both schemes. That is
the form validating exactly as it did, expressed the way the library expresses unavailability
everywhere: by having nowhere to send the value.

### A bonus this half settled, which it was not looking for

`b6-derived-names.png` is the derived preview in the real client, and it carries 016 BUG-003's fix
end to end:

```
Directory: .claude/worktrees/feat-abc-123_login-page
Branch:    feat/abc-123_login-page
```

The `_` boundary on **both**, and the sidebar row that came out of it reads "Login page" with an
`ABC-123` chip rather than "Feat abc 123 login page" with none. The ticketless case is there too —
`fix/crash-on-open` has no `_` anywhere and no issue chip. Those were unit-tested and screenshotted
in the gallery; this is the first time they have been seen surviving an actual `git worktree add`.

## What went wrong on the way, recorded because it touched something it should not have

Running `micold-ai-ide --help` to look for a path argument **launched the GUI on the user's own
display**: the binary parses no arguments, and the invocation inherited `DISPLAY=:0` and
`XDG_RUNTIME_DIR=/run/user/1000`, so a second client attached to the user's live daemon. It was
identified by `/proc/<pid>/environ` and killed within a minute, and the user's own instance was left
alone.

The rule the skill states for *launching* — always `env -u WAYLAND_DISPLAY DISPLAY=:NN` — applies to
running the binary **at all**, including to ask it a question. There is no read-only invocation of
this binary.

## What this pass still cannot answer

Unchanged from the gallery half: mid-flight animation and perceived smoothness. §B2's interruption
and press-during-exit clauses stay open, and T011, T028 and T040 stay open with them.

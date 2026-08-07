# Screenshots for the manual visual passes

`quickstart.md`'s Part B asks a person to look at the running application. Several of its items are
about *where something sits* — a chip's label on the pill's centre line, the app bar's ⋮ on the
trailing edge — and those are far easier to settle from a captured frame than from memory, because a
frame can be measured.

    mise run screenshot out.png

Writes one PNG of the current desktop. `--monitor DP-3` picks a connector (default: the first one
Mutter reports); `--timeout N` bounds the wait for a frame.

## Why it works this way

Nothing else on a stock GNOME/Wayland session can take a screenshot from a shell:

| Route | Result |
|---|---|
| `grim` | Wayland-native, but it needs `wlr-screencopy`, which Mutter does not implement |
| `import`, `xwd`, `maim` | X11 only; the session is Wayland and they capture an empty root window |
| `org.gnome.Shell.Screenshot` | `AccessDenied` — restricted to GNOME Shell itself |
| `org.freedesktop.portal.Screenshot` | Works, but opens a consent dialog, so it cannot be scripted |
| RDP (`grdctl` + a client) | Needs a package install, a TLS cert, and a listener on port 3389 |

What is left is the API the RDP server itself uses underneath: `org.gnome.Mutter.ScreenCast` hands
out a PipeWire node for a monitor, and PipeWire is readable from within the session without a
prompt. `scripts/screenshot-session.py` asks for the node and pulls one frame off it with
GStreamer's `pipewiresrc`. Same frames an RDP client would receive, without the server, the client,
or the open port.

Nothing needs installing: `gi` (Gio/GLib/Gst) and the `pipewiresrc` element are both already present
on a GNOME desktop.

## What it does and does not capture

**The whole monitor**, not a window. Mutter's `RecordWindow` needs a window id, and the only ways to
learn one — `org.gnome.Shell.Introspect.GetWindows` and `org.gnome.Shell.Eval` — are both
`AccessDenied` outside GNOME Shell. So the frame contains whatever else is on screen. Crop before
attaching one to a bug report.

That is also a reason to think before running it: it is a picture of the developer's desktop.

**Only what is on top.** A window behind another is occluded, and there is no way to raise one from
a script here. In practice: launch the application last, or capture while both are visible and
measure each in place — a single frame holding an old build and a new one side by side is the
strongest before/after evidence available, and is how BUG-002's fix was confirmed.

**The pointer is excluded** (`cursor-mode` 0). A screenshot used as evidence about layout should not
carry a cursor that happened to be over a control.

## What this is not

It is **not** an automated gate, and no test calls it. The application's automated visual checks
rasterise components through the headless renderer inside the crate —
`material/content_placement.rs` and `material/ripple_clipping.rs` — and its geometry is asserted by
`tests/layout_snapshot.rs` and `material/anatomy_size.rs`. Those run in CI, on any machine, with no
desktop at all.

This is for the half those cannot do: the judgement a person makes by looking. It makes that half
repeatable and reviewable rather than remembered.

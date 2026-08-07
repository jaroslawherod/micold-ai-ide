#!/usr/bin/env python3
"""Capture one PNG frame of the logged-in desktop, without a consent dialog.

Why this exists
---------------
Feature 018's quickstart asks for visual confirmation in the *running application* — the half of
`content_placement.rs` and `anatomy_size.rs` that a person has to look at. BUG-002 was reported from
two screenshots and fixed without one, because nothing on this machine could take a screenshot from
a shell: `grim` is Wayland-only-and-absent, `import` is X11 and the session is Wayland, and
`org.gnome.Shell.Screenshot` answers `AccessDenied` to anything that is not GNOME Shell itself.

The one remaining route is the API remote desktop uses underneath: `org.gnome.Mutter.ScreenCast`
hands out a PipeWire node for a monitor, and PipeWire is readable from this session without a
portal prompt. That is what this does — the same frames an RDP client would receive, taken directly
rather than round-tripped through a server and a client that would both have to be installed and,
in the RDP case, a listener opened on port 3389.

Requires nothing that is not already installed: `gi` (Gio/GLib/Gst), `gst-launch`'s `pipewiresrc`
element, and a running GNOME session on the current `DISPLAY`/`WAYLAND_DISPLAY`.

Usage
-----
    scripts/screenshot-session.py OUT.png [--monitor eDP-1] [--timeout 10]

`--monitor` defaults to the first monitor Mutter reports. Exits non-zero with a diagnosis if the
session is unavailable, so a script that depends on a screenshot fails loudly rather than silently
comparing nothing.
"""

import argparse
import sys

import gi

gi.require_version("Gst", "1.0")
from gi.repository import Gio, GLib, Gst  # noqa: E402

BUS = "org.gnome.Mutter.ScreenCast"
PATH = "/org/gnome/Mutter/ScreenCast"


def _proxy(bus, path, interface):
    return Gio.DBusProxy.new_sync(
        bus, Gio.DBusProxyFlags.NONE, None, BUS, path, interface, None
    )


def first_monitor(bus):
    """The connector name of the first monitor Mutter reports (e.g. `eDP-1`, `DP-3`)."""
    display = Gio.DBusProxy.new_sync(
        bus,
        Gio.DBusProxyFlags.NONE,
        None,
        "org.gnome.Mutter.DisplayConfig",
        "/org/gnome/Mutter/DisplayConfig",
        "org.gnome.Mutter.DisplayConfig",
        None,
    )
    state = display.call_sync("GetCurrentState", None, Gio.DBusCallFlags.NONE, -1, None)
    monitors = state.unpack()[1]
    if not monitors:
        raise SystemExit("mutter reports no monitors — is a session running on this seat?")
    return monitors[0][0][0]


def capture(out_path, monitor, timeout_s):
    Gst.init(None)
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)

    screencast = _proxy(bus, PATH, BUS)
    session_path = screencast.call_sync(
        "CreateSession", GLib.Variant("(a{sv})", ({},)), Gio.DBusCallFlags.NONE, -1, None
    ).unpack()[0]
    session = _proxy(bus, session_path, f"{BUS}.Session")

    # `cursor-mode` 1 embeds the pointer in the frame. Left at 0 (hidden): a screenshot used as
    # evidence about layout should not carry a cursor that happened to be over a control.
    stream_path = session.call_sync(
        "RecordMonitor",
        GLib.Variant("(sa{sv})", (monitor, {"cursor-mode": GLib.Variant("u", 0)})),
        Gio.DBusCallFlags.NONE,
        -1,
        None,
    ).unpack()[0]
    stream = _proxy(bus, stream_path, f"{BUS}.Stream")

    loop = GLib.MainLoop()
    node_id = []

    def on_signal(_proxy_, _sender, signal, params):
        if signal == "PipeWireStreamAdded":
            node_id.append(params.unpack()[0])
            loop.quit()

    stream.connect("g-signal", on_signal)
    GLib.timeout_add_seconds(timeout_s, loop.quit)
    session.call_sync("Start", None, Gio.DBusCallFlags.NONE, -1, None)
    loop.run()

    if not node_id:
        session.call_sync("Stop", None, Gio.DBusCallFlags.NONE, -1, None)
        raise SystemExit("mutter never published a PipeWire node for the stream")

    # Drop the first frames: the stream's first buffers can arrive before the compositor has
    # rendered into the new node, and a black frame is worse than no frame — it looks like an
    # answer. `num-buffers` counts what reaches the sink, so this keeps the last of five.
    pipeline = Gst.parse_launch(
        f"pipewiresrc path={node_id[0]} num-buffers=5 ! videoconvert ! "
        f"pngenc snapshot=false ! multifilesink location={out_path}"
    )
    pipeline.set_state(Gst.State.PLAYING)
    msg = pipeline.get_bus().timed_pop_filtered(
        timeout_s * Gst.SECOND, Gst.MessageType.EOS | Gst.MessageType.ERROR
    )
    pipeline.set_state(Gst.State.NULL)
    session.call_sync("Stop", None, Gio.DBusCallFlags.NONE, -1, None)

    if msg is None:
        raise SystemExit("the capture pipeline produced no frame before the timeout")
    if msg.type == Gst.MessageType.ERROR:
        err, _debug = msg.parse_error()
        raise SystemExit(f"the capture pipeline failed: {err.message}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("out", help="where to write the PNG")
    parser.add_argument("--monitor", help="connector name; defaults to the first monitor")
    parser.add_argument("--timeout", type=int, default=10, help="seconds to wait for a frame")
    args = parser.parse_args()

    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    monitor = args.monitor or first_monitor(bus)
    capture(args.out, monitor, args.timeout)
    print(f"{args.out} ({monitor})", file=sys.stderr)


if __name__ == "__main__":
    main()

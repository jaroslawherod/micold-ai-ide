#!/usr/bin/python3
# Real input through org.gnome.Mutter.RemoteDesktop on the PRIVATE bus, plus screenshots.
import os, sys, time, re
from gi.repository import Gio, GLib
addr = os.environ.get('DBUS_SESSION_BUS_ADDRESS', '')
assert 't091-probe' in addr and '/run/user/1000/bus' != addr.split('=')[-1], 'private bus only'
assert 'DISPLAY' not in os.environ
bus = Gio.bus_get_sync(Gio.BusType.SESSION)
RD = 'org.gnome.Mutter.RemoteDesktop'
def call(dest, path, iface, method, params=None):
    return bus.call_sync(dest, path, iface, method, params, None, Gio.DBusCallFlags.NONE, 8000, None)
def ts(): return '%.3f' % time.time()
sess = call(RD, '/org/gnome/Mutter/RemoteDesktop', RD, 'CreateSession').unpack()[0]
S = RD + '.Session'
call(RD, sess, S, 'Start')
print(ts(), '[drive] remote desktop session', sess, flush=True)
def rel(dx, dy): call(RD, sess, S, 'NotifyPointerMotionRelative', GLib.Variant('(dd)', (float(dx), float(dy))))
def moveto(x, y):
    # pin to the bottom-right corner (the top-left one is the overview hot corner), then go to x,y
    for _ in range(20): rel(200, 200); time.sleep(0.01)
    n = 20
    for _ in range(n): rel((x - 1279) / n, (y - 799) / n); time.sleep(0.01)
for cmd in sys.argv[1:]:
    op, _, arg = cmd.partition(':')
    if op == 'esc':
        for st in (True, False):
            call(RD, sess, S, 'NotifyKeyboardKeycode', GLib.Variant('(ub)', (1, st))); time.sleep(0.05)
    elif op == 'key':
        for st in (True, False):
            call(RD, sess, S, 'NotifyKeyboardKeycode', GLib.Variant('(ub)', (int(arg), st))); time.sleep(0.05)
    elif op == 'sleep': time.sleep(float(arg))
    elif op == 'wait':
        f, pat = arg.split(',', 1); t0 = time.time()
        while time.time() - t0 < 60:
            try:
                if re.search(pat, open(f).read()): break
            except OSError: pass
            time.sleep(0.2)
        else: print(ts(), '[drive] TIMEOUT waiting', pat, flush=True); continue
    elif op == 'shot':
        r = call('org.gnome.Shell.Screenshot', '/org/gnome/Shell/Screenshot', 'org.gnome.Shell.Screenshot', 'Screenshot', GLib.Variant('(bbs)', (True, False, arg)))
        print(ts(), '[drive] screenshot', r.unpack(), flush=True); continue
    elif op == 'move':
        x, y = map(float, arg.split(',')); moveto(x, y)
    elif op == 'click':
        x, y = map(float, arg.split(',')); moveto(x, y); time.sleep(0.3)
        print(ts(), '[drive] button press at', x, y, flush=True)
        call(RD, sess, S, 'NotifyPointerButton', GLib.Variant('(ib)', (272, True))); time.sleep(0.08)
        call(RD, sess, S, 'NotifyPointerButton', GLib.Variant('(ib)', (272, False)))
    elif op == 'press':
        print(ts(), '[drive] button press (no move)', flush=True)
        call(RD, sess, S, 'NotifyPointerButton', GLib.Variant('(ib)', (272, True))); time.sleep(0.08)
        call(RD, sess, S, 'NotifyPointerButton', GLib.Variant('(ib)', (272, False)))
    elif op == 'eval':
        r = call('org.gnome.Shell', '/org/gnome/Shell', 'org.gnome.Shell', 'Eval', GLib.Variant('(s)', (arg,)))
        print(ts(), '[drive] eval', arg, '->', r.unpack(), flush=True); continue
    print(ts(), '[drive]', cmd, 'done', flush=True)
call(RD, sess, S, 'Stop')

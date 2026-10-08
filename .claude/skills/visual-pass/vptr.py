"""Persistent zwlr_virtual_pointer_v1 client for a PRIVATE headless sway (visual-pass skill).

Usage: WAYLAND_DISPLAY=<socket> XDG_RUNTIME_DIR=/tmp/vpwN python3 vptr.py FIFO
Prints "ready" once attached. Write one command per line to FIFO (each write reopens it):
  abs X Y   move to X,Y (output 1600x1200)    rel DX DY   move relative
  click     left press+release                down / up   left button
  scroll N  N px, positive = down             quit        exit (removes the FIFO)

Refuses to run unless the resolved Wayland socket sits in a /tmp/vp<name><N>/ directory owned by
the caller, so it can never drive the user's own session. Never use uinput/ydotool instead: that
moves the real pointer.
"""
import os, re, signal, socket, struct, sys, time

fifo = sys.argv[1] if len(sys.argv) > 1 else sys.exit('usage: vptr.py FIFO')
rt = os.environ.get('XDG_RUNTIME_DIR', '')
sock = os.path.realpath(os.path.join(rt, os.environ.get('WAYLAND_DISPLAY', '')))
d = os.path.dirname(sock)
if not re.match(r'^/tmp/vp[0-9a-z]*[0-9]+/', sock) or not os.path.isdir(d) or os.stat(d).st_uid != os.getuid():
    sys.exit(f'refusing: {sock!r} is not a private /tmp/vp*N/ socket of this user')

s = socket.socket(socket.AF_UNIX)
s.connect(sock)

def pad(b): return b + b'\0' * ((4 - len(b) % 4) % 4)
def msg(o, op, payload=b''): s.sendall(struct.pack('<II', o, ((8 + len(payload)) << 16) | op) + payload)
def st(x): b = x.encode() + b'\0'; return struct.pack('<I', len(b)) + pad(b)
def recv(n):
    b = b''
    while len(b) < n:
        c = s.recv(n - len(b))
        if not c: sys.exit('compositor closed the connection')
        b += c
    return b
def events():
    while True:
        o, sz = struct.unpack('<II', recv(8)); op = sz & 0xffff; sz >>= 16
        yield o, op, recv(sz - 8)

msg(1, 1, struct.pack('<I', 2)); msg(1, 0, struct.pack('<I', 3))  # get_registry(2), sync(3)
g = {}
for o, op, b in events():
    if o == 2 and op == 0:
        n, l = struct.unpack('<II', b[:8]); name = b[8:8 + l - 1].decode()
        g[name] = (n, struct.unpack('<I', b[8 + ((l + 3) // 4) * 4:][:4])[0])
    if o == 3: break
M = 'zwlr_virtual_pointer_manager_v1'
if M not in g: sys.exit(f'compositor lacks {M}')
msg(2, 0, struct.pack('<I', g[M][0]) + st(M) + struct.pack('<II', 1, 4))
msg(4, 0, struct.pack('<II', 0, 5))  # create_virtual_pointer(null seat, id 5)

t = lambda: int(time.monotonic() * 1000) & 0xffffffff
fx = lambda x: int(x * 256)
def frame(): msg(5, 4)
def button(down): msg(5, 2, struct.pack('<III', t(), 272, 1 if down else 0)); frame()

def done(*_):
    try: os.unlink(fifo)
    except FileNotFoundError: pass
    sys.exit(0)
signal.signal(signal.SIGTERM, done); signal.signal(signal.SIGINT, done)

print('ready', flush=True)
while True:
    for line in open(fifo):
        a = line.split()
        if not a: continue
        try:
            if a[0] == 'abs': msg(5, 1, struct.pack('<IIIII', t(), int(a[1]), int(a[2]), 1600, 1200)); frame()
            elif a[0] == 'rel': msg(5, 0, struct.pack('<Iii', t(), fx(float(a[1])), fx(float(a[2])))); frame()
            elif a[0] == 'down': button(True)
            elif a[0] == 'up': button(False)
            elif a[0] == 'click': button(True); time.sleep(.05); button(False)
            elif a[0] == 'scroll': msg(5, 3, struct.pack('<IIi', t(), 0, fx(float(a[1])))); frame()
            elif a[0] == 'quit': done()
            else: print('unknown command:', line.strip(), file=sys.stderr, flush=True)
        except (ValueError, IndexError, struct.error):
            print('bad command:', line.strip(), file=sys.stderr, flush=True)

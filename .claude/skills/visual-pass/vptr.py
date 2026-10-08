"""Persistent zwlr_virtual_pointer_v1 client for a PRIVATE headless sway (visual-pass skill).

Usage: WAYLAND_DISPLAY=<socket> XDG_RUNTIME_DIR=/tmp/vpN python3 vptr.py FIFO
Prints "ready" once attached. Write one command per line to FIFO:
  abs X Y   move to X,Y (output 1600x1200)    rel DX DY   move relative
  click     left press+release                down / up   left button
  scroll N  N px, positive = down             quit        exit

Refuses to run unless the resolved Wayland socket is under /tmp/vp, so it can never drive the
user's own session. Never use uinput/ydotool instead: that moves the real pointer.
"""
import os, socket, struct, sys, time
rt = os.environ.get('XDG_RUNTIME_DIR', '')
sock = os.path.realpath(os.path.join(rt, os.environ.get('WAYLAND_DISPLAY', '')))
if not sock.startswith('/tmp/vp') or not rt.startswith('/tmp/vp'):
    sys.exit(f'refusing: {sock!r} is not a private /tmp/vp* socket')
s=socket.socket(socket.AF_UNIX); s.connect(sock)
def pad(b): return b+b'\0'*((4-len(b)%4)%4)
def msg(o,op,payload=b''): s.sendall(struct.pack('<II',o,((8+len(payload))<<16)|op)+payload)
def st(x): b=x.encode()+b'\0'; return struct.pack('<I',len(b))+pad(b)
def events():
    while True:
        h=s.recv(8)
        while len(h)<8: h+=s.recv(8-len(h))
        o,sz=struct.unpack('<II',h); op=sz&0xffff; sz>>=16
        body=b''
        while len(body)<sz-8: body+=s.recv(sz-8-len(body))
        yield o,op,body
msg(1,1,struct.pack('<I',2)); msg(1,0,struct.pack('<I',3))
g={}
for o,op,b in events():
    if o==2 and op==0:
        n,l=struct.unpack('<II',b[:8]); name=b[8:8+l-1].decode(); v=struct.unpack('<I',b[8+((l+3)//4)*4:][:4])[0]; g[name]=(n,v)
    if o==3: break
n,v=g['zwlr_virtual_pointer_manager_v1']; nm=pad(b'zwlr_virtual_pointer_manager_v1\0')
msg(2,0,struct.pack('<I',n)+st('zwlr_virtual_pointer_manager_v1')+struct.pack('<II',1,4))
msg(4,0,struct.pack('<II',0,5))  # create_virtual_pointer(null seat, id 5)
t=lambda:int(time.monotonic()*1000)&0xffffffff
fx=lambda x:int(x*256)
def frame(): msg(5,4)
fifo=sys.argv[1]
print('ready',flush=True)
while True:
    for line in open(fifo):
        a=line.split()
        if not a: continue
        if a[0]=='abs': msg(5,1,struct.pack('<IIIII',t(),int(a[1]),int(a[2]),1600,1200)); frame()
        elif a[0]=='rel': msg(5,0,struct.pack('<Iii',t(),fx(float(a[1])),fx(float(a[2])))); frame()
        elif a[0] in('down','up'): msg(5,2,struct.pack('<III',t(),272,1 if a[0]=='down' else 0)); frame()
        elif a[0]=='click':
            msg(5,2,struct.pack('<III',t(),272,1)); frame(); time.sleep(.05); msg(5,2,struct.pack('<III',t(),272,0)); frame()
        elif a[0]=='scroll': msg(5,3,struct.pack('<IIi',t(),0,fx(float(a[1])))); frame()
        elif a[0]=='quit': sys.exit(0)

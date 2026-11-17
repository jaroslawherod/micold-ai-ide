import { BrowserTransport, type BrowserEnv, type SocketLike } from "../../src/real/BrowserTransport";

class FakeSocket implements SocketLike {
  sent: string[] = [];
  closed = 0;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onclose: ((event: { code: number; wasClean: boolean }) => void) | null = null;
  send(data: string) {
    this.sent.push(data);
  }
  close() {
    this.closed += 1;
  }
}

function setup() {
  const sockets: FakeSocket[] = [];
  let listener: () => void = () => undefined;
  const doc = { visibilityState: "hidden", addEventListener: (_: "visibilitychange", cb: () => void) => (listener = cb) };
  const env: BrowserEnv = { createSocket: () => (sockets.push(new FakeSocket()), sockets[sockets.length - 1]!), document: doc };
  const transport = new BrowserTransport("ws://x/api/ws", env);
  const closes: unknown[] = [];
  const frames: string[] = [];
  let opens = 0;
  transport.onOpen(() => (opens += 1));
  transport.onClose((info) => closes.push(info));
  transport.onMessage((f) => frames.push(f));
  return { sockets, transport, closes, frames, opens: () => opens, doc, fire: () => listener() };
}

describe("BrowserTransport", () => {
  it("connects, passes text frames up and frames down", () => {
    const t = setup();
    t.transport.open();
    t.sockets[0]!.onopen!();
    t.sockets[0]!.onmessage!({ data: '{"type":"pong"}' });
    t.sockets[0]!.onmessage!({ data: new ArrayBuffer(1) });
    t.transport.send("hi");
    expect(t.opens()).toBe(1);
    expect(t.frames).toEqual(['{"type":"pong"}']);
    expect(t.sockets[0]!.sent).toEqual(["hi"]);
  });

  it.each([1001, 1006, 1011])("close code %i is retryable", (code) => {
    const t = setup();
    t.transport.open();
    t.sockets[0]!.onclose!({ code, wasClean: code !== 1006 });
    expect(t.closes).toEqual([{ clean: code !== 1006, code, fatal: false }]);
  });

  it("close code 1008 is fatal: no retry", () => {
    const t = setup();
    t.transport.open();
    t.sockets[0]!.onclose!({ code: 1008, wasClean: true });
    expect(t.closes).toEqual([{ clean: true, code: 1008, fatal: true }]);
  });

  it("forgets the old socket on close and reopen", () => {
    const t = setup();
    t.transport.open();
    const first = t.sockets[0]!;
    t.transport.open();
    expect(first.closed).toBe(1);
    first.onclose?.({ code: 1006, wasClean: false });
    expect(first.onclose).toBeNull();
    t.transport.close();
    expect(t.sockets[1]!.closed).toBe(1);
    expect(t.closes).toEqual([]);
  });

  it("reports the page becoming visible, not hidden", () => {
    const t = setup();
    let visible = 0;
    t.transport.onVisible(() => (visible += 1));
    t.fire();
    expect(visible).toBe(0);
    t.doc.visibilityState = "visible";
    t.fire();
    expect(visible).toBe(1);
  });
});

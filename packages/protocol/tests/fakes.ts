// Test doubles for the connection manager: a transport the test drives and a manual clock.
import type { Clock, Handle, Transport } from "../src/api";

export class FakeTransport implements Transport {
  sent: Record<string, unknown>[] = [];
  opens = 0;
  closes = 0;
  private openCb = () => undefined as void;
  private closeCb: (info: { clean: boolean; code?: number; fatal?: boolean }) => void = () => undefined;
  private visibleCb: () => void = () => undefined;
  private messageCb: (frame: string) => void = () => undefined;
  open() {
    this.opens += 1;
  }
  close() {
    this.closes += 1;
  }
  send(frame: string) {
    this.sent.push(JSON.parse(frame) as Record<string, unknown>);
  }
  onOpen(cb: () => void) {
    this.openCb = cb;
  }
  onClose(cb: (info: { clean: boolean; code?: number; fatal?: boolean }) => void) {
    this.closeCb = cb;
  }
  onMessage(cb: (frame: string) => void) {
    this.messageCb = cb;
  }
  // Driven by the test:
  accept() {
    this.openCb();
  }
  drop(clean = false, extra: { code?: number; fatal?: boolean } = {}) {
    this.closeCb({ clean, ...extra });
  }
  onVisible(cb: () => void) {
    this.visibleCb = cb;
  }
  becomeVisible() {
    this.visibleCb();
  }
  deliver(frame: Record<string, unknown>) {
    this.messageCb(JSON.stringify(frame));
  }
  deliverRaw(frame: string) {
    this.messageCb(frame);
  }
}

export class FakeClock implements Clock {
  private t = 0;
  private timers = new Map<number, { at: number; fn: () => void }>();
  private next = 1;
  now() {
    return this.t;
  }
  setTimeout(fn: () => void, ms: number): Handle {
    const id = this.next++;
    this.timers.set(id, { at: this.t + ms, fn });
    return id;
  }
  clear(h: Handle) {
    this.timers.delete(h as number);
  }
  /** Delays of the pending timers, relative to now. */
  pending(): number[] {
    return [...this.timers.values()].map((x) => x.at - this.t);
  }
  advance(ms: number) {
    this.t += ms;
    for (const [id, timer] of [...this.timers]) {
      if (timer.at <= this.t) {
        this.timers.delete(id);
        timer.fn();
      }
    }
  }
}

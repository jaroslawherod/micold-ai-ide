// The one browser-specific seam of the real client (research R6): it adapts a WebSocket to the
// `Transport` the connection logic speaks. It knows the close codes (1008 is a policy close, not
// worth retrying) and `visibilitychange` (a hidden tab's timers are throttled, so the client checks
// its connection when the page returns). Everything else is in `RealOrchestratorApi`, over `Clock`.
import type { Transport } from "../api";

/** The close code a server uses for a policy refusal (ws-protocol.md *Close codes*). */
export const POLICY_CLOSE = 1008;

export interface SocketLike {
  send(data: string): void;
  close(code?: number): void;
  onopen: (() => void) | null;
  onmessage: ((event: { data: unknown }) => void) | null;
  onclose: ((event: { code: number; wasClean: boolean }) => void) | null;
}

export interface VisibilitySource {
  readonly visibilityState: string;
  addEventListener(type: "visibilitychange", cb: () => void): void;
}

export interface BrowserEnv {
  createSocket(url: string): SocketLike;
  document?: VisibilitySource;
}

const browserEnv = (): BrowserEnv => ({
  createSocket: (url) => new WebSocket(url) as unknown as SocketLike,
  document: typeof document === "undefined" ? undefined : document,
});

export class BrowserTransport implements Transport {
  private socket: SocketLike | undefined;
  private openCb: () => void = () => undefined;
  private closeCb: (info: { clean: boolean; code?: number; fatal?: boolean }) => void = () => undefined;
  private messageCb: (frame: string) => void = () => undefined;

  constructor(
    private readonly url: string,
    private readonly env: BrowserEnv = browserEnv(),
  ) {}

  open(): void {
    this.detach();
    const socket = this.env.createSocket(this.url);
    this.socket = socket;
    socket.onopen = () => this.openCb();
    socket.onmessage = (event) => {
      // The server sends text frames only (FR-006); anything else is ignored.
      if (typeof event.data === "string") this.messageCb(event.data);
    };
    socket.onclose = (event) => this.closeCb({ clean: event.wasClean, code: event.code, fatal: event.code === POLICY_CLOSE });
  }

  /** Closes the socket; no further event of it reaches a callback. */
  close(): void {
    this.detach();
  }

  send(frame: string): void {
    this.socket?.send(frame);
  }

  onOpen(cb: () => void): void {
    this.openCb = cb;
  }
  onClose(cb: (info: { clean: boolean; code?: number; fatal?: boolean }) => void): void {
    this.closeCb = cb;
  }
  onMessage(cb: (frame: string) => void): void {
    this.messageCb = cb;
  }

  onVisible(cb: () => void): void {
    const doc = this.env.document;
    doc?.addEventListener("visibilitychange", () => {
      if (doc.visibilityState === "visible") cb();
    });
  }

  private detach(): void {
    const socket = this.socket;
    this.socket = undefined;
    if (!socket) return;
    socket.onopen = socket.onmessage = socket.onclose = null;
    socket.close();
  }
}

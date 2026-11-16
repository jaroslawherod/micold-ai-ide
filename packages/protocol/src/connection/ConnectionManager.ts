// Socket-side client logic over a `Transport` (FR-009, contracts/transport.md): reconnect with
// exponential backoff and full jitter, resume by the last applied seq, request/response correlation
// by id with a timeout, and the connection state the banner shows. Frames are JSON objects with
// `id`, `seq` or `method`; the real wire format belongs to #1732.
import { ApiError, type ApiErrorCode, type Clock, type ConnectionOptions, type Handle, type Transport, type Unsubscribe } from "../api";
import type { ConnectionState } from "../types";

export interface EventFrame {
  seq: number;
  [field: string]: unknown;
}

interface Pending {
  resolve(value: unknown): void;
  reject(err: ApiError): void;
  timer: Handle;
}

const DEFAULT_MAX_ATTEMPTS = 8;

export class ConnectionManager {
  private state: ConnectionState = { kind: "offline" };
  private listeners = new Set<(s: ConnectionState) => void>();
  private eventListeners = new Set<(f: EventFrame) => void>();
  private snapshotListeners = new Set<(s: unknown) => void>();
  private pending = new Map<number, Pending>();
  private nextId = 1;
  private attempt = 0;
  private retryTimer: Handle | undefined;
  private everOpened = false;
  private stopped = true;
  private seq = 0;

  constructor(
    private readonly transport: Transport,
    private readonly clock: Clock,
    private readonly options: ConnectionOptions,
  ) {
    transport.onOpen(() => this.handleOpen());
    transport.onClose(() => this.handleClose());
    transport.onMessage((frame) => this.handleFrame(frame));
  }

  get lastSeq(): number {
    return this.seq;
  }

  get(): ConnectionState {
    return this.state;
  }

  subscribe(cb: (s: ConnectionState) => void): Unsubscribe {
    this.listeners.add(cb);
    return () => this.listeners.delete(cb);
  }

  /** Event frames, in seq order, each at most once. */
  onEvent(cb: (frame: EventFrame) => void): Unsubscribe {
    this.eventListeners.add(cb);
    return () => this.eventListeners.delete(cb);
  }

  /** A fresh snapshot fetched after a gap in the seq. */
  onSnapshot(cb: (snapshot: unknown) => void): Unsubscribe {
    this.snapshotListeners.add(cb);
    return () => this.snapshotListeners.delete(cb);
  }

  start(): void {
    this.stopped = false;
    this.transport.open();
  }

  stop(): void {
    this.stopped = true;
    this.cancelRetry();
    this.transport.close();
    this.setState({ kind: "offline" });
  }

  /** Leave `offline` (or skip the wait) and open now. */
  retryNow(): void {
    this.cancelRetry();
    this.attempt = 0;
    this.start();
  }

  request(method: string, params?: unknown): Promise<unknown> {
    if (this.state.kind !== "connected") return Promise.reject(new ApiError("unavailable"));
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const timer = this.clock.setTimeout(() => this.settle(id, "timeout"), this.options.requestTimeoutMs);
      this.pending.set(id, { resolve, reject, timer });
      this.transport.send(JSON.stringify(params === undefined ? { id, method } : { id, method, params }));
    });
  }

  private handleOpen(): void {
    this.attempt = 0;
    if (this.everOpened) this.transport.send(JSON.stringify({ method: "resume", params: { sinceSeq: this.seq } }));
    this.everOpened = true;
    this.setState({ kind: "connected" });
  }

  private handleClose(): void {
    for (const id of [...this.pending.keys()]) this.settle(id, "unavailable");
    if (this.stopped) return;
    this.attempt += 1;
    const max = this.options.maxAttempts ?? DEFAULT_MAX_ATTEMPTS;
    if (this.attempt > max) {
      this.setState({ kind: "offline" });
      return;
    }
    this.setState({ kind: "reconnecting", attempt: this.attempt });
    const ceiling = Math.min(this.options.maxMs, this.options.baseMs * 2 ** (this.attempt - 1));
    this.retryTimer = this.clock.setTimeout(() => {
      this.retryTimer = undefined;
      this.transport.open();
    }, this.options.random() * ceiling);
  }

  private handleFrame(raw: string): void {
    let frame: unknown;
    try {
      frame = JSON.parse(raw);
    } catch {
      return;
    }
    if (typeof frame !== "object" || frame === null) return;
    const f = frame as Record<string, unknown>;
    if (typeof f.id === "number") {
      const p = this.pending.get(f.id);
      if (!p) return;
      if (typeof f.error === "string") this.settle(f.id, f.error as ApiErrorCode);
      else this.settle(f.id, undefined, f.result);
      return;
    }
    if (typeof f.seq === "number") this.handleEvent(f as EventFrame);
  }

  private handleEvent(frame: EventFrame): void {
    if (frame.seq <= this.seq) return;
    if (frame.seq > this.seq + 1) {
      void this.request("snapshot").then(
        (snapshot) => {
          const s = (snapshot as { seq?: unknown } | null)?.seq;
          if (typeof s === "number") this.seq = s;
          for (const cb of this.snapshotListeners) cb(snapshot);
        },
        () => undefined,
      );
      return;
    }
    this.seq = frame.seq;
    for (const cb of this.eventListeners) cb(frame);
  }

  private settle(id: number, error: ApiErrorCode | undefined, result?: unknown): void {
    const p = this.pending.get(id);
    if (!p) return;
    this.pending.delete(id);
    this.clock.clear(p.timer);
    if (error) p.reject(new ApiError(error));
    else p.resolve(result);
  }

  private cancelRetry(): void {
    if (this.retryTimer !== undefined) this.clock.clear(this.retryTimer);
    this.retryTimer = undefined;
  }

  private setState(next: ConnectionState): void {
    this.state = next;
    for (const cb of this.listeners) cb(next);
  }
}

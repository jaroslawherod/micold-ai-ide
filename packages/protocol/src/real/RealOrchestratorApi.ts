// The orchestrator API over `/api/ws` (#1732, contracts/ws-protocol.md). It owns the wire protocol:
// one `subscribe` per connection (a resume from `{epoch, seq}` after a drop, a fresh snapshot after
// `resync_needed`), a ping every 20 s with a 40 s silence deadline, and the connection state the
// banner shows, with the backoff the `ConnectionManager` uses. The position (`epoch`, `lastSeq`) lives
// in memory only (FR-016). The browser is reached through `Transport` and time through `Clock`.
import { ApiError, type ApiErrorCode, type Clock, type ConnectionOptions, type Handle, type OrchestratorApi, type SessionRef, type Transport, type Unsubscribe } from "../api";
import { DEFAULT_MAX_ATTEMPTS, retryDelay } from "../connection/backoff";
import type * as Wire from "../generated/client-api";
import { applyChanges } from "../order";
import type { Change, ConnectionState, Snapshot } from "../types";
import { changeFromWire, snapshotFromWire } from "../wire";

export interface RealOptions extends ConnectionOptions {
  /** Ping interval; 20 s (R7). */
  pingMs: number;
  /** Silence after which the connection is dead; 40 s (R7). */
  deadMs: number;
}

export const REAL_DEFAULTS: Omit<RealOptions, "random"> = {
  baseMs: 500,
  maxMs: 15_000,
  requestTimeoutMs: 10_000,
  maxAttempts: DEFAULT_MAX_ATTEMPTS,
  pingMs: 20_000,
  deadMs: 40_000,
};

interface Pending {
  resolve(message: Wire.ServerMessage): void;
  reject(err: ApiError): void;
  timer: Handle;
}

export class RealOrchestratorApi implements OrchestratorApi {
  private state: ConnectionState = { kind: "connecting" };
  private started = false;
  private stateListeners = new Set<(s: ConnectionState) => void>();
  private batchListeners = new Set<{ since: number; cb: (changes: Change[]) => void }>();
  private snapshotListeners = new Set<(s: Snapshot) => void>();
  private waiters: { resolve(s: Snapshot): void; reject(e: ApiError): void }[] = [];
  private pending = new Map<number, Pending>();
  private snapshot: Snapshot | null = null;
  private epoch: number | null = null;
  private lastSeq = 0;
  private nextId = 1;
  private attempt = 0;
  private lastRx = 0;
  private retryTimer: Handle | undefined;
  private pingTimer: Handle | undefined;
  private readonly options: RealOptions;

  constructor(
    private readonly transport: Transport,
    private readonly clock: Clock,
    options: Partial<RealOptions> = {},
  ) {
    this.options = { ...REAL_DEFAULTS, random: Math.random, ...options };
    transport.onOpen(() => this.handleOpen());
    transport.onClose((info) => this.handleClose(info.fatal === true));
    transport.onMessage((raw) => this.handleMessage(raw));
    transport.onVisible?.(() => this.handleVisible());
  }

  readonly connection = {
    get: (): ConnectionState => this.state,
    subscribe: (cb: (s: ConnectionState) => void): Unsubscribe => {
      this.stateListeners.add(cb);
      return () => this.stateListeners.delete(cb);
    },
  };

  /** Opens the connection; idempotent. The first read or subscription calls it. */
  start(): void {
    if (this.started) return;
    this.started = true;
    this.setState({ kind: "connecting" });
    this.transport.open();
  }

  getSnapshot(): Promise<Snapshot> {
    this.start();
    if (this.snapshot) return Promise.resolve(this.snapshot);
    if (this.state.kind === "offline") return Promise.reject(new ApiError("unavailable"));
    return new Promise((resolve, reject) => this.waiters.push({ resolve, reject }));
  }

  subscribe(sinceSeq: number, onBatch: (changes: Change[]) => void): Unsubscribe {
    this.start();
    const entry = { since: sinceSeq, cb: onBatch };
    this.batchListeners.add(entry);
    return () => this.batchListeners.delete(entry);
  }

  onSnapshot(cb: (snapshot: Snapshot) => void): Unsubscribe {
    this.snapshotListeners.add(cb);
    return () => this.snapshotListeners.delete(cb);
  }

  async readSession(ref: SessionRef): Promise<Wire.ContentEntry[]> {
    this.start();
    const reply = await this.request({ type: "read_session", daemon: ref.daemon, project: ref.project, session: ref.session });
    if (reply.type !== "session_content") throw new ApiError("internal");
    return reply.entries;
  }

  /** The user's retry from `offline`. */
  retry(): void {
    if (!this.started || this.state.kind !== "offline") return;
    this.attempt = 0;
    this.setState({ kind: "connecting" });
    this.transport.open();
  }

  stop(): void {
    this.started = false;
    this.cancelTimers();
    this.transport.close();
    this.failAll();
    this.setState({ kind: "offline" });
  }

  private handleOpen(): void {
    this.lastRx = this.clock.now();
    const since = this.epoch === null ? undefined : { epoch: this.epoch, seq: this.lastSeq };
    this.send(since ? { type: "subscribe", since } : { type: "subscribe" });
    this.schedulePing();
  }

  private handleClose(fatal: boolean): void {
    this.cancelTimers();
    this.failAll(false);
    if (fatal) {
      this.goOffline();
      return;
    }
    this.attempt += 1;
    if (this.attempt > (this.options.maxAttempts ?? DEFAULT_MAX_ATTEMPTS)) {
      this.goOffline();
      return;
    }
    this.setState({ kind: "reconnecting", attempt: this.attempt });
    this.retryTimer = this.clock.setTimeout(() => {
      this.retryTimer = undefined;
      this.transport.open();
    }, retryDelay(this.attempt, this.options));
  }

  /** Back in the foreground: a connection whose last word is older than a ping interval is suspect (R6). */
  private handleVisible(): void {
    if (!this.started) return;
    if (this.state.kind === "reconnecting") {
      // Waiting out a backoff: shorten the wait, keep the attempt count (the backoff stays honest).
      if (this.retryTimer === undefined) return;
      this.clock.clear(this.retryTimer);
      this.retryTimer = undefined;
      this.transport.open();
      return;
    }
    const stale = this.state.kind === "connected" && this.clock.now() - this.lastRx >= this.options.pingMs;
    if (!stale) return;
    this.cancelTimers();
    this.failAll(false);
    this.transport.close();
    this.attempt = 1;
    this.setState({ kind: "reconnecting", attempt: 1 });
    this.transport.open();
  }

  private goOffline(): void {
    this.setState({ kind: "offline" });
    this.failAll();
  }

  private schedulePing(): void {
    this.pingTimer = this.clock.setTimeout(() => {
      if (this.clock.now() - this.lastRx >= this.options.deadMs) {
        this.transport.close();
        this.handleClose(false);
        return;
      }
      this.send({ type: "ping" });
      this.schedulePing();
    }, this.options.pingMs);
  }

  private handleMessage(raw: string): void {
    this.lastRx = this.clock.now();
    let message: Wire.ServerMessage;
    try {
      message = JSON.parse(raw) as Wire.ServerMessage;
    } catch {
      return;
    }
    if (typeof message !== "object" || message === null) return;
    switch (message.type) {
      case "subscribed":
        this.handleSubscribed(message);
        return;
      case "changes":
        this.handleChanges(message.changes);
        return;
      case "resync_needed":
        // Our position is not servable: forget it and ask for a whole snapshot (US2 scenario 2).
        this.epoch = null;
        this.send({ type: "subscribe" });
        return;
      default:
        if (typeof message.id === "number") this.settle(message.id, message);
    }
  }

  private handleSubscribed(message: Extract<Wire.ServerMessage, { type: "subscribed" }>): void {
    this.attempt = 0;
    this.epoch = message.epoch;
    if (this.state.kind !== "connected") this.setState({ kind: "connected" });
    if (!message.snapshot) return;
    const replaced = this.snapshot !== null;
    this.snapshot = snapshotFromWire(message.snapshot);
    this.lastSeq = message.snapshot.seq;
    // A new snapshot may start a new epoch whose seq restarts lower: listeners follow from it.
    for (const entry of this.batchListeners) entry.since = this.lastSeq;
    for (const w of this.waiters.splice(0)) w.resolve(this.snapshot);
    if (replaced) for (const cb of this.snapshotListeners) cb(this.snapshot);
  }

  private handleChanges(wire: Wire.Change[]): void {
    const fresh = wire.filter((c) => c.seq > this.lastSeq).map(changeFromWire);
    if (fresh.length === 0) return;
    this.lastSeq = fresh[fresh.length - 1]!.seq;
    if (this.snapshot) this.snapshot = applyChanges(this.snapshot, fresh);
    for (const { since, cb } of [...this.batchListeners]) {
      const batch = fresh.filter((c) => c.seq > since);
      if (batch.length > 0) cb(batch);
    }
  }

  private send(body: Record<string, unknown>): number {
    const id = this.nextId++;
    this.transport.send(JSON.stringify({ id, ...body }));
    return id;
  }

  private request(body: Record<string, unknown>): Promise<Wire.ServerMessage> {
    if (this.state.kind !== "connected") return Promise.reject(new ApiError("unavailable"));
    return new Promise((resolve, reject) => {
      const id = this.send(body);
      const timer = this.clock.setTimeout(() => this.settle(id, undefined, "timeout"), this.options.requestTimeoutMs);
      this.pending.set(id, { resolve, reject, timer });
    });
  }

  private settle(id: number, message?: Wire.ServerMessage, failure?: ApiErrorCode): void {
    const p = this.pending.get(id);
    if (!p) return;
    this.pending.delete(id);
    this.clock.clear(p.timer);
    if (message?.type === "error") p.reject(new ApiError(message.code));
    else if (failure || !message) p.reject(new ApiError(failure ?? "unavailable"));
    else p.resolve(message);
  }

  /** Fails requests in flight; with `waiters`, the reads waiting for a first snapshot too. */
  private failAll(waiters = true): void {
    for (const id of [...this.pending.keys()]) this.settle(id, undefined, "unavailable");
    if (waiters) for (const w of this.waiters.splice(0)) w.reject(new ApiError("unavailable"));
  }

  private cancelTimers(): void {
    for (const h of [this.retryTimer, this.pingTimer]) if (h !== undefined) this.clock.clear(h);
    this.retryTimer = this.pingTimer = undefined;
  }

  private setState(next: ConnectionState): void {
    this.state = next;
    for (const cb of this.stateListeners) cb(next);
  }
}

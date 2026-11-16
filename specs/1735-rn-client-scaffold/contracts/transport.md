# Contract: `Transport` and `ConnectionManager` (`packages/protocol/src/connection/`)

```ts
export interface Transport {
  open(): void; close(): void; send(frame: string): void;
  onOpen(cb: () => void): void; onClose(cb: (info: { clean: boolean }) => void): void;
  onMessage(cb: (frame: string) => void): void;
}
export interface Clock { now(): number; setTimeout(fn: () => void, ms: number): Handle; clear(h: Handle): void }
export interface ConnectionOptions { baseMs: number; maxMs: number; random(): number; requestTimeoutMs: number }
```

Behaviour (each a Jest test over a `FakeTransport` and `FakeClock`):
1. close or failed open -> `reconnecting(attempt n)`; delay = `random() * min(maxMs, baseMs * 2^(n-1))` (full jitter).
2. attempts reset on open; reopening sends `resume { sinceSeq }` with the last applied seq; frames with
   `seq <= lastSeq` are dropped (idempotent).
3. a gap (`seq > lastSeq + 1`) triggers a fresh snapshot request.
4. `request(method, params)` returns a promise matched by id; rejects `timeout`; pending requests reject
   `unavailable` when the connection drops.
5. after `maxAttempts` (option, default 8) the state is `offline`; `retryNow()` restarts.
The wire frame format belongs to #1732; this feature defines frames only inside tests (JSON with `id`,
`seq`, `method`).

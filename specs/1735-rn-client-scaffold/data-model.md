# Data model: #1735 (TypeScript stand-in types, `packages/protocol/src/types.ts`)

All ids are opaque strings. Types are marked `STAND-IN`; #1732's generated types replace them.

| Entity | Fields |
|---|---|
| Daemon | `id`, `name`, `status: 'online' \| 'offline' \| 'needs-update'` |
| Project | `id`, `daemonId`, `name` |
| Location (worktree) | `id`, `projectId`, `name`, `branch`, `isDefault: boolean` |
| Session | `id`, `locationId`, `name`, `kind: 'structured' \| 'terminal'`, `state: 'working' \| 'waiting' \| 'idle' \| 'error' \| 'stopped'` |
| AuthProvider | `id`, `name`, `configured: boolean` |
| ConnectionState | `{ kind: 'connected' } \| { kind: 'reconnecting', attempt: number } \| { kind: 'offline' }` |
| Snapshot | `seq: number`, `daemons: Daemon[]`, `projects: Project[]`, `locations: Location[]`, `sessions: Session[]` (arrays in the orchestrator's order) |
| Change | `seq`, `kind: 'session-state' \| 'daemon-status' \| 'upsert' \| 'remove'`, payload by kind |

Invalid states made unrepresentable: `ConnectionState` is a discriminated union (attempt exists only
when reconnecting); ids are branded types (`DaemonId`, `ProjectId`, `LocationId`, `SessionId`) so a
session id cannot be passed as a location id; enums are literal unions, exhaustive `switch` with `never`.

## Row identity and order

- Row key: `daemonId/projectId/locationId` (location rows), `.../sessionId` (session rows); never an index.
- The client stores `orderIndex: Map<RowKey, number>` taken from the snapshot order. `session-state`
  and `daemon-status` changes mutate the row's value only. `upsert` of a new key appends at the position the
  orchestrator's snapshot order gives (the next snapshot or the change's `afterKey`); `remove` deletes the key
  and does not renumber others (indices are derived by iteration of the stored order, not stored per row).
- A batch applies in sequence order; the result is independent of arrival order within a batch for
  distinct keys (property test).
- Stale: `daemon.status === 'offline'` or `ConnectionState !== connected` marks rows stale; rows stay.

## Session (client) state

`AuthSession`: `{ accessToken: string, identity: { name: string } }` in memory only; no `toJSON`.
`LocalePreference`: `{ locale: LocaleCode }` in `localStorage`. `Message catalog`: see contracts/i18n-catalog.md.

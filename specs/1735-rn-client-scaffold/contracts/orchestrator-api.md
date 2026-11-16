# Contract: `OrchestratorApi` and `AuthService` (`packages/protocol/src/api.ts`)

```ts
export interface OrchestratorApi {
  /** Current snapshot, in the orchestrator's stable order. */
  getSnapshot(): Promise<Snapshot>;
  /** Changes after `sinceSeq`; the callback receives batches in sequence order. Returns unsubscribe. */
  subscribe(sinceSeq: number, onBatch: (changes: Change[]) => void): Unsubscribe;
  /** Connection state for the banner. */
  connection: { get(): ConnectionState; subscribe(cb: (s: ConnectionState) => void): Unsubscribe };
}

export interface AuthService {
  listProviders(): Promise<AuthProvider[]>;           // only `configured` ones are shown
  signIn(providerId: string): Promise<AuthSession | AuthFailure>; // AuthFailure: 'refused' | 'cancelled'
  currentSession(): AuthSession | null;               // memory only
  signOut(): Promise<void>;
  onSessionLost(cb: () => void): Unsubscribe;         // expiry mid-use
}

export interface Api { orchestrator: OrchestratorApi; auth: AuthService }
```

Rules: no method returns or accepts a refresh token; `AuthSession` has no serialiser; the mock and the
real client both implement `Api`; the app obtains `Api` only from `ApiProvider`. Mock extras
(`MockControl`: `pushChange`, `pushBatch`, `setProviders`, `dropConnection`, `restoreConnection`) are
exported only from `@micold/protocol/mock` and used by tests and the Playwright harness (through a
`window.__micoldMock` hook defined only in the mock build).

Errors: methods reject with `ApiError { code: 'unauthenticated' | 'unavailable' | 'timeout' }`;
`unauthenticated` triggers `onSessionLost`.

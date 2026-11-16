# Plan drift, M2

`plan-drift.sh` matches only paths that tasks.md spells out in full. M2's task lines name files by
their short path (`src/api/select.ts`, `app/sign-in.tsx`, `packages/protocol/tests/...`) or by
directory (`src/auth/`, `src/connection/`, `src/mock/`), so every file below was created or edited
as part of the named task. None changes scope.

## Unplanned paths

- packages/protocol/src/types.ts, src/api.ts, src/index.ts, package.json (`exports` for `./mock`, `@types/jest`), tsconfig.json (jest types, dom lib): T025.
- packages/protocol/tests/api.test.ts, tests/types.test-d.ts: T024.
- packages/protocol/src/connection/ConnectionManager.ts, src/connection/index.ts: T027.
- packages/protocol/tests/connection.test.ts, tests/fakes.ts (FakeTransport, FakeClock): T026.
- packages/protocol/src/mock/index.ts, src/mock/fixture.ts: T029.
- packages/protocol/tests/mock.test.ts: T028.
- apps/client/src/api/select.ts, src/api/ApiProvider.tsx: T031.
- apps/client/tests/api/select.test.ts, tests/api/ApiProvider.test.tsx: T030.
- apps/client/src/auth/sessionStore.ts, guard.ts, SessionProvider.tsx, AuthGate.tsx, apps/client/app/_layout.tsx, apps/client/app/(app)/_layout.tsx, apps/client/app/(app)/index.tsx (moved from app/index.tsx into the protected group): T033. The guard sits in the `(app)` group layout and returns a `Redirect`, so no protected route renders signed out.
- apps/client/tests/auth/guard.test.ts, sessionStore.test.ts, AuthGate.test.tsx, apps/client/tests/app/layout.test.tsx: T032.
- apps/client/app/sign-in.tsx, apps/client/src/i18n/en.json, pl.json: T035 (sign-in, banner and `api.unavailable` messages; pseudo is derived from en).
- apps/client/tests/app/signIn.test.tsx, tests/support/renderWithApi.tsx: T034.
- apps/client/src/components/ConnectionBanner.tsx: T037; its test tests/components/ConnectionBanner.test.tsx: T036.
- scripts/client/bundle-check.mjs, .github/workflows/ci.yml: T039. The CI step also exports the mock build and asserts the check refuses it.
- scripts/tests/bundle-check.test.sh: T038.
- apps/client/jest.config.js, apps/client/package.json, pnpm-lock.yaml: T026/T028 need the protocol tests to run and be covered. A third Jest project `protocol` was added (rootDir `packages/protocol`, `babel-jest` + `babel-preset-expo`, now direct dev dependencies), and `test` runs Jest from the repository root because babel-plugin-istanbul instruments only files under the process cwd. lcov paths are therefore repository paths, and the `SF:` rewrite M1 put in ci.yml is removed.

## Planned but not touched

- data-model.md, contracts/orchestrator-api.md, contracts/transport.md: read as the specification, not edited; the code follows them.

## Deviations and deferrals

- `unauthenticated` to `onSessionLost` (contracts/orchestrator-api.md, *Errors*): `ConnectionManager.onUnauthenticated` reports an `unauthenticated` answer (tested). Wiring it to `AuthService.onSessionLost` belongs to the real client (#1732, #1733), which owns both ends; the mock raises session loss through `MockControl.expireSession`.
- `returnTo` carries the path only: `usePathname()` drops the query, so the guard sends the path. `returnTarget` keeps a query and fragment when one is given, and accepts only a same-origin path that is not sign-in.
- `ConnectionState` before the first open is `offline`; the banner shows offline until the first connect (the mock starts connected).

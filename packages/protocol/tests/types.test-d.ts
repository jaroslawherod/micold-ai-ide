// Compile-time checks (T024): `pnpm --filter @micold/protocol typecheck` fails when one breaks.
import { assertNever, daemonId, locationId, sessionId, type ConnectionState, type LocationId, type SessionId } from "../src/types";

declare function takesLocation(id: LocationId): void;
declare function takesSession(id: SessionId): void;

takesLocation(locationId("l1"));
takesSession(sessionId("s1"));
// @ts-expect-error a session id is not a location id
takesLocation(sessionId("s1"));
// @ts-expect-error a daemon id is not a session id
takesSession(daemonId("d1"));
// @ts-expect-error a plain string is not a branded id
takesSession("s1");

export function describe(state: ConnectionState): string {
  switch (state.kind) {
    case "connecting":
      return "connecting";
    case "connected":
      return "connected";
    case "reconnecting":
      return `reconnecting ${state.attempt}`;
    case "offline":
      return "offline";
    default:
      return assertNever(state);
  }
}

// @ts-expect-error only the reconnecting state carries an attempt
export const bad: ConnectionState = { kind: "connected", attempt: 1 };

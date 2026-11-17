// Reconnect backoff shared by the `ConnectionManager` and the real client (research R6): exponential
// from `baseMs` to the `maxMs` ceiling, then full jitter, so many tabs do not retry in step.
import type { ConnectionOptions } from "../api";

/** Attempts before the state turns `offline` (R6). */
export const DEFAULT_MAX_ATTEMPTS = 8;

/** Milliseconds to wait before attempt number `attempt` (counted from 1). */
export function retryDelay(attempt: number, options: Pick<ConnectionOptions, "baseMs" | "maxMs" | "random">): number {
  const ceiling = Math.min(options.maxMs, options.baseMs * 2 ** (attempt - 1));
  return options.random() * ceiling;
}

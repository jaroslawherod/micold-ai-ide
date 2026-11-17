// The client never shows the server's `message` (ws-protocol.md *Errors*): it maps the typed `code`
// to a catalog key of its own (FR-015). A record over the full code union, so a new code fails the
// type check until it has a translation.
import { ApiError, type ApiErrorCode } from "@micold/protocol";
import type { MessageId } from "../../i18n/catalog";

const KEYS: Record<ApiErrorCode, MessageId> = {
  malformed: "error.malformed",
  unknown_type: "error.unknownType",
  terminal_not_supported: "error.terminalNotSupported",
  not_found: "error.notFound",
  daemon_unreachable: "error.daemonUnreachable",
  timeout: "error.timeout",
  internal: "error.internal",
  project_busy: "error.projectBusy",
  refused: "error.refused",
  unavailable: "error.unavailable",
  unauthenticated: "error.unauthenticated",
};

export function errorMessageId(code: ApiErrorCode): MessageId {
  return KEYS[code];
}

/** The catalog key for whatever a read threw; anything that is not an `ApiError` is `unavailable`. */
export function errorKeyOf(err: unknown): MessageId {
  return errorMessageId(err instanceof ApiError ? err.code : "unavailable");
}

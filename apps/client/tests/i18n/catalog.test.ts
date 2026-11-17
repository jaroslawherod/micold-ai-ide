import { TYPE, parse, type MessageFormatElement } from "@formatjs/icu-messageformat-parser";
import en from "../../src/i18n/en.json";
import pl from "../../src/i18n/pl.json";
import { errorMessageId } from "../../src/features/session/errors";

function argumentNames(elements: MessageFormatElement[], into = new Set<string>()): Set<string> {
  for (const el of elements) {
    if (el.type === TYPE.literal || el.type === TYPE.pound) continue;
    if (el.type === TYPE.tag) {
      argumentNames(el.children, into);
      continue;
    }
    into.add(el.value);
    if (el.type === TYPE.plural || el.type === TYPE.select) {
      for (const option of Object.values(el.options)) argumentNames(option.value, into);
    }
  }
  return into;
}

function pluralCategories(message: string): string[] {
  const out: string[] = [];
  const walk = (els: MessageFormatElement[]) => {
    for (const el of els) {
      if (el.type === TYPE.plural) {
        out.push(...Object.keys(el.options));
        Object.values(el.options).forEach((o) => walk(o.value));
      }
    }
  };
  walk(parse(message));
  return out;
}

describe("message catalogs", () => {
  it("parse as ICU messages", () => {
    for (const message of [...Object.values(en), ...Object.values(pl)]) {
      expect(() => parse(message)).not.toThrow();
    }
  });

  it("have every Polish id in English", () => {
    const missing = Object.keys(pl).filter((id) => !(id in en));
    expect(missing).toEqual([]);
  });

  it("have equal placeholder names in English and Polish for each shared id", () => {
    for (const [id, message] of Object.entries(pl)) {
      const source = (en as Record<string, string>)[id];
      expect(source).toBeDefined();
      expect([...argumentNames(parse(message))].sort()).toEqual(
        [...argumentNames(parse(source as string))].sort(),
      );
    }
  });

  it("give Polish plurals one, few, many and other", () => {
    const plurals = Object.entries(pl).filter(([, m]) => pluralCategories(m).length > 0);
    expect(plurals.length).toBeGreaterThan(0);
    for (const [, message] of plurals) {
      expect(pluralCategories(message).sort()).toEqual(["few", "many", "one", "other"]);
    }
  });

  it("use area.id keys", () => {
    for (const id of Object.keys(en)) expect(id).toMatch(/^[a-z]+(\.[a-zA-Z]+)+$/);
  });

  // FR-015, SC-007: the strings #1732's web client added, each in both languages.
  const NEW_KEYS = [
    "connection.connecting",
    "connection.reconnecting",
    "connection.unreachable.title",
    "connection.unreachable.body",
    "connection.retry",
    "sidebar.sessionUnreachable",
    "session.unreachable",
    "session.refresh",
    "session.content.label",
    "session.content.loading",
    "session.content.empty",
    "session.content.failed",
    "session.content.truncated",
    "session.role.user",
    "session.role.assistant",
    "session.role.system",
    "session.entry.toolCall",
    "session.entry.toolResult",
    "session.entry.status",
  ];

  it("translate every string of the real connection and content views", () => {
    for (const id of NEW_KEYS) {
      expect((en as Record<string, string>)[id]).toBeTruthy();
      expect((pl as Record<string, string>)[id]).toBeTruthy();
      expect((pl as Record<string, string>)[id]).not.toBe((en as Record<string, string>)[id]);
    }
  });

  it("have an error line for every error code, in both languages", () => {
    for (const code of ["malformed", "unknown_type", "terminal_not_supported", "not_found", "daemon_unreachable", "timeout", "internal", "project_busy", "refused", "unavailable", "unauthenticated"] as const) {
      const id = errorMessageId(code);
      expect((en as Record<string, string>)[id]).toBeTruthy();
      expect((pl as Record<string, string>)[id]).toBeTruthy();
    }
  });
});

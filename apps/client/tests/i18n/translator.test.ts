import { createTranslator } from "../../src/i18n/translator";
import en from "../../src/i18n/en.json";

describe("createTranslator", () => {
  it("formats English with plural", () => {
    const t = createTranslator("en").t;
    expect(t("session.count", { count: 1 })).toBe("1 session");
    expect(t("session.count", { count: 5 })).toBe("5 sessions");
  });

  it.each([
    [1, "1 sesja"],
    [2, "2 sesje"],
    [5, "5 sesji"],
    [22, "22 sesje"],
    [12, "12 sesji"],
  ])("uses Polish plural rules for %i", (count, expected) => {
    expect(createTranslator("pl").t("session.count", { count })).toBe(expected);
  });

  it("falls back to English for a message missing in the locale and reports the gap", () => {
    const onMissing = jest.fn();
    const t = createTranslator("pl", { onMissing, catalogs: { en, pl: {} } }).t;
    expect(t("app.title")).toBe(en["app.title"]);
    expect(onMissing).toHaveBeenCalledWith("pl", "app.title");
  });

  it("does not report a gap for English itself", () => {
    const onMissing = jest.fn();
    createTranslator("en", { onMissing }).t("app.title");
    expect(onMissing).not.toHaveBeenCalled();
  });

  it("renders the pseudo-locale from English for every message", () => {
    const t = createTranslator("pseudo").t;
    for (const id of Object.keys(en) as (keyof typeof en)[]) {
      const out = t(id, { count: 3 });
      expect(out.startsWith("[")).toBe(true);
      expect(out.endsWith("]")).toBe(true);
      expect(out).not.toBe(createTranslator("en").t(id, { count: 3 }));
    }
  });

  it("formats dates and relative times by locale, with an English fallback when Intl lacks the feature", () => {
    const { formatRelative, formatDate } = createTranslator("pl");
    expect(formatRelative(-2, "minute")).toMatch(/2/);
    expect(formatDate(new Date(Date.UTC(2026, 0, 5)))).toMatch(/2026/);
    const original = Intl.RelativeTimeFormat;
    // Simulating a browser without the API.
    Object.defineProperty(Intl, "RelativeTimeFormat", { value: undefined, configurable: true });
    try {
      expect(createTranslator("pl").formatRelative(-2, "minute")).toBe("2 minutes ago");
    } finally {
      Object.defineProperty(Intl, "RelativeTimeFormat", { value: original, configurable: true });
    }
  });
});

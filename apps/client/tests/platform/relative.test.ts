// Relative times through the intl wrapper (US5 scenario 4): the unit that reads best, in the locale.
import { relativeParts } from "../../src/platform/intl";
import { createTranslator } from "../../src/i18n/translator";

describe("relative times", () => {
  it("picks seconds, minutes, hours, then days", () => {
    const now = 10_000_000_000;
    expect(relativeParts(now - 30_000, now)).toEqual({ value: -30, unit: "second" });
    expect(relativeParts(now - 5 * 60_000, now)).toEqual({ value: -5, unit: "minute" });
    expect(relativeParts(now - 3 * 3_600_000, now)).toEqual({ value: -3, unit: "hour" });
    expect(relativeParts(now - 2 * 86_400_000, now)).toEqual({ value: -2, unit: "day" });
    expect(relativeParts(now + 60_000, now)).toEqual({ value: 1, unit: "minute" });
  });

  it("formats how long ago in English and Polish, and dates in the locale", () => {
    const now = Date.UTC(2026, 10, 16, 12);
    expect(createTranslator("en").formatSince(now - 5 * 60_000, now)).toBe("5 minutes ago");
    expect(createTranslator("pl").formatSince(now - 5 * 60_000, now)).toBe("5 minut temu");
    expect(createTranslator("pl").formatSince(now - 2 * 60_000, now)).toBe("2 minuty temu");
    expect(createTranslator("en").formatSince(now, now)).toBe("now");
    expect(createTranslator("pl").formatDate(new Date(now))).toMatch(/2026/);
  });
});

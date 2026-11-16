// Locale choice from the browser, guarded: any missing or throwing browser API ends in English.
export const DEFAULT_LOCALE = "en";

export function browserLanguages(): string[] {
  try {
    const nav = typeof navigator === "undefined" ? undefined : navigator;
    const list = nav?.languages?.length ? [...nav.languages] : nav?.language ? [nav.language] : [];
    return list.filter((tag): tag is string => typeof tag === "string" && tag.length > 0);
  } catch {
    return [];
  }
}

/** The supported locale nearest to a language tag: exact, then its primary subtag (`pl-PL` gives `pl`). */
export function nearestLocale(tag: string, supported: readonly string[]): string | undefined {
  const lower = tag.toLowerCase();
  const exact = supported.find((s) => s.toLowerCase() === lower);
  if (exact) return exact;
  const primary = lower.split("-")[0];
  return supported.find((s) => s.toLowerCase() === primary);
}

/** Stored choice if supported, else the browser's language, else English. */
export function chooseLocale(
  supported: readonly string[],
  stored: string | undefined,
  languages: () => string[] = browserLanguages,
): string {
  if (stored && supported.includes(stored)) return stored;
  for (const tag of languages()) {
    const found = nearestLocale(tag, supported);
    if (found) return found;
  }
  return DEFAULT_LOCALE;
}

export type RelativeUnit = "second" | "minute" | "hour" | "day";

/** A relative time (`-2, "minute"`) in `locale`; plain English when `Intl.RelativeTimeFormat` is missing. */
export function formatRelativeTime(locale: string, value: number, unit: RelativeUnit): string {
  try {
    if (typeof Intl !== "undefined" && typeof Intl.RelativeTimeFormat === "function") {
      return new Intl.RelativeTimeFormat(locale, { numeric: "auto" }).format(value, unit);
    }
  } catch {
    // fall through to English
  }
  const n = Math.abs(value);
  const noun = n === 1 ? unit : `${unit}s`;
  return value < 0 ? `${n} ${noun} ago` : `in ${n} ${noun}`;
}

/** A date in `locale`; ISO date when `Intl.DateTimeFormat` is missing. */
export function formatDateTime(locale: string, date: Date): string {
  try {
    if (typeof Intl !== "undefined" && typeof Intl.DateTimeFormat === "function") {
      return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeZone: "UTC" }).format(date);
    }
  } catch {
    // fall through to ISO
  }
  return date.toISOString().slice(0, 10);
}

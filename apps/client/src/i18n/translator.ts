// The one way a component gets text (FR-014): `t(id, values)` formats the ICU message of the
// chosen locale, falls back to English for a missing message and reports the gap (FR-015).
import { createIntl, createIntlCache } from "@formatjs/intl";
import { formatDateTime, formatRelativeTime, type RelativeUnit } from "../platform/intl";
import { PSEUDO_LOCALE, SOURCE_LOCALE, catalogs as defaultCatalogs, type Catalog, type MessageId } from "./catalog";
import { pseudoCatalog } from "./pseudo";

export type Values = Record<string, string | number | boolean | Date | null | undefined>;

export interface Translator {
  locale: string;
  t: (id: MessageId, values?: Values) => string;
  formatRelative: (value: number, unit: RelativeUnit) => string;
  formatDate: (date: Date) => string;
}

export interface TranslatorOptions {
  /** Called once per id missing in a non-English locale (development reporting). */
  onMissing?: (locale: string, id: string) => void;
  catalogs?: Readonly<Record<string, Catalog>>;
}

const cache = createIntlCache();

export function createTranslator(locale: string, options: TranslatorOptions = {}): Translator {
  const catalogs = options.catalogs ?? defaultCatalogs;
  const english = catalogs[SOURCE_LOCALE] ?? {};
  const isPseudo = locale === PSEUDO_LOCALE;
  const chosen = isPseudo ? pseudoCatalog(english) : (catalogs[locale] ?? {});
  const formatLocale = isPseudo ? SOURCE_LOCALE : locale;
  const reported = new Set<string>();
  const intl = createIntl(
    { locale: formatLocale, defaultLocale: SOURCE_LOCALE, messages: { ...english, ...chosen }, onError: () => undefined },
    cache,
  );
  return {
    locale,
    t: (id, values) => {
      if (locale !== SOURCE_LOCALE && !(id in chosen) && !reported.has(id)) {
        reported.add(id);
        options.onMissing?.(locale, id);
      }
      return intl.formatMessage({ id }, values as Record<string, string | number | boolean | Date | null | undefined>);
    },
    formatRelative: (value, unit) => formatRelativeTime(formatLocale, value, unit),
    formatDate: (date) => formatDateTime(formatLocale, date),
  };
}

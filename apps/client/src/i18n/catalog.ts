import en from "./en.json";
import pl from "./pl.json";

export type MessageId = keyof typeof en;
export type Catalog = Readonly<Record<string, string>>;

export const SOURCE_LOCALE = "en";
export const REAL_LOCALES = ["en", "pl"] as const;
export const PSEUDO_LOCALE = "pseudo";
export type RealLocale = (typeof REAL_LOCALES)[number];

export const catalogs: Readonly<Record<string, Catalog>> = { en, pl };

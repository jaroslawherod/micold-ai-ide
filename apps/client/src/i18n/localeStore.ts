// The chosen locale (FR-015, R9): a zustand store whose only persisted value is the locale code,
// under `localStorage` key `micold.locale` -- a preference, not a token. Storage may be absent or
// throw (private mode, blocked cookies); the choice then lasts for the page only.
import { createStore } from "zustand/vanilla";
import { useStore } from "zustand";
import { browserLanguages, chooseLocale } from "../platform/intl";
import { PSEUDO_LOCALE, REAL_LOCALES } from "./catalog";

export const LOCALE_STORAGE_KEY = "micold.locale";

export interface KeyValueStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

export interface LocaleState {
  locale: string;
  available: readonly string[];
  setLocale: (locale: string) => void;
}

export interface LocaleStoreOptions {
  storage: KeyValueStorage | undefined;
  languages?: () => string[];
  /** The pseudo-locale is offered only in dev and mock builds (R9). */
  pseudo?: boolean;
}

function safeRead(storage: KeyValueStorage | undefined): string | undefined {
  try {
    return storage?.getItem(LOCALE_STORAGE_KEY) ?? undefined;
  } catch {
    return undefined;
  }
}

export function createLocaleStore({ storage, languages = browserLanguages, pseudo = false }: LocaleStoreOptions) {
  const available: readonly string[] = pseudo ? [...REAL_LOCALES, PSEUDO_LOCALE] : REAL_LOCALES;
  return createStore<LocaleState>((set) => ({
    locale: chooseLocale(available, safeRead(storage), languages),
    available,
    setLocale: (locale) => {
      if (!available.includes(locale)) return;
      set({ locale });
      try {
        storage?.setItem(LOCALE_STORAGE_KEY, locale);
      } catch {
        // the choice lasts for this page only
      }
    },
  }));
}

function browserStorage(): KeyValueStorage | undefined {
  try {
    return typeof window !== "undefined" ? window.localStorage : undefined;
  } catch {
    return undefined;
  }
}

/** Pseudo-locale on in dev and in mock builds, never in a production export. */
export function pseudoEnabled(): boolean {
  return process.env.EXPO_PUBLIC_API_MODE === "mock" || process.env.NODE_ENV === "development";
}

export const localeStore = createLocaleStore({ storage: browserStorage(), pseudo: pseudoEnabled() });

export function useLocaleStore<T>(select: (state: LocaleState) => T): T {
  return useStore(localeStore, select);
}

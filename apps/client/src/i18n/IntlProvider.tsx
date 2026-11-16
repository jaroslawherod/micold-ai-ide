import React, { createContext, useContext, useEffect, useMemo, type ReactNode } from "react";
import { PSEUDO_LOCALE } from "./catalog";
import { useLocaleStore } from "./localeStore";
import { createTranslator, type Translator } from "./translator";

const TranslatorContext = createContext<Translator | undefined>(undefined);

function reportMissing(locale: string, id: string): void {
  if (process.env.NODE_ENV !== "production") {
    console.warn(`[i18n] message "${id}" is missing in locale "${locale}"; showing English`);
  }
}

/** Provides the translator for the chosen locale; switching the locale re-renders without a reload. */
export function IntlProvider({ children }: { children: ReactNode }) {
  const locale = useLocaleStore((s) => s.locale);
  const translator = useMemo(() => createTranslator(locale, { onMissing: reportMissing }), [locale]);
  useEffect(() => {
    if (typeof document !== "undefined") {
      document.documentElement.lang = locale === PSEUDO_LOCALE ? "en" : locale;
    }
  }, [locale]);
  return <TranslatorContext.Provider value={translator}>{children}</TranslatorContext.Provider>;
}

export function useTranslator(): Translator {
  const translator = useContext(TranslatorContext);
  if (!translator) throw new Error("useTranslator needs an IntlProvider above it");
  return translator;
}

/** `t(id, values)`: the only way a component gets user-visible text (FR-014). */
export function useT(): Translator["t"] {
  return useTranslator().t;
}

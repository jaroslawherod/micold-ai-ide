import { LOCALE_STORAGE_KEY, createLocaleStore } from "../../src/i18n/localeStore";

function fakeStorage(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return {
    data,
    getItem: (k: string) => data[k] ?? null,
    setItem: (k: string, v: string) => {
      data[k] = v;
    },
  };
}

describe("locale store", () => {
  const nav = (languages: string[]) => () => languages;

  it("defaults to the browser language, nearest language first", () => {
    expect(createLocaleStore({ storage: fakeStorage(), languages: nav(["pl-PL", "en"]) }).getState().locale).toBe("pl");
    expect(createLocaleStore({ storage: fakeStorage(), languages: nav(["de", "pl"]) }).getState().locale).toBe("pl");
  });

  it("falls back to English for an unsupported browser language", () => {
    expect(createLocaleStore({ storage: fakeStorage(), languages: nav(["de"]) }).getState().locale).toBe("en");
  });

  it("prefers a stored choice over the browser language", () => {
    const storage = fakeStorage({ [LOCALE_STORAGE_KEY]: "en" });
    expect(createLocaleStore({ storage, languages: nav(["pl"]) }).getState().locale).toBe("en");
  });

  it("ignores a stored value that is not a supported locale", () => {
    const storage = fakeStorage({ [LOCALE_STORAGE_KEY]: "xx" });
    expect(createLocaleStore({ storage, languages: nav(["pl"]) }).getState().locale).toBe("pl");
  });

  it("stores only the locale code under micold.locale", () => {
    const storage = fakeStorage();
    const store = createLocaleStore({ storage, languages: nav(["en"]) });
    store.getState().setLocale("pl");
    expect(store.getState().locale).toBe("pl");
    expect(storage.data).toEqual({ "micold.locale": "pl" });
  });

  it("works when storage throws on read and on write", () => {
    const storage = {
      getItem: () => {
        throw new Error("denied");
      },
      setItem: () => {
        throw new Error("denied");
      },
    };
    const store = createLocaleStore({ storage, languages: nav(["pl"]) });
    expect(store.getState().locale).toBe("pl");
    expect(() => store.getState().setLocale("en")).not.toThrow();
    expect(store.getState().locale).toBe("en");
  });

  it("works with no storage at all", () => {
    const store = createLocaleStore({ storage: undefined, languages: nav(["en"]) });
    store.getState().setLocale("pl");
    expect(store.getState().locale).toBe("pl");
  });

  it("offers the pseudo-locale only where enabled", () => {
    expect(createLocaleStore({ storage: fakeStorage(), languages: nav(["en"]), pseudo: false }).getState().available).toEqual(["en", "pl"]);
    expect(createLocaleStore({ storage: fakeStorage(), languages: nav(["en"]), pseudo: true }).getState().available).toEqual(["en", "pl", "pseudo"]);
  });
});

import { chooseLocale } from "../../src/platform/intl";
import { readColorScheme, subscribeColorScheme } from "../../src/platform/colorScheme";
import { loadFonts } from "../../src/platform/fonts";

describe("platform degradation", () => {
  const originalMatchMedia = window.matchMedia;
  afterEach(() => {
    window.matchMedia = originalMatchMedia;
    jest.restoreAllMocks();
  });

  it("falls back to light when matchMedia is missing", () => {
    // @ts-expect-error simulating a browser without the API
    delete window.matchMedia;
    expect(readColorScheme()).toBe("light");
    const unsubscribe = subscribeColorScheme(() => undefined);
    expect(() => unsubscribe()).not.toThrow();
  });

  it("falls back to light when matchMedia throws", () => {
    window.matchMedia = () => {
      throw new Error("blocked");
    };
    expect(readColorScheme()).toBe("light");
  });

  it("reads dark from matchMedia and notifies on change", () => {
    let listener: (() => void) | undefined;
    const mql = {
      matches: true,
      addEventListener: (_: string, l: () => void) => {
        listener = l;
      },
      removeEventListener: jest.fn(),
    };
    window.matchMedia = (() => mql) as unknown as typeof window.matchMedia;
    expect(readColorScheme()).toBe("dark");
    const seen = jest.fn();
    const unsubscribe = subscribeColorScheme(seen);
    mql.matches = false;
    listener?.();
    expect(seen).toHaveBeenCalledTimes(1);
    unsubscribe();
    expect(mql.removeEventListener).toHaveBeenCalled();
  });

  it("falls back to English when the browser language is unavailable", () => {
    const supported = ["en", "pl"];
    jest.spyOn(window.navigator, "languages", "get").mockReturnValue([]);
    jest.spyOn(window.navigator, "language", "get").mockReturnValue(undefined as unknown as string);
    expect(chooseLocale(supported, undefined)).toBe("en");
  });

  it("falls back to English when Intl.Locale is missing", () => {
    const original = Intl.Locale;
    // Simulating a browser without the API.
    Object.defineProperty(Intl, "Locale", { value: undefined, configurable: true });
    try {
      jest.spyOn(window.navigator, "languages", "get").mockReturnValue(["pl-PL"]);
      expect(chooseLocale(["en", "pl"], undefined)).toBe("pl");
      jest.spyOn(window.navigator, "languages", "get").mockReturnValue(["de"]);
      expect(chooseLocale(["en", "pl"], undefined)).toBe("en");
    } finally {
      Object.defineProperty(Intl, "Locale", { value: original, configurable: true });
    }
  });

  it("keeps system fonts when the font-loading API is missing or fails", async () => {
    await expect(loadFonts(undefined)).resolves.toBe(false);
    await expect(
      loadFonts(() => Promise.reject(new Error("no FontFace"))),
    ).resolves.toBe(false);
    await expect(loadFonts(() => Promise.resolve())).resolves.toBe(true);
  });
});

import { buildTheme } from "../../src/theme/buildTheme";
import { fontAssets, fontFamilies, monoFamily, sansFamily } from "../../src/platform/fonts";

describe("fonts", () => {
  it("registers Roboto for text and Fira Mono for code", () => {
    expect(sansFamily).toMatch(/Roboto/);
    expect(monoFamily).toMatch(/Fira ?Mono/i);
    expect(fontFamilies).toEqual(expect.arrayContaining([sansFamily, monoFamily]));
  });

  it("bundles a font file for every registered weight, none fetched from a URL", () => {
    const names = Object.keys(fontAssets);
    expect(names.some((n) => n.startsWith("Roboto"))).toBe(true);
    expect(names.some((n) => n.startsWith("FiraMono"))).toBe(true);
    for (const asset of Object.values(fontAssets)) {
      expect(typeof asset === "string" ? asset : "asset").not.toMatch(/^https?:/);
    }
  });

  it("puts the sans family on every Paper type role", () => {
    const theme = buildTheme("light");
    expect(theme.fonts.bodyMedium.fontFamily).toBe(sansFamily);
    expect(theme.fonts.titleLarge.fontFamily).toMatch(/^Roboto/);
  });
});

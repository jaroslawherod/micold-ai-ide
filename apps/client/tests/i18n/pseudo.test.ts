import { pseudoLocalize } from "../../src/i18n/pseudo";

describe("pseudoLocalize", () => {
  it("accents letters, brackets the text and pads it by at least 40 percent", () => {
    const out = pseudoLocalize("Language");
    expect(out.startsWith("[")).toBe(true);
    expect(out.endsWith("]")).toBe(true);
    expect(out).not.toMatch(/[A-Za-z]{3}/);
    expect(out.length).toBeGreaterThanOrEqual(Math.ceil("Language".length * 1.4) + 2);
  });

  it("leaves placeholders and plural keywords intact so the message still formats", () => {
    const out = pseudoLocalize("{count, plural, one {# session} other {# sessions of {name}}}");
    expect(out).toContain("{count,plural,");
    expect(out).toContain("one{");
    expect(out).toContain("{name}");
    expect(out).toContain("#");
    expect(out).not.toMatch(/session\b/);
  });

  it("is deterministic", () => {
    expect(pseudoLocalize("Hello {name}")).toBe(pseudoLocalize("Hello {name}"));
  });
});

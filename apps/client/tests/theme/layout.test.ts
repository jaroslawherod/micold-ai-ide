import { BREAKPOINT, insetStart, isWide, marginInline, paddingInline } from "../../src/theme/layout";

describe("layout helpers", () => {
  it("emit logical properties only", () => {
    const all = [insetStart(4), marginInline(8), paddingInline(2)];
    for (const style of all) {
      for (const key of Object.keys(style)) expect(key).not.toMatch(/left|right/i);
    }
    expect(insetStart(4)).toEqual({ start: 4 });
    expect(marginInline(8)).toEqual({ marginStart: 8, marginEnd: 8 });
    expect(paddingInline(2)).toEqual({ paddingStart: 2, paddingEnd: 2 });
  });

  it("treats the 600 breakpoint as the first wide width", () => {
    expect(BREAKPOINT).toBe(600);
    expect(isWide(599)).toBe(false);
    expect(isWide(600)).toBe(true);
  });
});

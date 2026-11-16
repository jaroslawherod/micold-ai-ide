import { colors, shape } from "../../src/theme/tokens.generated";
import { buildTheme, UnmappedRoleError } from "../../src/theme/buildTheme";

const schemes = ["light", "dark"] as const;

describe("buildTheme", () => {
  describe.each(schemes)("%s scheme", (scheme) => {
    const theme = buildTheme(scheme);

    it("is flagged dark only for the dark scheme", () => {
      expect(theme.dark).toBe(scheme === "dark");
    });

    it("equals the token value for every Paper role of the same name", () => {
      const paperRoles = Object.keys(colors[scheme]).filter((role) => role in theme.colors);
      // The roles Paper names must be a real overlap, not an empty loop.
      expect(paperRoles.length).toBeGreaterThan(25);
      for (const role of paperRoles) {
        expect((theme.colors as Record<string, unknown>)[role]).toBe(
          (colors[scheme] as Record<string, string>)[role],
        );
      }
    });

    it("carries the roles Paper has no name for on theme.extraColors", () => {
      expect(theme.extraColors.diffAdded).toBe(colors[scheme].diffAdded);
      expect(theme.extraColors.success).toBe(colors[scheme].success);
      expect(theme.extraColors.surfaceContainerHigh).toBe(colors[scheme].surfaceContainerHigh);
    });

    it("takes elevation surfaces from the token levels", () => {
      expect(theme.colors.elevation.level1).toBe(colors[scheme].surfaceContainerLow);
      expect(theme.colors.elevation.level5).toBe(colors[scheme].surfaceContainerHighest);
    });
  });

  it("uses the medium corner of the token shape scale as Paper roundness", () => {
    expect(buildTheme("light").roundness).toBe(shape.medium / 4);
  });

  it("fails on a token role that has no mapping", () => {
    const withUnknown = { ...colors.light, brandNew: "#123456" };
    expect(() => buildTheme("light", withUnknown)).toThrow(UnmappedRoleError);
    expect(() => buildTheme("light", withUnknown)).toThrow(/brandNew/);
  });
});

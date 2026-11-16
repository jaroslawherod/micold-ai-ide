// The Paper MD3 theme, built from the generated tokens (FR-003, R4). `tokens.generated.ts` is
// imported, never copied: each token colour role maps to the Paper role of the same name, the roles
// Paper has no name for ride on `extraColors`, and a role in neither list fails the build of the
// theme, so a new token cannot be dropped silently.
import { MD3DarkTheme, MD3LightTheme, configureFonts, type MD3Theme } from "react-native-paper";
import { sansFamilyForWeight } from "../platform/fonts";
import { colors, elevation, shape, state, typeScale } from "./tokens.generated";

export type Scheme = "light" | "dark";

/** Token roles Paper's MD3 theme has no field for; read through `theme.extraColors`. */
export const EXTENSION_ROLES = [
  "surfaceDim",
  "surfaceBright",
  "surfaceContainerLowest",
  "surfaceContainerLow",
  "surfaceContainer",
  "surfaceContainerHigh",
  "surfaceContainerHighest",
  "diffAdded",
  "diffRemoved",
  "warning",
  "onWarning",
  "warningContainer",
  "onWarningContainer",
  "success",
  "onSuccess",
  "successContainer",
  "onSuccessContainer",
] as const;

export type ExtensionRole = (typeof EXTENSION_ROLES)[number];
export type AppTheme = MD3Theme & { extraColors: Record<ExtensionRole, string> };

export class UnmappedRoleError extends Error {
  constructor(role: string) {
    super(`token colour role "${role}" has no Paper role and is not an extension role`);
    this.name = "UnmappedRoleError";
  }
}

const PAPER_ROLES = new Set(Object.keys(MD3LightTheme.colors).filter((role) => role !== "elevation"));
const EXTENSIONS = new Set<string>(EXTENSION_ROLES);

function withAlpha(hex: string, alpha: number): string {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
}

function buildFonts() {
  const config: Record<string, object> = {};
  for (const [role, t] of Object.entries(typeScale)) {
    config[role] = {
      fontFamily: sansFamilyForWeight(t.weight),
      fontSize: t.size,
      lineHeight: t.lineHeight,
      fontWeight: "normal",
      letterSpacing: 0,
    };
  }
  return configureFonts({ config, isV3: true });
}

export function buildTheme(
  scheme: Scheme,
  source: Readonly<Record<string, string>> = colors[scheme],
): AppTheme {
  const base = scheme === "dark" ? MD3DarkTheme : MD3LightTheme;
  const paper: Record<string, string> = {};
  const extra: Record<string, string> = {};
  for (const [role, value] of Object.entries(source)) {
    if (PAPER_ROLES.has(role)) paper[role] = value;
    else if (EXTENSIONS.has(role)) extra[role] = value;
    else throw new UnmappedRoleError(role);
  }
  const merged = { ...base.colors, ...paper };
  const level = (i: number) => {
    const surface = elevation.levels[i]?.surface as string;
    return (source[surface] ?? merged.surface) as string;
  };
  return {
    ...base,
    dark: scheme === "dark",
    roundness: shape.medium / 4,
    fonts: buildFonts(),
    colors: {
      ...merged,
      surfaceDisabled: withAlpha(merged.onSurface, state.opacity.disabledContainer),
      onSurfaceDisabled: withAlpha(merged.onSurface, state.opacity.disabledContent),
      backdrop: withAlpha(merged.scrim, elevation.scrimAlpha),
      elevation: {
        level0: level(0),
        level1: level(1),
        level2: level(2),
        level3: level(3),
        level4: level(4),
        level5: level(5),
      },
    },
    extraColors: extra as Record<ExtensionRole, string>,
  };
}

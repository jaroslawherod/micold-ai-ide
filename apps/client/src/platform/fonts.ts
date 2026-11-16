// Roboto (text) and Fira Mono (code) ship inside the export (FR-003, US4 scenario 3): the files come
// from the `@expo-google-fonts` packages through the bundler, never from a CDN. Loading goes through
// `loadFonts` so a browser without the font-loading API keeps the system fonts (Principle VI).
import * as Font from "expo-font";

/* eslint-disable @typescript-eslint/no-require-imports */
export const fontAssets = {
  Roboto_400Regular: require("@expo-google-fonts/roboto/400Regular/Roboto_400Regular.ttf"),
  Roboto_500Medium: require("@expo-google-fonts/roboto/500Medium/Roboto_500Medium.ttf"),
  Roboto_700Bold: require("@expo-google-fonts/roboto/700Bold/Roboto_700Bold.ttf"),
  FiraMono_400Regular: require("@expo-google-fonts/fira-mono/400Regular/FiraMono_400Regular.ttf"),
  FiraMono_500Medium: require("@expo-google-fonts/fira-mono/500Medium/FiraMono_500Medium.ttf"),
  FiraMono_700Bold: require("@expo-google-fonts/fira-mono/700Bold/FiraMono_700Bold.ttf"),
} as const;
/* eslint-enable @typescript-eslint/no-require-imports */

export const sansFamily = "Roboto_400Regular";
export const sansMediumFamily = "Roboto_500Medium";
export const sansBoldFamily = "Roboto_700Bold";
export const monoFamily = "FiraMono_400Regular";

export const fontFamilies: readonly string[] = Object.keys(fontAssets);

/** The family for a numeric token weight: 400 regular, 500 medium, 700 and above bold. */
export function sansFamilyForWeight(weight: number): string {
  if (weight >= 700) return sansBoldFamily;
  if (weight >= 500) return sansMediumFamily;
  return sansFamily;
}

export type FontLoader = (assets: typeof fontAssets) => Promise<unknown>;

/** Load the bundled fonts through `loader`; false (system fonts stay) when there is none or it fails. */
export async function loadFonts(loader: FontLoader | undefined): Promise<boolean> {
  if (!loader) return false;
  try {
    await loader(fontAssets);
    return true;
  } catch {
    return false;
  }
}

/** The loader of this browser, or undefined where the font-loading API is absent. */
function browserLoader(): FontLoader | undefined {
  const hasFontApi =
    typeof document !== "undefined" && typeof (document as { fonts?: unknown }).fonts !== "undefined";
  return hasFontApi ? (assets) => Font.loadAsync(assets) : undefined;
}

export function loadBundledFonts(): Promise<boolean> {
  return loadFonts(browserLoader());
}

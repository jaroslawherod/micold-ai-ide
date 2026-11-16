import { useSyncExternalStore } from "react";
import { readColorScheme, subscribeColorScheme, type ColorScheme } from "../platform/colorScheme";

/** The system colour scheme, following changes while the page is open (US4 scenario 2). */
export function useColorScheme(): ColorScheme {
  return useSyncExternalStore(subscribeColorScheme, readColorScheme, () => "light");
}

// The system colour scheme through `matchMedia`, guarded: a browser without it, or one whose call
// throws, gets the light theme (Principle VI, US4 scenario 2).
export type ColorScheme = "light" | "dark";

const QUERY = "(prefers-color-scheme: dark)";

function query(): MediaQueryList | undefined {
  try {
    return typeof window !== "undefined" && typeof window.matchMedia === "function"
      ? window.matchMedia(QUERY)
      : undefined;
  } catch {
    return undefined;
  }
}

export function readColorScheme(): ColorScheme {
  return query()?.matches ? "dark" : "light";
}

/** Call `onChange` when the system scheme flips; returns the unsubscribe. */
export function subscribeColorScheme(onChange: () => void): () => void {
  const mql = query();
  if (!mql || typeof mql.addEventListener !== "function") return () => undefined;
  mql.addEventListener("change", onChange);
  return () => mql.removeEventListener("change", onChange);
}

// Route guard (FR-007, FR-008): every route but sign-in needs a session; the asked-for route rides
// along as `returnTo` and is honoured only when it is a same-origin app path.
export const SIGN_IN_PATH = "/sign-in";

/** Where to send a visitor, or `null` to let the route render. */
export function guardRedirect(pathname: string, signedIn: boolean): string | null {
  if (signedIn || pathname === SIGN_IN_PATH) return null;
  return pathname === "/" ? SIGN_IN_PATH : `${SIGN_IN_PATH}?returnTo=${encodeURIComponent(pathname)}`;
}

/** The route to open after sign-in: the asked-for one if safe, else home. */
export function returnTarget(returnTo: string | string[] | undefined): string {
  const raw = Array.isArray(returnTo) ? returnTo[0] : returnTo;
  if (!raw || !raw.startsWith("/") || raw.startsWith("//") || raw.includes("\\")) return "/";
  if (raw === SIGN_IN_PATH || raw.startsWith(`${SIGN_IN_PATH}?`)) return "/";
  return raw;
}

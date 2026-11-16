// Route guard (FR-007, FR-008): every route but sign-in needs a session; the asked-for route rides
// along as `returnTo` and is honoured only when it is a same-origin app path.
export const SIGN_IN_PATH = "/sign-in";

const BASE = "http://micold.invalid";

/** Whether `pathname` is the sign-in page, ignoring case, percent-escapes and a trailing slash. */
export function isSignInPath(pathname: string): boolean {
  let decoded: string;
  try {
    decoded = decodeURIComponent(pathname);
  } catch {
    return false;
  }
  return (decoded.replace(/\/+$/, "") || "/").toLowerCase() === SIGN_IN_PATH;
}

/** Where to send a visitor, or `null` to let the route render. */
export function guardRedirect(pathname: string, signedIn: boolean): string | null {
  if (signedIn || isSignInPath(pathname)) return null;
  return pathname === "/" ? SIGN_IN_PATH : `${SIGN_IN_PATH}?returnTo=${encodeURIComponent(pathname)}`;
}

/** The route to open after sign-in (path, query and fragment) if it is safe, else home. */
export function returnTarget(returnTo: string | string[] | undefined): string {
  const raw = Array.isArray(returnTo) ? returnTo[0] : returnTo;
  // eslint-disable-next-line no-control-regex
  if (!raw || !raw.startsWith("/") || /[\u0000-\u001f\u007f\\]/.test(raw)) return "/";
  let url: URL;
  try {
    url = new URL(raw, BASE);
  } catch {
    return "/";
  }
  if (url.origin !== BASE || raw.startsWith("//")) return "/";
  if (isSignInPath(url.pathname) || /^\/%2f/i.test(url.pathname)) return "/";
  return `${url.pathname}${url.search}${url.hash}`;
}

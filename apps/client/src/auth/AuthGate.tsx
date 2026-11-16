// Applies the route guard in the `(app)` group layout: a signed-out visitor is redirected before
// any protected route renders, and an expired session redirects with the open route as `returnTo`.
import React, { type ReactNode } from "react";
import { Redirect, usePathname } from "expo-router";
import { guardRedirect } from "./guard";
import { useSession } from "./SessionProvider";

export function AuthGate({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const signedIn = useSession((s) => s.session !== null);
  const target = guardRedirect(pathname, signedIn);
  if (target) return <Redirect href={target as never} />;
  return <>{children}</>;
}

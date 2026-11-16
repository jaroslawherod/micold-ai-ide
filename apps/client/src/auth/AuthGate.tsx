// Applies the route guard to whatever the router renders below it.
import React, { useEffect, type ReactNode } from "react";
import { usePathname, useRouter } from "expo-router";
import { guardRedirect } from "./guard";
import { useSession } from "./SessionProvider";

export function AuthGate({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const signedIn = useSession((s) => s.session !== null);
  const target = guardRedirect(pathname, signedIn);
  useEffect(() => {
    if (target) router.replace(target);
  }, [target, router]);
  return <>{children}</>;
}

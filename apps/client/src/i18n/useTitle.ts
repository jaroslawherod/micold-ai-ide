import { useEffect } from "react";
import { useT } from "./IntlProvider";
import type { MessageId } from "./catalog";

/** Sets the page title from the catalog (FR-014); follows the locale. */
export function useTitle(id: MessageId): void {
  const t = useT();
  const title = t(id);
  useEffect(() => {
    if (typeof document !== "undefined") document.title = title;
  }, [title]);
}

/** @jest-environment jsdom */
import { createElement } from "react";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { IntlProvider } from "../../src/i18n/IntlProvider";
import { localeStore } from "../../src/i18n/localeStore";
import { useTitle } from "../../src/i18n/useTitle";

function Page() {
  useTitle("app.title");
  return null;
}

describe("useTitle", () => {
  it("sets document.title from the catalog and follows the locale", async () => {
    (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
    const root = createRoot(document.createElement("div"));
    await act(async () => root.render(createElement(IntlProvider, null, createElement(Page))));
    expect(document.title).toBe("Micold");
    await act(async () => localeStore.getState().setLocale("pl"));
    expect(document.documentElement.lang).toBe("pl");
    await act(async () => root.unmount());
  });
});

// Renders a screen inside the app's providers over a fresh mock API (component tests only).
import React, { type ReactElement } from "react";
import { render, screen } from "@testing-library/react-native";
import { PaperProvider } from "react-native-paper";
import { createMockApi } from "@micold/protocol/mock";
import { ApiProvider } from "../../src/api/ApiProvider";
import { SessionProvider, useSession } from "../../src/auth/SessionProvider";
import { IntlProvider } from "../../src/i18n/IntlProvider";
import { buildTheme } from "../../src/theme/buildTheme";

export const sessionProbe: { signedIn?: boolean } = {};
function SessionSpy() {
  sessionProbe.signedIn = useSession((s) => s.session !== null);
  return null;
}

export function renderWithApi(ui: ReactElement, mock = createMockApi()) {
  const wrap = (inner: ReactElement) => (
    <PaperProvider theme={buildTheme("light")}>
      <IntlProvider>
        <ApiProvider api={mock.api}>
          <SessionProvider>
            <SessionSpy />
            {inner}
          </SessionProvider>
        </ApiProvider>
      </IntlProvider>
    </PaperProvider>
  );
  const view = render(wrap(ui));
  /** Renders `next` in place of `ui`, keeping the providers and the mock. */
  const rerender = (next: ReactElement) => screen.rerender(wrap(next));
  return { mock, view, rerender };
}

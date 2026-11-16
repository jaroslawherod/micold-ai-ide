// Renders a screen inside the app's providers over a fresh mock API (component tests only).
import React, { type ReactElement } from "react";
import { render } from "@testing-library/react-native";
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
  const view = render(
    <PaperProvider theme={buildTheme("light")}>
      <IntlProvider>
        <ApiProvider api={mock.api}>
          <SessionProvider>
            <SessionSpy />
            {ui}
          </SessionProvider>
        </ApiProvider>
      </IntlProvider>
    </PaperProvider>,
  );
  return { mock, view };
}

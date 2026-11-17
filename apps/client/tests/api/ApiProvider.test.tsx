import React from "react";
import { render, screen } from "@testing-library/react-native";
import { Text } from "react-native";
import { ApiProvider } from "../../src/api/ApiProvider";
import * as select from "../../src/api/select";
import { IntlProvider } from "../../src/i18n/IntlProvider";
import { localeStore } from "../../src/i18n/localeStore";

const mode = process.env.EXPO_PUBLIC_API_MODE;
afterEach(() => {
  jest.restoreAllMocks();
  process.env.EXPO_PUBLIC_API_MODE = mode;
  localeStore.getState().setLocale("en");
});

// The one way a build has no client: choosing it throws (a socket the browser refuses to make).
async function mountReal(locale: "en" | "pl") {
  jest.spyOn(select, "selectApi").mockImplementation(() => {
    throw new Error("no client");
  });
  process.env.EXPO_PUBLIC_API_MODE = "real";
  delete (globalThis as { __micoldMock?: unknown }).__micoldMock;
  localeStore.getState().setLocale(locale);
  await render(
    <IntlProvider>
      <ApiProvider>
        <Text>child</Text>
      </ApiProvider>
    </IntlProvider>,
  );
}

describe("ApiProvider when the client cannot be created", () => {
  it("shows that the orchestrator is unavailable, renders no child and installs no mock hook", async () => {
    await mountReal("en");
    expect(screen.getByRole("alert")).toHaveTextContent("The orchestrator connection is not available in this build.");
    expect(screen.queryByText("child")).toBeNull();
    expect((globalThis as { __micoldMock?: unknown }).__micoldMock).toBeUndefined();
  });

  it("says so in Polish", async () => {
    await mountReal("pl");
    expect(screen.getByRole("alert")).toHaveTextContent("Połączenie z orkiestratorem nie jest dostępne w tej wersji.");
  });
});

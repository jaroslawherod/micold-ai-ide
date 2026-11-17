import React from "react";
import { render, screen } from "@testing-library/react-native";
import { Text } from "react-native";
import { ApiProvider } from "../../src/api/ApiProvider";
import { IntlProvider } from "../../src/i18n/IntlProvider";
import { localeStore } from "../../src/i18n/localeStore";

const mode = process.env.EXPO_PUBLIC_API_MODE;
afterEach(() => {
  process.env.EXPO_PUBLIC_API_MODE = mode;
  localeStore.getState().setLocale("en");
});

async function mountReal(locale: "en" | "pl") {
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

describe("ApiProvider without the mock", () => {
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

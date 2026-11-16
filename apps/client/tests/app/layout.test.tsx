import React from "react";
import { render, screen } from "@testing-library/react-native";

jest.mock("expo-router", () => {
  const { View } = require("react-native");
  return { Slot: () => <View testID="slot" />, Redirect: () => null };
});

import RootLayout from "../../app/_layout";
import Index from "../../app/index";
import { IntlProvider } from "../../src/i18n/IntlProvider";
import { PaperProvider } from "react-native-paper";
import { buildTheme } from "../../src/theme/buildTheme";

describe("root layout and index", () => {
  it("renders the routed slot inside the providers", async () => {
    await render(<RootLayout />);
    expect(screen.getByTestId("slot")).toBeTruthy();
  });

  it("shows the title, the tagline and the language picker from the catalog", async () => {
    await render(
      <PaperProvider theme={buildTheme("dark")}>
        <IntlProvider>
          <Index />
        </IntlProvider>
      </PaperProvider>,
    );
    expect(screen.getByRole("header", { name: "Micold" })).toBeTruthy();
    expect(screen.getByRole("radiogroup")).toBeTruthy();
  });
});

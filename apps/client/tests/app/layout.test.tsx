import React from "react";
import { AccessibilityInfo } from "react-native";
import { act, render, screen } from "@testing-library/react-native";

jest.mock("expo-router", () => {
  const { View } = require("react-native");
  return {
    Slot: () => <View testID="slot" />,
    Redirect: () => null,
    usePathname: () => "/sign-in",
    useRouter: () => ({ replace: jest.fn() }),
  };
});

import RootLayout from "../../app/_layout";
import Index from "../../app/(app)/index";
import ProtectedLayout from "../../app/(app)/_layout";
import { IntlProvider } from "../../src/i18n/IntlProvider";
import { renderWithApi } from "../support/renderWithApi";

describe("root layout and index", () => {
  const mode = process.env.EXPO_PUBLIC_API_MODE;
  afterEach(() => {
    process.env.EXPO_PUBLIC_API_MODE = mode;
  });

  it("renders the routed slot inside the providers", async () => {
    process.env.EXPO_PUBLIC_API_MODE = "mock";
    await render(<RootLayout />);
    expect(screen.getByTestId("slot")).toBeTruthy();
  });

  it("wraps the protected group's routes in the auth gate", async () => {
    process.env.EXPO_PUBLIC_API_MODE = "mock";
    const { ApiProvider } = require("../../src/api/ApiProvider");
    const { SessionProvider } = require("../../src/auth/SessionProvider");
    await render(
      <IntlProvider>
        <ApiProvider>
          <SessionProvider>
            <ProtectedLayout />
          </SessionProvider>
        </ApiProvider>
      </IntlProvider>,
    );
    expect(screen.getByTestId("slot")).toBeTruthy();
  });

  it("explains, instead of crashing, when the orchestrator client cannot be created", async () => {
    process.env.EXPO_PUBLIC_API_MODE = "real";
    jest.spyOn(require("../../src/api/select"), "selectApi").mockImplementation(() => {
      throw new Error("no client");
    });
    await render(<RootLayout />);
    expect(screen.queryByTestId("slot")).toBeNull();
    expect(screen.getByRole("alert")).toHaveTextContent("The orchestrator connection is not available in this build.");
  });

  it("shows the title, the tagline and the language picker from the catalog", async () => {
    // A pulsing indicator never settles `act`; reduced motion keeps the test to one frame.
    jest.spyOn(AccessibilityInfo, "isReduceMotionEnabled").mockResolvedValue(true);
    await renderWithApi(<Index />).view;
    await act(async () => {});
    expect(screen.getByRole("header", { name: "Micold" })).toBeTruthy();
    expect(screen.getByRole("radiogroup")).toBeTruthy();
  });
});

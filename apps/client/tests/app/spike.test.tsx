import React from "react";
import { render, screen } from "@testing-library/react-native";
import { PaperProvider } from "react-native-paper";
import SpikeRoute from "../../app/spike";
import { buildTheme } from "../../src/theme/buildTheme";

jest.mock("expo-router", () => {
  const { View } = require("react-native");
  const React = require("react");
  return { Redirect: () => React.createElement(View, { testID: "redirect-home" }) };
});

const original = process.env.EXPO_PUBLIC_API_MODE;
afterEach(() => {
  if (original === undefined) delete process.env.EXPO_PUBLIC_API_MODE;
  else process.env.EXPO_PUBLIC_API_MODE = original;
});

function renderRoute(mode: string | undefined) {
  if (mode === undefined) delete process.env.EXPO_PUBLIC_API_MODE;
  else process.env.EXPO_PUBLIC_API_MODE = mode;
  return render(
    <PaperProvider theme={buildTheme("light")}>
      <SpikeRoute />
    </PaperProvider>,
  );
}

describe("spike route", () => {
  it("renders the spike screen in mock mode", async () => {
    await renderRoute("mock");
    expect(screen.getByTestId("spike-screen")).toBeTruthy();
    expect(screen.queryByTestId("redirect-home")).toBeNull();
  });

  it.each([["real"], [undefined], ["production"]])("is absent (redirects home) when the mode is %s", async (mode) => {
    await renderRoute(mode);
    expect(screen.getByTestId("redirect-home")).toBeTruthy();
    expect(screen.queryByTestId("spike-screen")).toBeNull();
  });
});

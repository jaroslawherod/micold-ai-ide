import React from "react";
import { act, screen } from "@testing-library/react-native";
import { ConnectionBanner } from "../../src/components/ConnectionBanner";
import { localeStore } from "../../src/i18n/localeStore";
import { renderWithApi } from "../support/renderWithApi";

describe("ConnectionBanner", () => {
  beforeEach(() => localeStore.getState().setLocale("en"));

  it("shows nothing while connected", async () => {
    await renderWithApi(<ConnectionBanner />).view;
    expect(screen.queryByTestId("connection-banner")).toBeNull();
  });

  it("shows reconnecting with the attempt count, then offline, then clears", async () => {
    const { mock, view } = renderWithApi(<ConnectionBanner />);
    await view;
    await act(async () => mock.control.dropConnection());
    await act(async () => mock.control.dropConnection());
    expect(screen.getByRole("alert")).toHaveTextContent("Connection lost. Reconnecting (attempt 2)…");
    await act(async () => mock.control.goOffline());
    expect(screen.getByRole("alert")).toHaveTextContent("Offline. The orchestrator cannot be reached.");
    await act(async () => mock.control.restoreConnection());
    expect(screen.queryByTestId("connection-banner")).toBeNull();
  });

  it("speaks Polish", async () => {
    localeStore.getState().setLocale("pl");
    const { mock, view } = renderWithApi(<ConnectionBanner />);
    await view;
    await act(async () => mock.control.dropConnection());
    expect(screen.getByRole("alert")).toHaveTextContent("Utracono połączenie. Ponowne łączenie (próba 1)…");
  });
});

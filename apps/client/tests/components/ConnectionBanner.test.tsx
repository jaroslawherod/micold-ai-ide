import React from "react";
import { act, fireEvent, screen } from "@testing-library/react-native";
import { ConnectionBanner } from "../../src/components/ConnectionBanner";
import { localeStore } from "../../src/i18n/localeStore";
import { createMockApi } from "@micold/protocol/mock";
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
    expect(screen.getByRole("alert")).toHaveTextContent(/Cannot reach the orchestrator/);
    expect(screen.getByRole("alert")).toHaveTextContent(/Check that it is running, then try again\./);
    await act(async () => mock.control.restoreConnection());
    expect(screen.queryByTestId("connection-banner")).toBeNull();
  });

  it("offers Retry when unreachable, and it asks the API to connect again", async () => {
    const { mock, view } = renderWithApi(<ConnectionBanner />);
    await view;
    await act(async () => mock.control.goOffline());
    await act(async () => fireEvent.press(screen.getByRole("button", { name: "Retry" })));
    expect(screen.queryByTestId("connection-banner")).toBeNull();
  });

  it("calls Retry on the API object itself, as the class-based real API needs (#1732 visual pass)", async () => {
    const { api } = createMockApi();
    const offline = { kind: "offline" } as const;
    class Orchestrator {
      retried = 0;
      connection = { get: () => offline, subscribe: () => () => undefined };
      retry(): void {
        this.retried += 1;
      }
    }
    const orchestrator = new Orchestrator();
    const withClass = { ...api, orchestrator: Object.assign(Object.create(api.orchestrator), orchestrator, { retry: Orchestrator.prototype.retry }) };
    await renderWithApi(<ConnectionBanner />, { api: withClass, control: undefined as never }).view;
    fireEvent.press(screen.getByRole("button", { name: "Retry" }));
    expect(withClass.orchestrator.retried).toBe(1);
  });

  it("shows a translated connecting state", async () => {
    const { api } = createMockApi();
    const states = { kind: "connecting" } as const;
    const connecting = { ...api, orchestrator: { ...api.orchestrator, connection: { get: () => states, subscribe: () => () => undefined } } };
    await renderWithApi(<ConnectionBanner />, { api: connecting, control: undefined as never }).view;
    expect(screen.getByRole("alert")).toHaveTextContent("Connecting to the orchestrator…");
  });

  it("speaks Polish", async () => {
    localeStore.getState().setLocale("pl");
    const { mock, view } = renderWithApi(<ConnectionBanner />);
    await view;
    await act(async () => mock.control.dropConnection());
    expect(screen.getByRole("alert")).toHaveTextContent("Utracono połączenie. Ponowne łączenie (próba 1)…");
  });
});

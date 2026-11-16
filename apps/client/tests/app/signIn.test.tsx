import React from "react";
import { act, fireEvent, screen } from "@testing-library/react-native";
import { createMockApi } from "@micold/protocol/mock";
import { localeStore } from "../../src/i18n/localeStore";
import { renderWithApi, sessionProbe } from "../support/renderWithApi";

const mockReplace = jest.fn();
const mockParams: { returnTo?: string } = {};
jest.mock("expo-router", () => ({
  useRouter: () => ({ replace: mockReplace }),
  useLocalSearchParams: () => mockParams,
}));

import SignIn from "../../app/sign-in";

async function show(mock = createMockApi()) {
  const { view } = renderWithApi(<SignIn />, mock);
  await view;
  await act(async () => undefined);
  return mock;
}

describe("sign-in page", () => {
  beforeEach(() => {
    mockReplace.mockClear();
    delete mockParams.returnTo;
    localeStore.getState().setLocale("en");
  });

  it("lists exactly the configured providers, named by the provider", async () => {
    await show();
    expect(screen.getAllByRole("button").map((b) => b.props.accessibilityLabel)).toEqual(["Sign in with GitHub", "Sign in with Google"]);
    expect(screen.queryByText(/GitLab/)).toBeNull();
    expect(screen.getByRole("header", { name: "Sign in to Micold" })).toBeTruthy();
  });

  it("shows that sign-in is not set up when no provider is configured", async () => {
    const mock = createMockApi();
    mock.control.setProviders([{ id: "gitlab", name: "GitLab", configured: false }]);
    await show(mock);
    expect(screen.getByText(/Sign-in is not set up/)).toBeTruthy();
    expect(screen.queryAllByRole("button")).toEqual([]);
  });

  it("signs in and returns to the asked-for route", async () => {
    mockParams.returnTo = "/sessions/s1";
    await show();
    await fireEvent.press(screen.getByRole("button", { name: "Sign in with GitHub" }));
    expect(sessionProbe.signedIn).toBe(true);
    expect(mockReplace).toHaveBeenCalledWith("/sessions/s1");
  });

  it("goes home when no route was asked for", async () => {
    await show();
    await fireEvent.press(screen.getByRole("button", { name: "Sign in with Google" }));
    expect(mockReplace).toHaveBeenCalledWith("/");
  });

  it("shows a refusal with a retry, and no session", async () => {
    const mock = createMockApi();
    mock.control.setSignInOutcome("refused");
    await show(mock);
    await fireEvent.press(screen.getByRole("button", { name: "Sign in with GitHub" }));
    expect(screen.getByText("GitHub refused the sign-in. No session was started.")).toBeTruthy();
    expect(sessionProbe.signedIn).toBe(false);
    mock.control.setSignInOutcome("ok");
    await fireEvent.press(screen.getByRole("button", { name: "Try again" }));
    expect(sessionProbe.signedIn).toBe(true);
  });

  it("shows a cancellation", async () => {
    const mock = createMockApi();
    mock.control.setSignInOutcome("cancelled");
    await show(mock);
    await fireEvent.press(screen.getByRole("button", { name: "Sign in with Google" }));
    expect(screen.getByText("The sign-in with Google was cancelled.")).toBeTruthy();
    expect(mockReplace).not.toHaveBeenCalled();
  });

  it("offers a retry when the providers cannot be loaded", async () => {
    const mock = createMockApi();
    const list = jest.spyOn(mock.api.auth, "listProviders").mockRejectedValueOnce(new Error("down"));
    await show(mock);
    expect(screen.getByText("The sign-in options could not be loaded.")).toBeTruthy();
    await fireEvent.press(screen.getByRole("button", { name: "Try again" }));
    await act(async () => undefined);
    expect(list).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("button", { name: "Sign in with GitHub" })).toBeTruthy();
  });

  it("shows an error with a retry when the sign-in call itself fails", async () => {
    const mock = createMockApi();
    jest.spyOn(mock.api.auth, "signIn").mockRejectedValueOnce(new Error("network"));
    await show(mock);
    await fireEvent.press(screen.getByRole("button", { name: "Sign in with GitHub" }));
    expect(screen.getByText("The sign-in with GitHub failed. Check the connection and try again.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Sign in with GitHub" })).not.toBeDisabled();
    await fireEvent.press(screen.getByRole("button", { name: "Try again" }));
    expect(sessionProbe.signedIn).toBe(true);
  });

  it("takes every string from the catalog (Polish)", async () => {
    localeStore.getState().setLocale("pl");
    await show();
    expect(screen.getByRole("header", { name: "Zaloguj się do Micold" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Zaloguj przez GitHub" })).toBeTruthy();
  });
});

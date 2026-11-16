import React from "react";
import { act, render, screen } from "@testing-library/react-native";
import { Text } from "react-native";
import { createMockApi } from "@micold/protocol/mock";

const mockReplace = jest.fn();
let mockPathname = "/";
jest.mock("expo-router", () => ({
  usePathname: () => mockPathname,
  Redirect: ({ href }: { href: string }) => {
    mockReplace(href);
    return null;
  },
}));

import { ApiProvider } from "../../src/api/ApiProvider";
import { AuthGate } from "../../src/auth/AuthGate";
import { SessionProvider, useSession } from "../../src/auth/SessionProvider";
import { IntlProvider } from "../../src/i18n/IntlProvider";

let setSession: ((s: never) => void) | undefined;
function Setter() {
  setSession = useSession((s) => s.setSession) as never;
  return null;
}
function Probe() {
  const signedIn = useSession((s) => s.session !== null);
  return <Text>{signedIn ? "in" : "out"}</Text>;
}

function mount(mock = createMockApi()) {
  return {
    mock,
    view: render(
      <IntlProvider>
        <ApiProvider api={mock.api}>
          <SessionProvider>
            <Setter />
            <AuthGate>
              <Probe />
            </AuthGate>
          </SessionProvider>
        </ApiProvider>
      </IntlProvider>,
    ),
  };
}

describe("AuthGate", () => {
  beforeEach(() => mockReplace.mockClear());

  it("redirects a signed-out visitor to sign-in with the asked-for route", async () => {
    mockPathname = "/sessions/s1";
    await mount().view;
    expect(mockReplace).toHaveBeenCalledWith("/sign-in?returnTo=%2Fsessions%2Fs1");
    expect(screen.queryByText("out")).toBeNull();
  });

  it("does not redirect on the sign-in page", async () => {
    mockPathname = "/sign-in";
    await mount().view;
    expect(mockReplace).not.toHaveBeenCalled();
    expect(screen.getByText("out")).toBeTruthy();
  });

  it("returns to sign-in, keeping the open route, when the session expires mid-use", async () => {
    mockPathname = "/sessions/s2";
    const { mock, view } = mount();
    await view;
    const session = await mock.api.auth.signIn("github");
    await act(async () => setSession!(session as never));
    expect(screen.getByText("in")).toBeTruthy();
    mockReplace.mockClear();
    await act(async () => mock.control.expireSession());
    expect(screen.queryByText("in")).toBeNull();
    expect(screen.queryByText("out")).toBeNull();
    expect(mockReplace).toHaveBeenCalledWith("/sign-in?returnTo=%2Fsessions%2Fs2");
  });
});

describe("useSession", () => {
  it("needs a SessionProvider", async () => {
    jest.spyOn(console, "error").mockImplementation(() => undefined);
    await expect(render(<Probe />)).rejects.toThrow(/SessionProvider/);
  });
});

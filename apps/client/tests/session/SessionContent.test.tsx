// The content view (FR-014b, FR-014a): a snapshot read on open, translated empty / failed /
// truncated / refresh states, and the unreachable marker, in English and Polish.
import React from "react";
import { act, fireEvent, screen } from "@testing-library/react-native";
import { ApiError, sessionId, type ContentEntry } from "@micold/protocol";
import { createMockApi } from "@micold/protocol/mock";
import { SessionContent } from "../../src/features/session/SessionContent";
import { SnapshotProvider } from "../../src/features/sidebar/SnapshotProvider";
import { localeStore } from "../../src/i18n/localeStore";
import { renderWithApi } from "../support/renderWithApi";

async function open(read: (n: number) => Promise<ContentEntry[]>, id = "s0-0-0") {
  const mock = createMockApi();
  let calls = 0;
  mock.api.orchestrator.readSession = () => read(++calls);
  const { view } = renderWithApi(
    <SnapshotProvider>
      <SessionContent sessionId={sessionId(id)} />
    </SnapshotProvider>,
    mock,
  );
  await view;
  await act(async () => {});
  return mock;
}

describe("SessionContent", () => {
  beforeEach(() => localeStore.getState().setLocale("en"));
  const first = async (api: ReturnType<typeof createMockApi>) => (await api.api.orchestrator.getSnapshot()).sessions[0]!.id;

  it("shows the entries it read, and says when one was shortened", async () => {
    const id = await first(createMockApi());
    await open(async () => [
      { seq: 1, role: "user", kind: "text", text: "hello" },
      { seq: 2, role: "assistant", kind: "tool_call", text: "ls", truncated: true },
    ], id);
    expect(screen.getAllByTestId("content-entry")).toHaveLength(2);
    expect(screen.getByText("hello")).toBeTruthy();
    expect(screen.getByText("You")).toBeTruthy();
    expect(screen.getByText("Assistant · Tool call")).toBeTruthy();
    expect(screen.getByText("Shortened: the full text is not shown.")).toBeTruthy();
  });

  it("says so when there is no content", async () => {
    const id = await first(createMockApi());
    await open(async () => [], id);
    expect(screen.getByText("This session has no content yet.")).toBeTruthy();
  });

  it("shows a translated reason from the error code, never the server's text, and refresh retries", async () => {
    const id = await first(createMockApi());
    await open(async (n) => {
      if (n === 1) throw new ApiError("daemon_unreachable");
      return [{ seq: 1, role: "system", kind: "status", text: "back" }];
    }, id);
    expect(screen.getByText("The session content could not be loaded.")).toBeTruthy();
    expect(screen.getByText("The daemon holding this session cannot be reached.")).toBeTruthy();
    await act(async () => fireEvent.press(screen.getByRole("button", { name: "Refresh" })));
    expect(screen.getByText("back")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("speaks Polish", async () => {
    localeStore.getState().setLocale("pl");
    const id = await first(createMockApi());
    await open(async () => {
      throw new ApiError("not_found");
    }, id);
    expect(screen.getByText("Nie udało się wczytać treści sesji.")).toBeTruthy();
    expect(screen.getByText("Ta sesja już nie istnieje.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Odśwież" })).toBeTruthy();
  });

  it("marks a session whose daemon is unreachable", async () => {
    const mock = createMockApi();
    const snap = await mock.api.orchestrator.getSnapshot();
    const s = snap.sessions[0]!;
    mock.control.pushChange({ kind: "upsert", entity: { type: "session", value: { ...s, reachable: false } } });
    mock.api.orchestrator.readSession = async () => [];
    const { view } = renderWithApi(
      <SnapshotProvider>
        <SessionContent sessionId={s.id} />
      </SnapshotProvider>,
      mock,
    );
    await view;
    await act(async () => {});
    expect(screen.getByTestId("session-unreachable")).toHaveTextContent("Unreachable");
  });
});

import React from "react";
import { fireEvent, render, screen } from "@testing-library/react-native";
import { PaperProvider } from "react-native-paper";
import { SpikeScreen } from "../../src/spike/SpikeScreen";
import { buildTheme } from "../../src/theme/buildTheme";

function renderSpike() {
  return render(
    <PaperProvider theme={buildTheme("light")}>
      <SpikeScreen />
    </PaperProvider>,
  );
}

describe("SpikeScreen", () => {
  it("renders the transcript and records a measurement", async () => {
    await renderSpike();
    await fireEvent.press(screen.getByTestId("spike-run-transcript"));
    // No layout engine under test: the list reports its first layout by hand.
    await fireEvent(screen.getByTestId("spike-list-transcript"), "layout", { nativeEvent: { layout: { width: 400, height: 600 } } });
    expect(await screen.findByTestId("spike-result-transcript")).toHaveTextContent(/transcript: \d+(\.\d+)? ms/);
  });

  it("renders the diff and records a measurement", async () => {
    await renderSpike();
    await fireEvent.press(screen.getByTestId("spike-run-diff"));
    await fireEvent(screen.getByTestId("spike-list-diff"), "layout", { nativeEvent: { layout: { width: 400, height: 600 } } });
    expect(await screen.findByTestId("spike-result-diff")).toHaveTextContent(/diff: \d+(\.\d+)? ms/);
  });
});

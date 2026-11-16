import React from "react";
import { fireEvent, render, screen } from "@testing-library/react-native";
import { Text } from "react-native";
import { PaperProvider } from "react-native-paper";
import { LocalePicker } from "../../src/components/LocalePicker";
import { IntlProvider, useT } from "../../src/i18n/IntlProvider";
import { localeStore } from "../../src/i18n/localeStore";
import { buildTheme } from "../../src/theme/buildTheme";

function Heading() {
  const t = useT();
  return <Text>{t("app.tagline")}</Text>;
}

function renderPicker() {
  return render(
    <PaperProvider theme={buildTheme("light")}>
      <IntlProvider>
        <LocalePicker />
        <Heading />
      </IntlProvider>
    </PaperProvider>,
  );
}

describe("LocalePicker", () => {
  beforeEach(() => {
    localeStore.getState().setLocale("en");
  });

  it("is a group named by the catalog and lists every available locale", async () => {
    await renderPicker();
    expect(screen.getByRole("radiogroup", { name: "Language" })).toBeTruthy();
    for (const name of ["English", "Polski"]) expect(screen.getByText(name)).toBeTruthy();
  });

  it("switches the text without a reload and writes the preference", async () => {
    await renderPicker();
    expect(screen.getByText("Your coding sessions, in the browser.")).toBeTruthy();
    await fireEvent.press(screen.getByRole("radio", { name: "Polski" }));
    expect(screen.getByText("Twoje sesje kodowania, w przeglądarce.")).toBeTruthy();
    expect(screen.getByRole("radiogroup", { name: "Język" })).toBeTruthy();
    expect(window.localStorage.getItem("micold.locale")).toBe("pl");
  });

  it("marks the current locale as checked", async () => {
    await renderPicker();
    expect(screen.getByRole("radio", { name: "English" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "Polski" })).not.toBeChecked();
  });

  it("makes every option focusable for the keyboard", async () => {
    await renderPicker();
    for (const radio of screen.getAllByRole("radio")) {
      expect(radio.props.accessible).not.toBe(false);
      expect(radio.props.disabled).toBeFalsy();
    }
  });
});

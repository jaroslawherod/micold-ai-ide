import React, { useEffect, useMemo } from "react";
import { Slot } from "expo-router";
import { PaperProvider } from "react-native-paper";
import { ApiProvider } from "../src/api/ApiProvider";
import { SessionProvider } from "../src/auth/SessionProvider";
import { ConnectionBanner } from "../src/components/ConnectionBanner";
import { IntlProvider } from "../src/i18n/IntlProvider";
import { loadBundledFonts } from "../src/platform/fonts";
import { buildTheme } from "../src/theme/buildTheme";
import { useColorScheme } from "../src/theme/useColorScheme";

export default function RootLayout() {
  const scheme = useColorScheme();
  const theme = useMemo(() => buildTheme(scheme), [scheme]);
  useEffect(() => {
    void loadBundledFonts();
  }, []);
  return (
    <PaperProvider theme={theme}>
      <IntlProvider>
        <ApiProvider>
          <SessionProvider>
            <ConnectionBanner />
            <Slot />
          </SessionProvider>
        </ApiProvider>
      </IntlProvider>
    </PaperProvider>
  );
}

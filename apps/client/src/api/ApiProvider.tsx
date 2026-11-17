// Gives the app its `Api` (contracts/orchestrator-api.md): components reach the orchestrator only
// through `useApi()`, never the mock or a socket. Without a built real client the app shows why.
import React, { createContext, useContext, useState, type ReactNode } from "react";
import { View } from "react-native";
import { Text } from "react-native-paper";
import type { Api } from "@micold/protocol";
import { useT } from "../i18n/IntlProvider";
import { spacing } from "../theme/tokens.generated";
import { selectApi } from "./select";

const ApiContext = createContext<Api | undefined>(undefined);

function choose(): Api | Error {
  try {
    return selectApi();
  } catch (err) {
    return err as Error;
  }
}

export function ApiProvider({ api, children }: { api?: Api; children: ReactNode }) {
  const [chosen] = useState<Api | Error>(() => api ?? choose());
  const t = useT();
  if (chosen instanceof Error) {
    return (
      <View style={{ padding: spacing.lg }}>
        <Text accessibilityRole="alert" variant="bodyLarge">
          {t("api.unavailable")}
        </Text>
      </View>
    );
  }
  return <ApiContext.Provider value={chosen}>{children}</ApiContext.Provider>;
}

export function useApi(): Api {
  const api = useContext(ApiContext);
  if (!api) throw new Error("useApi needs an ApiProvider above it");
  return api;
}

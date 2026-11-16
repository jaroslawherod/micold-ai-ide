// Shows the connection state (FR-009): nothing while connected, the attempt count while
// reconnecting, and offline. Announced politely to assistive technology.
import React, { useSyncExternalStore } from "react";
import { View } from "react-native";
import { Text, useTheme } from "react-native-paper";
import { assertNever, type ConnectionState } from "@micold/protocol";
import { useApi } from "../api/ApiProvider";
import { useT } from "../i18n/IntlProvider";
import { spacing } from "../theme/tokens.generated";

export function ConnectionBanner() {
  const t = useT();
  const theme = useTheme();
  const { connection } = useApi().orchestrator;
  const state: ConnectionState = useSyncExternalStore(connection.subscribe, connection.get, connection.get);
  let text: string;
  switch (state.kind) {
    case "connected":
      return null;
    case "reconnecting":
      text = t("connection.reconnecting", { attempt: state.attempt });
      break;
    case "offline":
      text = t("connection.offline");
      break;
    default:
      return assertNever(state);
  }
  return (
    <View
      accessible
      accessibilityRole="alert"
      accessibilityLiveRegion="polite"
      testID="connection-banner"
      style={{ paddingVertical: spacing.sm, paddingHorizontal: spacing.md, backgroundColor: theme.colors.errorContainer }}
    >
      <Text style={{ color: theme.colors.onErrorContainer }}>{text}</Text>
    </View>
  );
}
